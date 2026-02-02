use crate::IdGenerator;

pub struct UuidGenerator;

impl UuidGenerator {
  pub fn new() -> Self {
    Self
  }
}

impl Default for UuidGenerator {
  fn default() -> Self {
    Self::new()
  }
}

impl IdGenerator<uuid::Uuid> for UuidGenerator {
  fn next(&self) -> uuid::Uuid {
    #[cfg(feature = "std")]
    {
      uuid::Uuid::now_v7()
    }
    #[cfg(not(feature = "std"))]
    {
      uuid::Uuid::new_v4()
    }
  }
}
