use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use mises_core::model::keys::KeyMeta;
use mises_key::{Key, KeyError};

#[test]
fn keymeta_roundtrip_master() {
  let seed = vec![0u8; 32];
  let key = Key::from(seed.clone());

  let km = KeyMeta::from(key.clone());
  let k2 = Key::try_from(km).expect("convert back to key");

  let a = key.secp256k1_secret_bytes().expect("orig secret");
  let b = k2.secp256k1_secret_bytes().expect("restored secret");
  assert_eq!(a, b);
}

#[test]
fn keymeta_roundtrip_derived() {
  let seed = vec![1u8; 32];
  let key = Key::from(seed.clone());
  let derived = key.child_from_derivation_path("m/44'/0'").expect("derive");

  let km = KeyMeta::from(derived.clone());
  assert_eq!(km.derivation_path, "m/44'/0'");

  let k2 = Key::try_from(km).expect("convert back to derived key");

  let a = derived
    .secp256k1_secret_bytes()
    .expect("orig derived secret");
  let b = k2
    .secp256k1_secret_bytes()
    .expect("restored derived secret");
  assert_eq!(a, b);
}

#[test]
fn invalid_b64_without_path_is_invalid_key() {
  let km = KeyMeta {
    public_key: "pub".to_string(),
    private_key: Some("!!!notbase64!!!".to_string()),
    derivation_path: String::from("m/44'"),
  };

  let res = Key::try_from(km);
  assert!(matches!(res, Err(KeyError::InvalidKey)));
}

#[test]
fn invalid_b64_with_path_is_invalid_key() {
  let km = KeyMeta {
    public_key: "pub".to_string(),
    private_key: Some("!!!notbase64!!!".to_string()),
    derivation_path: String::from("m/44'"),
  };

  let res = Key::try_from(km);
  assert!(matches!(res, Err(KeyError::InvalidKey)));
}

#[test]
fn valid_seed_but_bad_path_is_invalid_key() {
  let seed = vec![0u8; 32];
  let km = KeyMeta {
    public_key: "pub".to_string(),
    private_key: Some(URL_SAFE_NO_PAD.encode(&seed)),
    derivation_path: "bad_path".to_string(),
  };

  let res = Key::try_from(km);
  assert!(matches!(res, Err(KeyError::InvalidKey)));
}
