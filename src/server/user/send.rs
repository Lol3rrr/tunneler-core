use crate::general::ConnectionWriter;
use crate::message::Message;
use crate::streams::{error::RecvError, mpsc};

use log::error;

/// Returns:
/// * `true` if everything worked fine
/// * `false` if an error was encountered
async fn send_single<C>(
    client_id: u32,
    user_id: u32,
    con: &mut C,
    queue: &mut mpsc::StreamReader<Message>,
) -> bool
where
    C: ConnectionWriter + Send,
{
    let msg = match queue.recv().await {
        Ok(m) => m,
        Err(e) => {
            if e != RecvError::Closed {
                error!("[{}][{}] Receiving from Queue: {}", client_id, user_id, e);
            }
            return false;
        }
    };

    let data = msg.get_data();
    if let Err(e) = con.write_full(&data).await {
        error!("[{}][{}] Sending to User: {}", client_id, user_id, e);
        return false;
    }
    true
}

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
    C: ConnectionWriter + Send,
{
    loop {
        if !send_single(client_id, user_id, &mut con, &mut queue).await {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        general::mocks::MockWriter,
        message::{MessageHeader, MessageType},
    };

    #[tokio::test]
    async fn valid_send_single() {
        let mut mock_writer = MockWriter::new();
        let (queue_tx, mut queue_rx) = mpsc::stream();

        queue_tx
            .send(Message::new(
                MessageHeader::new(10, MessageType::Data, 5),
                vec![0, 1, 2, 3, 4],
            ))
            .unwrap();

        assert_eq!(
            true,
            send_single(1, 10, &mut mock_writer, &mut queue_rx).await
        );

        assert_eq!(vec![vec![0, 1, 2, 3, 4]], mock_writer.chunks());
    }
}
