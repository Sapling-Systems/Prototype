use std::collections::VecDeque;

use sapling_data_model::{Fact, Query, Subject};

use crate::{Database, QueryEngine, database::match_subject, iterators::NaiveFactIterator};

pub struct AbstractMachine<'a> {
  instructions: Vec<UnificationInstruction>,
  database: &'a Database,
  query_engine: &'a QueryEngine,
  stack: Vec<SearchFrame<'a>>,
  yielded: VecDeque<&'a Fact>,
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

    // Advance instruction pointer
    if let Some(current_frame) = self.stack.last_mut() {
      current_frame.current_instruction_index += 1;
    }

    // Handle instructions
    let instruction = self
      .instructions
      .get(instruction_index)
      .expect("Out of bounds instruction");
    //println!("EXECUTING INSTRUCTION: {:?}", instruction);
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
        let fact = frame.current_investigated_fact.unwrap();
        if !frame.unify(*variable, &fact.subject.subject) {
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
        let fact = frame.current_investigated_fact.unwrap();
        if !match_subject(subject, &fact.subject.subject) {
          let frame = self.stack.last_mut().unwrap();
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
          let new_frame = SearchFrame::new_dynamic(machine, instruction_index, previous_frame, 128);
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
    }

    let frame = self.stack.last_mut().unwrap();
    if frame.current_investigated_fact.is_none() && !self.exhaust_frame() {
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
      state: FrameState::StaticFrame { iterator },
      maybe_yielded: Vec::new(),
      current_investigated_fact,
    };

    if let Some(previous_frame) = previous_frame {
      me.variable_bindings = previous_frame.variable_bindings.clone();
    }

    me
  }

  pub fn new_dynamic(
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
      state: FrameState::DynamicFrame { machine },
      maybe_yielded: Vec::new(),
      current_investigated_fact,
    };

    if let Some(previous_frame) = previous_frame {
      me.variable_bindings = previous_frame.variable_bindings.clone();
    }

    me
  }
}

enum FrameState<'a> {
  StaticFrame { iterator: NaiveFactIterator<'a> },
  DynamicFrame { machine: AbstractMachine<'a> },
}

impl<'a> Iterator for FrameState<'a> {
  type Item = &'a Fact;

  fn next(&mut self) -> Option<Self::Item> {
    match self {
      FrameState::StaticFrame { iterator } => iterator.next(),
      FrameState::DynamicFrame { machine } => machine.next(),
    }
  }
}

impl<'a> SearchFrame<'a> {
  fn reset(&mut self) {
    // Advanced iterator
    match &mut self.state {
      FrameState::DynamicFrame { machine } => {
        self.current_investigated_fact = machine.next();
        println!(
          "Next dynamic frame item {:?}",
          self.current_investigated_fact
        );
      }
      FrameState::StaticFrame { iterator } => {
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

  UnifySubject {
    variable: usize,
  },
  UnifyProperty {
    variable: usize,
  },
  UnifyValue {
    variable: usize,
  },
}
