use alloc::{format, string::String, vec, vec::Vec};
use core::{convert::TryFrom, str::FromStr};

use bip32::{ChildNumber, DerivationPath, PublicKey as Bip32PublicKey, XPrv, XPub};
use bip39::Mnemonic;
use ed25519_dalek::{SigningKey as Ed25519SigningKey, VerifyingKey as Ed25519VerifyingKey};
use slip10::{
  Curve as Slip10Curve, derive_key_from_path as slip10_derive_key_from_path,
  path::BIP32Path as Slip10BIP32Path,
};
use zeroize::Zeroizing;

use crate::KeyError;

/// Ed25519 keypair for compatibility (contains both public and private components)
#[derive(Debug, Clone)]
pub struct Ed25519Keypair {
  pub public: Ed25519VerifyingKey,
  pub secret: Ed25519SigningKey,
}

#[derive(Debug, Clone)]
pub struct Key {
  xprv: XPrv,
  children: Vec<ChildNumber>,
  seed: Option<Zeroizing<Vec<u8>>>,
  is_master: bool,
  master_fingerprint: Option<[u8; 4]>,
}

const MASTER_PURPOSE: u32 = 44;

fn default_master_purpose_child() -> ChildNumber {
  ChildNumber::new(MASTER_PURPOSE, true).expect("valid child number")
}

impl TryFrom<Mnemonic> for Key {
  type Error = KeyError;

  fn try_from(mnemonic: Mnemonic) -> Result<Self, Self::Error> {
    let seed = mnemonic.to_seed_normalized("").to_vec();
    let mut xprv = XPrv::new(seed.as_slice())?;
    let purpose = default_master_purpose_child();
    xprv = xprv.derive_child(purpose)?;

    let fingerprint = Self::compute_fingerprint(&xprv);

    Ok(Self {
      xprv,
      children: vec![purpose],
      seed: Some(Zeroizing::new(seed)),
      is_master: true,
      master_fingerprint: Some(fingerprint),
    })
  }
}

impl Key {
  #[cfg(feature = "std")]
  pub fn new_master() -> Result<Self, KeyError> {
    let mut entropy = [0u8; 32];
    getrandom::getrandom(&mut entropy)?;
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

    let fingerprint = Self::compute_fingerprint(&xprv);

    Ok(Self {
      xprv,
      children: vec![purpose],
      seed: Some(Zeroizing::new(bytes)),
      is_master: true,
      master_fingerprint: Some(fingerprint),
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
      seed: None,
      is_master: false,
      master_fingerprint: self.master_fingerprint,
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
      seed: None,
      is_master: false,
      master_fingerprint: self.master_fingerprint,
    })
  }

  pub fn child_from_name<S: AsRef<str>>(&self, name: S) -> Result<Self, KeyError> {
    let cn = parse_child_number(name)?;
    self.child_number(cn)
  }

  pub fn extended_private_key(&self) -> Result<XPrv, KeyError> {
    Ok(self.xprv.clone())
  }

  pub fn extended_public_key(&self) -> Result<XPub, KeyError> {
    Ok(self.xprv.public_key())
  }

  pub fn seed_bytes(&self) -> Option<Vec<u8>> {
    use core::ops::Deref;
    self.seed.as_ref().map(|z| z.deref().clone())
  }

  pub fn is_master(&self) -> bool {
    self.is_master
  }

  pub fn master_fingerprint(&self) -> Option<[u8; 4]> {
    self.master_fingerprint
  }

  pub fn verify_derived_from(&self, master_xpub: &XPub) -> bool {
    let master_fp = Self::compute_fingerprint_from_xpub(master_xpub);
    self.master_fingerprint == Some(master_fp)
  }

  fn compute_fingerprint(xprv: &XPrv) -> [u8; 4] {
    let xpub = xprv.public_key();
    Self::compute_fingerprint_from_xpub(&xpub)
  }

  fn compute_fingerprint_from_xpub(xpub: &XPub) -> [u8; 4] {
    let public_key_bytes = xpub.public_key().to_bytes();
    let hash = crc32fast::hash(public_key_bytes.as_ref());
    hash.to_le_bytes()
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

  pub fn ed25519_signing_key(&self) -> Result<Ed25519SigningKey, KeyError> {
    let sk = self.ed25519_secret_bytes()?;
    let sk_array: [u8; 32] = sk.try_into().map_err(|_| KeyError::InvalidKey)?;
    Ok(Ed25519SigningKey::from(sk_array))
  }

  pub fn ed25519_verifying_key(&self) -> Result<Ed25519VerifyingKey, KeyError> {
    let signing_key = self.ed25519_signing_key()?;
    Ok(signing_key.verifying_key())
  }

  pub fn ed25519_keypair(&self) -> Result<Ed25519Keypair, KeyError> {
    let secret = self.ed25519_signing_key()?;
    let public = secret.verifying_key();
    Ok(Ed25519Keypair { public, secret })
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
  use alloc::{
    string::{String, ToString},
    vec,
  };
  use bip39::Mnemonic;
  use ed25519_dalek::{Signer as _, Verifier as _};
  use serde::{Deserialize, Serialize};
  use zeroize::Zeroize;

  use super::{ChildNumber, Key, KeyError};

  #[derive(Debug, Serialize, Deserialize)]
  struct Claims {
    sub: String,
    exp: usize,
  }

  static TEST_ENTROPY: [u8; 32] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F,
  ];

  #[test]
  fn key_generate() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");

    assert_eq!(key.children.len(), 1);
    assert_eq!(key.derivation_path(), "m/44'");
    assert!(key.seed_bytes().is_some());
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
    let sig = kp.secret.sign(msg);
    assert!(kp.public.verify(msg, &sig).is_ok());
  }

  #[test]
  fn key_child_from_name_and_error() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");
    let derived = key.child_from_name("0'").expect("child from name");

    assert_eq!(derived.children.len(), 2);
    let alpha_derived = key.child_from_name("alpha").expect("child from name");
    let alpha_derived_again = key.child_from_name("alpha").expect("child from name");
    assert_eq!(
      alpha_derived.derivation_path(),
      alpha_derived_again.derivation_path()
    );
    let alpha_hardened = key.child_from_name("alpha'").expect("child from name");
    assert_ne!(
      alpha_derived.derivation_path(),
      alpha_hardened.derivation_path()
    );
    let err = key.child_from_name("");
    assert!(matches!(err, Err(KeyError::EmptyChild)));
  }

  #[test]
  fn key_child_from_derivation_path_and_error() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");
    let derived = key
      .child_from_derivation_path("m/44'/0'")
      .expect("child from path");
    assert_eq!(derived.children.len(), 2);
    let err = key.child_from_derivation_path("bad_path");
    assert!(matches!(err, Err(KeyError::Bip32(_))));
    let err = key.child_from_derivation_path("m/45'/0'");
    assert!(matches!(err, Err(KeyError::InvalidKey)));
  }

  #[test]
  fn derive_key_updates_path() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");
    let derived = key
      .child_number(ChildNumber::new(0, true).expect("child number"))
      .expect("derive");

    assert_eq!(derived.children.len(), 2);
    assert_eq!(derived.derivation_path(), "m/44'/0'");
  }

  #[test]
  fn nested_derivation_matches_sequential() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");

    let sequential = key
      .child_number(ChildNumber::new(0, true).expect("child number"))
      .expect("derive")
      .child_number(ChildNumber::new(1, true).expect("child number"))
      .expect("derive");

    let nested = key
      .child_from_derivation_path("m/44'/0'/1'")
      .expect("child from path");

    let sequential_sk = sequential
      .ed25519_secret_bytes()
      .expect("sequential secret");
    let nested_sk = nested.ed25519_secret_bytes().expect("nested secret");

    assert_eq!(sequential_sk, nested_sk);
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
  fn from_entropy_invalid_length() {
    let err = Key::from_entropy(&[0u8; 15]);
    assert!(matches!(err, Err(KeyError::Bip39(_))));
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
    let sig = kp.secret.sign(msg);
    assert!(kp.public.verify(msg, &sig).is_ok());
  }

  #[test]
  fn ed25519_sign_and_verify_payload() {
    use ed25519_dalek::Signer as _;
    use ed25519_dalek::Verifier as _;

    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");
    let kp = key.ed25519_keypair().expect("keypair");

    let claims = Claims {
      sub: "user@example.com".to_string(),
      exp: 2000000000,
    };

    let payload = serde_json::to_vec(&claims).expect("serialize claims");

    let signature = kp.secret.sign(&payload);

    assert!(kp.public.verify(&payload, &signature).is_ok());

    let mut modified_payload = payload.clone();
    modified_payload[0] ^= 0xFF;
    assert!(kp.public.verify(&modified_payload, &signature).is_err());
  }

  #[test]
  fn hierarchical_derivation_with_unique_keys_and_signatures() {
    use ed25519_dalek::Signer as _;
    use ed25519_dalek::Verifier as _;

    let master = Key::from_entropy(&TEST_ENTROPY).expect("generate master key");

    let child = master
      .child_number(ChildNumber::new(0, true).expect("child number"))
      .expect("derive child");

    let grandchild = child
      .child_number(ChildNumber::new(1, true).expect("child number"))
      .expect("derive grandchild");

    assert_eq!(master.derivation_path(), "m/44'");
    assert_eq!(child.derivation_path(), "m/44'/0'");
    assert_eq!(grandchild.derivation_path(), "m/44'/0'/1'");

    let master_sk = master.ed25519_secret_bytes().expect("master secret bytes");
    let child_sk = child.ed25519_secret_bytes().expect("child secret bytes");
    let grandchild_sk = grandchild
      .ed25519_secret_bytes()
      .expect("grandchild secret bytes");

    assert_ne!(master_sk, child_sk);
    assert_ne!(child_sk, grandchild_sk);
    assert_ne!(master_sk, grandchild_sk);

    let master_kp = master.ed25519_keypair().expect("master keypair");
    let child_kp = child.ed25519_keypair().expect("child keypair");
    let grandchild_kp = grandchild.ed25519_keypair().expect("grandchild keypair");

    let msg = b"test message";

    let master_sig = master_kp.secret.sign(msg);
    assert!(master_kp.public.verify(msg, &master_sig).is_ok());
    assert!(child_kp.public.verify(msg, &master_sig).is_err());
    assert!(grandchild_kp.public.verify(msg, &master_sig).is_err());

    let child_sig = child_kp.secret.sign(msg);
    assert!(child_kp.public.verify(msg, &child_sig).is_ok());
    assert!(master_kp.public.verify(msg, &child_sig).is_err());
    assert!(grandchild_kp.public.verify(msg, &child_sig).is_err());

    let grandchild_sig = grandchild_kp.secret.sign(msg);
    assert!(grandchild_kp.public.verify(msg, &grandchild_sig).is_ok());
    assert!(master_kp.public.verify(msg, &grandchild_sig).is_err());
    assert!(child_kp.public.verify(msg, &grandchild_sig).is_err());
  }

  #[test]
  fn master_seed_not_stored_in_children() {
    let master = Key::from_entropy(&TEST_ENTROPY).expect("generate master key");
    assert!(master.is_master());
    assert!(master.seed_bytes().is_some());

    let child = master
      .child_number(ChildNumber::new(0, true).expect("child number"))
      .expect("derive child");

    assert!(!child.is_master());
    assert!(child.seed_bytes().is_none(), "child should not have seed");

    let grandchild = child
      .child_number(ChildNumber::new(1, true).expect("child number"))
      .expect("derive grandchild");

    assert!(!grandchild.is_master());
    assert!(
      grandchild.seed_bytes().is_none(),
      "grandchild should not have seed"
    );
  }

  #[test]
  fn verify_child_derived_from_master() {
    let master = Key::from_entropy(&TEST_ENTROPY).expect("generate master key");
    let master_xpub = master.extended_public_key().expect("master public key");

    let child = master
      .child_number(ChildNumber::new(0, true).expect("child number"))
      .expect("derive child");

    assert!(
      child.verify_derived_from(&master_xpub),
      "child should be verifiable as derived from master"
    );

    let grandchild = child
      .child_number(ChildNumber::new(1, true).expect("child number"))
      .expect("derive grandchild");

    assert!(
      grandchild.verify_derived_from(&master_xpub),
      "grandchild should be verifiable as derived from master"
    );
  }

  #[test]
  fn extended_public_key_available() {
    let master = Key::from_entropy(&TEST_ENTROPY).expect("generate master key");
    let _master_xpub = master.extended_public_key().expect("master public key");

    let child = master
      .child_number(ChildNumber::new(0, true).expect("child number"))
      .expect("derive child");

    let _child_xpub = child.extended_public_key().expect("child public key");
  }

  #[test]
  fn from_master_seed_bytes_generates_valid_key() {
    let seed = vec![0x42; 64];
    let key = Key::from_master_seed_bytes(seed.clone()).expect("from seed");

    assert!(key.is_master());
    assert_eq!(key.derivation_path(), "m/44'");
    assert!(key.seed_bytes().is_some());
    assert_eq!(key.seed_bytes().unwrap(), seed);
  }

  #[test]
  fn from_master_seed_bytes_produces_consistent_keys() {
    let seed = vec![0x11; 64];
    let key1 = Key::from_master_seed_bytes(seed.clone()).expect("key1");
    let key2 = Key::from_master_seed_bytes(seed.clone()).expect("key2");

    assert_eq!(
      key1.ed25519_secret_bytes().unwrap(),
      key2.ed25519_secret_bytes().unwrap()
    );
    assert_eq!(key1.derivation_path(), key2.derivation_path());
  }

  #[test]
  fn from_master_seed_bytes_invalid_sizes() {
    let invalid_sizes = [0, 1, 8, 15, 17, 31, 33, 100];

    for size in invalid_sizes.iter() {
      let seed = vec![0u8; *size];
      let result = Key::from_master_seed_bytes(seed);
      assert!(result.is_err(), "Seed size {} should fail", size);
    }
  }

  #[test]
  fn from_entropy_valid_sizes() {
    let valid_sizes = [16, 20, 24, 32];

    for size in valid_sizes.iter() {
      let entropy = vec![0xAA; *size];
      let key = Key::from_entropy(&entropy);
      assert!(key.is_ok(), "Entropy size {} should succeed", size);
    }
  }

  #[test]
  fn from_entropy_invalid_sizes() {
    let invalid_sizes = [0, 8, 15, 17, 18, 30, 33, 64];

    for size in invalid_sizes.iter() {
      let entropy = vec![0u8; *size];
      let result = Key::from_entropy(&entropy);
      assert!(result.is_err(), "Entropy size {} should fail", size);
    }
  }

  #[test]
  fn parse_child_number_hardened_variants() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");

    let with_prime = key.child_from_name("5'").expect("parse 5'");
    let with_h = key.child_from_name("5h").expect("parse 5h");
    let with_capital_h = key.child_from_name("5H").expect("parse 5H");

    assert_eq!(with_prime.derivation_path(), with_h.derivation_path());
    assert_eq!(with_h.derivation_path(), with_capital_h.derivation_path());
  }

  #[test]
  fn parse_child_number_edge_cases() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");

    let zero = key.child_from_name("0").expect("parse 0");
    assert_eq!(zero.derivation_path(), "m/44'/0");

    let zero_hardened = key.child_from_name("0'").expect("parse 0'");
    assert_eq!(zero_hardened.derivation_path(), "m/44'/0'");

    let large_num = key.child_from_name("1000000").expect("parse large number");
    assert!(large_num.derivation_path().ends_with("/1000000"));
  }

  #[test]
  fn parse_child_number_with_whitespace() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");

    let with_spaces = key.child_from_name(" 7' ").expect("parse with spaces");
    let without_spaces = key.child_from_name("7'").expect("parse without spaces");

    assert_eq!(
      with_spaces.derivation_path(),
      without_spaces.derivation_path()
    );
  }

  #[test]
  fn parse_child_number_hash_based_names() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");

    let alice = key.child_from_name("alice").expect("alice");
    let alice_again = key.child_from_name("alice").expect("alice again");
    let bob = key.child_from_name("bob").expect("bob");

    assert_eq!(alice.derivation_path(), alice_again.derivation_path());
    assert_ne!(alice.derivation_path(), bob.derivation_path());

    let alice_hardened = key.child_from_name("alice'").expect("alice hardened");
    assert_ne!(alice.derivation_path(), alice_hardened.derivation_path());
  }

  #[test]
  fn child_from_derivation_path_various_formats() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");

    let simple = key
      .child_from_derivation_path("m/44'/0'")
      .expect("simple path");
    assert_eq!(simple.derivation_path(), "m/44'/0'");

    let longer = key
      .child_from_derivation_path("m/44'/0'/1'/2/3'")
      .expect("longer path");
    assert_eq!(longer.derivation_path(), "m/44'/0'/1'/2/3'");

    let with_nonhardened = key
      .child_from_derivation_path("m/44'/0/1'/2")
      .expect("mixed hardening");
    assert!(with_nonhardened.derivation_path().contains("/0/"));
  }

  #[test]
  fn child_from_derivation_path_must_start_with_current() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");

    let derived = key
      .child_number(ChildNumber::new(0, true).expect("child number"))
      .expect("derive");

    let valid_continuation = derived
      .child_from_derivation_path("m/44'/0'/1'")
      .expect("valid continuation");
    assert_eq!(valid_continuation.children.len(), 3);

    let invalid_path = derived.child_from_derivation_path("m/45'/0'");
    assert!(matches!(invalid_path, Err(KeyError::InvalidKey)));

    let different_first = derived.child_from_derivation_path("m/43'/0'");
    assert!(matches!(different_first, Err(KeyError::InvalidKey)));
  }

  #[test]
  fn child_from_derivation_path_same_as_current() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");
    let derived = key
      .child_number(ChildNumber::new(0, true).expect("child number"))
      .expect("derive");

    let same_path = derived
      .child_from_derivation_path("m/44'/0'")
      .expect("same path");

    assert_eq!(derived.derivation_path(), same_path.derivation_path());
    assert_eq!(
      derived.ed25519_secret_bytes().unwrap(),
      same_path.ed25519_secret_bytes().unwrap()
    );
  }

  #[test]
  fn deep_derivation_chain_100_levels() {
    let mut key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");

    for i in 0..100 {
      key = key
        .child_number(ChildNumber::new(i, true).expect("child number"))
        .expect("child derivation");
    }

    assert_eq!(key.children.len(), 101);
    let path_str = key.derivation_path();
    assert!(path_str.starts_with("m/44'"));
    assert!(path_str.contains("/99'"));
  }

  #[test]
  fn deep_derivation_produces_valid_keys() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");
    let mut current = key.clone();

    for i in 0..50 {
      current = current
        .child_number(ChildNumber::new(i, false).expect("child number"))
        .expect("derive");
    }

    let sk = current.ed25519_secret_bytes().expect("secret bytes");
    assert_eq!(sk.len(), 32);

    let kp = current.ed25519_keypair().expect("keypair");
    let msg = b"deep derivation test";
    let sig = kp.secret.sign(msg);
    assert!(kp.public.verify(msg, &sig).is_ok());
  }

  #[test]
  fn cloned_key_independence() {
    let key1 = Key::from_entropy(&TEST_ENTROPY).expect("generate key");
    let key2 = key1.clone();

    let d1 = key1
      .child_number(ChildNumber::new(0, true).expect("child number"))
      .expect("derive");
    let d2 = key2
      .child_number(ChildNumber::new(1, true).expect("child number"))
      .expect("derive");

    assert_ne!(d1.derivation_path(), d2.derivation_path());
    assert_ne!(
      d1.ed25519_secret_bytes().unwrap(),
      d2.ed25519_secret_bytes().unwrap()
    );

    let kp1 = d1.ed25519_keypair().expect("kp1");
    let kp2 = d2.ed25519_keypair().expect("kp2");
    let msg = b"test";
    let sig1 = kp1.secret.sign(msg);
    assert!(kp2.public.verify(msg, &sig1).is_err());
  }

  #[test]
  fn multiple_children_from_same_parent_derive_differently() {
    let parent = Key::from_entropy(&TEST_ENTROPY).expect("generate key");

    let child0 = parent
      .child_number(ChildNumber::new(0, true).expect("child number"))
      .expect("derive");
    let child1 = parent
      .child_number(ChildNumber::new(1, true).expect("child number"))
      .expect("derive");
    let child0_again = parent
      .child_number(ChildNumber::new(0, true).expect("child number"))
      .expect("derive");

    assert_ne!(
      child0.ed25519_secret_bytes().unwrap(),
      child1.ed25519_secret_bytes().unwrap()
    );

    assert_eq!(
      child0.ed25519_secret_bytes().unwrap(),
      child0_again.ed25519_secret_bytes().unwrap()
    );
  }

  #[test]
  fn extended_private_key_consistency() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");

    let xprv1 = key.extended_private_key().expect("xprv1");
    let xprv2 = key.extended_private_key().expect("xprv2");

    assert_eq!(xprv1, xprv2);

    let different_key = Key::from_entropy(&[0xFF; 32]).expect("different key");
    let xprv3 = different_key.extended_private_key().expect("xprv3");

    assert_ne!(xprv1, xprv3);
  }

  #[test]
  fn extended_public_key_matches_private_key() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");

    let xprv = key.extended_private_key().expect("xprv");
    let xpub_from_key = key.extended_public_key().expect("xpub from key");
    let xpub_from_prv = xprv.public_key();

    assert_eq!(xpub_from_key, xpub_from_prv);
  }

  #[test]
  fn master_fingerprint_propagates_through_derivation() {
    let master = Key::from_entropy(&TEST_ENTROPY).expect("generate master");
    let master_fp = master.master_fingerprint().expect("master fingerprint");

    let child = master
      .child_number(ChildNumber::new(0, true).expect("child number"))
      .expect("derive");
    let child_fp = child.master_fingerprint().expect("child fingerprint");

    assert_eq!(master_fp, child_fp);

    let grandchild = child
      .child_number(ChildNumber::new(1, true).expect("child number"))
      .expect("derive");
    let grandchild_fp = grandchild
      .master_fingerprint()
      .expect("grandchild fingerprint");

    assert_eq!(child_fp, grandchild_fp);
  }

  #[test]
  fn different_seeds_produce_different_keys() {
    let seed1 = vec![0x11; 64];
    let seed2 = vec![0x22; 64];

    let key1 = Key::from_master_seed_bytes(seed1).expect("key1");
    let key2 = Key::from_master_seed_bytes(seed2).expect("key2");

    assert_ne!(
      key1.ed25519_secret_bytes().unwrap(),
      key2.ed25519_secret_bytes().unwrap()
    );
  }

  #[test]
  fn mnemonic_to_key_roundtrip() {
    let entropy = [0x55; 16];
    let mnemonic = Mnemonic::from_entropy(&entropy).expect("create mnemonic");
    let key1 = Key::try_from(mnemonic.clone()).expect("key from mnemonic");

    let key2 = Key::from_entropy(&entropy).expect("key from entropy");

    assert_eq!(
      key1.ed25519_secret_bytes().unwrap(),
      key2.ed25519_secret_bytes().unwrap()
    );
  }

  #[test]
  fn sequential_vs_batched_derivation_equivalence() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");

    let sequential = key
      .child_number(ChildNumber::new(0, true).expect("child number"))
      .expect("derive")
      .child_number(ChildNumber::new(1, true).expect("child number"))
      .expect("derive")
      .child_number(ChildNumber::new(2, false).expect("child number"))
      .expect("derive");

    let batched = key
      .child_from_derivation_path("m/44'/0'/1'/2")
      .expect("child from path");

    assert_eq!(sequential.derivation_path(), batched.derivation_path());
    assert_eq!(
      sequential.ed25519_secret_bytes().unwrap(),
      batched.ed25519_secret_bytes().unwrap()
    );

    let kp_seq = sequential.ed25519_keypair().expect("seq keypair");
    let kp_batch = batched.ed25519_keypair().expect("batch keypair");

    let msg = b"test message";
    let sig_seq = kp_seq.secret.sign(msg);
    assert!(kp_batch.public.verify(msg, &sig_seq).is_ok());
  }

  #[test]
  fn is_master_flag_accuracy() {
    let master = Key::from_entropy(&TEST_ENTROPY).expect("generate master");
    assert!(master.is_master());

    let child = master
      .child_number(ChildNumber::new(0, true).expect("child number"))
      .expect("derive");
    assert!(!child.is_master());

    let grandchild = child
      .child_number(ChildNumber::new(1, true).expect("child number"))
      .expect("derive");
    assert!(!grandchild.is_master());

    let from_seed = Key::from_master_seed_bytes(vec![0x99; 64]).expect("from seed");
    assert!(from_seed.is_master());
  }

  #[test]
  fn seed_bytes_only_in_master() {
    let master = Key::from_entropy(&TEST_ENTROPY).expect("generate master");
    assert!(master.seed_bytes().is_some());

    let child = master
      .child_number(ChildNumber::new(0, true).expect("child number"))
      .expect("derive");
    assert!(child.seed_bytes().is_none());

    let grandchild = child
      .child_number(ChildNumber::new(1, true).expect("child number"))
      .expect("derive");
    assert!(grandchild.seed_bytes().is_none());
  }

  #[test]
  fn verify_derived_from_with_unrelated_key() {
    let master1 = Key::from_entropy(&TEST_ENTROPY).expect("master1");
    let master2 = Key::from_entropy(&[0xFF; 32]).expect("master2");

    let child1 = master1
      .child_number(ChildNumber::new(0, true).expect("child number"))
      .expect("derive");

    let master1_xpub = master1.extended_public_key().expect("master1 xpub");
    let master2_xpub = master2.extended_public_key().expect("master2 xpub");

    assert!(child1.verify_derived_from(&master1_xpub));
    assert!(!child1.verify_derived_from(&master2_xpub));
  }

  #[test]
  fn signature_determinism() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");
    let kp = key.ed25519_keypair().expect("keypair");

    let msg = b"deterministic message";

    let sig1 = kp.secret.sign(msg);
    let sig2 = kp.secret.sign(msg);

    assert_eq!(sig1, sig2);
  }

  #[test]
  fn signature_different_for_different_messages() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");
    let kp = key.ed25519_keypair().expect("keypair");

    let msg1 = b"message one";
    let msg2 = b"message two";

    let sig1 = kp.secret.sign(msg1);
    let sig2 = kp.secret.sign(msg2);

    assert_ne!(sig1, sig2);
    assert!(kp.public.verify(msg1, &sig1).is_ok());
    assert!(kp.public.verify(msg2, &sig2).is_ok());
    assert!(kp.public.verify(msg1, &sig2).is_err());
    assert!(kp.public.verify(msg2, &sig1).is_err());
  }

  #[test]
  fn ed25519_signing_key_length() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");
    let sk_bytes = key.ed25519_secret_bytes().expect("secret bytes");

    assert_eq!(sk_bytes.len(), 32);

    let verifying_key = key.ed25519_verifying_key().expect("verifying key");
    let vk_bytes = verifying_key.to_bytes();
    assert_eq!(vk_bytes.len(), 32);
  }

  #[test]
  fn derivation_path_string_format() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");

    assert_eq!(key.derivation_path(), "m/44'");

    let d1 = key
      .child_number(ChildNumber::new(0, true).expect("child number"))
      .expect("derive");
    assert_eq!(d1.derivation_path(), "m/44'/0'");

    let d2 = d1
      .child_number(ChildNumber::new(42, false).expect("child number"))
      .expect("derive");
    assert_eq!(d2.derivation_path(), "m/44'/0'/42");

    let d3 = d2
      .child_number(ChildNumber::new(100, true).expect("child number"))
      .expect("derive");
    assert_eq!(d3.derivation_path(), "m/44'/0'/42/100'");
  }

  #[test]
  fn child_number_zero_vs_nonzero() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");

    let child_zero = key.child_from_name("0").expect("child zero");
    let child_one = key.child_from_name("1").expect("child one");

    assert_ne!(
      child_zero.ed25519_secret_bytes().unwrap(),
      child_one.ed25519_secret_bytes().unwrap()
    );
  }

  #[test]
  fn child_number_hardened_vs_unhardened() {
    let key = Key::from_entropy(&TEST_ENTROPY).expect("generate key");

    let unhardened = key.child_from_name("5").expect("unhardened");
    let hardened = key.child_from_name("5'").expect("hardened");

    assert_ne!(
      unhardened.ed25519_secret_bytes().unwrap(),
      hardened.ed25519_secret_bytes().unwrap()
    );
  }
}
