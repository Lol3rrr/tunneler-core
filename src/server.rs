//! This contains all the logic needed for running the Server-Side of this
//!
//! # Structure
//! ## Server
//! The Server itself is the overarching "Manager"/"Handler" for all how it all
//! works together. It is therefore also responsible for accepting the Client-
//! Connections and creating their appropriate Forwarders or adding a Client
//! to a new Forwarder.
//!
//! ## Forwarder
//! A Forwarder is responsible for accepting the connections from actual
//! Users and forwarding them to a given Client and managing their Data
//! exchange for the entire lifetime of the connection

use crate::{handshake, metrics, metrics::Metrics};

use rand::Rng;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::net::TcpListener;

mod forwarder;
use forwarder::{Client, ClientManager};
mod ports;
mod user;

use forwarder::Forwarder;
pub use ports::Strategy;

/// Holds all information needed to creating and running
/// a single Tunneler-Server
#[derive(Debug, PartialEq)]
pub struct Server<M> {
    listen_port: u32,
    port_strategy: Strategy,
    key: Vec<u8>,
    metrics: Arc<M>,
}

impl Server<metrics::Empty> {
    /// Creates a new Server-Instance from the given Data
    ///
    /// Params:
    /// * listen_port: The Port clients will connect to
    /// * port_strategy: The Strategy to determine if a port a client wants to use is valid
    /// * key: The Key/Password clients need to connect to the server
    pub fn new(listen_port: u32, port_strategy: Strategy, key: Vec<u8>) -> Self {
        Self::new_metrics(listen_port, port_strategy, key, metrics::Empty::new())
    }
}

impl<M> Server<M>
where
    M: Metrics,
{
    /// Creates a new Server-Instance from the given Data
    ///
    /// Params:
    /// * listen_port: The Port clients will connect to
    /// * port_strategy: The Strategy to determine if a port a client wants to use is valid
    /// * key: The Key/Password clients need to connect to the server
    /// * p_metrics: The Metrics-Collector to use
    pub fn new_metrics(
        listen_port: u32,
        port_strategy: Strategy,
        key: Vec<u8>,
        p_metrics: M,
    ) -> Self {
        Self {
            listen_port,
            port_strategy,
            key,
            metrics: Arc::new(p_metrics),
        }
    }

    /// Actually starts the Server and starts listening for incoming Connections from
    /// both users and clients.
    ///
    /// # Behaviour
    /// This function is not expected to return as all the connections will be handled
    /// internally or by other parts of the System, so this one can keep accepting new
    /// ones
    pub async fn listen(self) -> Result<(), ()> {
        info!("Starting...");

        let listen_bind_addr = format!("0.0.0.0:{}", self.listen_port);
        let client_listener = match TcpListener::bind(&listen_bind_addr).await {
            Ok(l) => l,
            Err(e) => {
                error!("Binding to Address('{}'): {:?}", listen_bind_addr, e);
                return Err(());
            }
        };

        info!("Listening for Clients on: {}", listen_bind_addr);

        let mut ports: BTreeMap<u16, Arc<ClientManager>> = BTreeMap::new();

        // Accept new Clients
        loop {
            // Get Client
            let mut client_socket = match client_listener.accept().await {
                Ok((socket, _)) => socket,
                Err(e) => {
                    error!("Accepting client-connection: {}", e);
                    continue;
                }
            };

            let port = match handshake::server::perform(&mut client_socket, &self.key, |port| {
                self.port_strategy.contains_port(port)
            })
            .await
            {
                Ok(p) => p,
                Err(e) => {
                    error!("Validating Client-Connection: {:?}", e);
                    continue;
                }
            };

            let clients = match ports.get(&port) {
                Some(c) => c.clone(),
                None => {
                    // Create new Client-List for the Port and start a Forwarder for
                    // the Port as well
                    let tmp = Arc::new(ClientManager::new());
                    let fwd = match Forwarder::new(port, tmp.clone()).await {
                        Ok(f) => f,
                        Err(e) => {
                            error!("Binding Forwader: {:?}", e);
                            continue;
                        }
                    };
                    tokio::task::spawn(fwd.start());

                    ports.insert(port, tmp.clone());
                    tmp
                }
            };

            let c_id: u32 = rand::thread_rng().gen();

            info!("Accepted client: {}", c_id);

            let (rx, tx) = client_socket.into_split();

            let (queue_tx, queue_rx) = tokio::sync::mpsc::unbounded_channel();

            let client = Client::new(c_id, clients.clone(), queue_tx);

            tokio::task::spawn(Client::sender(c_id, tx, queue_rx, clients.clone()));
            tokio::task::spawn(Client::receiver(
                c_id,
                rx,
                client.get_user_cons(),
                clients.clone(),
            ));

            clients.add(client);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_server() {
        assert_eq!(
            Server {
                listen_port: 8080,
                port_strategy: Strategy::Single(12),
                key: vec![2, 3, 1],
                metrics: Arc::new(metrics::Empty::new()),
            },
            Server::new(8080, Strategy::Single(12), vec![2, 3, 1])
        );
    }
}
