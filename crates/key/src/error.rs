#[derive(Debug, thiserror::Error)]
pub enum KeyError {
  #[error("BIP32 error: {0}")]
  Bip32(bip32::Error),
  #[error("SLIP10 error: {0}")]
  Slip10(slip10::Error),
  #[error("Ed25519 error: {0}")]
  Ed25519(#[from] ed25519_dalek::SignatureError),
  #[cfg(feature = "std")]
  #[error("Random generation error: {0}")]
  Random(#[from] getrandom::Error),
  #[error("BIP39 error: {0}")]
  Bip39(bip39::Error),
  #[error("Empty child index provided")]
  EmptyChild,
  #[error("Invalid private key length")]
  InvalidPrivateKeyLength,
  #[error("Invalid seed or failed to decode (master key)")]
  InvalidSeed,
  #[error("Seed missing")]
  MissingSeed,
  #[error("Invalid key: seed + derivation path failed to construct a key")]
  InvalidKey,
}

impl From<bip32::Error> for KeyError {
  fn from(err: bip32::Error) -> Self {
    KeyError::Bip32(err)
  }
}

impl From<slip10::Error> for KeyError {
  fn from(err: slip10::Error) -> Self {
    KeyError::Slip10(err)
  }
}

impl From<bip39::Error> for KeyError {
  fn from(err: bip39::Error) -> Self {
    KeyError::Bip39(err)
  }
}
