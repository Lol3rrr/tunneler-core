use crate::general::ConnectionWriter;
use crate::message::Message;
use crate::streams::{error::RecvError, mpsc};

use log::error;

/// Reads messages from the Client for this User and sends them to the User
///
/// Params:
/// * client_id: The ID of the client that handles this
/// * user_id: The ID of the User for this connection
/// * con: The User-Connection
/// * queue: The Queue for messages that need to be send to the user
pub async fn send<C>(
    client_id: u32,
    user_id: u32,
    mut con: C,
    mut queue: mpsc::StreamReader<Message>,
) where
    C: ConnectionWriter,
{
    loop {
        let msg = match queue.recv().await {
            Ok(m) => m,
            Err(e) => {
                if e != RecvError::Closed {
                    error!("[{}][{}] Receiving from Queue: {}", client_id, user_id, e);
                }
                return;
            }
        };

        let data = msg.get_data();
        match con.write_full(&data).await {
            Ok(_) => {}
            Err(e) => {
                error!("[{}][{}] Sending to User: {}", client_id, user_id, e);
                return;
            }
        };
    }
}
