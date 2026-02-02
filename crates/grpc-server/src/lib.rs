#![forbid(unsafe_code)]

mod bootstrap_service;

pub use mises_proto as proto;
pub use mises_proto::bootstrap_service_server::*;

pub use bootstrap_service::BootstrapService;
