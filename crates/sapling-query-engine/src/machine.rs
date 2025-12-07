use std::{
  collections::{HashMap, VecDeque},
  fmt::Debug,
};

use sapling_data_model::{Fact, Query, Subject};

use crate::{
  Database, ExplainConstraintEvaluation, ExplainFactEvent, ExplainResult, QueryEngine, System,
  database::match_subject,
  explain::{ExplainConstraintEvaluationOutcome, ExplainConstraintEvaluationOutcomeReason},
  instructions::UnificationInstruction,
  iterators::NaiveFactIterator,
};

macro_rules! tracing_constraint_check {
  ($self:ident, $frame:ident, $created_trace_event:ident, $variant:ident, $target:ident, $fact_property:expr) => {{
    if let Some(constraint) = $frame.tracing {
      $self
        .explain_result
        .fact_events
        .push(ExplainFactEvent::EvaluatingConstraint {
          constraint_id: constraint,
          evaluation: ExplainConstraintEvaluation::$variant {
            target: $target.clone(),
            actual: $fact_property.clone(),
            operator: System::CORE_OPERATOR_EQ.clone(),
          },
          outcome: ExplainConstraintEvaluationOutcome::Passed,
        });
      $created_trace_event = true;
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
  pub explain_result: ExplainResult,
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
  ) -> Self {
    Self {
      database,
      query_engine,
      follow_evaluated_subjects: true,
      fallback_instruction_pointer: 0,
      yielded: VecDeque::new(),
      stack: Vec::new(),
      explain_result: ExplainResult {
        constraints: vec![],
        subject: None,
        fact_events: vec![],
        instruction: instructions.clone(),
      },
      instructions,
    }
  }

  fn exhaust_frame(&mut self) -> bool {
    if self.stack.is_empty() || self.stack.len() == 1 {
      return false;
    }

    self.stack.pop();
    if let Some(frame) = self.stack.last_mut() {
      frame.reset();
    }
    true
  }

  fn exhaust_stack_to_continue(&mut self) {
    let last_continue_marker = self
      .stack
      .iter()
      .rposition(|frame| frame.continue_marker)
      .unwrap_or(0);
    self.stack.truncate(last_continue_marker + 1);
    self.stack[last_continue_marker].reset();
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
      self.exhaust_stack_to_continue();
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

    match instruction {
      // Simply allocates a new frame on the stack
      UnificationInstruction::AllocateFrame { size } => {
        let new_frame = SearchFrame::new_static(
          self.database,
          instruction_index + 1,
          self.stack.last(),
          *size,
          self.stack.is_empty(),
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

        let current_variable = frame
          .variable_bindings
          .get(*variable)
          .and_then(|b| {
            if let VariableBinding::Bound(s) = b {
              Some(s.clone())
            } else {
              None
            }
          })
          .unwrap_or_else(|| fact.subject.subject.clone());

        tracing_constraint_check!(
          self,
          frame,
          created_trace_event,
          Subject,
          current_variable,
          fact.subject.subject
        );

        let direct_match = !fact.subject.evaluated && frame.unify(*variable, &fact.subject.subject);

        if direct_match {
        } else if fact.subject.evaluated && self.follow_evaluated_subjects {
          let mut machine = self.query_engine.query(&Query {
            subject: fact.subject.subject.clone(),
            evaluated: fact.subject.evaluated,
            meta: None,
            property: None,
          });
          machine.follow_evaluated_subjects = false;

          if matches!(frame.variable_bindings[*variable], VariableBinding::Unbound) {
            let new_frame = SearchFrame::new_subject_unification(
              machine,
              instruction_index + 1,
              Some(frame),
              64,
              *variable,
              frame.continue_marker,
            );
            self.stack.push(new_frame);
          } else {
            // If variable is already bound we can simply this to an check instructions and
            // just look if the underlying subject query would yield anything that
            // unifies with the binding
            let matching_subject = machine.find(|inner_fact| {
              let unifies = frame.unify(*variable, &inner_fact.fact.subject.subject);
              !inner_fact.fact.subject.evaluated && unifies
            });

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

        let current_variable = frame
          .variable_bindings
          .get(*variable)
          .and_then(|b| {
            if let VariableBinding::Bound(s) = b {
              Some(s.clone())
            } else {
              None
            }
          })
          .unwrap_or_else(|| fact.property.subject.clone());

        tracing_constraint_check!(
          self,
          frame,
          created_trace_event,
          Property,
          current_variable,
          fact.property.subject
        );

        if !frame.unify(*variable, &fact.property.subject) {
          reset_frame = true;
        }
      }
      UnificationInstruction::UnifyValue { variable } => {
        let frame = self.stack.last_mut().unwrap();
        let fact = frame.current_investigated_fact.as_ref().unwrap().fact;

        let current_variable = frame
          .variable_bindings
          .get(*variable)
          .and_then(|b| {
            if let VariableBinding::Bound(s) = b {
              Some(s.clone())
            } else {
              None
            }
          })
          .unwrap_or_else(|| fact.value.subject.clone());

        tracing_constraint_check!(
          self,
          frame,
          created_trace_event,
          Value,
          current_variable,
          fact.value.subject
        );

        if !frame.unify(*variable, &fact.value.subject) {
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
          subject,
          fact.subject.subject
        );

        let direct_match = match_subject(subject, &fact.subject.subject);

        if direct_match {
        } else if fact.subject.evaluated && self.follow_evaluated_subjects {
          let mut machine = self.query_engine.query(&Query {
            subject: fact.subject.subject.clone(),
            evaluated: fact.subject.evaluated,
            meta: None,
            property: None,
          });
          machine.follow_evaluated_subjects = self.follow_evaluated_subjects;

          let evalutes_to_expected_subject = machine.any(|inner_fact| {
            !inner_fact.fact.subject.evaluated
              && match_subject(&inner_fact.fact.subject.subject, subject)
          });

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
          property,
          fact.property.subject
        );

        if !match_subject(property, &fact.property.subject) {
          reset_frame = true;
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
          value,
          fact.value.subject
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
          let mut machine = self.query_engine.query(&Query {
            subject: fact.value.subject.clone(),
            evaluated: fact.value.evaluated,
            meta: None,
            property: fact.value.property.clone(),
          });
          machine.follow_evaluated_subjects = self.follow_evaluated_subjects;

          let previous_frame = self.stack.last();

          let new_frame =
            SearchFrame::new_sub_query(machine, instruction_index, previous_frame, 128, false);
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
          operator,
          fact.operator
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
        self
          .explain_result
          .constraints
          .push((*constraint, *fact_index));
        self.fallback_instruction_pointer += 1;
      }
      UnificationInstruction::TraceBindVariable { variable, binding } => {
        if *variable == 0 {
          self.explain_result.subject = Some(binding.clone());
        }
        let frame = self.stack.last_mut().unwrap();
        frame.variable_bindings[*variable] = VariableBinding::Bound(binding.clone());
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

    if let Some(frame) = self.stack.last_mut() {
      if reset_frame {
        if let Some(last_event) = self.explain_result.fact_events.last_mut()
          && created_trace_event
        {
          last_event.update_outcome(ExplainConstraintEvaluationOutcome::Rejected(
            ExplainConstraintEvaluationOutcomeReason::NotFound,
          ));
        }
        frame.reset();
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
  pub(crate) variable_bindings: Vec<VariableBinding>,
  subject_bindings: HashMap<u128, VariableBinding>,
  tracing: Option<usize>,
  trail: Vec<usize>,
  start_instruction_index: usize,
  current_instruction_index: usize,
  state: FrameState<'a>,
  maybe_yielded: Vec<FoundFact<'a>>,
  current_investigated_fact: Option<FoundFact<'a>>,
  debug: Option<Subject>,
  database: &'a Database,
  continue_marker: bool,
}

impl<'a> SearchFrame<'a> {
  pub fn new_static(
    database: &'a Database,
    start_instruction_index: usize,
    previous_frame: Option<&SearchFrame>,
    variable_count: usize,
    continue_marker: bool,
  ) -> Self {
    let mut iterator = database.iter_naive_facts();
    let current_investigated_fact = iterator.next().map(|(fact_index, fact)| FoundFact {
      fact,
      fact_index,
      subject_binding: None,
    });

    let mut me = Self {
      variable_bindings: vec![VariableBinding::Unbound; variable_count],
      subject_bindings: HashMap::new(),
      database,
      tracing: None,
      trail: Vec::new(),
      start_instruction_index,
      current_instruction_index: start_instruction_index,
      state: FrameState::Static { iterator },
      maybe_yielded: Vec::new(),
      current_investigated_fact,
      continue_marker,
      debug: None,
    };

    if let Some(previous_frame) = previous_frame {
      me.variable_bindings = previous_frame.variable_bindings.clone();
      me.subject_bindings
        .extend(previous_frame.subject_bindings.clone());
    }

    me
  }

  pub fn new_sub_query(
    mut machine: AbstractMachine<'a>,
    start_instruction_index: usize,
    previous_frame: Option<&SearchFrame>,
    variable_count: usize,
    continue_marker: bool,
  ) -> Self {
    let current_investigated_fact = machine.next();

    let mut me = Self {
      variable_bindings: vec![VariableBinding::Unbound; variable_count],
      subject_bindings: HashMap::new(),
      database: machine.database,
      trail: Vec::new(),
      tracing: None,
      start_instruction_index,
      continue_marker,
      debug: None,
      current_instruction_index: start_instruction_index,
      state: FrameState::SubQuery { machine },
      maybe_yielded: Vec::new(),
      current_investigated_fact,
    };

    if let Some(previous_frame) = previous_frame {
      me.variable_bindings = previous_frame.variable_bindings.clone();
      me.subject_bindings
        .extend(previous_frame.subject_bindings.clone());
    }

    me
  }

  pub fn new_subject_unification(
    mut machine: AbstractMachine<'a>,
    start_instruction_index: usize,
    previous_frame: Option<&SearchFrame<'a>>,
    variable_count: usize,
    variable: usize,
    continue_marker: bool,
  ) -> Self {
    let current_investigated_fact =
      previous_frame.and_then(|frame| frame.current_investigated_fact.clone());

    machine.follow_evaluated_subjects = false;

    let mut me = Self {
      variable_bindings: vec![VariableBinding::Unbound; variable_count],
      subject_bindings: HashMap::new(),
      database: machine.database,
      trail: Vec::new(),
      tracing: None,
      continue_marker,
      start_instruction_index,
      current_instruction_index: start_instruction_index,
      debug: None,
      state: FrameState::SubjectUnification { machine, variable },
      maybe_yielded: Vec::new(),
      current_investigated_fact,
    };

    if let Some(previous_frame) = previous_frame {
      me.variable_bindings = previous_frame.variable_bindings.clone();
      me.subject_bindings
        .extend(previous_frame.subject_bindings.clone());
    }

    me.reset();

    me
  }
}

enum FrameState<'a> {
  Static {
    iterator: NaiveFactIterator<'a>,
  },
  SubQuery {
    machine: AbstractMachine<'a>,
  },
  SubjectUnification {
    machine: AbstractMachine<'a>,
    variable: usize,
  },
}

impl<'a> std::fmt::Debug for FrameState<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      FrameState::Static { .. } => f.debug_struct("FrameState::Static").finish(),
      FrameState::SubQuery { .. } => f.debug_struct("FrameState::SubQuery").finish(),
      FrameState::SubjectUnification { variable, .. } => f
        .debug_struct("FrameState::SubjectUnification")
        .field("variable", variable)
        .finish(),
    }
  }
}

impl<'a> Iterator for FrameState<'a> {
  type Item = FoundFact<'a>;

  fn next(&mut self) -> Option<Self::Item> {
    match self {
      FrameState::Static { iterator } => iterator.next().map(|(fact_index, fact)| FoundFact {
        fact,
        fact_index,
        subject_binding: None,
      }),
      FrameState::SubQuery { machine } => machine.next(),
      FrameState::SubjectUnification { machine, .. } => machine.next(),
    }
  }
}

impl<'a> SearchFrame<'a> {
  fn reset(&mut self) {
    // Advanced iterator
    match &mut self.state {
      FrameState::SubQuery { machine } => {
        self.current_investigated_fact = machine.next();
      }
      FrameState::SubjectUnification { machine, variable } => {
        if let Some(next_subject_fact) = machine.find(|f| !f.fact.subject.evaluated) {
          let subject = next_subject_fact.fact.subject.subject.clone();
          self.variable_bindings[*variable] = VariableBinding::Bound(subject.clone());
          self.debug = Some(subject.clone());
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
      }
    }

    // Reset tracing
    self.tracing = None;

    // Reset variable bindings from trail to unbound again
    for index in &self.trail {
      self.variable_bindings[*index] = VariableBinding::Unbound;
    }
    self.trail.clear();

    // Reset instruction pointer to the start
    self.current_instruction_index = self.start_instruction_index;

    // Clear the yield
    self.maybe_yielded.clear();
  }

  fn unify(&mut self, index: usize, subject: &Subject) -> bool {
    match &self.variable_bindings[index] {
      VariableBinding::Unbound => {
        self.variable_bindings[index] = VariableBinding::Bound(subject.clone());
        self.trail.push(index);
        true
      }
      VariableBinding::Bound(bound_subject) => match_subject(bound_subject, subject),
    }
  }
}

#[derive(Clone, Debug)]
pub(crate) enum VariableBinding {
  Unbound,
  Bound(Subject),
}
