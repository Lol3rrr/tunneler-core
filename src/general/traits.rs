use async_trait::async_trait;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Used to read from an actual TCP-Connection
#[async_trait]
pub trait ConnectionReader {
    async fn read_full(&mut self, buf: &mut [u8]) -> std::io::Result<usize>;
}

/// Used to write over an actual TCP-Connection
#[async_trait]
pub trait ConnectionWriter {
    async fn write_full(&mut self, buf: &[u8]) -> std::io::Result<()>;
}

#[async_trait]
impl<T> ConnectionReader for T
where
    T: AsyncReadExt + Send + Unpin,
{
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
