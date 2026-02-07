use alloc::string::String;

/// Simple response representation for the `/authorize` endpoint.
/// For now this is a minimal placeholder used by the provider service.
pub struct AuthorizeResponse {
  /// If present, the provider should redirect the user-agent to this URI
  /// (includes any query/fragment as appropriate for the response mode).
  pub redirect_uri: Option<String>,
}
