//! Contains some more specific Details for regarding the User-Connections

use std::sync::Arc;

use super::mpsc;
use crate::{
    client::{Receiver, Sender},
    connections::Connections,
    message::{Message, MessageHeader, MessageType},
    streams::error::RecvError,
};

use async_trait::async_trait;

/// The owned Version of the Receiver-Half of a User-Connection
pub struct OwnedReceiver {
    rx: mpsc::StreamReader<Message>,
}

impl OwnedReceiver {
    pub(crate) fn new(rx: mpsc::StreamReader<Message>) -> Self {
        Self { rx }
    }
}

#[async_trait]
impl Receiver for OwnedReceiver {
    type ReceivingError = RecvError;

    async fn recv_msg(&mut self) -> Result<Message, Self::ReceivingError> {
        self.rx.recv().await
    }
}

/// The owned Version of the Sender-Halfer of a User-Connection
pub struct OwnedSender {
    id: u32,
    tx: tokio::sync::mpsc::UnboundedSender<Message>,
    all_client_cons: Arc<Connections<mpsc::StreamWriter<Message>>>,
}

impl OwnedSender {
    pub(crate) fn new(
        id: u32,
        tx: tokio::sync::mpsc::UnboundedSender<Message>,
        cons: std::sync::Arc<Connections<mpsc::StreamWriter<Message>>>,
    ) -> Self {
        Self {
            id,
            tx,
            all_client_cons: cons,
        }
    }

    /// Closes the Sender and therefore consuming itself
    pub fn close(self) {
        self.all_client_cons.remove(self.id);
        debug!("[Sender][{}] Removed Connection", self.id);

        let close_msg = Message::new(MessageHeader::new(self.id, MessageType::Close, 0), vec![]);
        match self.tx.send(close_msg) {
            Ok(_) => {
                debug!("[Sender][{}] Sent Close", self.id);
            }
            Err(e) => {
                error!("Sending Close-Message for {}: {}", self.id, e);
            }
        };
    }
}

impl Drop for OwnedSender {
    fn drop(&mut self) {
        match self.all_client_cons.remove(self.id) {
            Some(_) => {}
            None => {
                return;
            }
        };
        debug!("[Sender][{}] Removed Connection", self.id);

        let close_msg = Message::new(MessageHeader::new(self.id, MessageType::Close, 0), vec![]);
        if let Err(e) = self.tx.send(close_msg) {
            error!("Sending Close-Message for {}: {}", self.id, e);
        }
    }
}

#[async_trait]
impl Sender for OwnedSender {
    type SendingError = tokio::sync::mpsc::error::SendError<Message>;

    async fn send_msg(&self, data: Vec<u8>, length: u64) -> Result<(), Self::SendingError> {
        // Create the right Header and Message
        let header = MessageHeader::new(self.id, MessageType::Data, length);
        let msg = Message::new(header, data);

        self.tx.send(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sender_send() {
        let clients = std::sync::Arc::new(Connections::<mpsc::StreamWriter<Message>>::new());
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let sender = OwnedSender::new(123, tx, clients);

        sender.send_msg(vec![0, 1], 2).await.unwrap();
        let received = rx.recv().await;
        assert_eq!(true, received.is_some());
        assert_eq!(
            Message::new(MessageHeader::new(123, MessageType::Data, 2), vec![0, 1]),
            received.unwrap(),
        );
    }

    #[tokio::test]
    async fn sender_close() {
        let clients = std::sync::Arc::new(Connections::<mpsc::StreamWriter<Message>>::new());
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let sender = OwnedSender::new(123, tx, clients);
        sender.close();

        let received = rx.recv().await;
        assert_eq!(true, received.is_some());
        assert_eq!(
            Message::new(MessageHeader::new(123, MessageType::Close, 0), vec![]),
            received.unwrap(),
        );
    }

    #[tokio::test]
    async fn sender_drop() {
        let (tx, _rx) = mpsc::stream();
        let clients = std::sync::Arc::new(Connections::<mpsc::StreamWriter<Message>>::new());
        clients.set(123, tx);

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let sender = OwnedSender::new(123, tx, clients);
        drop(sender);

        let received = rx.recv().await;
        assert_eq!(true, received.is_some());
        assert_eq!(
            Message::new(MessageHeader::new(123, MessageType::Close, 0), vec![]),
            received.unwrap(),
        );
    }
}
