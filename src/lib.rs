#![warn(missing_docs)]
//! This crate provides a simply way to start a tunneler
//! server and client that can also easily be integradted into
//! other projects allowing you to expose your services running
//! in a private network to be exposed through a public server

/// Provides all the Client related functionality
#[cfg(feature = "client")]
pub mod client;
mod connections;
pub use connections::Destination;
/// Messages are used for all Communication between Server and Client
pub mod message;
/// A Generic Object-Pool that allows for easy reuse of Objects
/// between actions
pub(crate) mod objectpool;
/// Provides all the Server related functionality
#[cfg(feature = "server")]
pub mod server;
/// Provides all the Stream/Queue related functionality
pub mod streams;

pub(crate) mod general;
pub(crate) mod handshake;
