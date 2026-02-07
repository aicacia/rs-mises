use alloc::string::String;

/// Minimal End Session (logout) request representation for RP-initiated logout.
/// This is a small placeholder capturing common parameters per OIDC spec.
pub struct EndSessionRequest {
  pub id_token_hint: Option<String>,
  pub post_logout_redirect_uri: Option<String>,
  pub state: Option<String>,
}
