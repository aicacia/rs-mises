use alloc::string::String;

/// Token Exchange (RFC 8693) minimal request placeholder.
pub struct TokenExchangeRequest {
  pub subject_token: String,
  pub subject_token_type: Option<String>,
  pub requested_token_type: Option<String>,
  pub audience: Option<String>,
}
