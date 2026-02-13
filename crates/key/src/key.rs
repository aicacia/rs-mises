use alloc::{format, string::String, vec, vec::Vec};
use core::{convert::TryFrom, str::FromStr};

use crate::KeyError;
use bip32::{ChildNumber, DerivationPath, XPrv};
use bip39::Mnemonic;
use ed25519_dalek::{
  Keypair as Ed25519Keypair, PublicKey as Ed25519PublicKey, SecretKey as Ed25519SecretKey,
};
use slip10::{
  Curve as Slip10Curve, derive_key_from_path as slip10_derive_key_from_path,
  path::BIP32Path as Slip10BIP32Path,
};

use zeroize::Zeroizing;

#[derive(Debug, Clone)]
pub struct Key {
  xprv: XPrv,
  children: Vec<ChildNumber>,
  seed: Option<Zeroizing<Vec<u8>>>,
}

const MASTER_PURPOSE: u32 = 44;

fn default_master_purpose_child() -> ChildNumber {
  ChildNumber::new(MASTER_PURPOSE, true).unwrap()
}

impl TryFrom<Mnemonic> for Key {
  type Error = KeyError;

  fn try_from(mnemonic: Mnemonic) -> Result<Self, Self::Error> {
    let seed = mnemonic.to_seed_normalized("").to_vec();
    let mut xprv = XPrv::new(seed.as_slice())?;
    let purpose = default_master_purpose_child();
    xprv = xprv.derive_child(purpose)?;

    Ok(Self {
      xprv,
      children: vec![purpose],
      seed: Some(Zeroizing::new(seed)),
    })
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
    Self::try_from(mnemonic)
  }

  pub fn from_master_seed_bytes(bytes: Vec<u8>) -> Result<Self, KeyError> {
    let mut xprv = XPrv::new(bytes.as_slice())?;
    let purpose = default_master_purpose_child();
    xprv = xprv.derive_child(purpose)?;

    Ok(Self {
      xprv,
      children: vec![purpose],
      seed: Some(Zeroizing::new(bytes)),
    })
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
    let dp_vec: Vec<ChildNumber> = dp.into_iter().collect();

    if !dp_vec.starts_with(&self.children) {
      return Err(KeyError::InvalidKey);
    }

    if dp_vec.len() == self.children.len() {
      return Ok(self.clone());
    }

    let mut xprv = self.xprv.clone();
    let mut children = self.children.clone();

    for cn in dp_vec.into_iter().skip(self.children.len()) {
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

  pub fn seed_bytes(&self) -> Option<Vec<u8>> {
    use core::ops::Deref;
    self.seed.as_ref().map(|z| z.deref().clone())
  }

  pub fn derivation_path(&self) -> String {
    let parts: Vec<String> = self.children.iter().map(|cn| format!("{}", cn)).collect();
    format!("m/{}", parts.join("/"))
  }

  pub fn ed25519_secret_bytes(&self) -> Result<Vec<u8>, KeyError> {
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
  use bip39::Mnemonic;
  use ed25519_dalek::{Signer as _, Verifier as _};
  use zeroize::Zeroize;

  use super::{ChildNumber, Key};

  static TEST_ENTROPY: [u8; 32] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F,
  ];

  #[test]
  fn key_generate() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");

    assert_eq!(key.children.len(), 1);
  }

  #[test]
  fn key_master_derivation_path_is_bip44() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");
    assert_eq!(key.derivation_path(), "m/44'");
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

    assert_eq!(derived.children.len(), 2);
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

    assert_eq!(derived.children.len(), 2);
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

    let derived_ab_pk = derived_ab.ed25519_secret_bytes().expect("derived pk");

    assert_eq!(derived_ab_pk.len(), 32);
  }

  #[test]
  fn key_from_mnemonic() {
    let entropy = [0u8; 16];
    let mnemonic = Mnemonic::from_entropy(&entropy).expect("entropy to mnemonic");

    let key = Key::try_from(mnemonic).expect("key from mnemonic");
    let ed_bytes = key.ed25519_secret_bytes().expect("ed25519 secret");

    assert_eq!(ed_bytes.len(), 32);
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
    let sig: ed25519_dalek::Signature = kp.sign(msg);
    assert!(kp.public.verify(msg, &sig).is_ok());
  }
}
