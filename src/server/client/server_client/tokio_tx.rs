use crate::message::Message;
use crate::server::client::ClientManager;

use log::error;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp;

pub async fn send(
    id: u32,
    write_con: &mut tcp::OwnedWriteHalf,
    queue: &mut tokio::sync::mpsc::UnboundedReceiver<Message>,
    client_manager: &std::sync::Arc<ClientManager>,
    header_buf: &mut [u8; 13],
) -> Result<(), ()> {
    let msg = match queue.recv().await {
        Some(m) => m,
        None => {
            error!("[{}][Sender] Receiving Message from Queue", id);
            client_manager.remove(id);
            return Err(());
        }
    };

    let data = msg.serialize(header_buf);
    match write_con.write_all(header_buf).await {
        Ok(_) => {}
        Err(e) => {
            error!("[{}][Sender] Sending Message: {}", id, e);
            client_manager.remove(id);
            return Err(());
        }
    };
    match write_con.write_all(&data).await {
        Ok(_) => {}
        Err(e) => {
            error!("[{}][Sender] Sending Message: {}", id, e);
            client_manager.remove(id);
            return Err(());
        }
    };

    Ok(())
}
