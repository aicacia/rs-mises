#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

pub mod error;
pub mod model;
pub mod service;

pub use model::*;

pub use error::OidcError;
pub use service::OidcProvider;
