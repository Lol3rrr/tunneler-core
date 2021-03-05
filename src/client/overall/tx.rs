use crate::{general::ConnectionWriter, message::Message};

use log::{debug, error};

#[cfg(test)]
use crate::general::mocks;
#[cfg(test)]
use crate::message::{MessageHeader, MessageType};

async fn send_single<C>(
    con: &mut C,
    queue: &mut tokio::sync::mpsc::UnboundedReceiver<Message>,
    head_buf: &mut [u8; 13],
) -> bool
where
    C: ConnectionWriter + Send,
{
    let msg = match queue.recv().await {
        Some(m) => m,
        None => {
            debug!("All Queue-Senders have been closed");
            return false;
        }
    };

    let data = msg.serialize(head_buf);
    if let Err(e) = con.write_full(head_buf).await {
        error!("Sending Header: {}", e);
        return false;
    }
    if let Err(e) = con.write_full(data).await {
        error!("Sending Data: {}", e);
        return false;
    }

    true
}

/// Sends all the messages to the server
pub async fn sender(
    mut server_con: tokio::net::tcp::OwnedWriteHalf,
    mut queue: tokio::sync::mpsc::UnboundedReceiver<Message>,
) {
    let mut h_data = [0; 13];
    loop {
        if !send_single(&mut server_con, &mut queue, &mut h_data).await {
            return;
        }
    }
}

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
        send_single(&mut mock_connection, &mut queue_rx, &mut head_buf).await
    );

    assert_eq!(
        &vec![vec![12, 0, 0, 0, 3, 5, 0, 0, 0, 0, 0, 0, 0], vec![2; 5]],
        mock_connection.chunks()
    );
}
