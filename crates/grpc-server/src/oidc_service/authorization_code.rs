use mises_async_kv_bytes::KeyValueStoreExecutor;
use serde::{Deserialize, Serialize};
use tonic::Status;
use uuid::Uuid;

const AUTHORIZATION_CODE_PREFIX: &[u8] = b"oidc:authorization_code:";
const AUTHORIZATION_CODE_LENGTH: usize = 32;
const AUTHORIZATION_CODE_TTL_SECONDS: i64 = 600;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationCodeData {
  pub client_id: Uuid,
  pub subject: Uuid,
  pub redirect_uri: String,
  pub scope: Option<String>,
  pub nonce: Option<String>,
  pub code_challenge: Option<String>,
  pub code_challenge_method: Option<String>,
  pub created_at: i64,
  pub expires_at: i64,
}

pub fn generate_authorization_code() -> String {
  let mut bytes = [0u8; AUTHORIZATION_CODE_LENGTH];
  getrandom::getrandom(&mut bytes).expect("failed to generate random bytes");
  base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes)
}

fn authorization_code_key(code: &str) -> Vec<u8> {
  let mut key = Vec::with_capacity(AUTHORIZATION_CODE_PREFIX.len() + code.len());
  key.extend_from_slice(AUTHORIZATION_CODE_PREFIX);
  key.extend_from_slice(code.as_bytes());
  key
}

pub async fn store_authorization_code<S>(
  store: &S,
  code: &str,
  data: AuthorizationCodeData,
) -> Result<(), Status>
where
  S: KeyValueStoreExecutor,
{
  let key = authorization_code_key(code);
  let value = serde_json::to_vec(&data)
    .map_err(|e| Status::internal(format!("serialization failed: {}", e)))?;
  store
    .put(key, value)
    .await
    .map_err(|e| Status::internal(format!("failed to store authorization code: {}", e)))?;
  Ok(())
}

pub async fn get_and_delete_authorization_code<S>(
  store: &S,
  code: &str,
) -> Result<Option<AuthorizationCodeData>, Status>
where
  S: KeyValueStoreExecutor,
{
  let key = authorization_code_key(code);
  let value = store
    .get(&key)
    .await
    .map_err(|e| Status::internal(format!("failed to retrieve authorization code: {}", e)))?;

  if let Some(bytes) = value {
    store
      .delete(key)
      .await
      .map_err(|e| Status::internal(format!("failed to delete authorization code: {}", e)))?;

    let data: AuthorizationCodeData = serde_json::from_slice(&bytes)
      .map_err(|e| Status::internal(format!("deserialization failed: {}", e)))?;
    Ok(Some(data))
  } else {
    Ok(None)
  }
}

impl AuthorizationCodeData {
  pub fn new(
    client_id: Uuid,
    subject: Uuid,
    redirect_uri: String,
    scope: Option<String>,
    nonce: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
  ) -> Self {
    let now = chrono::Utc::now().timestamp();
    Self {
      client_id,
      subject,
      redirect_uri,
      scope,
      nonce,
      code_challenge,
      code_challenge_method,
      created_at: now,
      expires_at: now + AUTHORIZATION_CODE_TTL_SECONDS,
    }
  }

  pub fn is_expired(&self) -> bool {
    chrono::Utc::now().timestamp() > self.expires_at
  }
}
