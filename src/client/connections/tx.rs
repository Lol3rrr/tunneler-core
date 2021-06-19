use std::sync::Arc;

use crate::{
    general::{ConnectionWriter, Metrics},
    message::Message,
};

#[derive(Debug)]
enum SendError {
    ReceivingMessage,
    Sending(std::io::Error),
}

async fn send_single<C, M>(
    con: &mut C,
    queue: &mut tokio::sync::mpsc::UnboundedReceiver<Message>,
    head_buf: &mut [u8; 13],
    metrics: &M,
) -> Result<(), SendError>
where
    C: ConnectionWriter + Send,
    M: Metrics + Send + Sync,
{
    let msg = match queue.recv().await {
        Some(m) => m,
        None => return Err(SendError::ReceivingMessage),
    };

    if let Err(e) = con.write_msg(&msg, head_buf).await {
        return Err(SendError::Sending(e));
    }

    metrics.send_msg();
    metrics.send_bytes(msg.get_header().get_length());

    Ok(())
}

/// Sends all the messages to the server
pub async fn sender<M>(
    mut server_con: tokio::net::tcp::OwnedWriteHalf,
    mut queue: tokio::sync::mpsc::UnboundedReceiver<Message>,
    metrics: Arc<M>,
) where
    M: Metrics + Send + Sync,
{
    let mut h_data = [0; 13];
    loop {
        match send_single(&mut server_con, &mut queue, &mut h_data, metrics.as_ref()).await {
            Ok(_) => {}
            Err(e) => {
                error!("Sending-Single: {:?}", e);
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::general::mocks;
    use crate::message::{MessageHeader, MessageType};
    use crate::metrics::Empty;

    #[tokio::test]
    async fn valid_send_single() {
        let mut mock_connection = mocks::MockWriter::new();
        let (queue_tx, mut queue_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut head_buf = [0; 13];

        let id = 12;
        queue_tx
            .send(Message::new(
                MessageHeader::new(id, MessageType::Data, 5),
                vec![2; 5],
            ))
            .unwrap();

        assert_eq!(
            true,
            send_single(
                &mut mock_connection,
                &mut queue_rx,
                &mut head_buf,
                &Empty::new()
            )
            .await
            .is_ok()
        );

        assert_eq!(
            &vec![vec![12, 0, 0, 0, 3, 5, 0, 0, 0, 0, 0, 0, 0], vec![2; 5]],
            mock_connection.chunks()
        );
    }
}
