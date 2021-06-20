use std::sync::Arc;

use crate::{message::Message, Details};

use async_trait::async_trait;

use super::connections::UserCon;

/// A Generic trait that abstracts over the actual receiving
/// type and implementation allowing you to easily mock it
/// and also allows for more versitility when using this in
/// general as well as giving more freedom to the underlying
/// types
#[async_trait]
pub trait Receiver {
    /// The Error-Type that the Receiver will return if it encounters
    /// any problems
    type ReceivingError: std::fmt::Debug;

    /// Receives a single Message over the Connection
    async fn recv_msg(&mut self) -> Result<Message, Self::ReceivingError>;
}

/// A Generic trait that abstract away the actual underlying
/// type and implementation for sending Data back to the
/// User over that connection
#[async_trait]
pub trait Sender {
    /// The Error-Type that the Sender will return if it encounters
    /// any problems
    type SendingError: std::fmt::Debug;

    /// Sends a single Message over the Connection
    async fn send_msg(&self, data: Vec<u8>, length: u64) -> Result<(), Self::SendingError>;
}

/// The Interface that every Handler needs to implement to be used by the
/// Client.
#[async_trait]
pub trait Handler {
    /// This method is called every time a new Connection is received and
    /// should therefore handle all the initial stuff for dealing with
    /// the new Connection
    async fn new_con(self: Arc<Self>, id: u32, details: Details, con: UserCon);
}
