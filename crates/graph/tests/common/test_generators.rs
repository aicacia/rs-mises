use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use mises_graph::IdGenerator;

pub struct AtomicIdGenerator<T> {
  counter: Arc<T>,
}

impl<T> AtomicIdGenerator<T> {
  pub fn new(counter: T) -> Self {
    Self {
      counter: Arc::new(counter),
    }
  }
}

impl<T> Clone for AtomicIdGenerator<T> {
  fn clone(&self) -> Self {
    Self {
      counter: Arc::clone(&self.counter),
    }
  }
}

impl IdGenerator<usize> for AtomicIdGenerator<AtomicUsize> {
  fn next(&self) -> usize {
    self.counter.fetch_add(1, Ordering::SeqCst)
  }
}

impl IdGenerator<u64> for AtomicIdGenerator<AtomicU64> {
  fn next(&self) -> u64 {
    self.counter.fetch_add(1, Ordering::SeqCst)
  }
}

pub type UsizeGenerator = AtomicIdGenerator<AtomicUsize>;
pub type U64Generator = AtomicIdGenerator<AtomicU64>;

impl UsizeGenerator {
  pub fn new_usize() -> Self {
    Self::new(AtomicUsize::new(0))
  }
}

impl U64Generator {
  pub fn new_u64() -> Self {
    Self::new(AtomicU64::new(1))
  }
}
