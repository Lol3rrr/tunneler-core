use async_trait::async_trait;

use tokio::io::AsyncReadExt;

/// Used to read from an actual TCP-Connection
#[async_trait]
pub trait ConnectionReader {
    async fn read_full(&mut self, buf: &mut [u8]) -> std::io::Result<usize>;
}

/// Used to write over an actual TCP-Connection
#[async_trait]
pub trait ConnectionWriter {}

#[async_trait]
impl<T> ConnectionReader for T
where
    T: AsyncReadExt + Send + Unpin,
{
    async fn read_full(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.read_exact(buf).await
    }
}
