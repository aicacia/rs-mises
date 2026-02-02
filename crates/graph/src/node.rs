use alloc::string::String;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::Id;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(bound(deserialize = "M: serde::de::Deserialize<'de>"))]
pub struct Node<I, M>
where
  I: Id,
  M: Serialize + Clone + PartialEq + Eq,
{
  pub id: I,
  pub r#type: String,
  pub updated_at: DateTime<Utc>,
  pub created_at: DateTime<Utc>,
  pub metadata: M,
}
