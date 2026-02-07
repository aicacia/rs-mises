use alloc::string::String;

/// Token request payloads for different grant types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenRequest {
  AuthorizationCode {
    code: String,
    redirect_uri: Option<String>,
    client_id: Option<String>,
    code_verifier: Option<String>,
  },
  RefreshToken {
    refresh_token: String,
    scope: Option<String>,
    client_id: Option<String>,
  },
  ClientCredentials {
    scope: Option<String>,
    client_id: Option<String>,
  },
  DeviceCode {
    device_code: String,
    client_id: Option<String>,
  },
}
