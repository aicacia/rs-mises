use alloc::{
  boxed::Box,
  string::{String, ToString},
};
use core::{
  fmt::{Display, Formatter},
  str::FromStr,
};

use serde::{Deserialize, Serialize};

use crate::model::{
  identity::IdentityMeta,
  keys::KeyMeta,
  policy::PolicyMeta,
  requests::{Approval, Denial, Request},
  resource::ResourceMeta,
};

/// Graph node metadata representing different entity types in the system.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeMeta {
  /// Identity metadata (user, group, device, etc.)
  Identity(Box<IdentityMeta>),
  /// Cryptographic key metadata
  Key(KeyMeta),
  /// Resource metadata
  Resource(ResourceMeta),
  /// Access policy metadata
  Policy(PolicyMeta),
  /// Request metadata
  Request(Request),
  /// Request approval metadata
  Approval(Approval),
  /// Request denial metadata
  Denial(Denial),
}

/// Enumeration of node types available in the graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeType {
  /// Identity node (user, group, device, etc.)
  Identity,
  /// Cryptographic key node
  Key,
  /// Resource node
  Resource,
  /// Access policy node
  Policy,
  /// Request node
  Request,
  /// Request approval node
  Approval,
  /// Request denial node
  Denial,
}

impl NodeType {
  /// Get the string representation of this node type.
  pub fn as_str(&self) -> &'static str {
    match self {
      NodeType::Identity => "Identity",
      NodeType::Key => "Key",
      NodeType::Resource => "Resource",
      NodeType::Policy => "Policy",
      NodeType::Request => "Request",
      NodeType::Approval => "Approval",
      NodeType::Denial => "Denial",
    }
  }
}

impl Display for NodeType {
  fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
    f.write_str(self.as_str())
  }
}

impl From<NodeType> for String {
  fn from(n: NodeType) -> Self {
    n.to_string()
  }
}

impl FromStr for NodeType {
  type Err = ();

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "Identity" => Ok(NodeType::Identity),
      "Key" => Ok(NodeType::Key),
      "Resource" => Ok(NodeType::Resource),
      "Policy" => Ok(NodeType::Policy),
      "Request" => Ok(NodeType::Request),
      "Approval" => Ok(NodeType::Approval),
      "Denial" => Ok(NodeType::Denial),
      _ => Err(()),
    }
  }
}
