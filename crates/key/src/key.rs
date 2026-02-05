#[cfg(not(feature = "std"))]
use alloc::{sync::Arc, vec, vec::Vec};
#[cfg(feature = "std")]
use std::sync::Arc;

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
use zeroize::Zeroizing;

#[cfg(test)]
use ed25519_dalek::{Signer as EdSigner, Verifier as EdVerifier};
#[cfg(test)]
use k256::ecdsa::{signature::Signer as KSigner, signature::Verifier as KVerifier};
#[cfg(test)]
use zeroize::Zeroize;

#[derive(Debug, Clone)]
pub struct MasterKey {
  bytes: Arc<Zeroizing<Vec<u8>>>,
}

impl From<Mnemonic> for MasterKey {
  fn from(mnemonic: Mnemonic) -> Self {
    Self {
      bytes: Arc::new(Zeroizing::new(mnemonic.to_seed_normalized("").to_vec())),
    }
  }
}

impl From<Vec<u8>> for MasterKey {
  fn from(bytes: Vec<u8>) -> Self {
    Self {
      bytes: Arc::new(Zeroizing::new(bytes)),
    }
  }
}

impl MasterKey {
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

  pub fn as_bytes(&self) -> &[u8] {
    &self.bytes
  }

  pub fn child_number(&self, child: ChildNumber) -> DerivedKey {
    DerivedKey {
      master_key: self.bytes.clone(),
      children: Vec::from([child]),
    }
  }

  pub fn child_from_derivation_path<S: AsRef<str>>(&self, path: S) -> Result<DerivedKey, KeyError> {
    let dp = DerivationPath::from_str(path.as_ref())?;
    let mut children = Vec::new();

    for cn in dp.into_iter() {
      children.push(cn);
    }

    Ok(DerivedKey {
      master_key: self.bytes.clone(),
      children,
    })
  }

  pub fn child_from_name<S: AsRef<str>>(&self, name: S) -> Result<DerivedKey, KeyError> {
    let cn = parse_child_number(name)?;
    Ok(self.child_number(cn))
  }

  pub fn extended_private_key(&self) -> Result<XPrv, KeyError> {
    let xprv = XPrv::new(self.bytes.as_slice())?;
    Ok(xprv)
  }

  pub fn secp256k1_secret_bytes(&self) -> Result<Vec<u8>, KeyError> {
    let xprv = self.extended_private_key()?;
    let sk = xprv.private_key();

    Ok(sk.to_bytes().to_vec())
  }

  pub fn secp256k1_keypair(&self) -> Result<(EcdsaSigningKey, EcdsaVerifyingKey), KeyError> {
    let sk_bytes = self.secp256k1_secret_bytes()?;
    let signing_key = EcdsaSigningKey::from_slice(&sk_bytes)?;
    let verifying_key = *signing_key.verifying_key();
    Ok((signing_key, verifying_key))
  }

  pub fn ed25519_secret_bytes(&self) -> Result<Vec<u8>, KeyError> {
    let path = Slip10BIP32Path::from(vec![]);
    let key = slip10_derive_key_from_path(self.bytes.as_slice(), Slip10Curve::Ed25519, &path)?;
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

#[derive(Debug, Clone)]
pub struct DerivedKey {
  master_key: Arc<Zeroizing<Vec<u8>>>,
  children: Vec<ChildNumber>,
}

impl DerivedKey {
  pub fn child_number(&self, child: ChildNumber) -> Self {
    let mut children = self.children.clone();

    children.push(child);

    Self {
      master_key: self.master_key.clone(),
      children,
    }
  }

  pub fn child_from_name<S: AsRef<str>>(&self, name: S) -> Result<Self, KeyError> {
    let cn = parse_child_number(name)?;
    Ok(self.child_number(cn))
  }

  pub fn extended_private_key(&self) -> Result<XPrv, KeyError> {
    let mut xprv = XPrv::new(self.master_key.as_slice())?;

    for cn in &self.children {
      xprv = xprv.derive_child(*cn)?;
    }

    Ok(xprv)
  }

  pub fn secp256k1_secret_bytes(&self) -> Result<Vec<u8>, KeyError> {
    let xprv = self.extended_private_key()?;
    let sk = xprv.private_key();

    Ok(sk.to_bytes().to_vec())
  }

  pub fn secp256k1_keypair(&self) -> Result<(EcdsaSigningKey, EcdsaVerifyingKey), KeyError> {
    let sk_bytes = self.secp256k1_secret_bytes()?;
    let signing_key = EcdsaSigningKey::from_slice(&sk_bytes)?;
    let verifying_key = *signing_key.verifying_key();
    Ok((signing_key, verifying_key))
  }

  pub fn ed25519_secret_bytes(&self) -> Result<Vec<u8>, KeyError> {
    let mut nums = Vec::new();
    for cn in &self.children {
      let mut idx = cn.index();
      if cn.is_hardened() {
        idx |= 0x80000000;
      }
      nums.push(idx);
    }

    let path = Slip10BIP32Path::from(nums);
    let key = slip10_derive_key_from_path(self.master_key.as_slice(), Slip10Curve::Ed25519, &path)?;
    Ok(key.key.to_vec())
  }

  pub fn ed25519_keypair(&self) -> Result<Ed25519Keypair, KeyError> {
    let sk = self.ed25519_secret_bytes()?;
    let secret = Ed25519SecretKey::from_bytes(&sk)?;
    let public = Ed25519PublicKey::from(&secret);
    Ok(Ed25519Keypair { secret, public })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  static TEST_ENTROPY: [u8; 32] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F,
  ];

  #[test]
  fn master_generate() {
    let master = MasterKey::from_entropy(&TEST_ENTROPY).expect("generate master key");
    assert_eq!(master.bytes.len(), 64);
  }

  #[test]
  fn master_as_bytes() {
    let master = MasterKey::from_entropy(&TEST_ENTROPY).expect("generate master key");
    let bytes = master.as_bytes();
    assert_eq!(bytes.len(), 64);
    assert_eq!(bytes, master.bytes.as_slice());
  }

  #[test]
  fn master_secp256k1_secret_bytes_and_keypair() {
    let master = MasterKey::from_entropy(&TEST_ENTROPY).expect("generate master key");
    let sk_bytes = master
      .secp256k1_secret_bytes()
      .expect("secp256k1 secret bytes");
    assert_eq!(sk_bytes.len(), 32);
    let (sk, vk) = master.secp256k1_keypair().expect("secp256k1 keypair");
    let msg = b"test message";
    let sig: k256::ecdsa::Signature = sk.sign(msg);
    assert!(vk.verify(msg, &sig).is_ok());
  }

  #[test]
  fn master_ed25519_secret_bytes_and_keypair() {
    let master = MasterKey::from_entropy(&TEST_ENTROPY).expect("generate master key");
    let sk_bytes = master.ed25519_secret_bytes().expect("ed25519 secret bytes");
    assert_eq!(sk_bytes.len(), 32);
    let kp = master.ed25519_keypair().expect("ed25519 keypair");
    let msg = b"test message";
    let sig = kp.sign(msg);
    assert!(kp.public.verify(msg, &sig).is_ok());
  }

  #[test]
  fn master_child_from_name_and_error() {
    let master = MasterKey::from_entropy(&TEST_ENTROPY).expect("generate master key");
    let derived = master.child_from_name("0'").expect("child from name");
    assert_eq!(derived.children.len(), 1);
    let err = master.child_from_name("");
    assert!(err.is_err());
  }

  #[test]
  fn master_child_from_derivation_path_and_error() {
    let master = MasterKey::from_entropy(&TEST_ENTROPY).expect("generate master key");
    let derived = master
      .child_from_derivation_path("m/44'/0'")
      .expect("child from path");
    assert_eq!(derived.children.len(), 2);
    let err = master.child_from_derivation_path("bad_path");
    assert!(err.is_err());
  }

  #[test]
  fn derive_key_updates_path() {
    let master = MasterKey::from_entropy(&TEST_ENTROPY).expect("generate master key");
    let derived = master.child_number(ChildNumber::new(0, true).expect("child number"));

    assert_eq!(derived.children.len(), 1);
  }

  #[test]
  fn nested_derivation_matches_sequential() {
    let master = MasterKey::from_entropy(&TEST_ENTROPY).expect("generate master key");

    let derived_a = master.child_number(ChildNumber::new(44, true).expect("child number"));
    let derived_ab = derived_a.child_number(ChildNumber::new(0, true).expect("child number"));

    let mut xprv = XPrv::new(master.bytes.as_ref().as_slice()).expect("create xprv");
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
  fn master_from_mnemonic() {
    let entropy = [0u8; 16];
    let mnemonic = Mnemonic::from_entropy(&entropy).expect("entropy to mnemonic");

    let master = MasterKey::from(mnemonic.clone());

    assert_eq!(
      master.bytes.as_ref().as_slice(),
      mnemonic.to_seed_normalized("").as_slice()
    );
  }

  #[test]
  fn zeroize_manual() {
    let master = MasterKey::from_entropy(&TEST_ENTROPY).expect("generate master key");
    let mut v = master.bytes.as_ref().to_vec();
    v.zeroize();
    assert!(v.iter().all(|&b| b == 0));
  }

  #[test]
  fn ed25519_derivation_matches_slip10() {
    let master = MasterKey::from_entropy(&TEST_ENTROPY).expect("generate master key");

    let derived = master
      .child_number(ChildNumber::new(44, true).expect("child number"))
      .child_number(ChildNumber::new(0, true).expect("child number"));

    let sk = derived.ed25519_secret_bytes().expect("derived sk");

    let nums = Vec::from([44 | 0x80000000, 0x80000000]);
    let path = Slip10BIP32Path::from(nums);
    let key = slip10_derive_key_from_path(
      master.bytes.as_ref().as_slice(),
      Slip10Curve::Ed25519,
      &path,
    )
    .expect("slip10 derive");

    assert_eq!(sk, key.key.to_vec());
  }

  #[test]
  fn ed25519_sign_verify() {
    let master = MasterKey::from_entropy(&TEST_ENTROPY).expect("generate master key");
    let derived = master.child_number(ChildNumber::new(1, true).expect("child number"));
    let kp = derived.ed25519_keypair().expect("keypair");

    let msg = b"hello world";
    let sig = kp.sign(msg);
    assert!(kp.public.verify(msg, &sig).is_ok());
  }

  #[test]
  fn secp256k1_sign_verify() {
    let master = MasterKey::from_entropy(&TEST_ENTROPY).expect("generate master key");
    let derived = master.child_number(ChildNumber::new(1, true).expect("child number"));
    let (sk, vk) = derived.secp256k1_keypair().expect("keypair");

    let msg = b"hello world";
    let sig: k256::ecdsa::Signature = sk.sign(msg);
    assert!(vk.verify(msg, &sig).is_ok());
  }
}
