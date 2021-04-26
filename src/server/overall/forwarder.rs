use crate::server::client::ClientManager;

use std::sync::Arc;
use tokio::net::TcpListener;

/// The Forwarder is the actual Part that accepts User-Connections
/// and then forwards them to one of the Clients that listen on that
/// port
pub struct Forwarder {
    /// The External Port where users connect to
    user_port: u16,
    /// All the Clients that want to receive connections from this
    /// instance
    clients: Arc<ClientManager>,
}

impl Forwarder {
    /// Creates a new Forwarder
    pub fn new(port: u16, clients: Arc<ClientManager>) -> Self {
        Self {
            user_port: port,
            clients,
        }
    }

    /// Actually starts the Forwarder
    pub async fn start(self) {
        let req_bind_addr = format!("0.0.0.0:{}", self.user_port);
        let req_listener = match TcpListener::bind(&req_bind_addr).await {
            Ok(r) => r,
            Err(e) => {
                log::error!("Binding Forwarder: {:?}", e);
                return;
            }
        };

        log::info!("Listening for Users on: {}", req_bind_addr);

        let mut id: u32 = 0;

        // Accepting User-Requests
        loop {
            let socket = match req_listener.accept().await {
                Ok((raw_socket, _)) => raw_socket,
                Err(e) => {
                    log::error!("Accepting Req-Connection: {}", e);
                    continue;
                }
            };

            let client = match self.clients.get() {
                Some(c) => c,
                None => {
                    log::error!("Could not obtain a Client-Connection");
                    continue;
                }
            };

            id = id.wrapping_add(1);
            client.new_con(id, socket);
        }
    }
}
