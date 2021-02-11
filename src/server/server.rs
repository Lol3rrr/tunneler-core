use crate::server::client::ClientManager;

use rand::Rng;
use tokio::net::TcpListener;

use log::{error, info};

mod accept_clients;

/// Holds all information needed to creating and running
/// a single Tunneler-Server
#[derive(Debug, PartialEq)]
pub struct Server {
    listen_port: u32,
    public_port: u32,
    key: Vec<u8>,
}

impl Server {
    /// Creates a new Server-Instance from the given Data
    ///
    /// Params:
    /// * public_port: The Port on which user-requests should be accepted on
    /// * listen_port: The Port clients will connect to
    /// * key: The Key/Password clients need to connect to the server
    pub fn new(public_port: u32, listen_port: u32, key: Vec<u8>) -> Self {
        Self {
            public_port,
            listen_port,
            key,
        }
    }

    /// Starts the actual server itself along with all the needed tasks
    ///
    /// This is a blocking call and is not expected to return
    pub async fn start(self) -> std::io::Result<()> {
        info!("Starting...");

        let listen_bind_addr = format!("0.0.0.0:{}", self.listen_port);
        let listen_listener = TcpListener::bind(&listen_bind_addr).await?;

        let req_bind_addr = format!("0.0.0.0:{}", self.public_port);
        let req_listener = TcpListener::bind(&req_bind_addr).await?;

        let clients = std::sync::Arc::new(ClientManager::new());

        info!("Listening for Clients on: {}", listen_bind_addr);
        info!("Listening for User on: {}", req_bind_addr);

        // Task to async accept new clients
        tokio::task::spawn(accept_clients::accept_clients(
            listen_listener,
            self.key,
            clients.clone(),
        ));

        let mut rng = rand::thread_rng();

        // Accepting User-Requests
        loop {
            let socket = match req_listener.accept().await {
                Ok((raw_socket, _)) => raw_socket,
                Err(e) => {
                    error!("Accepting Req-Connection: {}", e);
                    continue;
                }
            };

            let id = rng.gen();
            let client = clients.get();
            if client.is_none() {
                error!("Could not obtain a Client-Connection");
                continue;
            }

            client.unwrap().new_con(id, socket);
        }
    }
}

#[test]
fn new_server() {
    assert_eq!(
        Server {
            public_port: 80,
            listen_port: 8080,
            key: vec![2, 3, 1],
        },
        Server::new(80, 8080, vec![2, 3, 1])
    );
}
