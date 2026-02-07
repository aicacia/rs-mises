extern crate alloc;

use alloc::{
  format,
  string::{String, ToString},
  vec::Vec,
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL_SAFE};
use jsonwebtoken::jwk::{EllipticCurveKeyType, PublicKeyUse};
use k256::ecdsa::{
  Signature as KSignature,
  signature::{Signer, Verifier},
};
use mises_core::model::keys::KeyMeta;
use mises_key::Key;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Algorithm {
  ES256K,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Curve {
  #[serde(rename = "secp256k1")]
  SECP256K1,
}

/// JSON Web Key
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Jwk {
  pub kid: String,
  pub kty: EllipticCurveKeyType,
  pub r#use: PublicKeyUse,
  pub alg: Algorithm,
  pub crv: Curve,
  pub x: String,
  pub y: String,
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

    let header_b64 = Self::header_b64_for_alg_kid("ES256K", &self.kid)?;
    let payload_bytes =
      serde_json::to_vec(claims).map_err(|_| crate::error::OidcError::Other(None))?;
    let payload_b64 = BASE64_URL_SAFE.encode(&payload_bytes);

    let signing_input = [header_b64.as_bytes(), b".", payload_b64.as_bytes()].concat();

    // Sign using k256 ECDSA (secp256k1); prefer compact r||s bytes.
    let sig: KSignature = sk.sign(&signing_input);

    // Use compact 64-byte (r||s) via `to_bytes()`
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
    let sig_bytes = Self::decode_b64(parts[2], "invalid signature encoding")?;

    // reassemble verifying key from x/y
    let encoded_point = self.ec_encoded_point()?;

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
    let payload_bytes = Self::decode_b64(parts[1], "invalid payload encoding")?;

    let payload: T = serde_json::from_slice(&payload_bytes)
      .map_err(|_| crate::error::OidcError::InvalidRequest(Some("invalid payload".into())))?;

    Ok(payload)
  }

  // Decode base64url with a uniform error
  fn decode_b64(s: &str, msg: &'static str) -> Result<Vec<u8>, crate::error::OidcError> {
    BASE64_URL_SAFE
      .decode(s.as_bytes())
      .map_err(|_| crate::error::OidcError::InvalidRequest(Some(msg.into())))
  }

  // Build minimal header and return base64url-encoded header (for ES256K signing)
  fn header_b64_for_alg_kid(alg: &str, kid: &str) -> Result<String, crate::error::OidcError> {
    let mut header = serde_json::map::Map::new();
    header.insert(
      "alg".to_string(),
      serde_json::Value::String(alg.to_string()),
    );
    header.insert(
      "typ".to_string(),
      serde_json::Value::String("JWT".to_string()),
    );
    header.insert(
      "kid".to_string(),
      serde_json::Value::String(kid.to_string()),
    );

    let header_json = serde_json::Value::Object(header);
    let header_bytes =
      serde_json::to_vec(&header_json).map_err(|_| crate::error::OidcError::Other(None))?;
    Ok(BASE64_URL_SAFE.encode(&header_bytes))
  }

  fn ec_encoded_point(&self) -> Result<Vec<u8>, crate::error::OidcError> {
    let x = Self::decode_b64(&self.x, "invalid x coordinate")?;
    let y = Self::decode_b64(&self.y, "invalid y coordinate")?;

    if x.len() != 32 || y.len() != 32 {
      return Err(crate::error::OidcError::InvalidRequest(Some(
        "invalid coordinate sizes".into(),
      )));
    }

    let mut encoded_point = Vec::with_capacity(65);
    encoded_point.push(0x04);
    encoded_point.extend_from_slice(&x);
    encoded_point.extend_from_slice(&y);

    Ok(encoded_point)
  }
}

impl TryFrom<(uuid::Uuid, KeyMeta)> for Jwk {
  type Error = crate::error::OidcError;

  fn try_from((id, km): (uuid::Uuid, KeyMeta)) -> Result<Self, Self::Error> {
    // Extract EC coordinates (must be uncompressed point)
    if let Some((x_b64, y_b64)) = km.ec_coords_b64() {
      Ok(Jwk {
        kid: id.to_string(),
        kty: EllipticCurveKeyType::EC,
        r#use: PublicKeyUse::Signature,
        alg: Algorithm::ES256K,
        crv: Curve::SECP256K1,
        x: x_b64,
        y: y_b64,
      })
    } else {
      Err(crate::error::OidcError::InvalidRequest(Some(
        "public key must be uncompressed EC point".into(),
      )))
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use alloc::vec::Vec;

  #[test]
  fn jwk_from_keymeta_and_key() {
    // sample seed bytes
    let seed: Vec<u8> = (0u8..32u8).collect();

    // construct Key and KeyMeta
    let key = Key::from(seed.clone());
    let km = KeyMeta::from(key.clone());

    // TryFrom<(uuid::Uuid, KeyMeta)>
    let jwk = Jwk::try_from((uuid::Uuid::new_v4(), km.clone())).expect("convert from KeyMeta");

    assert_eq!(jwk.kty, EllipticCurveKeyType::EC);
    assert_eq!(jwk.r#use, PublicKeyUse::Signature);
    assert_eq!(jwk.alg, Algorithm::ES256K);
    assert_eq!(jwk.crv, Curve::SECP256K1);

    // decode x/y to ensure they are 32 bytes
    let xb = BASE64_URL_SAFE.decode(jwk.x.as_bytes()).expect("decode x");
    let yb = BASE64_URL_SAFE.decode(jwk.y.as_bytes()).expect("decode y");
    assert_eq!(xb.len(), 32);
    assert_eq!(yb.len(), 32);

    // TryFrom<(uuid::Uuid, KeyMeta)>
    let jwk2 = Jwk::try_from((uuid::Uuid::new_v4(), km)).expect("convert from KeyMeta");
    assert_eq!(jwk2.x, jwk.x);
    assert_eq!(jwk2.y, jwk.y);
  }
}
