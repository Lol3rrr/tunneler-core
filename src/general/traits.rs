use async_trait::async_trait;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::message::Message;

/// Used to read from an actual TCP-Connection
#[async_trait]
pub trait ConnectionReader {
    /// Reads an arbitrary amount of bytes from the Connection
    /// to at most fill the buffer
    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize>;

    /// Reads from the Connection until the Buffer is filled
    async fn read_full(&mut self, buf: &mut [u8]) -> std::io::Result<usize>;

    /// Reads the next `size` amount of bytes from the connection
    /// and throws them away
    async fn drain(&mut self, size: usize) {
        let mut buf = vec![0; size];
        if let Err(e) = self.read_full(&mut buf).await {
            error!("Draining: {}", e);
        }
    }
}

/// Used to write over an actual TCP-Connection
#[async_trait]
pub trait ConnectionWriter {
    /// Attempts to write the entire buffer to the underlying
    /// Connection
    async fn write_full(&mut self, buf: &[u8]) -> std::io::Result<()>;

    /// Attempts to write the message to the underlying connection
    async fn write_msg(&mut self, msg: &Message, tmp_buf: &mut [u8; 13]) -> std::io::Result<()> {
        let data = msg.serialize(tmp_buf);
        if let Err(e) = self.write_full(tmp_buf).await {
            return Err(e);
        }
        if let Err(e) = self.write_full(data).await {
            return Err(e);
        }

        Ok(())
    }
}

#[async_trait]
impl<T> ConnectionReader for T
where
    T: AsyncReadExt + Send + Unpin,
{
    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.read(buf).await
    }

    async fn read_full(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.read_exact(buf).await
    }
}

#[async_trait]
impl<T> ConnectionWriter for T
where
    T: AsyncWriteExt + Send + Unpin,
{
    async fn write_full(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.write_all(buf).await
    }
}
