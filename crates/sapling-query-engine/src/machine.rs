use std::collections::VecDeque;

use sapling_data_model::{Fact, Subject};

use crate::{Database, database::match_subject, iterators::NaiveFactIterator};

pub struct AbstractMachine<'a> {
  instructions: Vec<UnificationInstruction>,
  database: &'a Database,
  stack: Vec<SearchFrame<'a>>,
  yielded: VecDeque<&'a Fact>,
  current_investigated_fact: Option<&'a Fact>,
}

impl<'a> AbstractMachine<'a> {
  pub fn new(instructions: Vec<UnificationInstruction>, database: &'a Database) -> Self {
    Self {
      instructions,
      database,
      yielded: VecDeque::new(),
      stack: Vec::new(),
      current_investigated_fact: None,
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
        let mut new_frame = SearchFrame {
          current_instruction_index: instruction_index + 1,
          start_instruction_index: instruction_index + 1,
          variable_bindings: vec![VariableBinding::Unbound; *size],
          maybe_yielded: Vec::new(),
          iterator: self.database.iter_naive_facts(),
          trail: Vec::new(),
        };
        if let Some(current_frame) = self.stack.last_mut() {
          new_frame.variable_bindings = current_frame.variable_bindings.clone();
        }
        self.stack.push(new_frame);
      }
      // Try to advance to the next fact otherwise exhaust the frame and stop if needed
      UnificationInstruction::NextFact => {
        let frame = self.stack.last_mut().unwrap();
        if let Some(fact) = frame.iterator.next() {
          self.current_investigated_fact = Some(fact);
        } else if !self.exhaust_frame() {
          return false;
        }
      }

      // Yielding
      UnificationInstruction::MaybeYield => {
        let frame = self.stack.last_mut().unwrap();
        if let Some(fact) = self.current_investigated_fact {
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
        let fact = self.current_investigated_fact.unwrap();
        if !frame.unify(*variable, &fact.subject.subject) {
          frame.reset();
        }
      }
      UnificationInstruction::UnifyProperty { variable } => {
        let frame = self.stack.last_mut().unwrap();
        let fact = self.current_investigated_fact.unwrap();
        if !frame.unify(*variable, &fact.property.subject) {
          frame.reset();
        }
      }
      UnificationInstruction::UnifyValue { variable } => {
        let frame = self.stack.last_mut().unwrap();
        let fact = self.current_investigated_fact.unwrap();
        if !frame.unify(*variable, &fact.value.subject) {
          frame.reset();
        }
      }

      // Simple checks against the current fact
      UnificationInstruction::CheckSubject { subject } => {
        let fact = self.current_investigated_fact.unwrap();
        if !match_subject(subject, &fact.subject.subject) {
          let frame = self.stack.last_mut().unwrap();
          frame.reset();
        }
      }
      UnificationInstruction::CheckProperty { property } => {
        let fact = self.current_investigated_fact.unwrap();
        if !match_subject(property, &fact.property.subject) {
          let frame = self.stack.last_mut().unwrap();
          frame.reset();
        }
      }
      UnificationInstruction::CheckValue { value } => {
        let fact = self.current_investigated_fact.unwrap();
        if !match_subject(value, &fact.value.subject) {
          let frame = self.stack.last_mut().unwrap();
          frame.reset();
        }
      }
      UnificationInstruction::CheckOperator { operator } => {
        let fact = self.current_investigated_fact.unwrap();
        if !match_subject(operator, &fact.operator) {
          let frame = self.stack.last_mut().unwrap();
          frame.reset();
        }
      }
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
  iterator: NaiveFactIterator<'a>,
  maybe_yielded: Vec<&'a Fact>,
}

impl<'a> SearchFrame<'a> {
  fn reset(&mut self) {
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
  AllocateFrame { size: usize },
  NextFact,

  MaybeYield,
  YieldAll,

  CheckSubject { subject: Subject },
  CheckProperty { property: Subject },
  CheckValue { value: Subject },
  CheckOperator { operator: Subject },

  UnifySubject { variable: usize },
  UnifyProperty { variable: usize },
  UnifyValue { variable: usize },
}
