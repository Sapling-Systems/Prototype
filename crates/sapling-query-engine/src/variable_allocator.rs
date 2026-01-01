use std::{cell::RefCell, collections::HashMap, rc::Rc};

use sapling_data_model::Subject;

pub struct VariableAllocator {
  next_variable_id: usize,
  subject_map: HashMap<u128, usize>,
}

impl VariableAllocator {
  fn new() -> Self {
    VariableAllocator {
      next_variable_id: 0,
      subject_map: HashMap::with_capacity(128),
    }
  }

  fn reset(&mut self) {
    self.next_variable_id = 0;
    self.subject_map.clear();
  }

  fn get_subject_map(&self) -> HashMap<u128, usize> {
    self.subject_map.clone()
  }

  fn allocate_raw_variable(&mut self) -> usize {
    let id = self.next_variable_id;
    self.next_variable_id += 1;
    id
  }

  fn allocate_for_subject(&mut self, subject: &Subject) -> usize {
    let Subject::Static { uuid } = subject else {
      panic!("Invalid subject type")
    };

    if let Some(&id) = self.subject_map.get(uuid) {
      id
    } else {
      let id = self.allocate_raw_variable();
      self.subject_map.insert(*uuid, id);
      id
    }
  }
}

#[derive(Clone)]
pub struct SharedVariableAllocator {
  instance: Rc<RefCell<VariableAllocator>>,
}

impl SharedVariableAllocator {
  pub fn new() -> Self {
    SharedVariableAllocator {
      instance: Rc::new(RefCell::new(VariableAllocator::new())),
    }
  }

  pub fn allocate_raw_variable(&self) -> usize {
    self.instance.borrow_mut().allocate_raw_variable()
  }

  pub fn allocate_for_subject(&self, subject: &Subject) -> usize {
    self.instance.borrow_mut().allocate_for_subject(subject)
  }

  pub fn get_subject_map(&self) -> HashMap<u128, usize> {
    self.instance.borrow().get_subject_map()
  }

  pub fn reset(&self) {
    self.instance.borrow_mut().reset();
  }
}
