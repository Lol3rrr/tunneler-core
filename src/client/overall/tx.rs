use crate::connections::Connection;
use crate::message::Message;

use log::{debug, error, info};

/// Sends all the messages to the server
pub async fn sender(
    server_con: std::sync::Arc<Connection>,
    mut queue: tokio::sync::mpsc::UnboundedReceiver<Message>,
) {
    let mut h_data = [0; 13];
    loop {
        let msg = match queue.recv().await {
            Some(m) => m,
            None => {
                info!("All Queue-Senders have been closed");
                return;
            }
        };

        let data = msg.serialize(&mut h_data);
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
