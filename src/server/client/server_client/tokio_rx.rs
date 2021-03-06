use crate::connections::Connections;
use crate::general::ConnectionReader;
use crate::message::{Message, MessageHeader, MessageType};
use crate::objectpool::Pool;
use crate::server::client::ClientManager;
use crate::streams::mpsc;

use log::error;

pub async fn receive<C>(
    id: u32,
    read_con: &mut C,
    user_cons: &Connections<mpsc::StreamWriter<Message>>,
    client_manager: &std::sync::Arc<ClientManager>,
    obj_pool: &Pool<Vec<u8>>,
    header_buf: &mut [u8; 13],
) -> Result<(), ()>
where
    C: ConnectionReader + Send,
{
    let header = match read_con.read_full(header_buf).await {
        Ok(_) => match MessageHeader::deserialize(header_buf) {
            Some(h) => h,
            None => {
                error!("[{}] Deserializing Header: {:?}", id, header_buf);
                return Ok(());
            }
        },
        Err(e) => {
            error!("[{}] Reading from Client-Connection: {}", id, e);
            client_manager.remove(id);
            return Err(());
        }
    };

    match header.get_kind() {
        MessageType::Data => {}
        MessageType::Close => {
            user_cons.remove(header.get_id());
            return Ok(());
        }
        MessageType::Heartbeat => {
            return Ok(());
        }
        _ => {
            error!(
                "[{}][{}] Unexpected Operation: {:?}",
                id,
                header.get_id(),
                header.get_kind()
            );
            read_con.drain(header.get_length() as usize).await;
            return Ok(());
        }
    };

    let user_id = header.get_id();

    // Forwarding the message to the actual user
    let stream = match user_cons.get_clone(user_id) {
        Some(s) => s,
        None => {
            // Removes this message and drain all the Data belonging to this message
            // as well
            read_con.drain(header.get_length() as usize).await;
            return Ok(());
        }
    };

    let body_length = header.get_length() as usize;
    let mut body_buf = obj_pool.get();
    body_buf.resize(body_length, 0);
    match read_con.read_full(&mut body_buf).await {
        Ok(_) => {}
        Err(e) => {
            error!("[{}][{}] Reading Body from Client: {}", id, user_id, e);
        }
    };

    match stream.send(Message::new_guarded(header, body_buf)) {
        Ok(_) => {}
        Err(e) => {
            error!("[{}][{}] Adding to User-Queue: {}", id, user_id, e);
        }
    };
    Ok(())
}
