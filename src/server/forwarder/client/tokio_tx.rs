use crate::{general::ConnectionWriter, message::Message};

#[derive(Debug)]
pub enum SendError {
    QueueReceive,
    IO(std::io::Error),
}

impl From<std::io::Error> for SendError {
    fn from(other: std::io::Error) -> Self {
        Self::IO(other)
    }
}

pub async fn send<C>(
    write_con: &mut C,
    queue: &mut tokio::sync::mpsc::UnboundedReceiver<Message>,
    header_buf: &mut [u8; 13],
) -> Result<(), SendError>
where
    C: ConnectionWriter + Send,
{
    let msg = match queue.recv().await {
        Some(m) => m,
        None => return Err(SendError::QueueReceive),
    };

    write_con.write_msg(&msg, header_buf).await?;

    Ok(())
}
