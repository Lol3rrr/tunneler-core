use crate::connections::Connections;
use crate::message::{Message, MessageHeader, MessageType};
use crate::objectpool;
use crate::server::client::ClientManager;
use crate::server::user;
use crate::streams::mpsc;

use log::error;

mod tokio_rx;
mod tokio_tx;

/// This Client represents a single Connection a Client Instance
///
/// All User-Connections are handled by an instance of this Struct
#[derive(Clone, Debug)]
pub struct Client {
    id: u32,
    user_cons: Connections<mpsc::StreamWriter<Message>>,
    client_manager: std::sync::Arc<ClientManager>,
    client_send_queue: tokio::sync::mpsc::UnboundedSender<Message>,
}

impl Client {
    /// Creates a new Client that is then ready to start up
    pub fn new(
        id: u32,
        client_manager: std::sync::Arc<ClientManager>,
        send_queue: tokio::sync::mpsc::UnboundedSender<Message>,
    ) -> Client {
        Client {
            id,
            user_cons: Connections::new(),
            client_manager,
            client_send_queue: send_queue,
        }
    }

    /// The Client-ID itself
    pub fn get_id(&self) -> u32 {
        self.id
    }

    /// Returns the Connections managed by this Client
    pub fn get_user_cons(&self) -> Connections<mpsc::StreamWriter<Message>> {
        self.user_cons.clone()
    }

    async fn close_user_connection(
        user_id: u32,
        client_id: u32,
        user_cons: Connections<mpsc::StreamWriter<Message>>,
        send_queue: tokio::sync::mpsc::UnboundedSender<Message>,
    ) {
        user_cons.remove(user_id);

        let header = MessageHeader::new(user_id, MessageType::Close, 0);
        let msg = Message::new(header, vec![0; 0]);
        match send_queue.send(msg) {
            Ok(_) => {}
            Err(e) => {
                error!("[{}][{}] Sending Close Message: {}", client_id, user_id, e);
            }
        };
    }

    /// Adds a new user connection to this server-client
    ///
    /// Params:
    /// * id: The ID of the new user connection
    /// * con: The new user connection
    pub fn new_con(&self, user_id: u32, con: tokio::net::TcpStream) {
        // Notify the client of the new connection
        if let Err(e) = self.client_send_queue.send(Message::new(
            MessageHeader::new(user_id, MessageType::Connect, 0),
            vec![],
        )) {
            error!(
                "[{}][{}] Sending Connect message: {:?}",
                self.id, user_id, e
            );
        }

        let (read_con, write_con) = con.into_split();
        let (tx, rx) = mpsc::stream();
        self.user_cons.set(user_id, tx);

        let client_id = self.id;
        tokio::task::spawn(user::send(client_id, user_id, write_con, rx));
        let cloned_cons = self.user_cons.clone();
        let send_queue = self.client_send_queue.clone();
        tokio::task::spawn(user::recv(
            self.id,
            user_id,
            read_con,
            self.client_send_queue.clone(),
            Client::close_user_connection(user_id, client_id, cloned_cons, send_queue),
        ));
    }

    /// This listens to the Client-Connection and forwards the messages to the
    /// correct User-Connections
    ///
    /// Params:
    /// * id: The ID of the Client
    /// * read_con: The Reader-Half of the Client-Connection
    /// * user_cons: The User-Connections
    /// * client_manager: The Manager for this client
    /// * obj_pool: The Object-Pool which should be used for Messages
    pub async fn receiver(
        id: u32,
        mut read_con: tokio::net::tcp::OwnedReadHalf,
        user_cons: Connections<mpsc::StreamWriter<Message>>,
        client_manager: std::sync::Arc<ClientManager>,
    ) {
        let obj_pool: objectpool::Pool<Vec<u8>> = objectpool::Pool::new(100);

        let mut header_buffer = [0; 13];
        loop {
            if let Err(e) =
                tokio_rx::receive(id, &mut read_con, &user_cons, &obj_pool, &mut header_buffer)
                    .await
            {
                log::error!("[{}] Receiving Client-Message: {:?}", id, e);
                client_manager.remove(id);
                return;
            }
        }
    }

    /// This Receives messages from users and then forwards them to the
    /// Client-Connection
    ///
    /// Params:
    /// * id: The ID of the Client
    /// * write_con: The Write-Half of the Client-Connection
    /// * queue: The Queue of messages to forward to the Client
    /// * client_manager: The Client-Manager
    pub async fn sender(
        id: u32,
        mut write_con: tokio::net::tcp::OwnedWriteHalf,
        mut queue: tokio::sync::mpsc::UnboundedReceiver<Message>,
        client_manager: std::sync::Arc<ClientManager>,
    ) {
        let mut h_data = [0; 13];
        loop {
            if let Err(e) = tokio_tx::send(&mut write_con, &mut queue, &mut h_data).await {
                log::error!("[{}] Sending Client-Message: {:?}", id, e);
                client_manager.remove(id);
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_client() {
        let manager_arc = std::sync::Arc::new(ClientManager::new());
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();

        let client = Client::new(123, manager_arc, tx);

        assert_eq!(123, client.get_id());
    }
}
