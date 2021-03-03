use crate::{
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
    async fn recv(&mut self) -> Result<Message, RecvError>;
}

/// A Generic trait that abstract away the actual underlying
/// type and implementation for sending Data back to the
/// User over that connection
#[async_trait]
pub trait Sender {
    /// Sends a single Message over the Connection
    async fn send(&self, data: Vec<u8>, lenth: u64) -> bool;
}

#[async_trait]
impl Receiver for mpsc::StreamReader<Message> {
    async fn recv(&mut self) -> Result<Message, RecvError> {
        mpsc::StreamReader::<Message>::recv(self).await
    }
}

#[async_trait]
impl Sender for queues::Sender {
    async fn send(&self, data: Vec<u8>, length: u64) -> bool {
        queues::Sender::send(self, data, length).await
    }
}
