use alloc::string::String;

/// Front-channel logout request placeholder.
pub struct FrontChannelLogoutRequest {
  pub logout_token: Option<String>,
}
