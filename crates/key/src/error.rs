use core::fmt;

#[cfg(not(feature = "std"))]
use core::error::Error;

#[cfg(feature = "std")]
use std::error::Error as StdError;

#[derive(Debug)]
pub enum KeyError {
  Bip32(bip32::Error),
  Slip10(slip10::Error),
  Ed25519(ed25519_dalek::SignatureError),
  #[cfg(feature = "std")]
  Random(getrandom::Error),
  Bip39(bip39::Error),
  EmptyChild,
  InvalidPrivateKeyLength,
  InvalidSeed,
  MissingSeed,
  InvalidKey,
}

impl fmt::Display for KeyError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      KeyError::Bip32(err) => write!(f, "BIP32 error: {}", err),
      KeyError::Slip10(err) => write!(f, "SLIP10 error: {}", err),
      KeyError::Ed25519(err) => write!(f, "Ed25519 error: {}", err),
      #[cfg(feature = "std")]
      KeyError::Random(err) => write!(f, "Random generation error: {}", err),
      KeyError::Bip39(err) => write!(f, "BIP39 error: {}", err),
      KeyError::EmptyChild => write!(f, "Empty child index provided"),
      KeyError::InvalidPrivateKeyLength => write!(f, "Invalid private key length"),
      KeyError::InvalidSeed => write!(f, "Invalid seed or failed to decode (master key)"),
      KeyError::MissingSeed => write!(f, "Seed missing"),
      KeyError::InvalidKey => write!(
        f,
        "Invalid key: seed + derivation path failed to construct a key"
      ),
    }
  }
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

impl From<ed25519_dalek::SignatureError> for KeyError {
  fn from(err: ed25519_dalek::SignatureError) -> Self {
    KeyError::Ed25519(err)
  }
}

#[cfg(feature = "std")]
impl From<getrandom::Error> for KeyError {
  fn from(err: getrandom::Error) -> Self {
    KeyError::Random(err)
  }
}

impl From<bip39::Error> for KeyError {
  fn from(err: bip39::Error) -> Self {
    KeyError::Bip39(err)
  }
}

#[cfg(not(feature = "std"))]
impl Error for KeyError {}

#[cfg(feature = "std")]
impl StdError for KeyError {}
