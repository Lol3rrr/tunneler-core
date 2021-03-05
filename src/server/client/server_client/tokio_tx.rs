use crate::server::client::ClientManager;
use crate::{general::ConnectionWriter, message::Message};

use log::error;

pub async fn send<C>(
    id: u32,
    write_con: &mut C,
    queue: &mut tokio::sync::mpsc::UnboundedReceiver<Message>,
    client_manager: &std::sync::Arc<ClientManager>,
    header_buf: &mut [u8; 13],
) -> Result<(), ()>
where
    C: ConnectionWriter + Send,
{
    let msg = match queue.recv().await {
        Some(m) => m,
        None => {
            error!("[{}][Sender] Receiving Message from Queue", id);
            client_manager.remove(id);
            return Err(());
        }
    };

    if let Err(e) = write_con.write_msg(&msg, header_buf).await {
        error!("[{}][Sender] Sending Message: {}", id, e);
        client_manager.remove(id);
        return Err(());
    }

    Ok(())
}
