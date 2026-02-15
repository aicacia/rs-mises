use alloc::{string::String, vec::Vec};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::model::edge::EdgeType;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelationshipRequest {
  pub relationship_type: EdgeType,
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
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum RequestStatus {
  #[default]
  Pending,
  Approved,
  Denied,
  Applied,
}

impl core::fmt::Display for RequestStatus {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      RequestStatus::Pending => write!(f, "pending"),
      RequestStatus::Approved => write!(f, "approved"),
      RequestStatus::Denied => write!(f, "denied"),
      RequestStatus::Applied => write!(f, "applied"),
    }
  }
}

impl RequestStatus {
  pub fn as_str(&self) -> &'static str {
    match self {
      RequestStatus::Pending => "pending",
      RequestStatus::Approved => "approved",
      RequestStatus::Denied => "denied",
      RequestStatus::Applied => "applied",
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum RequestOwnership {
  #[default]
  Identity,
  Requestor,
  Explicit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Approval {
  pub approver: Uuid,
  pub decided_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Denial {
  pub approver: Uuid,
  pub decided_at: DateTime<Utc>,
  pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RequestInput {
  pub resource_id: Option<Uuid>,
  pub resource_type: Option<String>,
  pub actions: Vec<String>,
  pub scope: Scope,
  pub requestor: Uuid,
  pub owners: Option<Vec<Uuid>>,
  pub ownership: Option<RequestOwnership>,
  pub quorum: Option<usize>,
  pub create_if_missing: Option<bool>,
  pub relationship_requests: Vec<RelationshipRequest>,
  pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Request {
  pub resource_id: Option<Uuid>,
  pub resource_type: Option<String>,
  pub actions: Vec<String>,
  pub scope: Scope,
  pub requestor: Uuid,
  pub requested_for: Option<Uuid>,
  pub owners: Option<Vec<Uuid>>,
  #[serde(default = "default_create_if_missing")]
  pub create_if_missing: bool,
  #[serde(default)]
  pub ownership: RequestOwnership,
  pub relationship_requests: Vec<RelationshipRequest>,
  #[serde(default)]
  pub status: RequestStatus,
  #[serde(default)]
  pub quorum: usize,
  pub created_at: DateTime<Utc>,
  pub applied_at: Option<DateTime<Utc>>,
  pub expires_at: Option<DateTime<Utc>>,
}

fn default_create_if_missing() -> bool {
  true
}
