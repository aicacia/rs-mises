use alloc::{string::String, vec};

use argon2::{Config, ThreadMode, Variant, Version};
use base64::engine::{Engine, general_purpose};
use getrandom::getrandom;

use crate::{CoreError, InvalidInput};

fn get_config() -> Config<'static> {
  Config {
    variant: Variant::Argon2id,
    version: Version::Version13,
    mem_cost: 19456,
    time_cost: 4,
    lanes: 2,
    thread_mode: ThreadMode::Parallel,
    secret: &[],
    ad: &[],
    hash_length: 32,
  }
}

/// Hash a password using Argon2id with a random salt.
///
/// # Arguments
///
/// * `password` - The plain-text password to hash
///
/// # Returns
///
/// A hashed password string suitable for storage
pub fn hash_password(password: &str) -> crate::Result<String> {
  let config = get_config();

  let mut salt = [0u8; 16];
  getrandom(&mut salt).map_err(|e| {
    CoreError::InvalidInput(InvalidInput::Other(alloc::format!(
      "failed to generate random salt: {}",
      e
    )))
  })?;

  let hash = argon2::hash_encoded(password.as_bytes(), &salt, &config).map_err(|e| {
    CoreError::InvalidInput(InvalidInput::Other(alloc::format!(
      "password hash failed: {}",
      e
    )))
  })?;

  Ok(hash)
}

/// Verify a plain-text password against a stored Argon2id hash.
///
/// # Arguments
///
/// * `password` - The plain-text password to verify
/// * `hash` - The stored Argon2id hash
///
/// # Returns
///
/// `true` if the password matches the hash, `false` otherwise
pub fn verify_password(password: &str, hash: &str) -> crate::Result<bool> {
  argon2::verify_encoded(hash, password.as_bytes()).map_err(|e| {
    CoreError::InvalidInput(InvalidInput::Other(alloc::format!(
      "password verification failed: {}",
      e
    )))
  })
}

/// Generate a random secret of specified byte length, encoded in base64.
///
/// # Arguments
///
/// * `size` - Number of random bytes to generate
///
/// # Returns
///
/// A base64-encoded random secret string
///
/// # Errors
///
/// Returns an error if `size` is 0 or if random generation fails.
pub fn generate_secret(size: usize) -> crate::Result<String> {
  if size == 0 {
    return Err(CoreError::InvalidInput(InvalidInput::Other(
      "secret size must be greater than 0".into(),
    )));
  }
  let mut bytes = vec![0u8; size];

  getrandom(&mut bytes).map_err(|e| {
    CoreError::InvalidInput(InvalidInput::Other(alloc::format!(
      "failed to generate random secret: {}",
      e
    )))
  })?;

  Ok(general_purpose::STANDARD_NO_PAD.encode(&bytes))
}
