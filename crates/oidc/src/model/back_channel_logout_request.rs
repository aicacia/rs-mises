use alloc::string::String;

/// Back-channel logout request placeholder.
pub struct BackChannelLogoutRequest {
  pub logout_token: String,
}
