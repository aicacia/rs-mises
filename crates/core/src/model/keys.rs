use alloc::{format, string::String};

use base64::{Engine, prelude::BASE64_URL_SAFE};
use ed25519_dalek::pkcs8::EncodePrivateKey;
use jsonwebtoken::{DecodingKey, EncodingKey};
use mises_key::Key;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum KeyMaterial {
  #[default]
  Seed,
  Ed25519Secret,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct KeyMeta {
  pub public_key: String,
  pub private_key: Option<String>,
  pub derivation_path: String,
  pub key_material: KeyMaterial,
}

impl TryFrom<Key> for KeyMeta {
  type Error = mises_key::KeyError;

  fn try_from(key: Key) -> Result<Self, Self::Error> {
    let kp = key.ed25519_keypair()?;
    let public_key = BASE64_URL_SAFE.encode(kp.public.as_bytes());

    let private_key = key
      .seed_bytes()
      .map(|s| BASE64_URL_SAFE.encode(s.as_slice()));

    let derivation_path = key.derivation_path();

    Ok(KeyMeta {
      public_key,
      private_key,
      derivation_path,
      key_material: KeyMaterial::Seed,
    })
  }
}

impl TryFrom<KeyMeta> for Key {
  type Error = mises_key::KeyError;

  fn try_from(km: KeyMeta) -> Result<Self, Self::Error> {
    if let Some(b64) = km.private_key {
      if km.key_material != KeyMaterial::Seed {
        return Err(mises_key::KeyError::InvalidKey);
      }

      let bytes = match BASE64_URL_SAFE.decode(b64.as_bytes()) {
        Ok(b) => b,
        Err(_) => {
          return Err(mises_key::KeyError::InvalidKey);
        }
      };

      if km.derivation_path.is_empty() {
        Key::from_master_seed_bytes(bytes)
      } else {
        let base_key = Key::from_master_seed_bytes(bytes)?;
        base_key.child_from_derivation_path(&km.derivation_path)
      }
    } else {
      Err(mises_key::KeyError::MissingSeed)
    }
  }
}

impl KeyMeta {
  pub fn jwt_encoding_key(&self) -> crate::Result<EncodingKey> {
    let key = Key::try_from(self.clone()).map_err(|e| {
      crate::CoreError::InvalidInput(crate::InvalidInput::Other(format!(
        "failed to derive key from keymeta: {}",
        e
      )))
    })?;

    let signing_key = key.ed25519_signing_key().map_err(|e| {
      crate::CoreError::InvalidInput(crate::InvalidInput::Other(format!(
        "failed to derive ed25519 signing key: {}",
        e
      )))
    })?;

    let pkcs8 = signing_key.to_pkcs8_der().map_err(|e| {
      crate::CoreError::InvalidInput(crate::InvalidInput::Other(format!(
        "failed to encode ed25519 private key: {}",
        e
      )))
    })?;

    Ok(EncodingKey::from_ed_der(pkcs8.as_bytes()))
  }

  pub fn jwt_decoding_key(&self) -> crate::Result<DecodingKey> {
    let public_key_b64 = &self.public_key;
    let public_key_bytes = BASE64_URL_SAFE
      .decode(public_key_b64.as_bytes())
      .or_else(|_| {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(public_key_b64.as_bytes())
      })
      .or_else(|_| base64::engine::general_purpose::STANDARD.decode(public_key_b64.as_bytes()))
      .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(public_key_b64.as_bytes()))
      .map_err(|e| {
        crate::CoreError::InvalidInput(crate::InvalidInput::Other(format!(
          "invalid public key base64: {}",
          e
        )))
      })?;

    if public_key_bytes.len() != 32 {
      return Err(crate::CoreError::InvalidInput(crate::InvalidInput::Other(
        format!(
          "invalid ed25519 public key length: expected 32, got {}",
          public_key_bytes.len()
        ),
      )));
    }

    Ok(DecodingKey::from_ed_der(&public_key_bytes))
  }

  pub fn public_key_b64_unpadded(&self) -> Option<String> {
    let bytes = BASE64_URL_SAFE.decode(self.public_key.as_bytes()).ok()?;

    if bytes.len() == 32 {
      Some(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes))
    } else {
      None
    }
  }

  /// Alias for compatibility with existing code
  pub fn ec_coords_b64(&self) -> Option<String> {
    self.public_key_b64_unpadded()
  }
}

#[cfg(test)]
mod tests {
  use alloc::string::String;

  use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD, prelude::BASE64_URL_SAFE};

  use super::{KeyMaterial, KeyMeta};

  #[test]
  fn keymeta_public_key_b64_unpadded_and_coords() {
    let bytes = [1u8; 32];
    let pub_b64 = BASE64_URL_SAFE.encode(bytes.as_slice());

    let km = KeyMeta {
      public_key: pub_b64.clone(),
      private_key: None,
      derivation_path: String::from("m/44'"),
      key_material: KeyMaterial::Seed,
    };

    let coord = km.public_key_b64_unpadded().expect("coords should exist");
    assert_eq!(coord, URL_SAFE_NO_PAD.encode([1u8; 32]));

    let coord_alias = km.ec_coords_b64().expect("alias should work");
    assert_eq!(coord_alias, coord);
  }

  #[test]
  fn ec_coords_b64_non_ec_returns_none() {
    let bytes = [0u8; 64];
    let pub_b64 = BASE64_URL_SAFE.encode(bytes.as_slice());

    let km = KeyMeta {
      public_key: pub_b64,
      private_key: None,
      derivation_path: String::from("m/44/0'"),
      key_material: KeyMaterial::Seed,
    };

    assert!(km.public_key_b64_unpadded().is_none());
    assert!(km.ec_coords_b64().is_none());
  }
}
