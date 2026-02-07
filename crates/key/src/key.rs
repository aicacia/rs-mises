#[cfg(not(feature = "std"))]
use alloc::{format, string::String, vec, vec::Vec};

use core::str::FromStr;

use crate::KeyError;
use bip32::{ChildNumber, DerivationPath, XPrv};
use bip39::Mnemonic;
use ed25519_dalek::{
  Keypair as Ed25519Keypair, PublicKey as Ed25519PublicKey, SecretKey as Ed25519SecretKey,
};
use k256::ecdsa::{SigningKey as EcdsaSigningKey, VerifyingKey as EcdsaVerifyingKey};
use slip10::{
  Curve as Slip10Curve, derive_key_from_path as slip10_derive_key_from_path,
  path::BIP32Path as Slip10BIP32Path,
};

#[cfg(test)]
use ed25519_dalek::{Signer as EdSigner, Verifier as EdVerifier};
#[cfg(test)]
use k256::ecdsa::{signature::Signer as KSigner, signature::Verifier as KVerifier};
#[cfg(test)]
use zeroize::Zeroize;

#[derive(Debug, Clone)]
pub struct Key {
  xprv: XPrv,
  /// Path from the root (empty for root keys)
  children: Vec<ChildNumber>,
  /// Optional normalized seed bytes used to construct this key (if available).
  seed: Option<Vec<u8>>,
}

impl From<Mnemonic> for Key {
  fn from(mnemonic: Mnemonic) -> Self {
    let seed = mnemonic.to_seed_normalized("").to_vec();
    let xprv = XPrv::new(seed.as_slice()).expect("create xprv");
    Self {
      xprv,
      children: Vec::new(),
      seed: Some(seed),
    }
  }
}

impl From<Vec<u8>> for Key {
  fn from(bytes: Vec<u8>) -> Self {
    let xprv = XPrv::new(bytes.as_slice()).expect("create xprv");
    Self {
      xprv,
      children: Vec::new(),
      seed: Some(bytes),
    }
  }
}

impl Key {
  #[cfg(feature = "std")]
  pub fn new_master() -> Result<Self, KeyError> {
    let mut entropy = [0u8; 32];
    getrandom::fill(&mut entropy)?;
    Self::from_entropy(&entropy)
  }

  pub fn from_entropy(entropy: &[u8]) -> Result<Self, KeyError> {
    let mnemonic = Mnemonic::from_entropy(entropy)?;
    Ok(Self::from(mnemonic))
  }

  pub fn child_number(&self, child: ChildNumber) -> Result<Self, KeyError> {
    let mut xprv = self.xprv.clone();
    xprv = xprv.derive_child(child)?;

    let mut children = self.children.clone();
    children.push(child);

    Ok(Self {
      xprv,
      children,
      seed: self.seed.clone(),
    })
  }

  pub fn child_from_derivation_path<S: AsRef<str>>(&self, path: S) -> Result<Self, KeyError> {
    let dp = DerivationPath::from_str(path.as_ref())?;
    let mut xprv = self.xprv.clone();
    let mut children = self.children.clone();

    for cn in dp.into_iter() {
      xprv = xprv.derive_child(cn)?;
      children.push(cn);
    }

    Ok(Self {
      xprv,
      children,
      seed: self.seed.clone(),
    })
  }

  pub fn child_from_name<S: AsRef<str>>(&self, name: S) -> Result<Self, KeyError> {
    let cn = parse_child_number(name)?;
    self.child_number(cn)
  }

  pub fn extended_private_key(&self) -> Result<XPrv, KeyError> {
    Ok(self.xprv.clone())
  }

  /// Return a copy of the normalized seed bytes if available.
  pub fn seed_bytes(&self) -> Option<Vec<u8>> {
    self.seed.clone()
  }

  /// Return a derivation path string (e.g. "m/44'/0'") if this key has children.
  pub fn derivation_path(&self) -> Option<String> {
    if self.children.is_empty() {
      return None;
    }
    let mut parts: Vec<String> = Vec::new();
    for cn in self.children.iter() {
      parts.push(format!("{}", cn));
    }
    Some(format!("m/{}", parts.join("/")))
  }

  pub fn secp256k1_secret_bytes(&self) -> Result<Vec<u8>, KeyError> {
    let sk = self.xprv.private_key();
    Ok(sk.to_bytes().to_vec())
  }

  pub fn secp256k1_keypair(&self) -> Result<(EcdsaSigningKey, EcdsaVerifyingKey), KeyError> {
    let sk_bytes = self.secp256k1_secret_bytes()?;
    let signing_key = EcdsaSigningKey::from_slice(&sk_bytes)?;
    let verifying_key = *signing_key.verifying_key();
    Ok((signing_key, verifying_key))
  }

  pub fn ed25519_secret_bytes(&self) -> Result<Vec<u8>, KeyError> {
    // Node-only keys derive ed25519 from this node's private key bytes via SLIP-0010.
    let seed = self.xprv.private_key().to_bytes();
    let path = Slip10BIP32Path::from(vec![]);
    let key = slip10_derive_key_from_path(seed.as_ref(), Slip10Curve::Ed25519, &path)?;
    Ok(key.key.to_vec())
  }

  pub fn ed25519_keypair(&self) -> Result<Ed25519Keypair, KeyError> {
    let sk = self.ed25519_secret_bytes()?;
    let secret = Ed25519SecretKey::from_bytes(&sk)?;
    let public = Ed25519PublicKey::from(&secret);
    Ok(Ed25519Keypair { secret, public })
  }
}

fn parse_child_number<S: AsRef<str>>(s: S) -> Result<ChildNumber, KeyError> {
  let s = s.as_ref().trim();
  if s.is_empty() {
    return Err(KeyError::EmptyChild);
  }

  let (base, hardened) = if let Some(b) = s.strip_suffix('\'') {
    (b, true)
  } else if let Some(b) = s.strip_suffix('h') {
    (b, true)
  } else if let Some(b) = s.strip_suffix('H') {
    (b, true)
  } else {
    (s, false)
  };

  let base = base.trim();

  if let Ok(idx) = base.parse::<u32>() {
    return Ok(ChildNumber::new(idx, hardened)?);
  }

  let hash = crc32fast::hash(base.as_bytes());
  let index = hash & 0x7FFFFFFF;

  Ok(ChildNumber::new(index, hardened)?)
}

#[cfg(test)]
mod tests {
  use super::*;

  static TEST_ENTROPY: [u8; 32] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F,
  ];

  #[test]
  fn key_generate() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");
    // root keys are node-only and do not retain the seed
    assert_eq!(key.children.len(), 0);
  }

  #[test]
  fn key_secp256k1_secret_bytes_and_keypair() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");
    let sk_bytes = key
      .secp256k1_secret_bytes()
      .expect("secp256k1 secret bytes");
    assert_eq!(sk_bytes.len(), 32);
    let (sk, vk) = key.secp256k1_keypair().expect("secp256k1 keypair");
    let msg = b"test message";
    let sig: k256::ecdsa::Signature = sk.sign(msg);
    assert!(vk.verify(msg, &sig).is_ok());
  }

  #[test]
  fn key_ed25519_secret_bytes_and_keypair() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");
    let sk_bytes = key.ed25519_secret_bytes().expect("ed25519 secret bytes");
    assert_eq!(sk_bytes.len(), 32);
    let kp = key.ed25519_keypair().expect("ed25519 keypair");
    let msg = b"test message";
    let sig = kp.sign(msg);
    assert!(kp.public.verify(msg, &sig).is_ok());
  }

  #[test]
  fn key_child_from_name_and_error() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");
    let derived = key.child_from_name("0'").expect("child from name");
    assert_eq!(derived.children.len(), 1);
    let err = key.child_from_name("");
    assert!(err.is_err());
  }

  #[test]
  fn key_child_from_derivation_path_and_error() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");
    let derived = key
      .child_from_derivation_path("m/44'/0'")
      .expect("child from path");
    assert_eq!(derived.children.len(), 2);
    let err = key.child_from_derivation_path("bad_path");
    assert!(err.is_err());
  }

  #[test]
  fn derive_key_updates_path() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");
    let derived = key
      .child_number(ChildNumber::new(0, true).expect("child number"))
      .expect("derive");

    assert_eq!(derived.children.len(), 1);
  }

  #[test]
  fn nested_derivation_matches_sequential() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");

    let derived_a = key
      .child_number(ChildNumber::new(44, true).expect("child number"))
      .expect("derive");
    let derived_ab = derived_a
      .child_number(ChildNumber::new(0, true).expect("child number"))
      .expect("derive");

    let mut xprv = key.extended_private_key().expect("create xprv");
    xprv = xprv
      .derive_child(ChildNumber::new(44, true).expect("child number"))
      .expect("derive 44'");
    xprv = xprv
      .derive_child(ChildNumber::new(0, true).expect("child number"))
      .expect("derive 0'");
    let seq_pk = xprv.private_key().to_bytes();

    let derived_ab_pk = derived_ab.secp256k1_secret_bytes().expect("derived pk");

    assert_eq!(derived_ab_pk, seq_pk.to_vec());
  }

  #[test]
  fn key_from_mnemonic() {
    let entropy = [0u8; 16];
    let mnemonic = Mnemonic::from_entropy(&entropy).expect("entropy to mnemonic");

    let seed = mnemonic.to_seed_normalized("").to_vec();
    let xprv = XPrv::new(seed.as_slice()).expect("create xprv");
    let expected = xprv.private_key().to_bytes().to_vec();

    let key = Key::from(mnemonic.clone());

    assert_eq!(key.secp256k1_secret_bytes().unwrap(), expected);
  }

  #[test]
  fn zeroize_manual() {
    let entropy = [0u8; 16];
    let mnemonic = Mnemonic::from_entropy(&entropy).expect("entropy to mnemonic");
    let mut v = mnemonic.to_seed_normalized("").to_vec();
    v.zeroize();
    assert!(v.iter().all(|&b| b == 0));
  }

  #[test]
  fn ed25519_sign_verify() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");
    let derived = key
      .child_number(ChildNumber::new(1, true).expect("child number"))
      .expect("derive");
    let kp = derived.ed25519_keypair().expect("keypair");

    let msg = b"hello world";
    let sig = kp.sign(msg);
    assert!(kp.public.verify(msg, &sig).is_ok());
  }

  #[test]
  fn secp256k1_sign_verify() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");
    let derived = key
      .child_number(ChildNumber::new(1, true).expect("child number"))
      .expect("derive");
    let (sk, vk) = derived.secp256k1_keypair().expect("keypair");

    let msg = b"hello world";
    let sig: k256::ecdsa::Signature = sk.sign(msg);
    assert!(vk.verify(msg, &sig).is_ok());
  }
}
