use alloc::string::String;

use uuid::Uuid;

use serde::{Deserialize, Serialize};

// Convenience enum for node types matching those in IdentityMeta
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum IdentityType {
  Group,
  Device,
  #[default]
  User,
  Persona,
  Service,
  Application,
}

impl IdentityType {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::User => "user",
      Self::Persona => "persona",
      Self::Group => "group",
      Self::Device => "device",
      Self::Service => "service",
      Self::Application => "application",
    }
  }

  pub fn can_authenticate(&self) -> bool {
    !matches!(self, Self::Group | Self::Persona)
  }

  pub fn can_delegate(&self) -> bool {
    matches!(self, Self::User | Self::Device | Self::Application)
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum IdentityMeta {
  User {
    name: String,
    local: bool,
  },
  Persona {
    name: String,
    local: bool,
  },
  Group {
    name: String,
    local: bool,
  },
  Device {
    name: String,
    local: bool,
    root: Option<Uuid>,
  },
  Service {
    name: String,
    local: bool,
  },
  Application {
    name: String,
    local: bool,
  },
}

impl IdentityMeta {
  pub fn identity_type(&self) -> IdentityType {
    match self {
      IdentityMeta::User { .. } => IdentityType::User,
      IdentityMeta::Persona { .. } => IdentityType::Persona,
      IdentityMeta::Group { .. } => IdentityType::Group,
      IdentityMeta::Device { .. } => IdentityType::Device,
      IdentityMeta::Service { .. } => IdentityType::Service,
      IdentityMeta::Application { .. } => IdentityType::Application,
    }
  }

  pub fn name(&self) -> &str {
    match self {
      IdentityMeta::User { name, .. }
      | IdentityMeta::Persona { name, .. }
      | IdentityMeta::Group { name, .. }
      | IdentityMeta::Device { name, .. }
      | IdentityMeta::Service { name, .. }
      | IdentityMeta::Application { name, .. } => name,
    }
  }
}
