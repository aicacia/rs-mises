use alloc::string::String;
use alloc::{format, vec::Vec};

use base64::{Engine, prelude::BASE64_URL_SAFE};
use serde::{Deserialize, Serialize};

use mises_key::Key;

/// Key metadata used to store information about a cryptographic key in a node.
///
/// - `public_key` is a base64url-encoded public point (uncompressed EC point expected)
/// - `private_key` is an optional base64url-encoded seed/secret used to reconstruct the key
/// - `derivation_path` is a BIP32 style derivation path (e.g. `m/44'`)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct KeyMeta {
  pub public_key: String,
  /// Base64 URL-safe encoded secret/seed bytes (optional).
  pub private_key: Option<String>,
  /// BIP32 derivation path (required). e.g. "m/44'"
  pub derivation_path: String,
}

impl TryFrom<Key> for KeyMeta {
  type Error = mises_key::KeyError;

  fn try_from(key: Key) -> Result<Self, Self::Error> {
    let (_sk, vk) = key.secp256k1_keypair()?;
    let encoded_point = vk.to_encoded_point(false);
    let public_key = BASE64_URL_SAFE.encode(encoded_point.as_bytes());

    let private_key = key
      .seed_bytes()
      .map(|s| BASE64_URL_SAFE.encode(s.as_slice()));

    let derivation_path = key.derivation_path();

    Ok(KeyMeta {
      public_key,
      private_key,
      derivation_path,
    })
  }
}

impl TryFrom<KeyMeta> for Key {
  type Error = mises_key::KeyError;

  fn try_from(km: KeyMeta) -> Result<Self, Self::Error> {
    if let Some(b64) = km.private_key {
      let bytes = match BASE64_URL_SAFE.decode(b64.as_bytes()) {
        Ok(b) => b,
        Err(_) => {
          return Err(mises_key::KeyError::InvalidKey);
        }
      };

      let base_key = Key::from(bytes);

      match base_key.child_from_derivation_path(km.derivation_path) {
        Ok(derived) => Ok(derived),
        Err(_) => Err(mises_key::KeyError::InvalidKey),
      }
    } else {
      Err(mises_key::KeyError::MissingSeed)
    }
  }
}

impl KeyMeta {
  /// Decode the base64url-encoded public key into raw bytes.
  ///
  /// Returns a `crate::Result<Vec<u8>>` so callers get `CoreError::InvalidInput` on failure.
  pub fn to_bytes(&self) -> crate::Result<Vec<u8>> {
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
  /// returns `(x_b64url, y_b64url)` as base64url strings.
  ///
  /// Returns `None` for other formats or decoding failures.
  pub fn ec_coords_b64(&self) -> Option<(String, String)> {
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
  use super::{BASE64_URL_SAFE, KeyMeta};
  use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

  #[test]
  fn keymeta_to_bytes_and_coords() {
    let mut bytes = Vec::with_capacity(65);
    bytes.push(0x04);
    bytes.extend([1u8; 32]);
    bytes.extend([2u8; 32]);
    let pub_b64 = BASE64_URL_SAFE.encode(bytes.as_slice());

    let km = KeyMeta {
      public_key: pub_b64.clone(),
      private_key: None,
      derivation_path: String::from("m/44'"),
    };

    let decoded = km.to_bytes().expect("should decode");
    assert_eq!(decoded, bytes);

    let coords = km.ec_coords_b64().expect("coords should exist");
    assert_eq!(coords.0, URL_SAFE_NO_PAD.encode([1u8; 32]));
    assert_eq!(coords.1, URL_SAFE_NO_PAD.encode([2u8; 32]));
  }

  #[test]
  fn to_bytes_invalid_base64_returns_err() {
    let km = KeyMeta {
      public_key: String::from("!!!notbase64!!!"),
      private_key: None,
      derivation_path: String::from("m/44'"),
    };

    let res = km.to_bytes();
    assert!(
      matches!(res, Err(crate::CoreError::InvalidInput(_))),
      "expected InvalidInput error"
    );
  }

  #[test]
  fn ec_coords_b64_non_ec_returns_none() {
    let bytes = [0u8; 64];
    let pub_b64 = BASE64_URL_SAFE.encode(bytes.as_slice());

    let km = KeyMeta {
      public_key: pub_b64,
      private_key: None,
      derivation_path: String::from("m/44'"),
    };

    assert!(km.ec_coords_b64().is_none());
  }
}
