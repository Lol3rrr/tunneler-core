#![warn(missing_docs)]
//! This crate provides a simply way to start a tunneler
//! server and client that can also easily be integradted into
//! other projects allowing you to expose your services running
//! in a private network to be exposed through a public server

#[macro_use]
mod logging;

#[cfg(feature = "client")]
pub mod client;
mod connections;
pub use connections::Destination;
/// Messages are used for all Communication between Server and Client
pub mod message;
#[cfg(feature = "server")]
pub mod server;
mod streams;

/// All the Metrics related functionality
pub mod metrics;

pub(crate) mod general;
pub use general::Details;
pub(crate) mod handshake;
