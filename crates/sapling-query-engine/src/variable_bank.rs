use std::{cell::RefCell, rc::Rc};

use sapling_data_model::Subject;

use crate::database::match_subject;

#[derive(Clone, Debug)]
pub(crate) enum VariableBinding {
  Unbound,
  Bound(Subject),
}

pub struct VariableBank {
  variables: Vec<VariableBinding>,
  checkpoints: Vec<Checkpoint>,
}

#[derive(Debug)]
pub struct Checkpoint {
  trail: Vec<usize>,
}

impl VariableBank {
  fn new(size: usize) -> Self {
    VariableBank {
      variables: vec![VariableBinding::Unbound; size],
      checkpoints: Vec::new(),
    }
  }

  fn push_checkpoint(&mut self) -> usize {
    self.checkpoints.push(Checkpoint { trail: vec![] });
    self.checkpoints.len() - 1
  }

  fn truncate_checkpoints(&mut self, index: usize) {
    let count = self.checkpoints.len() - index;
    for _ in 0..count {
      self.pop_checkpoint();
    }
  }

  fn pop_checkpoint(&mut self) {
    self.trail_checkpoint();
    let _ = self.checkpoints.pop();
  }

  fn trail_checkpoint(&mut self) {
    if let Some(checkpoint) = self.checkpoints.last_mut() {
      for index in checkpoint.trail.iter() {
        self.variables[*index] = VariableBinding::Unbound;
      }
      checkpoint.trail.clear();
    }
  }

  fn bind(&mut self, index: usize, subject: &Subject) {
    self.variables[index] = VariableBinding::Bound(subject.clone());
  }

  fn unbind(&mut self, index: usize) {
    self.variables[index] = VariableBinding::Unbound;
  }

  fn get(&self, index: usize) -> Option<&Subject> {
    match &self.variables[index] {
      VariableBinding::Unbound => None,
      VariableBinding::Bound(subject) => Some(subject),
    }
  }

  fn unify(&mut self, index: usize, subject: &Subject) -> bool {
    match &self.variables[index] {
      VariableBinding::Unbound => {
        self.variables[index] = VariableBinding::Bound(subject.clone());
        if let Some(checkpoint) = self.checkpoints.last_mut() {
          checkpoint.trail.push(index);
        }
        true
      }
      VariableBinding::Bound(bound_subject) => match_subject(bound_subject, subject),
    }
  }

  fn debug_print(&self) {
    println!("Variable Bank:");
    println!("Checkpoints: {:#?}", self.checkpoints);
    for (index, binding) in self.variables.iter().enumerate() {
      match binding {
        VariableBinding::Unbound => println!("  {}: Unbound", index),
        VariableBinding::Bound(subject) => println!("  {}: Bound({:?})", index, subject),
      }
    }
  }
}

#[derive(Clone)]
pub struct SharedVariableBank {
  instance: Rc<RefCell<VariableBank>>,
}

impl SharedVariableBank {
  pub fn new(size: usize) -> Self {
    SharedVariableBank {
      instance: Rc::new(RefCell::new(VariableBank::new(size))),
    }
  }

  pub fn push_checkpoint(&self) -> usize {
    self.instance.borrow_mut().push_checkpoint()
  }

  pub fn pop_checkpoint(&self) {
    self.instance.borrow_mut().pop_checkpoint();
  }

  pub fn trail_checkpoint(&self) {
    self.instance.borrow_mut().trail_checkpoint();
  }

  pub fn truncate_checkpoint(&self, index: usize) {
    self.instance.borrow_mut().truncate_checkpoints(index);
  }

  pub fn bind(&self, index: usize, subject: &Subject) {
    self.instance.borrow_mut().bind(index, subject);
  }

  pub fn unbind(&self, index: usize) {
    self.instance.borrow_mut().unbind(index);
  }

  pub fn get(&self, index: usize) -> Option<Subject> {
    self.instance.borrow().get(index).cloned()
  }

  pub fn unify(&self, index: usize, subject: &Subject) -> bool {
    self.instance.borrow_mut().unify(index, subject)
  }

  pub fn debug_print(&self) {
    self.instance.borrow().debug_print();
  }
}
