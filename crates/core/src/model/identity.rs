use alloc::{boxed::Box, string::String};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::model::oidc::OidcClientMeta;

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

  pub fn as_u32(&self) -> u32 {
    match self {
      Self::User => 0,
      Self::Device => 1,
      Self::Group => 2,
      Self::Service => 3,
      Self::Application => 4,
      Self::Persona => 5,
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
    encrypted_password: String,
    force_password_reset: Option<bool>,
  },
  Persona {
    name: String,
  },
  Group {
    name: String,
  },
  Device {
    name: String,
    root: Option<Uuid>,
  },
  Service {
    name: String,
  },
  Application {
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    oidc: Box<Option<OidcClientMeta>>,
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
