use crate::connections::Connections;
use crate::message::{Message, MessageHeader, MessageType};
use crate::objectpool;
use crate::server::client::ClientManager;
use crate::server::user;
use crate::streams::mpsc;

use log::{debug, error};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

    /// Drains `size` amount of bytes from the Connection
    async fn drain(read_con: &mut tokio::net::tcp::OwnedReadHalf, size: usize) {
        let mut tmp_buf = vec![0; size];
        match read_con.read_exact(&mut tmp_buf).await {
            Ok(_) => {}
            Err(e) => {
                error!("Draining: {}", e);
            }
        };
    }

    async fn receive(
        id: u32,
        read_con: &mut tokio::net::tcp::OwnedReadHalf,
        user_cons: &Connections<mpsc::StreamWriter<Message>>,
        client_manager: &std::sync::Arc<ClientManager>,
        obj_pool: &objectpool::Pool<Vec<u8>>,
    ) -> Result<(), ()> {
        let mut head_buf = [0; 13];
        let header = match read_con.read_exact(&mut head_buf).await {
            Ok(_) => {
                let h = MessageHeader::deserialize(head_buf);
                if h.is_none() {
                    error!("[{}] Deserializing Header: {:?}", id, head_buf);
                    return Ok(());
                }
                h.unwrap()
            }
            Err(e) => {
                error!("[{}] Reading from Client-Connection: {}", id, e);
                client_manager.remove(id);
                return Err(());
            }
        };

        match header.get_kind() {
            MessageType::Data => {}
            MessageType::Close => {
                user_cons.remove(header.get_id());
                return Ok(());
            }
            MessageType::Heartbeat => {
                return Ok(());
            }
            _ => {
                error!(
                    "[{}][{}] Unexpected Operation: {:?}",
                    id,
                    header.get_id(),
                    header.get_kind()
                );
                Client::drain(read_con, header.get_length() as usize).await;
                return Ok(());
            }
        };

        let user_id = header.get_id();

        // Forwarding the message to the actual user
        let stream = match user_cons.get_clone(user_id) {
            Some(s) => s,
            None => {
                // Removes this message and drain all the Data belonging to this message
                // as well
                Client::drain(read_con, header.get_length() as usize).await;
                return Ok(());
            }
        };

        let body_length = header.get_length() as usize;
        let mut body_buf = obj_pool.get();
        body_buf.resize(body_length, 0);
        match read_con.read_exact(&mut body_buf).await {
            Ok(_) => {}
            Err(e) => {
                error!("[{}][{}] Reading Body from Client: {}", id, user_id, e);
            }
        };

        match stream.send(Message::new_guarded(header, body_buf)) {
            Ok(_) => {}
            Err(e) => {
                error!("[{}][{}] Adding to User-Queue: {}", id, user_id, e);
            }
        };
        Ok(())
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
        obj_pool: objectpool::Pool<Vec<u8>>,
    ) {
        loop {
            if Self::receive(id, &mut read_con, &user_cons, &client_manager, &obj_pool)
                .await
                .is_err()
            {
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
            let msg = match queue.recv().await {
                Some(m) => m,
                None => {
                    error!("[{}][Sender] Receiving Message from Queue", id);
                    client_manager.remove(id);
                    return;
                }
            };

            let data = msg.serialize(&mut h_data);
            match write_con.write_all(&h_data).await {
                Ok(_) => {}
                Err(e) => {
                    error!("[{}][Sender] Sending Message: {}", id, e);
                    client_manager.remove(id);
                    return;
                }
            };
            match write_con.write_all(&data).await {
                Ok(_) => {}
                Err(e) => {
                    error!("[{}][Sender] Sending Message: {}", id, e);
                    client_manager.remove(id);
                    return;
                }
            };
            debug!("[{}][Sender] Sent out Message", id);
        }
    }
}

#[test]
fn new_client() {
    let manager_arc = std::sync::Arc::new(ClientManager::new());
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();

    let client = Client::new(123, manager_arc, tx);

    assert_eq!(123, client.get_id());
}
