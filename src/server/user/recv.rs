use crate::message::{Message, MessageHeader, MessageType};

use log::error;
use tokio::io::AsyncReadExt;

const BUFFER_SIZE: usize = 4096;

/// Reads from a new User-Connection and sends it to the client
///
/// Params:
/// * client: The Server-Client to use
/// * id: The ID of the user-connection
/// * con: The User-Connection
/// * send_queue: The Queue for requests going out to the Client
/// * user_cons: The User-Connections belonging to this Client
pub async fn recv<F>(
    client_id: u32,
    user_id: u32,
    mut con: tokio::net::tcp::OwnedReadHalf,
    send_queue: tokio::sync::mpsc::UnboundedSender<Message>,
    close_user: F,
) where
    F: std::future::Future<Output = ()>,
{
    // Reads and forwards all the data from the socket to the client
    loop {
        let mut buf = vec![0; BUFFER_SIZE];

        // Try to read data from the user
        //
        // this may still fail with `WouldBlock` if the readiness event is
        // a false positive.
        match con.read(&mut buf).await {
            Ok(0) => {
                // Package the Users-Data in a new custom-message
                let header = MessageHeader::new(user_id, MessageType::EOF, 0);
                let msg = Message::new(header, buf);

                // Puts the message in the queue to be send to the client
                match send_queue.send(msg) {
                    Ok(_) => {}
                    Err(e) => {
                        error!(
                            "[{}][{}] Forwarding EOF message to client: {}",
                            client_id, user_id, e
                        );
                    }
                };

                break;
            }
            Ok(n) => {
                // Package the Users-Data in a new custom-message
                let header = MessageHeader::new(user_id, MessageType::Data, n as u64);
                let msg = Message::new(header, buf);

                // Puts the message in the queue to be send to the client
                match send_queue.send(msg) {
                    Ok(_) => {}
                    Err(e) => {
                        error!(
                            "[{}][{}] Forwarding message to client: {}",
                            client_id, user_id, e
                        );
                        break;
                    }
                };
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => {
                error!("[{}][{}] Reading from User-Con: {}", client_id, user_id, e);
                break;
            }
        }
    }

    // This then actually closes the Connection
    close_user.await;
}
