use alloc::string::String;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct KeyMeta {
  pub public_key: String,
  // BIP32 derivation path None for master keys. i.e. "m/44'/0'/0'/0/0"
  pub derivation_path: Option<String>,
}
