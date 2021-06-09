use crate::{
    connections::{Connections, Destination},
    general::Metrics,
    message::Message,
    metrics,
    streams::mpsc,
};

#[cfg(test)]
pub(crate) mod mocks;

mod traits;
pub use traits::*;

mod queues;
pub use queues::Sender as QueueSender;

use rand::RngCore;
use std::sync::Arc;

use log::info;

use self::connections::establish_connection::EstablishConnectionError;

mod connections;
mod heartbeat;

#[derive(Debug)]
pub(crate) enum ConnectError {
    Establish(EstablishConnectionError),
}

impl From<EstablishConnectionError> for ConnectError {
    fn from(other: EstablishConnectionError) -> Self {
        Self::Establish(other)
    }
}

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
    ///
    /// # Behaviour
    /// This starts 2 more tasks needed for the Client to work
    /// properly and then blocks on the 3. function.
    /// Therefore this function should only return once the
    /// Connection is being terminated
    async fn start_con<H>(&self, handler: Arc<H>) -> Result<(), ConnectError>
    where
        H: Handler + Send + Sync + 'static,
    {
        info!(
            "Conneting to server: {}",
            self.server_destination.get_full_address()
        );

        let (read_con, write_con) = connections::establish_connection::establish_connection(
            self.server_destination.get_full_address(),
            &self.key,
            self.external_port,
        )
        .await?
        .into_split();

        info!("Connected to server");

        let (queue_tx, queue_rx) = tokio::sync::mpsc::unbounded_channel();
        let outgoing = std::sync::Arc::new(Connections::<mpsc::StreamWriter<Message>>::new());

        // The Heartbeat loop used to keep the Connection open and verify that it
        // is still working
        tokio::task::spawn(heartbeat::keep_alive(
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
            handler,
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
    pub async fn start<H>(self, handler: Arc<H>) -> !
    where
        H: Handler + Send + Sync + 'static,
    {
        info!("Starting...");

        let mut attempts = 0;

        loop {
            match self.start_con(handler.clone()).await {
                Ok(_) => {
                    attempts = 0;
                }
                Err(e) => {
                    log::error!("Connecting: {:?}", e);

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
