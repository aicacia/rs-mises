#![cfg(feature = "in-memory")]

use base64::engine::{Engine, general_purpose};

use mises_core::service::password::{generate_secret, hash_password, verify_password};

#[tokio::test]
async fn hash_password_produces_valid_argon2_hash() {
  let password = "test_password_123";
  let hash = hash_password(password).expect("hash should succeed");

  assert!(!hash.is_empty());
  assert!(hash.starts_with("$argon2"), "hash should be argon2 format");
}

#[tokio::test]
async fn verify_password_succeeds_with_correct_password() {
  let password = "my_secure_password";
  let hash = hash_password(password).expect("hash should succeed");

  let is_valid = verify_password(password, &hash).expect("verification should succeed");
  assert!(is_valid, "correct password should verify");
}

#[tokio::test]
async fn verify_password_fails_with_wrong_password() {
  let password = "correct_password";
  let hash = hash_password(password).expect("hash should succeed");

  let is_valid = verify_password("wrong_password", &hash).expect("verification should succeed");
  assert!(!is_valid, "wrong password should not verify");
}

#[tokio::test]
async fn same_password_hashes_to_different_values() {
  let password = "duplicate_password";
  let hash1 = hash_password(password).expect("hash1 should succeed");
  let hash2 = hash_password(password).expect("hash2 should succeed");

  assert_ne!(hash1, hash2, "same password should produce different hashes due to random salt");

  let valid1 = verify_password(password, &hash1).expect("verification should succeed");
  let valid2 = verify_password(password, &hash2).expect("verification should succeed");
  assert!(valid1 && valid2, "both hashes should verify the same password");
}

#[tokio::test]
async fn generate_secret_produces_base64_string() {
  let secret = generate_secret(32).expect("secret generation should succeed");

  assert!(!secret.is_empty());

  let decoded = general_purpose::STANDARD_NO_PAD
    .decode(secret.as_bytes())
    .expect("secret should be valid base64");
  assert_eq!(decoded.len(), 32, "decoded secret should be 32 bytes");
}

#[tokio::test]
async fn generate_secret_with_different_sizes() {
  for size in [16, 32, 64, 128] {
    let secret = generate_secret(size).expect("secret generation should succeed");

    let decoded = general_purpose::STANDARD_NO_PAD
      .decode(secret.as_bytes())
      .expect("secret should be valid base64");
    assert_eq!(decoded.len(), size, "decoded secret should match requested size");
  }
}

#[tokio::test]
async fn generate_secret_fails_with_zero_size() {
  let result = generate_secret(0);

  match result {
    Err(mises_core::CoreError::InvalidInput(_)) => {}
    _ => panic!("expected InvalidInput error for zero size"),
  }
}

#[tokio::test]
async fn generate_secret_produces_different_values() {
  let secret1 = generate_secret(32).expect("secret1 generation should succeed");
  let secret2 = generate_secret(32).expect("secret2 generation should succeed");

  assert_ne!(secret1, secret2, "each call should produce a different secret");
}

#[tokio::test]
async fn verify_password_rejects_malformed_hash() {
  let password = "test_password";
  let malformed_hash = "not_a_valid_argon2_hash";

  let result = verify_password(password, malformed_hash);
  assert!(result.is_err(), "malformed hash should cause verification to fail");
}

#[tokio::test]
async fn empty_password_can_be_hashed_and_verified() {
  let password = "";
  let hash = hash_password(password).expect("empty password should hash");

  let is_valid = verify_password(password, &hash).expect("verification should succeed");
  assert!(is_valid, "empty password should verify");

  let is_invalid = verify_password("not_empty", &hash).expect("verification should succeed");
  assert!(!is_invalid, "non-empty password should not match empty password hash");
}
