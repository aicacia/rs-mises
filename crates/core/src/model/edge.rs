use core::{
  fmt::{Display, Formatter},
  str::FromStr,
};

use alloc::{
  fmt,
  string::{String, ToString},
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Convinience enum for edge types matching those in EdgeProps
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EdgeType {
  MemberOf,
  RevokedBy,
  HasKey,
  Owns,
  RequestedFor,
  ApprovedBy,
  DeniedBy,
  AppliesTo,
  HasApproval,
  HasDenial,
}

impl EdgeType {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::MemberOf => "MEMBER_OF",
      Self::RevokedBy => "REVOKED_BY",
      Self::HasKey => "HAS_KEY",
      Self::Owns => "OWNS",
      Self::RequestedFor => "REQUESTED_FOR",
      Self::ApprovedBy => "APPROVED_BY",
      Self::DeniedBy => "DENIED_BY",
      Self::AppliesTo => "APPLIES_TO",
      Self::HasApproval => "HAS_APPROVAL",
      Self::HasDenial => "HAS_DENIAL",
    }
  }
}

impl Display for EdgeType {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    f.write_str(self.as_str())
  }
}

impl From<EdgeType> for String {
  fn from(e: EdgeType) -> Self {
    e.to_string()
  }
}

impl FromStr for EdgeType {
  type Err = ();

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "MEMBER_OF" => Ok(EdgeType::MemberOf),
      "REVOKED_BY" => Ok(EdgeType::RevokedBy),
      "HAS_KEY" => Ok(EdgeType::HasKey),
      "OWNS" => Ok(EdgeType::Owns),
      "REQUESTED_FOR" => Ok(EdgeType::RequestedFor),
      "APPROVED_BY" => Ok(EdgeType::ApprovedBy),
      "DENIED_BY" => Ok(EdgeType::DeniedBy),
      "APPLIES_TO" => Ok(EdgeType::AppliesTo),
      "HAS_APPROVAL" => Ok(EdgeType::HasApproval),
      "HAS_DENIAL" => Ok(EdgeType::HasDenial),
      _ => Err(()),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EdgeProps {
  MemberOf {
    since: Option<DateTime<Utc>>,
    until: Option<DateTime<Utc>>,
  },
  RevokedBy {
    since: Option<DateTime<Utc>>,
    until: Option<DateTime<Utc>>,
  },
  HasKey {
    since: Option<DateTime<Utc>>,
    until: Option<DateTime<Utc>>,
  },
  Owns {
    since: Option<DateTime<Utc>>,
    until: Option<DateTime<Utc>>,
  },
  RequestedFor {
    at: DateTime<Utc>,
  },
  ApprovedBy {
    at: DateTime<Utc>,
  },
  DeniedBy {
    at: DateTime<Utc>,
    reason: Option<String>,
  },
  AppliesTo {
    at: DateTime<Utc>,
  },
  HasApproval {
    at: DateTime<Utc>,
  },
  HasDenial {
    at: DateTime<Utc>,
  },
}

impl EdgeProps {
  pub fn edge_type(&self) -> EdgeType {
    match self {
      EdgeProps::MemberOf { .. } => EdgeType::MemberOf,
      EdgeProps::RevokedBy { .. } => EdgeType::RevokedBy,
      EdgeProps::HasKey { .. } => EdgeType::HasKey,
      EdgeProps::Owns { .. } => EdgeType::Owns,
      EdgeProps::RequestedFor { .. } => EdgeType::RequestedFor,
      EdgeProps::ApprovedBy { .. } => EdgeType::ApprovedBy,
      EdgeProps::DeniedBy { .. } => EdgeType::DeniedBy,
      EdgeProps::AppliesTo { .. } => EdgeType::AppliesTo,
      EdgeProps::HasApproval { .. } => EdgeType::HasApproval,
      EdgeProps::HasDenial { .. } => EdgeType::HasDenial,
    }
  }
}
