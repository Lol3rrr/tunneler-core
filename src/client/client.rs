use crate::client::queues;
use crate::connections::{Connection, Connections, Destination};
use crate::message::{Message, MessageHeader, MessageType};
use crate::streams::mpsc;

use rand::RngCore;
use std::future::Future;

use rsa::{BigUint, PaddingScheme, PublicKey, RSAPublicKey};

use log::{debug, error, info};

/// The Client instance in general that is responsible for handling
/// all the interactions with the Server
pub struct Client {
    server_destination: Destination,
    key: Vec<u8>,
}

impl Client {
    /// Creates the raw Client instance that is configured to
    /// connect to the given Server Destination and authenticate
    /// using the provided Key
    pub fn new(server: Destination, key: Vec<u8>) -> Self {
        Self {
            server_destination: server,
            key,
        }
    }

    async fn establish_connection(adr: &str, key: &[u8]) -> Option<std::sync::Arc<Connection>> {
        let connection = match tokio::net::TcpStream::connect(&adr).await {
            Ok(c) => c,
            Err(e) => {
                error!("Establishing-Connection: {}", e);
                return None;
            }
        };
        let connection_arc = std::sync::Arc::new(Connection::new(connection));

        // Step 2 - Receive
        let mut head_buf = [0; 13];
        let header = match connection_arc.read_total(&mut head_buf, 13).await {
            Ok(_) => {
                let msg = MessageHeader::deserialize(head_buf);
                msg.as_ref()?;
                msg.unwrap()
            }
            Err(e) => {
                error!("Reading Message-Header: {}", e);
                return None;
            }
        };
        if *header.get_kind() != MessageType::Key {
            return None;
        }

        let mut key_buf = [0; 4092];
        let mut recv_pub_key = match connection_arc.read_raw(&mut key_buf).await {
            Ok(0) => {
                return None;
            }
            Ok(n) => key_buf[0..n].to_vec(),
            Err(e) => {
                error!("Reading Public-Key from Server: {}", e);
                return None;
            }
        };

        let e_bytes = recv_pub_key.split_off(256);
        let n_bytes = recv_pub_key;

        let pub_key = RSAPublicKey::new(
            BigUint::from_bytes_le(&n_bytes),
            BigUint::from_bytes_le(&e_bytes),
        )
        .expect("Could not create Public-Key");

        let encrypted_key = pub_key
            .encrypt(&mut rand::rngs::OsRng, PaddingScheme::PKCS1v15Encrypt, key)
            .expect("Could not encrypt Key");

        let msg_header = MessageHeader::new(0, MessageType::Verify, encrypted_key.len() as u64);
        let msg = Message::new(msg_header, encrypted_key);

        match connection_arc.write_raw(&msg.serialize()).await {
            Ok(_) => {}
            Err(e) => {
                error!("Sending Encrypted Key/Password: {}", e);
                return None;
            }
        };

        let mut buf = [0; 13];
        let header = match connection_arc.read_total(&mut buf, 13).await {
            Ok(_) => match MessageHeader::deserialize(buf) {
                Some(c) => c,
                None => {
                    return None;
                }
            },
            Err(e) => {
                error!("Reading response: {}", e);
                return None;
            }
        };

        if *header.get_kind() != MessageType::Acknowledge {
            return None;
        }

        Some(connection_arc)
    }

    async fn heartbeat(
        send_queue: tokio::sync::mpsc::UnboundedSender<Message>,
        wait_time: std::time::Duration,
    ) {
        loop {
            debug!("Sending Heartbeat");

            let msg_header = MessageHeader::new(0, MessageType::Heartbeat, 0);
            let msg = Message::new(msg_header, Vec::new());

            match send_queue.send(msg) {
                Ok(_) => {
                    debug!("Successfully send Heartbeat");
                }
                Err(e) => {
                    error!("[Heartbeat] Sending: {}", e);
                    return;
                }
            };

            tokio::time::sleep(wait_time).await;
        }
    }

    /// Sends all the messages to the server
    async fn sender(
        server_con: std::sync::Arc<Connection>,
        mut queue: tokio::sync::mpsc::UnboundedReceiver<Message>,
    ) {
        loop {
            let msg = match queue.recv().await {
                Some(m) => m,
                None => {
                    info!("[Sender] All Queue-Senders have been closed");
                    return;
                }
            };

            let data = msg.serialize();
            let total_data_length = data.len();

            match server_con.write_total(&data, total_data_length).await {
                Ok(_) => {}
                Err(e) => {
                    error!("Sending Message: {}", e);
                    return;
                }
            };
        }
    }

    /// Receives all the messages from the server
    ///
    /// Then adds the message to the matching connection queue.
    /// If there is no matching queue, it creates and starts a new client,
    /// which will then be placed into the Connection Manager for further
    /// requests
    ///
    ///
    ///
    async fn receiver<F, Fut, T>(
        server_con: std::sync::Arc<Connection>,
        send_queue: tokio::sync::mpsc::UnboundedSender<Message>,
        client_cons: std::sync::Arc<Connections<mpsc::StreamWriter<Message>>>,
        handler: &F,
        handler_data: &Option<T>,
    ) where
        F: Fn(u32, mpsc::StreamReader<Message>, queues::Sender, Option<T>) -> Fut,
        Fut: Future<Output = ()>,
        T: Sized + Send + Clone,
    {
        loop {
            let mut head_buf = [0; 13];
            let header = match server_con.read_total(&mut head_buf, 13).await {
                Ok(_) => match MessageHeader::deserialize(head_buf) {
                    Some(s) => s,
                    None => {
                        error!("[Receiver] Deserializing Header: {:?}", head_buf);
                        return;
                    }
                },
                Err(e) => {
                    error!("[Receiver] Reading Data: {}", e);
                    return;
                }
            };

            let id = header.get_id();
            let kind = header.get_kind();
            match kind {
                MessageType::Close => {
                    client_cons.remove(id);
                    continue;
                }
                MessageType::Data => {}
                _ => {
                    error!("[Receiver][{}] Unexpected Operation: {:?}", id, kind);
                    continue;
                }
            };

            let data_length = header.get_length() as usize;
            let mut buf = vec![0; data_length];
            let msg = match server_con.read_total(&mut buf, data_length).await {
                Ok(_) => Message::new(header, buf),
                Err(e) => {
                    error!("[Receiver][{}] Receiving Data: {}", e, id);
                    client_cons.remove(id);
                    continue;
                }
            };

            let con_queue = match client_cons.get(id) {
                Some(send_queue) => send_queue.clone(),
                // In case there is no matching user-connection, create a new one
                None => {
                    // Setup the send channel for requests for this user
                    let (tx, handle_rx) = mpsc::stream();
                    // Add the Connection to the current map of user-connection
                    client_cons.set(id, tx.clone());

                    let handle_tx =
                        queues::Sender::new(id, send_queue.clone(), client_cons.clone());

                    handler(id, handle_rx, handle_tx, handler_data.clone()).await;
                    tx
                }
            };

            match con_queue.send(msg) {
                Ok(_) => {}
                Err(e) => {
                    error!("[Receiver][{}] Adding to Queue: {}", id, e);
                }
            };
        }
    }

    /// This starts the client and all the needed tasks
    ///
    /// This function essentially never returns and should
    /// therefor be run in an independant task
    ///
    /// The handler will be called for every new user-connection that is
    /// received together with the provided handler Data
    ///
    /// The Handler will be passed as arguments:
    /// * The ID of the new user-connection
    /// * A Reader where all the Messages for this user can be read from
    /// * A Writer which can be used to send data back to the user
    /// * The handler_data that can be used to share certain information when needed
    pub async fn start<F, Fut, T>(self, handler: F, handler_data: Option<T>) -> !
    where
        F: Fn(u32, mpsc::StreamReader<Message>, queues::Sender, Option<T>) -> Fut,
        Fut: Future<Output = ()>,
        T: Sized + Send + Clone,
    {
        info!("Starting...");

        let mut attempts = 0;
        let wait_base: u64 = 2;

        loop {
            info!(
                "Conneting to server: {}",
                self.server_destination.get_full_address()
            );

            let connection_arc = match Self::establish_connection(
                &self.server_destination.get_full_address(),
                &self.key,
            )
            .await
            {
                Some(c) => c,
                None => {
                    attempts += 1;
                    let raw_time = std::time::Duration::from_secs(wait_base.pow(attempts));
                    let final_wait_time = raw_time
                        .checked_add(std::time::Duration::from_millis(
                            rand::rngs::ThreadRng::default().next_u64() % 1000,
                        ))
                        .unwrap();
                    info!(
                        "Waiting {:?} before trying to connect again",
                        final_wait_time
                    );
                    tokio::time::sleep(final_wait_time).await;

                    continue;
                }
            };

            info!("Connected to server");

            attempts = 0;

            let (queue_tx, queue_rx) = tokio::sync::mpsc::unbounded_channel();
            let outgoing = std::sync::Arc::new(Connections::<mpsc::StreamWriter<Message>>::new());

            tokio::task::spawn(Self::sender(connection_arc.clone(), queue_rx));

            tokio::task::spawn(Self::heartbeat(
                queue_tx.clone(),
                std::time::Duration::from_secs(15),
            ));

            Client::receiver(
                connection_arc,
                queue_tx.clone(),
                outgoing,
                &handler,
                &handler_data,
            )
            .await;
        }
    }
}
