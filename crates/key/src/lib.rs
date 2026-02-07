#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

pub mod error;
pub mod key;

pub use error::KeyError;
pub use key::Key;
