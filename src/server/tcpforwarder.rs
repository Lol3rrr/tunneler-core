mod client;
pub use client::TCPClient;

use std::sync::Arc;
use tokio::net::TcpListener;

use super::clientmanager::ClientManager;

/// The TCP-Forwarder is the actual Part that accepts User-Connections
/// and then forwards them to one of the Clients that listen on that
/// port
pub struct TCPForwarder {
    /// The External Port where users connect to
    user_port: u16,
    /// The Listener of the Forwarder
    listener: TcpListener,
    /// All the Clients that want to receive connections from this
    /// instance
    clients: Arc<ClientManager<TCPClient>>,
}

impl TCPForwarder {
    /// Creates a new Forwarder
    ///
    /// # Params:
    /// * 'port': The Public facing User-Port
    /// * 'clients': The List of Clients for this Port/Forwarder
    pub async fn new(
        port: u16,
        clients: Arc<ClientManager<TCPClient>>,
    ) -> Result<Self, std::io::Error> {
        let bind_addr = format!("0.0.0.0:{}", port);
        let listener = TcpListener::bind(&bind_addr).await?;

        Ok(Self {
            user_port: port,
            listener,
            clients,
        })
    }

    /// Actually starts the Forwarder
    /// This will never return
    pub async fn start(self) -> ! {
        info!("Listening for Users on Port: {}", self.user_port);

        let mut id: u32 = 0;

        // Accepting User-Requests
        loop {
            let user_socket = match self.listener.accept().await {
                Ok((raw_socket, _)) => raw_socket,
                Err(e) => {
                    error!("[{}] Accepting Req-Connection: {}", self.user_port, e);
                    continue;
                }
            };

            // Get a connect Client for this new User Connection
            let client = match self.clients.get() {
                Some(c) => c,
                None => {
                    error!("[{}] Could not obtain a Client-Connection", self.user_port);
                    continue;
                }
            };

            id = id.wrapping_add(1);
            client.new_con(id, user_socket);
        }
    }
}
