use alloc::string::String;

/// Response from the Pushed Authorization Request (PAR) endpoint.
/// Minimal placeholder with the provided request_uri and expiration.
pub struct PushedAuthorizeResponse {
  pub request_uri: String,
  pub expires_in: Option<u64>,
}
