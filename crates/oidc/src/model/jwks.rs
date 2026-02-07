use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::Jwk;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Jwks {
  pub keys: Vec<Jwk>,
}
