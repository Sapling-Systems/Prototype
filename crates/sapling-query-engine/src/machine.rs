use std::collections::VecDeque;

use sapling_data_model::{Fact, Query, Subject};

use crate::{Database, QueryEngine, System, database::match_subject, iterators::NaiveFactIterator};

pub struct AbstractMachine<'a> {
  instructions: Vec<UnificationInstruction>,
  database: &'a Database,
  query_engine: &'a QueryEngine,
  stack: Vec<SearchFrame<'a>>,
  yielded: VecDeque<&'a Fact>,
  follow_evaluated_subjects: bool,
}

impl<'a> AbstractMachine<'a> {
  pub fn new(
    instructions: Vec<UnificationInstruction>,
    database: &'a Database,
    query_engine: &'a QueryEngine,
  ) -> Self {
    Self {
      instructions,
      database,
      query_engine,
      follow_evaluated_subjects: true,
      yielded: VecDeque::new(),
      stack: Vec::new(),
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

  fn exhaust_stack(&mut self) {
    self.stack.truncate(1);
    self.stack[0].reset();
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
      .unwrap_or(0);
    if instruction_index >= self.instructions.len() {
      self.exhaust_stack();
      instruction_index = 1;
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

    match instruction {
      // Simply allocates a new frame on the stack
      UnificationInstruction::AllocateFrame { size } => {
        let new_frame = SearchFrame::new_static(
          self.database,
          instruction_index + 1,
          self.stack.last(),
          *size,
        );
        self.stack.push(new_frame);
      }

      // Yielding
      UnificationInstruction::MaybeYield => {
        let frame = self.stack.last_mut().unwrap();
        if let Some(fact) = frame.current_investigated_fact {
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
        let fact = frame.get_subject_fact().unwrap();
        let direct_match = frame.unify(*variable, &fact.subject.subject);

        if direct_match {
        } else if fact.subject.evaluated && self.follow_evaluated_subjects {
          println!("instructions:\n{:#?}", self.instructions);
          println!("starting subject frame for target subject = {:?}", variable);
          let machine = self.query_engine.query(&Query {
            subject: fact.subject.subject.clone(),
            evaluated: true,
            meta: None,
            property: None,
          });
          let previous_frame = self.stack.last();
          let new_frame =
            SearchFrame::new_subject(machine, instruction_index + 1, previous_frame, 128);
          self.stack.push(new_frame);
        } else {
          frame.reset();
        }
      }
      UnificationInstruction::UnifyProperty { variable } => {
        let frame = self.stack.last_mut().unwrap();
        let fact = frame.current_investigated_fact.unwrap();
        if !frame.unify(*variable, &fact.property.subject) {
          frame.reset();
        }
      }
      UnificationInstruction::UnifyValue { variable } => {
        let frame = self.stack.last_mut().unwrap();
        let fact = frame.current_investigated_fact.unwrap();
        if !frame.unify(*variable, &fact.value.subject) {
          frame.reset();
        }
      }

      // Simple checks against the current fact
      UnificationInstruction::CheckSubject { subject } => {
        let frame = self.stack.last_mut().unwrap();
        let fact = frame.get_subject_fact().unwrap();
        let direct_match = match_subject(subject, &fact.subject.subject);

        if direct_match {
        } else if fact.subject.evaluated && self.follow_evaluated_subjects {
          let mut machine = self.query_engine.query(&Query {
            subject: fact.subject.subject.clone(),
            evaluated: fact.subject.evaluated,
            meta: None,
            property: None,
          });
          let evalutes_to_expected_subject = machine.any(|inner_fact| {
            !inner_fact.subject.evaluated && match_subject(&inner_fact.subject.subject, subject)
          });
          if !evalutes_to_expected_subject {
            frame.reset();
          }
        } else {
          frame.reset();
        }
      }
      UnificationInstruction::CheckProperty { property } => {
        let frame = self.stack.last_mut().unwrap();
        let fact = frame.current_investigated_fact.unwrap();
        if !match_subject(property, &fact.property.subject) {
          let frame = self.stack.last_mut().unwrap();
          frame.reset();
        }
      }
      UnificationInstruction::CheckValue { value, property } => {
        let frame = self.stack.last_mut().unwrap();
        let fact = frame.current_investigated_fact.unwrap();

        let direct_match = match_subject(value, &fact.value.subject);
        let property_match = match (&fact.value.property, property) {
          (None, None) => true,
          (Some(a), Some(b)) if match_subject(&a, b) => true,
          _ => false,
        };

        // Simple case, we have a direct match of the value as well as property
        if direct_match && property_match {
        } else if fact.value.evaluated || fact.value.property.is_some() {
          let machine = self.query_engine.query(&Query {
            subject: fact.value.subject.clone(),
            evaluated: fact.value.evaluated,
            meta: None,
            property: fact.value.property.clone(),
          });
          let previous_frame = self.stack.last();
          let new_frame =
            SearchFrame::new_sub_query(machine, instruction_index, previous_frame, 128);
          self.stack.push(new_frame);
        } else {
          frame.reset();
        }
      }
      UnificationInstruction::CheckOperator { operator } => {
        let frame = self.stack.last_mut().unwrap();
        let fact = frame.current_investigated_fact.unwrap();
        if !match_subject(operator, &fact.operator) {
          let frame = self.stack.last_mut().unwrap();
          frame.reset();
        }
      }
      UnificationInstruction::CheckMeta { skip_system } => {
        let frame = self.stack.last_mut().unwrap();
        let fact = frame.get_subject_fact().unwrap();

        if *skip_system && match_subject(&fact.meta, &System::CORE_META) {
          frame.reset();
        }
      }
      UnificationInstruction::SkipSubject { subject } => {
        let frame = self.stack.last_mut().unwrap();
        let fact = frame.get_subject_fact().unwrap();
        let direct_match = match_subject(subject, &fact.subject.subject);
        if direct_match {
          frame.reset();
        }
      }
    }

    if !self.unwind_stack() {
      return true;
    }

    if self.stack.is_empty() {
      return false;
    }

    true
  }

  pub fn execute_until_yield(&mut self) -> Option<&'a Fact> {
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
  type Item = &'a Fact;

  fn next(&mut self) -> Option<Self::Item> {
    self.execute_until_yield()
  }
}

struct SearchFrame<'a> {
  variable_bindings: Vec<VariableBinding>,
  trail: Vec<usize>,
  start_instruction_index: usize,
  current_instruction_index: usize,
  state: FrameState<'a>,
  maybe_yielded: Vec<&'a Fact>,
  current_investigated_fact: Option<&'a Fact>,
  current_investigated_subject_fact: Option<&'a Fact>,
}

impl<'a> SearchFrame<'a> {
  pub fn new_static(
    database: &'a Database,
    start_instruction_index: usize,
    previous_frame: Option<&SearchFrame>,
    variable_count: usize,
  ) -> Self {
    let mut iterator = database.iter_naive_facts();
    let current_investigated_fact = iterator.next();

    let mut me = Self {
      variable_bindings: vec![VariableBinding::Unbound; variable_count],
      trail: Vec::new(),
      start_instruction_index,
      current_instruction_index: start_instruction_index,
      state: FrameState::Static { iterator },
      maybe_yielded: Vec::new(),
      current_investigated_fact,
      current_investigated_subject_fact: None,
    };

    if let Some(previous_frame) = previous_frame {
      me.variable_bindings = previous_frame.variable_bindings.clone();
    }

    me
  }

  pub fn new_sub_query(
    mut machine: AbstractMachine<'a>,
    start_instruction_index: usize,
    previous_frame: Option<&SearchFrame>,
    variable_count: usize,
  ) -> Self {
    let current_investigated_fact = machine.next();

    let mut me = Self {
      variable_bindings: vec![VariableBinding::Unbound; variable_count],
      trail: Vec::new(),
      start_instruction_index,
      current_instruction_index: start_instruction_index,
      state: FrameState::SubQuery { machine },
      maybe_yielded: Vec::new(),
      current_investigated_fact,
      current_investigated_subject_fact: None,
    };

    if let Some(previous_frame) = previous_frame {
      me.variable_bindings = previous_frame.variable_bindings.clone();
    }

    me
  }

  pub fn new_subject<'b>(
    mut machine: AbstractMachine<'a>,
    start_instruction_index: usize,
    previous_frame: Option<&'b SearchFrame<'a>>,
    variable_count: usize,
  ) -> Self {
    machine.follow_evaluated_subjects = false;
    let subject_fact = machine.next();

    let mut me = Self {
      variable_bindings: vec![VariableBinding::Unbound; variable_count],
      trail: Vec::new(),
      start_instruction_index,
      current_instruction_index: start_instruction_index,
      state: FrameState::Subject { machine },
      maybe_yielded: Vec::new(),
      current_investigated_fact: subject_fact,
      current_investigated_subject_fact: subject_fact,
    };

    if let Some(fact) = subject_fact {
      me.variable_bindings[0] = VariableBinding::Bound(fact.subject.subject.clone());
    } else {
      me.current_investigated_fact = None;
    }

    me
  }
}

enum FrameState<'a> {
  Static { iterator: NaiveFactIterator<'a> },
  SubQuery { machine: AbstractMachine<'a> },
  Subject { machine: AbstractMachine<'a> },
}

impl<'a> Iterator for FrameState<'a> {
  type Item = &'a Fact;

  fn next(&mut self) -> Option<Self::Item> {
    match self {
      FrameState::Static { iterator } => iterator.next(),
      FrameState::SubQuery { machine } => machine.next(),
      FrameState::Subject { machine } => machine.next(),
    }
  }
}

impl<'a> SearchFrame<'a> {
  fn get_subject_fact(&self) -> Option<&'a Fact> {
    self.current_investigated_fact
    /*    self
    .current_investigated_subject_fact
    .or(self.current_investigated_fact)*/
  }

  fn reset(&mut self) {
    // Advanced iterator
    match &mut self.state {
      FrameState::Subject { machine } => {
        let subject_fact = machine.next();
        if let Some(fact) = subject_fact {
          self.variable_bindings[0] = VariableBinding::Bound(fact.subject.subject.clone());
          self.current_investigated_subject_fact = subject_fact;
          self.current_investigated_fact = subject_fact;
        } else {
          // Causes a reset of this frame
          self.current_investigated_fact = None;
          self.current_investigated_subject_fact = None;
        }
      }
      FrameState::SubQuery { machine } => {
        self.current_investigated_fact = machine.next();
      }
      FrameState::Static { iterator } => {
        self.current_investigated_fact = iterator.next();
      }
    }

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

#[derive(Clone)]
enum VariableBinding {
  Unbound,
  Bound(Subject),
}

#[derive(Debug)]
pub enum UnificationInstruction {
  AllocateFrame {
    size: usize,
  },

  MaybeYield,
  YieldAll,

  CheckSubject {
    subject: Subject,
  },
  CheckProperty {
    property: Subject,
  },
  CheckValue {
    value: Subject,
    property: Option<Subject>,
  },
  CheckOperator {
    operator: Subject,
  },
  CheckMeta {
    skip_system: bool,
  },

  UnifySubject {
    variable: usize,
  },
  UnifyProperty {
    variable: usize,
  },
  UnifyValue {
    variable: usize,
  },

  SkipSubject {
    subject: Subject,
  },
}
