use crate::{
    message::Message,
    streams::{error::RecvError, mpsc},
};

use super::{Receiver, Sender};

use async_trait::async_trait;

pub mod establish_connection;
pub mod rx;
pub mod tx;

/// This represents a single User-Connection and is used to send and receive
/// Data for that one specific User
pub struct UserCon {
    receiver: user_con::OwnedReceiver,
    sender: user_con::OwnedSender,
}

impl UserCon {
    pub(crate) fn new(recv: mpsc::StreamReader<Message>, send: user_con::OwnedSender) -> Self {
        Self {
            receiver: user_con::OwnedReceiver::new(recv),
            sender: send,
        }
    }

    /// Splits the Connection into its owned (Reader, Writer)-Halfes, which
    /// allows you to use them more independantly of one another and enables
    /// you to send them to different Threads without having to use any extra
    /// sync between the Halfes
    pub fn into_split(self) -> (user_con::OwnedReceiver, user_con::OwnedSender) {
        (self.receiver, self.sender)
    }
}

#[async_trait]
impl Receiver for UserCon {
    type ReceivingError = RecvError;

    async fn recv_msg(&mut self) -> Result<Message, Self::ReceivingError> {
        self.receiver.recv_msg().await
    }
}
#[async_trait]
impl Sender for UserCon {
    type SendingError = tokio::sync::mpsc::error::SendError<Message>;

    async fn send_msg(&self, data: Vec<u8>, length: u64) -> Result<(), Self::SendingError> {
        self.sender.send_msg(data, length).await
    }
}

pub mod user_con;
