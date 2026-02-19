pub const RESPONSE_MODES: &[&str] = &["query", "fragment", "form_post"];

pub const RESPONSE_TYPE_CODE: &str = "code";
pub const RESPONSE_TYPE_TOKEN: &str = "token";
pub const RESPONSE_TYPE_ID_TOKEN: &str = "id_token";
pub const RESPONSE_TYPES_SUPPORTED: &[&str] = &[
  RESPONSE_TYPE_CODE,
  RESPONSE_TYPE_TOKEN,
  RESPONSE_TYPE_ID_TOKEN,
];

pub const GRANT_TYPE_AUTHORIZATION_CODE: &str = "authorization_code";
pub const GRANT_TYPE_REFRESH_TOKEN: &str = "refresh_token";
pub const GRANT_TYPE_CLIENT_CREDENTIALS: &str = "client_credentials";
pub const GRANT_TYPE_DEVICE_CODE: &str = "urn:ietf:params:oauth:grant-type:device_code";
pub const GRANT_TYPES_SUPPORTED: &[&str] = &[
  GRANT_TYPE_AUTHORIZATION_CODE,
  GRANT_TYPE_REFRESH_TOKEN,
  GRANT_TYPE_CLIENT_CREDENTIALS,
  GRANT_TYPE_DEVICE_CODE,
];

pub const TOKEN_ENDPOINT_AUTH_METHOD_NONE: &str = "none";
pub const TOKEN_ENDPOINT_AUTH_METHOD_CLIENT_SECRET_BASIC: &str = "client_secret_basic";
pub const TOKEN_ENDPOINT_AUTH_METHOD_CLIENT_SECRET_POST: &str = "client_secret_post";
pub const TOKEN_AUTH_METHODS_SUPPORTED: &[&str] = &[
  TOKEN_ENDPOINT_AUTH_METHOD_NONE,
  TOKEN_ENDPOINT_AUTH_METHOD_CLIENT_SECRET_BASIC,
  TOKEN_ENDPOINT_AUTH_METHOD_CLIENT_SECRET_POST,
];

pub const TOKEN_AUTH_SIGNING_ALG_EDDSA: &str = "EdDSA";
pub const TOKEN_AUTH_SIGNING_ALGS_SUPPORTED: &[&str] = &[TOKEN_AUTH_SIGNING_ALG_EDDSA];

pub const CODE_CHALLENGE_METHOD_S256: &str = "S256";
pub const CODE_CHALLENGE_METHODS_SUPPORTED: &[&str] = &[CODE_CHALLENGE_METHOD_S256];

pub const SUBJECT_TYPE_PUBLIC: &str = "public";
pub const SUBJECT_TYPES_SUPPORTED: &[&str] = &[SUBJECT_TYPE_PUBLIC];

pub const ID_TOKEN_SIGNING_ALGS_SUPPORTED: &[&str] = &[TOKEN_AUTH_SIGNING_ALG_EDDSA];

pub const SCOPES_SUPPORTED: &[&str] = &["openid", "profile", "email", "offline_access"];

pub const SCOPE_OPENID: &str = "openid";

pub const CLAIMS_SUPPORTED: &[&str] = &[
  "iss",
  "aud",
  "exp",
  "jti",
  "scope",
  "acting_for",
  "sub",
  "name",
  "given_name",
  "family_name",
  "preferred_username",
  "email",
  "email_verified",
  "picture",
];

pub const ERR_REDIRECT_URI_EMPTY: &str = "redirect_uri provided is empty";
pub const ERR_SCOPE_MUST_INCLUDE_OPENID: &str = "invalid_request: scope must include 'openid'";
