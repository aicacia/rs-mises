use alloc::string::String;

/// Minimal CIBA (backchannel) auth request representation.
pub struct BackchannelAuthRequest {
  pub client_id: Option<String>,
  pub scope: Option<String>,
  pub login_hint: Option<String>,
}
