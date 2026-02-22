use alloc::string::{String, ToString};
use core::{
  fmt::{self, Display, Formatter},
  str::FromStr,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Types of directed edges in the identity graph.
///
/// Edges represent relationships between nodes such as membership, ownership, and authorization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EdgeType {
  /// Identity is a member of another identity (group membership).
  MemberOf,
  /// Identity has been revoked by another identity.
  RevokedBy,
  /// Identity possesses a cryptographic key.
  HasKey,
  /// Identity owns another entity.
  Owns,
  /// A request was made for another identity.
  RequestedFor,
  /// An approval was granted by another identity.
  ApprovedBy,
  /// A request was denied by another identity.
  DeniedBy,
  /// A policy applies to an identity or resource.
  AppliesTo,
  /// An approval record is associated with a request.
  HasApproval,
  /// A denial record is associated with a request.
  HasDenial,
}

impl EdgeType {
  /// Get the string representation of this edge type.
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

/// Edge properties containing metadata specific to each edge type.
///
/// Properties include temporal information (since/until times) and other relationship-specific metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EdgeProps {
  /// Properties for a membership edge with optional validity time window.
  MemberOf {
    /// Start of membership period.
    since: Option<DateTime<Utc>>,
    /// End of membership period.
    until: Option<DateTime<Utc>>,
  },
  /// Properties for a revocation edge with optional validity time window.
  RevokedBy {
    /// Revocation start time.
    since: Option<DateTime<Utc>>,
    /// Revocation end time.
    until: Option<DateTime<Utc>>,
  },
  /// Properties for a key possession edge with optional validity time window.
  HasKey {
    /// Start of key validity.
    since: Option<DateTime<Utc>>,
    /// End of key validity.
    until: Option<DateTime<Utc>>,
  },
  /// Properties for an ownership edge with optional validity time window.
  Owns {
    /// Start of ownership.
    since: Option<DateTime<Utc>>,
    /// End of ownership.
    until: Option<DateTime<Utc>>,
  },
  /// Properties for a request relationship with timestamp.
  RequestedFor {
    /// When the request was made.
    at: DateTime<Utc>,
  },
  /// Properties for an approval with timestamp.
  ApprovedBy {
    /// When the approval was granted.
    at: DateTime<Utc>,
  },
  /// Properties for a denial with timestamp and optional reason.
  DeniedBy {
    /// When the denial was issued.
    at: DateTime<Utc>,
    /// Optional reason for the denial.
    reason: Option<String>,
  },
  /// Properties for a policy application with timestamp.
  AppliesTo {
    /// When the policy was applied.
    at: DateTime<Utc>,
  },
  /// Properties for an approval record link.
  HasApproval {
    /// When the approval was recorded.
    at: DateTime<Utc>,
  },
  /// Properties for a denial record link.
  HasDenial {
    /// When the denial was recorded.
    at: DateTime<Utc>,
  },
}

impl EdgeProps {
  /// Get the edge type corresponding to these properties.
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
