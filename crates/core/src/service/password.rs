use alloc::string::String;

use argon2::{Config, ThreadMode, Variant, Version};
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

pub fn verify_password(password: &str, hash: &str) -> crate::Result<bool> {
  argon2::verify_encoded(hash, password.as_bytes()).map_err(|e| {
    CoreError::InvalidInput(InvalidInput::Other(alloc::format!(
      "password verification failed: {}",
      e
    )))
  })
}
