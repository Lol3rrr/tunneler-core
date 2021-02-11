use crate::connections::Connections;
use crate::message::{Message, MessageHeader, MessageType};
use crate::streams::mpsc;

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

    /// Adds the Data to the queue to be send to the Server
    pub fn send(&self, data: Vec<u8>, length: u64) -> bool {
        // Create the right Header and Message
        let header = MessageHeader::new(self.id, MessageType::Data, length);
        let msg = Message::new(header, data);

        self.tx.send(msg).is_ok()
    }
}

// This just handles all the clean-up for this user-connection:
// - Removes this connection from the list of user connections
// - Sends a Close-Message to the Server
impl Drop for Sender {
    fn drop(&mut self) {
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
        }
    }
}
