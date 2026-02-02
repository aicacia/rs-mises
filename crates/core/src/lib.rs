#![cfg_attr(not(feature = "std"), no_std)]
#![allow(async_fn_in_trait)]
#![forbid(unsafe_code)]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub mod error;
pub mod model;
pub mod service;
pub mod traits;

pub use error::{CoreError, InvalidInput, Result};
