use alloc::string::String;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::Id;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(bound(deserialize = "P: serde::de::Deserialize<'de>"))]
pub struct Edge<I, P>
where
  I: Id,
  P: Serialize + Clone + PartialEq + Eq,
{
  pub id: I,
  pub r#type: String,
  pub from_id: I,
  pub to_id: I,
  pub updated_at: DateTime<Utc>,
  pub created_at: DateTime<Utc>,
  pub properties: P,
}
