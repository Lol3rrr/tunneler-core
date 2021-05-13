use std::sync::Arc;

use crate::{
    client::QueueSender,
    message::Message,
    streams::{error::RecvError, mpsc},
};

use async_trait::async_trait;

use super::queues;

/// A Generic trait that abstracts over the actual receiving
/// type and implementation allowing you to easily mock it
/// and also allows for more versitility when using this in
/// general as well as giving more freedom to the underlying
/// types
#[async_trait]
pub trait Receiver {
    /// Receives a single Message over the Connection
    async fn recv_msg(&mut self) -> Result<Message, RecvError>;
}

/// A Generic trait that abstract away the actual underlying
/// type and implementation for sending Data back to the
/// User over that connection
#[async_trait]
pub trait Sender {
    /// Sends a single Message over the Connection
    async fn send_msg(&self, data: Vec<u8>, lenth: u64) -> bool;
}

#[async_trait]
impl Receiver for mpsc::StreamReader<Message> {
    async fn recv_msg(&mut self) -> Result<Message, RecvError> {
        self.recv().await
    }
}

#[async_trait]
impl Sender for queues::Sender {
    async fn send_msg(&self, data: Vec<u8>, length: u64) -> bool {
        self.send(data, length).await
    }
}

/// This defines a single Handler that will receive every new Connection
/// that is established
#[async_trait]
pub trait Handler {
    /// This method is called every time a new Connection is received and
    /// should therefore handle all the initial stuff for dealing with
    /// the new Connection
    async fn new_con(self: Arc<Self>, id: u32, rx: mpsc::StreamReader<Message>, tx: QueueSender);
}
