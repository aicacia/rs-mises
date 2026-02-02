#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

use uuid::Uuid;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelationshipRequest {
  pub relationship_type: String,
  pub subject: Uuid,
  pub object: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
  Owner,
  Requestor,
  #[serde(rename = "owner+requestor")]
  OwnerRequestor,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Request {
  pub resource_id: Uuid,
  pub resource_type: Option<String>,
  pub actions: Vec<String>,
  pub scope: Scope,
  pub requestor: Uuid,
  pub requested_for: Option<Uuid>,
  pub owners: Option<Vec<Uuid>>,
  pub create_if_missing: Option<bool>,
  pub relationship_requests: Vec<RelationshipRequest>,
  pub created_at: DateTime<Utc>,
  pub expires_at: Option<DateTime<Utc>>,
}
