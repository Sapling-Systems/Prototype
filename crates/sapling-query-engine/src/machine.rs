use std::{
  collections::{HashMap, VecDeque},
  fmt::Debug,
};

use sapling_data_model::{Fact, Query, Subject};

use crate::{
  Database, ExplainConstraintEvaluation, ExplainFactEvent, ExplainResult, QueryEngine,
  SharedVariableAllocator, SharedVariableBank, System,
  database::match_subject,
  explain::{
    EvaluationType, ExplainConstraintEvaluationOutcome, ExplainConstraintEvaluationOutcomeReason,
  },
  instructions::UnificationInstruction,
  iterators::NaiveFactIterator,
};

macro_rules! tracing_constraint_check {
  ($self:ident, $frame:ident, $created_trace_event:ident, $variant:ident, $target:expr, $fact_property:expr, $ty:expr) => {{
    if let Some(constraint) = $frame.tracing {
      if $target.is_some() {
        $self
          .explain_result
          .fact_events
          .push(ExplainFactEvent::EvaluatingConstraint {
            constraint_id: constraint,
            ty: $ty,
            evaluation: ExplainConstraintEvaluation::$variant {
              target: $target,
              actual: $fact_property.clone(),
              operator: System::CORE_OPERATOR_EQ.clone(),
            },
            outcome: ExplainConstraintEvaluationOutcome::Passed,
          });
        $created_trace_event = true;
      }
    }
  }};
}

pub struct AbstractMachine<'a> {
  pub instructions: Vec<UnificationInstruction>,
  fallback_instruction_pointer: usize,
  database: &'a Database,
  query_engine: &'a QueryEngine,
  stack: Vec<SearchFrame<'a>>,
  yielded: VecDeque<FoundFact<'a>>,
  follow_evaluated_subjects: bool,
  variable_bank: SharedVariableBank,
  variable_allocator: SharedVariableAllocator,
  pub log_instructions: bool,
  pub explain_result: ExplainResult,
  explain_enabled: bool,
}

#[derive(Clone, Debug)]
pub struct FoundFact<'a> {
  pub fact: &'a Fact,
  pub fact_index: usize,
  pub subject_binding: Option<Subject>,
}

impl<'a> AbstractMachine<'a> {
  pub fn new(
    instructions: Vec<UnificationInstruction>,
    database: &'a Database,
    query_engine: &'a QueryEngine,
    variable_bank: SharedVariableBank,
    variable_allocator: SharedVariableAllocator,
  ) -> Self {
    Self {
      database,
      query_engine,
      follow_evaluated_subjects: true,
      fallback_instruction_pointer: 0,
      yielded: VecDeque::new(),
      stack: Vec::new(),
      log_instructions: false,
      explain_enabled: false,
      explain_result: ExplainResult {
        constraints: vec![],
        subject: None,
        fact_events: vec![],
        instruction: instructions.clone(),
        variables: HashMap::new(),
      },
      instructions,
      variable_bank,
      variable_allocator,
    }
  }

  pub fn reset_machine(&mut self) {
    self.fallback_instruction_pointer = 0;
    self.yielded.clear();
    self.stack.clear();
    self.variable_bank.reset();
  }

  fn exhaust_frame(&mut self) -> bool {
    if self.stack.is_empty() || self.stack.len() == 1 {
      return false;
    }

    if self.log_instructions {
      println!("  => Exhausting frame");
      println!("=============================BEFORE EXHAUST===================");
      println!("    => stack size: {}", self.stack.len());
      self.variable_bank.debug_print();
      println!("=============================================================");
    }
    if let Some(mut frame) = self.stack.pop() {
      frame.before_drop(&self.variable_bank);
    }

    if let Some(frame) = self.stack.last_mut() {
      frame.reset(&self.variable_bank);
    }

    if self.log_instructions {
      println!("  => Exhausting frame");
      println!("=============================AFTER EXHAUST===================");
      println!("    => stack size: {}", self.stack.len());
      self.variable_bank.debug_print();
      println!("=============================================================");
    }
    true
  }

  fn exhaust_stack_to_continue(&mut self) {
    let last_continue_marker = self
      .stack
      .iter()
      .rposition(|frame| frame.continue_marker)
      .unwrap_or(0);

    if self.log_instructions {
      println!("=============================BEFORE===================");
      println!("    => stack size: {}", self.stack.len());
      self.variable_bank.debug_print();
      println!("======================================================");
    }

    let drained = self.stack.drain((last_continue_marker + 1)..);
    for mut frame in drained {
      frame.before_drop(&self.variable_bank);
    }

    self.stack[last_continue_marker].reset(&self.variable_bank);

    if self.log_instructions {
      println!("=============================BEFORE===================");
      println!("    => stack size: {}", self.stack.len());
      self.variable_bank.debug_print();
      println!("======================================================");
    }
  }

  fn unwind_stack(&mut self) -> bool {
    while self
      .stack
      .last()
      .map(|f| f.current_investigated_fact.is_none())
      .unwrap_or(false)
    {
      if !self.exhaust_frame() {
        return false;
      }
    }

    true
  }

  fn step(&mut self) -> bool {
    let mut instruction_index = self
      .stack
      .last_mut()
      .map(|f| f.current_instruction_index)
      .unwrap_or(self.fallback_instruction_pointer);
    if instruction_index >= self.instructions.len() {
      if self.log_instructions {
        println!("  => EOF, exhaust to continue");
      }
      self.exhaust_stack_to_continue();
      if self.log_instructions {
        println!(
          "  => 0: {}",
          self
            .variable_bank
            .get(0)
            .and_then(|x| System::get_subject_name(self.database, &x))
            .unwrap_or("-".to_string())
        );
      }

      if let Some(frame) = self.stack.last_mut() {
        instruction_index = frame.start_instruction_index;
      } else {
        instruction_index = 1 + self.fallback_instruction_pointer;
      }
    }

    if !self.unwind_stack() {
      return false;
    }

    // Advance instruction pointer
    if let Some(current_frame) = self.stack.last_mut() {
      current_frame.current_instruction_index += 1;
    }

    // Handle instructions
    let instruction = self
      .instructions
      .get(instruction_index)
      .expect("Out of bounds instruction");

    let mut reset_frame = false;
    let mut created_trace_event = false;

    if self.log_instructions {
      println!(
        "Instruction: {:?} with {} and 0: {}",
        instruction,
        if let Some(fact) = self
          .stack
          .last_mut()
          .and_then(|frame| frame.current_investigated_fact.as_ref())
        {
          System::get_human_readable_fact(self.database, fact.fact)
        } else {
          "None".to_string()
        },
        self
          .variable_bank
          .get(0)
          .and_then(|x| System::get_subject_name(self.database, &x))
          .unwrap_or_default()
      );
    }

    match instruction {
      UnificationInstruction::DebugComment { .. } => {}
      // Simply allocates a new frame on the stack
      UnificationInstruction::AllocateFrame { size } => {
        self.variable_bank.push_checkpoint();

        let new_frame =
          SearchFrame::new_static(self.database, instruction_index + 1, self.stack.is_empty());
        self.stack.push(new_frame);
      }
      UnificationInstruction::AllocateFact {
        fact_index,
        reset_address,
      } => {
        let fact = self
          .database
          .get_fact(*fact_index)
          .expect("Fact does not exist");

        let found_fact = FoundFact {
          fact,
          fact_index: *fact_index,
          subject_binding: None,
        };

        self.variable_bank.push_checkpoint();
        let new_frame = SearchFrame::new_constant_frame(
          found_fact,
          self.stack.is_empty(),
          reset_address.unwrap_or(instruction_index + 1),
        );
        self.stack.push(new_frame);
      }

      // Yielding
      UnificationInstruction::MaybeYield => {
        let frame = self.stack.last_mut().unwrap();
        if let Some(fact) = frame.current_investigated_fact.clone() {
          frame.maybe_yielded.push(fact)
        }
      }
      UnificationInstruction::YieldAll => self.yielded.extend(
        self
          .stack
          .iter_mut()
          .flat_map(|frame| frame.maybe_yielded.drain(..)),
      ),

      // Unification
      UnificationInstruction::UnifySubject { variable } => {
        let frame = self.stack.last_mut().unwrap();
        let fact = frame.current_investigated_fact.as_ref().unwrap().fact;
        let current_variable = self.variable_bank.get(*variable);

        tracing_constraint_check!(
          self,
          frame,
          created_trace_event,
          Subject,
          current_variable,
          fact.subject.subject,
          EvaluationType::Unification
        );

        let direct_match =
          !fact.subject.evaluated && self.variable_bank.unify(*variable, &fact.subject.subject);

        if direct_match {
        } else if fact.subject.evaluated && self.follow_evaluated_subjects {
          let mut machine = self.query_engine.query(
            self.database,
            &Query {
              subject: fact.subject.subject.clone(),
              evaluated: fact.subject.evaluated,
              meta: None,
              property: None,
            },
            self.variable_bank.clone(),
            self.variable_allocator.clone(),
          );
          machine.follow_evaluated_subjects = false;

          if self.variable_bank.get(*variable).is_none() {
            let new_frame = SearchFrame::new_subject_unification(
              machine,
              instruction_index + 1,
              Some(frame),
              *variable,
              frame.continue_marker,
              &self.variable_bank,
            );
            self.stack.push(new_frame);
          } else {
            // If variable is already bound we can simply this to an check instructions and
            // just look if the underlying subject query would yield anything that
            // unifies with the binding
            let checkpoint_id = self.variable_bank.push_checkpoint();
            let matching_subject = machine.find(|inner_fact| {
              let unifies = self
                .variable_bank
                .unify(*variable, &inner_fact.fact.subject.subject);
              !inner_fact.fact.subject.evaluated && unifies
            });
            self.variable_bank.truncate_checkpoint(checkpoint_id);

            if let Some(matching_subject) = matching_subject {
              let fact = frame.current_investigated_fact.as_mut().unwrap();
              fact.subject_binding = Some(matching_subject.fact.subject.subject.clone());
            } else {
              reset_frame = true;
            }
          }
        } else {
          reset_frame = true;
        }
      }
      UnificationInstruction::UnifyProperty { variable } => {
        let frame = self.stack.last_mut().unwrap();
        let fact = frame.current_investigated_fact.as_ref().unwrap().fact;

        let current_variable = self.variable_bank.get(*variable);

        tracing_constraint_check!(
          self,
          frame,
          created_trace_event,
          Property,
          current_variable,
          fact.property.subject,
          EvaluationType::Unification
        );

        if !self.variable_bank.unify(*variable, &fact.value.subject) {
          reset_frame = true;
        }
      }
      UnificationInstruction::UnifyValue { variable } => {
        let frame = self.stack.last_mut().unwrap();
        let fact = frame.current_investigated_fact.as_ref().unwrap().fact;

        let current_variable = self.variable_bank.get(*variable);

        tracing_constraint_check!(
          self,
          frame,
          created_trace_event,
          Value,
          current_variable,
          fact.value.subject,
          EvaluationType::Unification
        );

        if !self.variable_bank.unify(*variable, &fact.value.subject) {
          reset_frame = true;
        }
      }

      // Simple checks against the current fact
      UnificationInstruction::CheckSubject { subject } => {
        let frame = self.stack.last_mut().unwrap();
        let fact = frame.current_investigated_fact.as_ref().unwrap().fact;

        tracing_constraint_check!(
          self,
          frame,
          created_trace_event,
          Subject,
          Some(subject.clone()),
          fact.subject.subject,
          EvaluationType::Check
        );

        let direct_match = match_subject(subject, &fact.subject.subject);

        if direct_match {
        } else if fact.subject.evaluated && self.follow_evaluated_subjects {
          let checkpoint_id = self.variable_bank.push_checkpoint();

          let mut machine = self.query_engine.query(
            self.database,
            &Query {
              subject: fact.subject.subject.clone(),
              evaluated: fact.subject.evaluated,
              meta: None,
              property: None,
            },
            self.variable_bank.clone(),
            self.variable_allocator.clone(),
          );
          machine.follow_evaluated_subjects = self.follow_evaluated_subjects;

          let evalutes_to_expected_subject = machine.any(|inner_fact| {
            !inner_fact.fact.subject.evaluated
              && match_subject(&inner_fact.fact.subject.subject, subject)
          });
          self.variable_bank.truncate_checkpoint(checkpoint_id);

          if !evalutes_to_expected_subject {
            reset_frame = true;
          } else {
            let fact = frame.current_investigated_fact.as_mut().unwrap();
            fact.subject_binding = Some(subject.clone());
          }
        } else {
          reset_frame = true;
        }
      }
      UnificationInstruction::CheckProperty { property } => {
        let frame = self.stack.last_mut().unwrap();
        let fact = frame.current_investigated_fact.as_ref().unwrap().fact;

        tracing_constraint_check!(
          self,
          frame,
          created_trace_event,
          Property,
          Some(property.clone()),
          fact.property.subject,
          EvaluationType::Check
        );

        if !match_subject(property, &fact.property.subject) {
          reset_frame = true;
        }
      }
      UnificationInstruction::CheckPropertyConstAnyInteger => {
        let frame = self.stack.last_mut().unwrap();
        let fact = frame.current_investigated_fact.as_ref().unwrap().fact;

        let non_value: Option<Subject> = None;

        tracing_constraint_check!(
          self,
          frame,
          created_trace_event,
          Property,
          non_value,
          fact.property.subject,
          EvaluationType::Check
        );

        match fact.property.subject {
          Subject::Integer { .. } => {}
          _ => {
            reset_frame = true;
          }
        }
      }
      UnificationInstruction::CheckValue { value, property } => {
        let frame = self.stack.last_mut().unwrap();
        let fact = frame.current_investigated_fact.as_ref().unwrap().fact;

        tracing_constraint_check!(
          self,
          frame,
          created_trace_event,
          Value,
          Some(value.clone()),
          fact.value.subject,
          EvaluationType::Check
        );

        let direct_match = match_subject(value, &fact.value.subject);
        let property_match = match (&fact.value.property, property) {
          (None, None) => true,
          (Some(a), Some(b)) if match_subject(a, b) => true,
          _ => false,
        };

        // Simple case, we have a direct match of the value as well as property
        if direct_match && property_match {
        } else if fact.value.evaluated || fact.value.property.is_some() {
          let mut machine = self.query_engine.query(
            self.database,
            &Query {
              subject: fact.value.subject.clone(),
              evaluated: fact.value.evaluated,
              meta: None,
              property: fact.value.property.clone(),
            },
            self.variable_bank.clone(),
            self.variable_allocator.clone(),
          );
          machine.follow_evaluated_subjects = self.follow_evaluated_subjects;

          println!(
            "Executing sub-query for fact: {}",
            System::get_human_readable_fact(self.database, fact)
          );
          let new_frame = SearchFrame::new_sub_query(machine, instruction_index, false);
          self.stack.push(new_frame);
        } else {
          reset_frame = true;
        }
      }
      UnificationInstruction::CheckOperator { operator } => {
        let frame = self.stack.last_mut().unwrap();
        let fact = frame.current_investigated_fact.as_ref().unwrap().fact;

        tracing_constraint_check!(
          self,
          frame,
          created_trace_event,
          Operator,
          Some(operator.clone()),
          fact.operator,
          EvaluationType::Check
        );

        if !match_subject(operator, &fact.operator) {
          reset_frame = true;
        }
      }
      UnificationInstruction::CheckMeta { skip_system } => {
        let frame = self.stack.last_mut().unwrap();
        let fact = frame.current_investigated_fact.as_ref().unwrap().fact;

        if *skip_system && match_subject(&fact.meta, &System::CORE_META) {
          reset_frame = true;
        }
      }
      UnificationInstruction::SkipSubject { subject } => {
        let frame = self.stack.last_mut().unwrap();
        let fact = frame.current_investigated_fact.as_ref().unwrap().fact;
        let direct_match = match_subject(subject, &fact.subject.subject);
        if direct_match {
          reset_frame = true;
        }
      }
      // Tracing
      UnificationInstruction::TraceConstraintCreate {
        constraint,
        fact_index,
      } => {
        self.explain_enabled = true;
        self
          .explain_result
          .constraints
          .push((*constraint, *fact_index));
        self.fallback_instruction_pointer += 1;
      }
      UnificationInstruction::BindVariable { variable, binding } => {
        if *variable == 0 {
          self.explain_result.subject = Some(binding.clone());
        }
        self.variable_bank.bind(*variable, binding);
      }
      UnificationInstruction::TraceStartFact { fact, constraint } => {
        let frame = self.stack.last_mut().unwrap();
        if frame.current_investigated_fact.as_ref().unwrap().fact_index == *fact {
          frame.tracing = Some(*constraint);
          self
            .explain_result
            .fact_events
            .push(ExplainFactEvent::EvaluatingExpectedFact {
              constraint_id: *constraint,
              fact_id: *fact,
            });
        }
      }
      UnificationInstruction::TraceSubQuery { query, variable } => {
        let frame = self.stack.last_mut().unwrap();
        if let Some(constraint) = frame.tracing {
          let variable = self.variable_bank.get(*variable);
          self
            .explain_result
            .fact_events
            .push(ExplainFactEvent::EvaluatingSubQuery {
              constraint_id: constraint,
              target_query: query.clone(),
              target: variable.unwrap(),
              outcome: ExplainConstraintEvaluationOutcome::Passed,
            });
          frame.waiting_for_subquery_trace = true;
        }
      }
      UnificationInstruction::TraceLogYield {
        constraint,
        fact_index,
      } => {
        let yielded = self
          .stack
          .iter()
          .flat_map(|frame| frame.maybe_yielded.iter())
          .find(|yielded| yielded.fact_index == *fact_index);

        if let Some(yielded) = yielded {
          self
            .explain_result
            .fact_events
            .push(ExplainFactEvent::YieldingFact {
              constraint_id: *constraint,
              fact_id: *fact_index,
              subject_variable: yielded.subject_binding.clone(),
            });
        }
      }
    }

    self.capture_explain_variables();

    if let Some(frame) = self.stack.last_mut() {
      if reset_frame {
        if let Some(last_event) = self.explain_result.fact_events.last_mut()
          && (created_trace_event || frame.waiting_for_subquery_trace)
        {
          last_event.update_outcome(ExplainConstraintEvaluationOutcome::Rejected(
            ExplainConstraintEvaluationOutcomeReason::NotFound,
          ));
          frame.waiting_for_subquery_trace = false;
        }

        if self.log_instructions {
          println!(
            "  => Resetting frame 0: {}",
            self
              .variable_bank
              .get(0)
              .and_then(|x| System::get_subject_name(self.database, &x))
              .unwrap_or("-".to_string())
          );
        }
        frame.reset(&self.variable_bank);
        if self.log_instructions {
          println!(
            "  => Post Resetting frame 0: {}",
            self
              .variable_bank
              .get(0)
              .and_then(|x| System::get_subject_name(self.database, &x))
              .unwrap_or("-".to_string())
          );
        }
      }
    } else {
      return true;
    }

    if !self.unwind_stack() {
      return true;
    }

    if self.stack.is_empty() {
      return false;
    }

    true
  }

  fn capture_explain_variables(&mut self) {
    if self.explain_enabled {
      let subject_map = self.variable_allocator.get_subject_map();
      for (subject_id, variable) in subject_map {
        let subject_name =
          System::get_subject_name(self.database, &Subject::Static { uuid: subject_id });
        if let Some(variable_value) = self.variable_bank.get(variable) {
          self.explain_result.variables.insert(
            subject_name.unwrap_or_else(|| "-".to_string()),
            variable_value,
          );
        }
      }
    }
  }

  pub fn execute_until_yield(&mut self) -> Option<FoundFact<'a>> {
    if let Some(fact) = self.yielded.pop_front() {
      return Some(fact);
    }

    while self.step() {
      if let Some(fact) = self.yielded.pop_front() {
        return Some(fact);
      }
    }

    None
  }
}

impl<'a> Iterator for AbstractMachine<'a> {
  type Item = FoundFact<'a>;

  fn next(&mut self) -> Option<Self::Item> {
    self.execute_until_yield()
  }
}

#[derive(Debug)]
pub(crate) struct SearchFrame<'a> {
  tracing: Option<usize>,
  start_instruction_index: usize,
  current_instruction_index: usize,
  state: FrameState<'a>,
  maybe_yielded: Vec<FoundFact<'a>>,
  current_investigated_fact: Option<FoundFact<'a>>,
  debug: Option<Subject>,
  continue_marker: bool,
  waiting_for_subquery_trace: bool,
}

impl<'a> SearchFrame<'a> {
  pub fn new_static(
    database: &'a Database,
    start_instruction_index: usize,
    continue_marker: bool,
  ) -> Self {
    let mut iterator = database.iter_naive_facts();
    let current_investigated_fact = iterator.next().map(|(fact_index, fact)| FoundFact {
      fact,
      fact_index,
      subject_binding: None,
    });

    let me = Self {
      tracing: None,
      start_instruction_index,
      waiting_for_subquery_trace: false,
      current_instruction_index: start_instruction_index,
      state: FrameState::Static { iterator },
      maybe_yielded: Vec::new(),
      current_investigated_fact,
      continue_marker,
      debug: None,
    };

    me
  }

  pub fn new_sub_query(
    mut machine: AbstractMachine<'a>,
    start_instruction_index: usize,
    continue_marker: bool,
  ) -> Self {
    let bank_checkpoint_id = machine.variable_bank.push_checkpoint();
    let current_investigated_fact = machine.next();

    let me = Self {
      tracing: None,
      start_instruction_index,
      waiting_for_subquery_trace: false,
      continue_marker,
      debug: None,
      current_instruction_index: start_instruction_index,
      state: FrameState::SubQuery {
        machine,
        bank_checkpoint_id,
      },
      maybe_yielded: Vec::new(),
      current_investigated_fact,
    };

    me
  }

  pub fn new_subject_unification(
    mut machine: AbstractMachine<'a>,
    start_instruction_index: usize,
    previous_frame: Option<&SearchFrame<'a>>,
    variable: usize,
    continue_marker: bool,
    bank: &SharedVariableBank,
  ) -> Self {
    let bank_checkpoint_id = bank.push_checkpoint();
    let current_investigated_fact =
      previous_frame.and_then(|frame| frame.current_investigated_fact.clone());

    machine.follow_evaluated_subjects = false;

    let mut me = Self {
      tracing: None,
      waiting_for_subquery_trace: false,
      continue_marker,
      start_instruction_index,
      current_instruction_index: start_instruction_index,
      debug: None,
      state: FrameState::SubjectUnification {
        machine,
        variable,
        bank_checkpoint_id,
      },
      maybe_yielded: Vec::new(),
      current_investigated_fact,
    };

    me.reset(bank);

    me
  }

  pub fn new_constant_frame(
    fact: FoundFact<'a>,
    continue_marker: bool,
    start_instruction_index: usize,
  ) -> Self {
    Self {
      tracing: None,
      waiting_for_subquery_trace: false,
      continue_marker,
      start_instruction_index,
      current_instruction_index: start_instruction_index,
      debug: None,
      state: FrameState::Constant,
      maybe_yielded: Vec::new(),
      current_investigated_fact: Some(fact),
    }
  }
}

enum FrameState<'a> {
  Constant,
  Static {
    iterator: NaiveFactIterator<'a>,
  },
  SubQuery {
    machine: AbstractMachine<'a>,
    bank_checkpoint_id: usize,
  },
  SubjectUnification {
    machine: AbstractMachine<'a>,
    variable: usize,
    bank_checkpoint_id: usize,
  },
}

impl<'a> std::fmt::Debug for FrameState<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      FrameState::Constant { .. } => f.debug_struct("FrameState::Constant").finish(),
      FrameState::Static { .. } => f.debug_struct("FrameState::Static").finish(),
      FrameState::SubQuery { .. } => f.debug_struct("FrameState::SubQuery").finish(),
      FrameState::SubjectUnification { variable, .. } => f
        .debug_struct("FrameState::SubjectUnification")
        .field("variable", variable)
        .finish(),
    }
  }
}

impl<'a> SearchFrame<'a> {
  fn before_drop(&mut self, bank: &SharedVariableBank) {
    match self.state {
      FrameState::Constant => {
        bank.pop_checkpoint();
      }
      FrameState::Static { .. } => {
        bank.pop_checkpoint();
      }
      FrameState::SubQuery {
        bank_checkpoint_id, ..
      } => {
        bank.truncate_checkpoint(bank_checkpoint_id);
      }
      FrameState::SubjectUnification {
        bank_checkpoint_id,
        variable,
        ..
      } => {
        bank.truncate_checkpoint(bank_checkpoint_id);
        bank.unbind(variable);
      }
    }
  }

  fn reset(&mut self, bank: &SharedVariableBank) {
    // Advanced iterator
    match &mut self.state {
      FrameState::Constant => {
        self.current_investigated_fact = None;
      }
      FrameState::SubQuery { machine, .. } => {
        self.current_investigated_fact = machine.next();
      }
      FrameState::SubjectUnification {
        machine, variable, ..
      } => {
        if let Some(next_subject_fact) = machine.find(|f| !f.fact.subject.evaluated) {
          let subject = next_subject_fact.fact.subject.subject.clone();
          bank.bind(*variable, &subject);
          let current_fact = self.current_investigated_fact.as_mut().unwrap();
          current_fact.subject_binding = Some(subject)
        } else {
          // Signals that frame can be exhausted.
          self.current_investigated_fact = None;
        }
      }
      FrameState::Static { iterator } => {
        self.current_investigated_fact = iterator.next().map(|(fact_index, fact)| FoundFact {
          fact,
          fact_index,
          subject_binding: None,
        });
        bank.trail_checkpoint();
      }
    }

    // Reset tracing
    self.tracing = None;

    // Reset instruction pointer to the start
    self.current_instruction_index = self.start_instruction_index;

    // Clear the yield
    self.maybe_yielded.clear();
  }
}
