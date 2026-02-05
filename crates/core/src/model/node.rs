use core::{
  fmt::{Display, Formatter},
  str::FromStr,
};

use alloc::string::{String, ToString};

use serde::{Deserialize, Serialize};

use crate::model::{
  identity::IdentityMeta,
  keys::KeyMeta,
  policy::PolicyMeta,
  requests::{Approval, Denial, Request},
  resource::ResourceMeta,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeMeta {
  Identity(IdentityMeta),
  Key(KeyMeta),
  Resource(ResourceMeta),
  Policy(PolicyMeta),
  Request(Request),
  Approval(Approval),
  Denial(Denial),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeType {
  Identity,
  Key,
  Resource,
  Policy,
  Request,
  Approval,
  Denial,
}

impl NodeType {
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
