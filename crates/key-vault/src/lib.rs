#![forbid(unsafe_code)]

pub mod key_vault;

pub use key_vault::FileKeyVault;
pub use key_vault::GraphKeyVault;
