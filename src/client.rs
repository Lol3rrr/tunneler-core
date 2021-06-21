//! This module contains all the Client-Specific logic
//!
//! # Client
//! A single Client connects to a single Server-Instance to then receive
//! User-Connections from said Server. Once a new Connection has been started
//! the Handler of the Client will be called with the sending and receiving
//! halfes

use crate::{
    connections::{Connections, Destination},
    handshake,
    message::Message,
    metrics,
    metrics::Metrics,
    streams::mpsc,
};

#[cfg(test)]
pub(crate) mod mocks;

mod traits;
pub use traits::*;

use rand::RngCore;
use std::sync::Arc;

mod connections;
mod heartbeat;

pub use connections::{user_con, UserCon};

#[derive(Debug)]
pub(crate) enum ConnectError {
    IO(std::io::Error),
    Handshake(handshake::HandshakeError),
}

impl From<std::io::Error> for ConnectError {
    fn from(other: std::io::Error) -> Self {
        Self::IO(other)
    }
}
impl From<handshake::HandshakeError> for ConnectError {
    fn from(other: handshake::HandshakeError) -> Self {
        Self::Handshake(other)
    }
}

/// The Client instance itself, which connects to the configured Server-
/// Instance and manages all the underlying communication with ther Server
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
    /// This uses an empty Metrics-Collector, meaning that no metrics
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

impl<M> Client<M> {
    /// Calculates the Time that should be waited before retrying
    fn exponential_backoff(
        attempt: u32,
        max_time: Option<std::time::Duration>,
    ) -> std::time::Duration {
        let raw_time = std::time::Duration::from_secs(2u64.pow(attempt));
        let raw_jitter = rand::rngs::ThreadRng::default().next_u64() % 1000;
        let raw_calced = raw_time.checked_add(std::time::Duration::from_millis(raw_jitter));

        match (max_time, raw_calced) {
            (Some(max), Some(calced)) if calced > max => max,
            (_, Some(calced)) => calced,
            (Some(max), _) => max,
            _ => std::time::Duration::from_millis(0),
        }
    }
}

impl<M> Client<M>
where
    M: Metrics + Send + Sync + 'static,
{
    /// Behaves the same as the `new` implementation with the difference being
    /// that you can specify and provide your own Metrics-Collector.
    pub fn new_with_metrics(
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
        info!("Establishing Connection...");

        let target_addr = self.server_destination.get_full_address();
        debug!("Conneting to server: {}", target_addr);
        let mut connection = tokio::net::TcpStream::connect(target_addr).await?;
        debug!("Connected to Server");

        debug!("Starting Handshake...");
        handshake::client::perform(&mut connection, &self.key, self.external_port).await?;
        debug!("Performed Handshake");

        let (read_con, write_con) = connection.into_split();

        info!("Established Conection");

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

    /// This starts up the Client to receive new Connections from the Server.
    ///
    /// The `handler` will be used to actually handle and "process" new
    /// connections
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
                    error!("Connecting: {:?}", e);

                    attempts += 1;
                    let wait_time = Self::exponential_backoff(
                        attempts,
                        Some(std::time::Duration::from_secs(60)),
                    );
                    info!("Waiting {:?} before trying to connect again", wait_time);
                    tokio::time::sleep(wait_time).await;
                }
            };
        }
    }
}
