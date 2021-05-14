use crate::{
    connections::Connections,
    message::{Message, MessageHeader, MessageType},
    streams::mpsc,
};

use log::{debug, error};

/// Handles all the sending related to a single user-connection
/// as well as the correct clean up handling once this is dropped
pub struct Sender {
    id: u32,
    tx: tokio::sync::mpsc::UnboundedSender<Message>,
    total_client_cons: std::sync::Arc<Connections<mpsc::StreamWriter<Message>>>,
}

impl Sender {
    /// Creates a new Sender from the given Data
    pub fn new(
        id: u32,
        tx: tokio::sync::mpsc::UnboundedSender<Message>,
        cons: std::sync::Arc<Connections<mpsc::StreamWriter<Message>>>,
    ) -> Self {
        Self {
            id,
            tx,
            total_client_cons: cons,
        }
    }

    /// Creates a new Data-Message with the given Data and Length and adds
    /// the new Message to the Queue to be send to the external Server
    pub async fn send(&self, data: Vec<u8>, length: u64) -> bool {
        // Create the right Header and Message
        let header = MessageHeader::new(self.id, MessageType::Data, length);
        let msg = Message::new(header, data);

        self.tx.send(msg).is_ok()
    }

    /// Closes the Sender and therefore consuming itself
    pub async fn close(self) {
        self.total_client_cons.remove(self.id);
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

impl Drop for Sender {
    fn drop(&mut self) {
        match self.total_client_cons.remove(self.id) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sender_send() {
        let clients = std::sync::Arc::new(Connections::<mpsc::StreamWriter<Message>>::new());
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let sender = Sender::new(123, tx, clients);

        sender.send(vec![0, 1], 2).await;
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

        let sender = Sender::new(123, tx, clients);
        sender.close().await;

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

        let sender = Sender::new(123, tx, clients);
        drop(sender);

        let received = rx.recv().await;
        assert_eq!(true, received.is_some());
        assert_eq!(
            Message::new(MessageHeader::new(123, MessageType::Close, 0), vec![]),
            received.unwrap(),
        );
    }
}
