use crate::client::queues;
use crate::connections::{Connections, Destination};
use crate::message::{Message, MessageHeader, MessageType};
use crate::streams::mpsc;

use rand::RngCore;
use std::future::Future;

use log::{debug, error, info};

mod establish_connection;
mod rx;
mod tx;

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

    async fn heartbeat(send_queue: &tokio::sync::mpsc::UnboundedSender<Message>) {
        let msg_header = MessageHeader::new(0, MessageType::Heartbeat, 0);
        let msg = Message::new(msg_header, Vec::new());

        match send_queue.send(msg) {
            Ok(_) => {
                debug!("[Heartbeat] Sent");
            }
            Err(e) => {
                error!("[Heartbeat] Sending: {}", e);
                return;
            }
        };
    }

    async fn heartbeat_loop(
        send_queue: tokio::sync::mpsc::UnboundedSender<Message>,
        wait_time: std::time::Duration,
    ) {
        loop {
            Self::heartbeat(&send_queue).await;
            tokio::time::sleep(wait_time).await;
        }
    }

    /// This starts the client and all the needed tasks
    ///
    /// This function essentially never returns and should
    /// therefor be run in an independant task
    ///
    /// The Handler will be passed as arguments:
    /// * The ID of the new user-connection
    /// * A Reader where all the Messages for this user can be read from
    /// * A Writer which can be used to send data back to the user
    /// * The handler_data that can be used to share certain information when needed
    ///
    /// The `start_handler` is only ever called once for every new connection in a
    /// seperate tokio::Task
    /// All Messages the Handler receives only Data or EOF Messages
    pub async fn start<F, Fut, T>(self, start_handler: F, start_handler_data: Option<T>) -> !
    where
        F: Fn(u32, mpsc::StreamReader<Message>, queues::Sender, Option<T>) -> Fut,
        Fut: Future + Send + 'static,
        Fut::Output: Send,
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

            let connection = match establish_connection::establish_connection(
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

            let (read_con, write_con) = connection.into_split();

            let (queue_tx, queue_rx) = tokio::sync::mpsc::unbounded_channel();
            let outgoing = std::sync::Arc::new(Connections::<mpsc::StreamWriter<Message>>::new());

            tokio::task::spawn(tx::sender(write_con, queue_rx));

            tokio::task::spawn(Self::heartbeat_loop(
                queue_tx.clone(),
                std::time::Duration::from_secs(15),
            ));

            rx::receiver(
                read_con,
                queue_tx.clone(),
                outgoing,
                &start_handler,
                &start_handler_data,
            )
            .await;
        }
    }
}

#[tokio::test]
async fn heartbeat() {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    Client::heartbeat(&tx).await;

    assert_eq!(
        Some(Message::new(
            MessageHeader::new(0, MessageType::Heartbeat, 0),
            vec![]
        )),
        rx.recv().await
    );
}
