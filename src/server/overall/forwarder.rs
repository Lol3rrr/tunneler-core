use crate::server::client::ClientManager;

use std::sync::Arc;
use tokio::net::TcpListener;

pub struct Forwarder {
    user_port: u16,
    clients: Arc<ClientManager>,
}

impl Forwarder {
    pub fn new(port: u16, clients: Arc<ClientManager>) -> Self {
        Self {
            user_port: port,
            clients,
        }
    }

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
