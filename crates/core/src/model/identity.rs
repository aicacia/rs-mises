use alloc::{boxed::Box, string::String};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::model::oidc::OidcClientMeta;

/// Types of identities in the system.
///
/// Each variant represents a different kind of entity that can authenticate or delegate permissions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum IdentityType {
  /// User identity (can authenticate and delegate)
  #[default]
  User,
  /// Persona identity (non-authenticating identity for representation)
  Persona,
  /// Group identity (non-authenticating collection of identities)
  Group,
  /// Device identity (can authenticate and delegate)
  Device,
  /// Service identity (service accounts)
  Service,
  /// Application identity with OIDC client (can authenticate and delegate)
  Application,
}

impl IdentityType {
  /// Get the string representation of this identity type.
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

  /// Get the numeric identifier for this identity type.
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

  /// Check if this identity type can authenticate directly.
  pub fn can_authenticate(&self) -> bool {
    !matches!(self, Self::Group | Self::Persona)
  }

  /// Check if this identity type can delegate permissions.
  pub fn can_delegate(&self) -> bool {
    matches!(self, Self::User | Self::Device | Self::Application)
  }
}

/// Metadata for a specific identity, containing type-specific information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum IdentityMeta {
  /// User identity with optional password and reset flag
  User {
    /// User display name
    name: String,
    /// Encrypted password hash
    encrypted_password: String,
    /// Whether user must reset password on next login
    force_password_reset: Option<bool>,
  },
  /// Persona identity (non-authenticating representation)
  Persona {
    /// Persona display name
    name: String,
  },
  /// Group identity (collection of other identities)
  Group {
    /// Group display name
    name: String,
  },
  /// Device identity
  Device {
    /// Device display name
    name: String,
    /// Root identity owning this device
    root: Option<Uuid>,
    /// Device hardware identifier
    device_id: Option<String>,
  },
  /// Service identity
  Service {
    /// Service display name
    name: String,
  },
  /// Application identity with OIDC configuration
  Application {
    /// OpenID Connect client metadata
    #[serde(default)]
    oidc: Box<OidcClientMeta>,
  },
}

impl IdentityMeta {
  /// Get the identity type of this metadata.
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

  /// Get the display name of this identity.
  pub fn name(&self) -> &str {
    match self {
      IdentityMeta::User { name, .. }
      | IdentityMeta::Persona { name, .. }
      | IdentityMeta::Group { name, .. }
      | IdentityMeta::Device { name, .. }
      | IdentityMeta::Service { name, .. } => name,
      IdentityMeta::Application { oidc, .. } => &oidc.client_name,
    }
  }
}
