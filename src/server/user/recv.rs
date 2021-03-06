use crate::general::ConnectionReader;
use crate::message::{Message, MessageHeader, MessageType};

use log::error;

#[cfg(test)]
use crate::general::mocks::MockReader;

const BUFFER_SIZE: usize = 4096;

/// Reads from a new User-Connection and sends it to the client
///
/// Params:
/// * client: The Server-Client to use
/// * id: The ID of the user-connection
/// * con: The User-Connection
/// * send_queue: The Queue for requests going out to the Client
/// * user_cons: The User-Connections belonging to this Client
pub async fn recv<F, C>(
    client_id: u32,
    user_id: u32,
    mut con: C,
    send_queue: tokio::sync::mpsc::UnboundedSender<Message>,
    close_user: F,
) where
    C: ConnectionReader + Send,
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
            Ok(n) => {
                let message_type = if n > 0 {
                    MessageType::Data
                } else {
                    MessageType::EOF
                };

                buf.truncate(n);

                // Package the Users-Data in a new custom-message
                let header = MessageHeader::new(user_id, message_type, n as u64);
                let msg = Message::new(header, buf);

                // Puts the message in the queue to be send to the client
                if let Err(e) = send_queue.send(msg) {
                    error!(
                        "[{}][{}] Forwarding message to client: {}",
                        client_id, user_id, e
                    );
                    break;
                }
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

#[tokio::test]
async fn valid_read() {
    let mut reader = MockReader::new();
    reader.add_bytes(&vec![0, 1, 2, 3, 4, 5, 6]);
    reader.close();

    let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    async fn close_con(tmp: std::sync::Arc<std::sync::atomic::AtomicBool>) {
        tmp.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    let (queue_tx, mut queue_rx) = tokio::sync::mpsc::unbounded_channel();

    let client_id = 12;
    let user_id = 5;
    recv(
        client_id,
        user_id,
        reader,
        queue_tx,
        close_con(called.clone()),
    )
    .await;

    assert_eq!(
        Some(Message::new(
            MessageHeader::new(user_id, MessageType::Data, 7),
            vec![0, 1, 2, 3, 4, 5, 6]
        )),
        queue_rx.recv().await
    );
    assert_eq!(
        Some(Message::new(
            MessageHeader::new(user_id, MessageType::EOF, 0),
            vec![]
        )),
        queue_rx.recv().await
    );
    assert_eq!(true, called.load(std::sync::atomic::Ordering::SeqCst));
}
