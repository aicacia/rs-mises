use alloc::string::String;
use alloc::{format, vec::Vec};

use base64::{Engine, prelude::BASE64_URL_SAFE};
use serde::{Deserialize, Serialize};

use mises_key::Key;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct KeyMeta {
  pub public_key: String,
  /// Base64 URL-safe encoded secret/seed bytes (optional).
  pub private_key: Option<String>,
  // BIP32 derivation path None for master keys. i.e. "m/44'/0'/0'/0/0"
  pub derivation_path: Option<String>,
}

impl From<Key> for KeyMeta {
  fn from(key: Key) -> Self {
    // create public key
    let (_sk, vk) = key.secp256k1_keypair().expect("create keypair");
    let encoded_point = vk.to_encoded_point(false);
    let public_key = BASE64_URL_SAFE.encode(encoded_point.as_bytes());

    // private seed if present
    let private_key = key
      .seed_bytes()
      .map(|s| BASE64_URL_SAFE.encode(s.as_slice()));

    // optional derivation path from key
    let derivation_path = key.derivation_path();

    KeyMeta {
      public_key,
      private_key,
      derivation_path,
    }
  }
}

impl TryFrom<KeyMeta> for Key {
  type Error = mises_key::KeyError;

  fn try_from(km: KeyMeta) -> Result<Self, Self::Error> {
    // We must have a seed to reconstruct a Key
    if let Some(b64) = km.private_key {
      // decode base64 first
      let bytes = match BASE64_URL_SAFE.decode(b64.as_bytes()) {
        Ok(b) => b,
        Err(_) => {
          // If no derivation path, this is a master key — invalid seed
          if km.derivation_path.is_none() {
            return Err(mises_key::KeyError::InvalidSeed);
          } else {
            // If a derivation path exists, the combination is invalid
            return Err(mises_key::KeyError::InvalidKey);
          }
        }
      };

      // construct base Key from seed bytes
      let base_key = Key::from(bytes);

      // if derivation path is present, derive the child key using the path
      if let Some(dp) = km.derivation_path {
        match base_key.child_from_derivation_path(dp) {
          Ok(derived) => Ok(derived),
          Err(_) => Err(mises_key::KeyError::InvalidKey),
        }
      } else {
        Ok(base_key)
      }
    } else {
      Err(mises_key::KeyError::MissingSeed)
    }
  }
}

impl KeyMeta {
  /// Decode the base64url-encoded public key into raw bytes.
  /// Returns a `crate::Result<Vec<u8>>` so callers get `CoreError` on failure.
  pub fn to_bytes(&self) -> crate::Result<Vec<u8>> {
    // Try the prelude engine first, fall back to the NO_PAD engine on error.
    if let Ok(b) = BASE64_URL_SAFE.decode(self.public_key.as_bytes()) {
      return Ok(b);
    }

    match base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(self.public_key.as_bytes()) {
      Ok(b) => Ok(b),
      Err(e) => Err(crate::CoreError::InvalidInput(crate::InvalidInput::Other(
        format!("invalid public_key base64: {}", e),
      ))),
    }
  }

  /// If this `KeyMeta` represents an uncompressed EC public point (0x04 || X || Y),
  /// returns `(x_b64url, y_b64url)`. Returns `None` for other formats or decoding failures.
  pub fn ec_coords_b64(&self) -> Option<(String, String)> {
    // Try to decode using the prelude engine, fall back to NO_PAD engine when necessary.
    let bytes = if let Ok(b) = BASE64_URL_SAFE.decode(self.public_key.as_bytes()) {
      b
    } else if let Ok(b) =
      base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(self.public_key.as_bytes())
    {
      b
    } else {
      return None;
    };

    if bytes.len() == 65 && bytes[0] == 0x04 {
      let xb = &bytes[1..33];
      let yb = &bytes[33..65];
      Some((
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(xb),
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(yb),
      ))
    } else {
      None
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn keymeta_to_bytes_and_coords() {
    // build uncompressed point [0x04, 1*32, 2*32]
    let mut bytes = Vec::with_capacity(65);
    bytes.push(0x04);
    bytes.extend([1u8; 32]);
    bytes.extend([2u8; 32]);
    let pub_b64 = BASE64_URL_SAFE.encode(bytes.as_slice());

    let km = KeyMeta {
      public_key: pub_b64.clone(),
      private_key: None,
      derivation_path: None,
    };

    let decoded = km.to_bytes().expect("should decode");
    assert_eq!(decoded, bytes);

    let coords = km.ec_coords_b64().expect("coords should exist");
    assert_eq!(
      coords.0,
      base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&[1u8; 32])
    );
    assert_eq!(
      coords.1,
      base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&[2u8; 32])
    );
  }
}
