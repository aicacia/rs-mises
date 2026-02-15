use alloc::{format, string::String, vec::Vec};

use base64::{Engine, prelude::BASE64_URL_SAFE};
use mises_key::Key;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct KeyMeta {
  pub public_key: String,
  pub private_key: Option<String>,
  pub derivation_path: String,
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

      let base_key = Key::from_master_seed_bytes(bytes)?;

      base_key.child_from_derivation_path(km.derivation_path)
    } else {
      Err(mises_key::KeyError::MissingSeed)
    }
  }
}

impl KeyMeta {
  fn decode_public_key_bytes(&self) -> Result<Vec<u8>, base64::DecodeError> {
    BASE64_URL_SAFE
      .decode(self.public_key.as_bytes())
      .or_else(|_| {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(self.public_key.as_bytes())
      })
      .or_else(|_| base64::engine::general_purpose::STANDARD.decode(self.public_key.as_bytes()))
      .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(self.public_key.as_bytes()))
  }

  pub fn decode_private_key(&self) -> crate::Result<Vec<u8>> {
    let private_key_b64 = self.private_key.as_ref().ok_or_else(|| {
      crate::CoreError::InvalidInput(crate::InvalidInput::Other("private key not present".into()))
    })?;

    BASE64_URL_SAFE
      .decode(private_key_b64.as_bytes())
      .or_else(|_| {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(private_key_b64.as_bytes())
      })
      .or_else(|_| base64::engine::general_purpose::STANDARD.decode(private_key_b64.as_bytes()))
      .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(private_key_b64.as_bytes()))
      .map_err(|e| {
        crate::CoreError::InvalidInput(crate::InvalidInput::Other(format!(
          "invalid private_key base64: {}",
          e
        )))
      })
  }

  pub fn to_bytes(&self) -> crate::Result<Vec<u8>> {
    self.decode_public_key_bytes().map_err(|e| {
      crate::CoreError::InvalidInput(crate::InvalidInput::Other(format!(
        "invalid public_key base64: {}",
        e
      )))
    })
  }

  pub fn ec_coords_b64(&self) -> Option<String> {
    let bytes = self.decode_public_key_bytes().ok()?;

    if bytes.len() == 32 {
      Some(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes))
    } else {
      None
    }
  }
}

#[cfg(test)]
mod tests {
  use alloc::string::String;

  use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD, prelude::BASE64_URL_SAFE};

  use super::KeyMeta;

  #[test]
  fn keymeta_to_bytes_and_coords() {
    let bytes = [1u8; 32];
    let pub_b64 = BASE64_URL_SAFE.encode(bytes.as_slice());

    let km = KeyMeta {
      public_key: pub_b64.clone(),
      private_key: None,
      derivation_path: String::from("m/44'"),
    };

    let decoded = km.to_bytes().expect("should decode");
    assert_eq!(decoded, bytes);

    let coord = km.ec_coords_b64().expect("coords should exist");
    assert_eq!(coord, URL_SAFE_NO_PAD.encode([1u8; 32]));
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
