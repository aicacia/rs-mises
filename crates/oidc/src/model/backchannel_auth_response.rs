use alloc::string::String;

/// Minimal CIBA response placeholder.
pub struct BackchannelAuthResponse {
  pub auth_req_id: String,
  pub expires_in: Option<u64>,
  pub interval: Option<u64>,
}
