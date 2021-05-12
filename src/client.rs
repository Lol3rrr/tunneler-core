use crate::{
    connections::{Connections, Destination},
    general::Metrics,
    message::{Message, MessageHeader, MessageType},
    metrics,
    streams::mpsc,
};

mod traits;
pub use traits::*;

mod queues;
pub use queues::Sender as QueueSender;

use rand::RngCore;
use std::{future::Future, sync::Arc};

use log::info;

mod connections;

/// The Client instance in general that is responsible for handling
/// all the interactions with the Server
pub struct Client<M> {
    server_destination: Destination,
    external_port: u16,
    key: Vec<u8>,
    metrics: Arc<M>,
}

impl Client<metrics::Empty> {
    /// Creates a new Client instance that is configured to
    /// connect to the given Server Destination and authenticate
    /// using the provided Key.
    ///
    /// This uses an empty Metrics-Collector meaning that no metrics
    /// will be collected
    pub fn new(server: Destination, external_port: u16, key: Vec<u8>) -> Self {
        Self {
            server_destination: server,
            external_port,
            key,
            metrics: Arc::new(metrics::Empty::new()),
        }
    }
}

impl<M> Client<M>
where
    M: Metrics + Send + Sync + 'static,
{
    /// Behaves the same as the `new` implementation with the difference being
    /// that you can specify and provide your own Metrics-Collector.
    pub fn new_metrics(
        server: Destination,
        external_port: u16,
        key: Vec<u8>,
        metrics_collector: M,
    ) -> Self {
        Self {
            server_destination: server,
            external_port,
            key,
            metrics: Arc::new(metrics_collector),
        }
    }

    async fn heartbeat_loop(
        send_queue: tokio::sync::mpsc::UnboundedSender<Message>,
        wait_time: std::time::Duration,
    ) {
        loop {
            let msg_header = MessageHeader::new(0, MessageType::Heartbeat, 0);
            let msg = Message::new(msg_header, Vec::new());
            if let Err(e) = send_queue.send(msg) {
                log::error!("Sending Heartbeat: {}", e);
                return;
            };
            tokio::time::sleep(wait_time).await;
        }
    }

    /// Calculates the Time that should be waited before retrying
    fn exponential_backoff(
        attempt: u32,
        max_time: Option<std::time::Duration>,
    ) -> Option<std::time::Duration> {
        let raw_time = std::time::Duration::from_secs(2u64.pow(attempt));
        let calced_result = raw_time.checked_add(std::time::Duration::from_millis(
            rand::rngs::ThreadRng::default().next_u64() % 1000,
        ))?;

        match max_time {
            Some(max) if calced_result > max => Some(max),
            _ => Some(calced_result),
        }
    }

    /// Establishes and then also runs a new Connection
    async fn start_con<F, Fut, T>(
        &self,
        start_handler: &F,
        start_handler_data: &Option<T>,
    ) -> Result<(), ()>
    where
        F: Fn(u32, mpsc::StreamReader<Message>, QueueSender, Option<T>) -> Fut,
        Fut: Future + Send + 'static,
        Fut::Output: Send,
        T: Sized + Send + Clone,
    {
        info!(
            "Conneting to server: {}",
            self.server_destination.get_full_address()
        );

        let (read_con, write_con) = match connections::establish_connection::establish_connection(
            self.server_destination.get_full_address(),
            &self.key,
            self.external_port,
        )
        .await
        {
            Some(c) => c.into_split(),
            None => {
                return Err(());
            }
        };

        info!("Connected to server");

        let (queue_tx, queue_rx) = tokio::sync::mpsc::unbounded_channel();
        let outgoing = std::sync::Arc::new(Connections::<mpsc::StreamWriter<Message>>::new());

        // The Heartbeat loop used to keep the Connection open and verify that it
        // is still working
        tokio::task::spawn(Self::heartbeat_loop(
            queue_tx.clone(),
            std::time::Duration::from_secs(15),
        ));

        // Runs the Sender in the Background
        // This task is responsible for sending out all the Queued up Messages
        tokio::task::spawn(connections::tx::sender(
            write_con,
            queue_rx,
            self.metrics.clone(),
        ));

        // This task is responsible for receiving all the Messages by the Server
        // and adds them to the fitting Queue
        connections::rx::receiver(
            read_con,
            queue_tx.clone(),
            outgoing,
            start_handler,
            start_handler_data,
            self.metrics.clone(),
        )
        .await;

        Ok(())
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

        loop {
            match self.start_con(&start_handler, &start_handler_data).await {
                Ok(_) => {
                    attempts = 0;
                }
                Err(_) => {
                    attempts += 1;
                    if let Some(wait_time) = Self::exponential_backoff(
                        attempts,
                        Some(std::time::Duration::from_secs(60)),
                    ) {
                        info!("Waiting {:?} before trying to connect again", wait_time);
                        tokio::time::sleep(wait_time).await;
                    }
                }
            };
        }
    }
}
