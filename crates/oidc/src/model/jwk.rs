extern crate alloc;

use alloc::{
  format,
  string::{String, ToString},
  vec::Vec,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL_SAFE};
use core::str::FromStr;
use jsonwebtoken::jwk::{
  EllipticCurve as JwtEllipticCurve, KeyAlgorithm as JwtKeyAlgorithm,
  PublicKeyUse as JwtPublicKeyUse,
};
use k256::ecdsa::{
  Signature as KSignature,
  signature::{Signer, Verifier},
};
use mises_core::model::keys::KeyMeta;
use mises_key::Key;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// JSON Web Key (minimal fields used by the client)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Jwk {
  pub kty: KeyType,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub r#use: Option<KeyUse>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub alg: Option<Alg>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub kid: Option<String>,

  // RSA
  #[serde(skip_serializing_if = "Option::is_none")]
  pub n: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub e: Option<String>,

  // EC
  #[serde(skip_serializing_if = "Option::is_none")]
  pub crv: Option<Crv>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub x: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub y: Option<String>,
}

impl Jwk {
  /// Sign `claims` with ES256K using the provided `KeyMeta` (must contain the private seed)
  /// Returns a compact JWT string.
  pub fn sign_es256k<T: serde::Serialize>(
    &self,
    claims: &T,
    key_meta: &KeyMeta,
  ) -> Result<String, crate::error::OidcError> {
    // reconstruct Key from KeyMeta (requires private seed)
    let key = Key::try_from(key_meta.clone()).map_err(|_| crate::error::OidcError::Other(None))?;
    let (sk, _vk) = key
      .secp256k1_keypair()
      .map_err(|_| crate::error::OidcError::Other(None))?;

    // header
    let mut header = serde_json::map::Map::new();
    header.insert(
      "alg".to_string(),
      serde_json::Value::String("ES256K".to_string()),
    );
    header.insert(
      "typ".to_string(),
      serde_json::Value::String("JWT".to_string()),
    );
    if let Some(kid) = &self.kid {
      header.insert("kid".to_string(), serde_json::Value::String(kid.clone()));
    }

    // encode header and payload
    let header_json = serde_json::Value::Object(header);
    let header_bytes =
      serde_json::to_vec(&header_json).map_err(|_| crate::error::OidcError::Other(None))?;
    let payload_bytes =
      serde_json::to_vec(claims).map_err(|_| crate::error::OidcError::Other(None))?;

    let header_b64 = BASE64_URL_SAFE.encode(&header_bytes);
    let payload_b64 = BASE64_URL_SAFE.encode(&payload_bytes);

    let signing_input = [header_b64.as_bytes(), b".", payload_b64.as_bytes()].concat();

    // sign using k256 (ECDSA over secp256k1). Use signature's raw bytes (r||s) when possible.
    let sig: KSignature = sk.sign(&signing_input);

    // k256's `Signature` supports `to_vec()`/`to_bytes()`; prefer compact 64-byte (r||s)
    let sig_bytes = sig.to_bytes();
    let sig_compact: &[u8] = sig_bytes.as_ref();

    let sig_b64 = BASE64_URL_SAFE.encode(sig_compact);

    Ok(format!("{}.{}.{}", header_b64, payload_b64, sig_b64))
  }

  /// Verify an ES256K-signed compact JWT and deserialize the payload into `T`.
  pub fn verify_es256k<T: DeserializeOwned>(
    &self,
    token: &str,
  ) -> Result<T, crate::error::OidcError> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
      return Err(crate::error::OidcError::InvalidRequest(Some(
        "invalid token format".into(),
      )));
    }

    let signing_input = [parts[0].as_bytes(), b".", parts[1].as_bytes()].concat();

    // decode signature
    let sig_bytes = BASE64_URL_SAFE.decode(parts[2].as_bytes()).map_err(|_| {
      crate::error::OidcError::InvalidRequest(Some("invalid signature encoding".into()))
    })?;

    // reassemble verifying key from x/y
    let x_b64 = self.x.as_ref().ok_or_else(|| {
      crate::error::OidcError::InvalidRequest(Some("missing x coordinate".into()))
    })?;
    let y_b64 = self.y.as_ref().ok_or_else(|| {
      crate::error::OidcError::InvalidRequest(Some("missing y coordinate".into()))
    })?;

    let x = BASE64_URL_SAFE
      .decode(x_b64.as_bytes())
      .map_err(|_| crate::error::OidcError::InvalidRequest(Some("invalid x coordinate".into())))?;
    let y = BASE64_URL_SAFE
      .decode(y_b64.as_bytes())
      .map_err(|_| crate::error::OidcError::InvalidRequest(Some("invalid y coordinate".into())))?;

    // uncompressed point: 0x04 || x || y
    if x.len() != 32 || y.len() != 32 {
      return Err(crate::error::OidcError::InvalidRequest(Some(
        "invalid coordinate sizes".into(),
      )));
    }

    let mut encoded_point = Vec::with_capacity(65);
    encoded_point.push(0x04);
    encoded_point.extend_from_slice(&x);
    encoded_point.extend_from_slice(&y);

    let vk = k256::ecdsa::VerifyingKey::from_sec1_bytes(&encoded_point)
      .map_err(|_| crate::error::OidcError::InvalidRequest(Some("invalid public key".into())))?;

    // parse signature from compact bytes
    let sig = KSignature::from_bytes(sig_bytes.as_slice().into()).map_err(|_| {
      crate::error::OidcError::InvalidRequest(Some("invalid signature format".into()))
    })?;

    vk.verify(&signing_input, &sig).map_err(|_| {
      crate::error::OidcError::InvalidRequest(Some("signature verification failed".into()))
    })?;

    // decode payload
    let payload_bytes = BASE64_URL_SAFE.decode(parts[1].as_bytes()).map_err(|_| {
      crate::error::OidcError::InvalidRequest(Some("invalid payload encoding".into()))
    })?;

    let payload: T = serde_json::from_slice(&payload_bytes)
      .map_err(|_| crate::error::OidcError::InvalidRequest(Some("invalid payload".into())))?;

    Ok(payload)
  }
}

/// Key Type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyType {
  EC,
  Rsa,
  Other(String),
}

impl serde::Serialize for KeyType {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let s = match self {
      KeyType::EC => "EC",
      KeyType::Rsa => "RSA",
      KeyType::Other(o) => o.as_str(),
    };
    serializer.serialize_str(s)
  }
}

impl<'de> serde::Deserialize<'de> for KeyType {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let s = String::deserialize(deserializer)?;
    Ok(match s.as_str() {
      "EC" => KeyType::EC,
      "RSA" => KeyType::Rsa,
      other => KeyType::Other(other.to_string()),
    })
  }
}

/// Key use (sig) — use jsonwebtoken's `PublicKeyUse`
pub type KeyUse = JwtPublicKeyUse;

/// Algorithm wrapper — use jsonwebtoken's `KeyAlgorithm` when possible, fallback to other strings
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Alg {
  Jwt(JwtKeyAlgorithm),
  Other(String),
}

impl serde::Serialize for Alg {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    match self {
      Alg::Jwt(k) => serde::Serialize::serialize(k, serializer),
      Alg::Other(s) => serializer.serialize_str(s.as_str()),
    }
  }
}

impl<'de> serde::Deserialize<'de> for Alg {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let s = String::deserialize(deserializer)?;
    // Try to parse as known KeyAlgorithm
    if let Ok(k) = JwtKeyAlgorithm::from_str(s.as_str()) {
      Ok(Alg::Jwt(k))
    } else {
      Ok(Alg::Other(s))
    }
  }
}

/// Curve wrapper — use jsonwebtoken's `EllipticCurve` when possible, fallback to other strings
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Crv {
  Jwt(JwtEllipticCurve),
  Other(String),
}

impl serde::Serialize for Crv {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    match self {
      Crv::Jwt(c) => serde::Serialize::serialize(c, serializer),
      Crv::Other(s) => serializer.serialize_str(s.as_str()),
    }
  }
}

impl<'de> serde::Deserialize<'de> for Crv {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let s = String::deserialize(deserializer)?;
    match s.as_str() {
      "P-256" | "P-384" | "P-521" | "Ed25519" => {
        // Deserialize via JwtEllipticCurve's Deserialize
        if let Ok(c) = JwtEllipticCurve::deserialize(serde::de::IntoDeserializer::<
          serde::de::value::Error,
        >::into_deserializer(s.clone()))
        {
          Ok(Crv::Jwt(c))
        } else {
          Ok(Crv::Other(s))
        }
      }
      other => Ok(Crv::Other(other.to_string())),
    }
  }
}

impl From<KeyMeta> for Jwk {
  fn from(km: KeyMeta) -> Self {
    let mut crv = None;
    let mut x = None;
    let mut y = None;

    // Use KeyMeta helpers (no base64 dependency here)
    if let Some((xb64, yb64)) = km.ec_coords_b64() {
      crv = Some(Crv::Other(String::from("secp256k1")));
      x = Some(xb64);
      y = Some(yb64);
    }

    Jwk {
      kty: KeyType::EC,
      r#use: Some(JwtPublicKeyUse::Signature),
      // jsonwebtoken does not support ES256K so store as Other
      alg: Some(Alg::Other(String::from("ES256K"))),
      kid: Some(km.public_key),
      n: None,
      e: None,
      crv,
      x,
      y,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use alloc::vec;

  #[test]
  fn from_keymeta_ec_unpack_coords() {
    // precomputed base64url of [0x04, 1*32, 2*32]
    let pub_b64 =
      "BAEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI"
        .to_string();

    let km = KeyMeta {
      public_key: pub_b64.clone(),
      private_key: None,
      derivation_path: None,
    };
    // debug: ensure KeyMeta can decode to bytes and coords
    // verify base64 decode using local engine
    let direct = BASE64_URL_SAFE.decode(pub_b64.as_bytes());
    assert!(direct.is_ok());

    let decoded = km.to_bytes();
    assert!(decoded.is_ok());
    let coords = km.ec_coords_b64();
    assert!(coords.is_some());
    let jwk = Jwk::from(km);

    assert_eq!(jwk.kty, KeyType::EC);
    assert_eq!(jwk.crv, Some(Crv::Other("secp256k1".to_string())));
    assert_eq!(
      jwk.x,
      Some("AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE".to_string())
    );
    assert_eq!(
      jwk.y,
      Some("AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI".to_string())
    );
    assert_eq!(jwk.kid, Some(pub_b64));
  }

  #[test]
  fn es256k_sign_verify_roundtrip() {
    use super::*;
    use crate::model::IdTokenClaims;

    // seed of 32 bytes for deterministic key
    let seed = vec![1u8; 32];
    let key = Key::from(seed);
    let km = KeyMeta::from(key.clone());
    let jwk = Jwk::from(km.clone());

    let claims = IdTokenClaims {
      iss: "https://example.com".to_string(),
      sub: "user123".to_string(),
      aud: vec!["client1".to_string()],
      exp: 9999999999u64,
      iat: None,
      jti: None,
      acting_for: None,
      scope: None,
    };

    let token = jwk
      .sign_es256k(&claims, &km)
      .expect("signing should succeed");
    let decoded: IdTokenClaims = jwk
      .verify_es256k(&token)
      .expect("verification should succeed");
    assert_eq!(decoded, claims);
  }
}
