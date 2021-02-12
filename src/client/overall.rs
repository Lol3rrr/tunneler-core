use crate::client::queues;
use crate::connections::{Connection, Connections, Destination};
use crate::message::{Message, MessageHeader, MessageType};
use crate::objectpool;
use crate::streams::mpsc;

use rand::RngCore;
use std::future::Future;

use log::{debug, error, info};

mod establish_connection;

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

    async fn heartbeat(
        send_queue: tokio::sync::mpsc::Sender<Message>,
        wait_time: std::time::Duration,
    ) {
        loop {
            let msg_header = MessageHeader::new(0, MessageType::Heartbeat, 0);
            let msg = Message::new(msg_header, Vec::new());

            match send_queue.send(msg).await {
                Ok(_) => {
                    debug!("[Heartbeat] Sent");
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
        mut queue: tokio::sync::mpsc::Receiver<Message>,
    ) {
        loop {
            let msg = match queue.recv().await {
                Some(m) => m,
                None => {
                    info!("[Sender] All Queue-Senders have been closed");
                    return;
                }
            };

            let (h_data, data) = msg.serialize();
            match server_con.write_total(&h_data, h_data.len()).await {
                Ok(_) => {
                    debug!("Sent Header");
                }
                Err(e) => {
                    error!("Sending Header: {}", e);
                    return;
                }
            };
            match server_con.write_total(data, data.len()).await {
                Ok(_) => {
                    debug!("Sent Data");
                }
                Err(e) => {
                    error!("Sending Data: {}", e);
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
    async fn receiver<F, Fut, T>(
        server_con: std::sync::Arc<Connection>,
        send_queue: tokio::sync::mpsc::Sender<Message>,
        client_cons: std::sync::Arc<Connections<mpsc::StreamWriter<Message>>>,
        handler: &F,
        handler_data: &Option<T>,
        obj_pool: objectpool::Pool<Vec<u8>>,
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
            let mut buf = obj_pool.get();
            buf.resize(data_length, 0);

            let msg = match server_con.read_total(&mut buf, data_length).await {
                Ok(_) => Message::new_guarded(header, buf),
                Err(e) => {
                    error!("[Receiver][{}] Receiving Data: {}", e, id);
                    client_cons.remove(id);
                    continue;
                }
            };

            let con_queue = match client_cons.get(id) {
                Some(q) => q.clone(),
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

            match con_queue.send(msg).await {
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

            let connection_arc = match establish_connection::establish_connection(
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

            let (queue_tx, queue_rx) = tokio::sync::mpsc::channel(30);
            let outgoing = std::sync::Arc::new(Connections::<mpsc::StreamWriter<Message>>::new());

            tokio::task::spawn(Self::sender(connection_arc.clone(), queue_rx));

            tokio::task::spawn(Self::heartbeat(
                queue_tx.clone(),
                std::time::Duration::from_secs(15),
            ));

            let obj_pool = objectpool::Pool::new(100);
            Client::receiver(
                connection_arc,
                queue_tx.clone(),
                outgoing,
                &handler,
                &handler_data,
                obj_pool,
            )
            .await;
        }
    }
}
