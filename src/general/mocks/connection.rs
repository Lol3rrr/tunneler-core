use super::{MockReader, MockWriter};
use crate::general::{ConnectionReader, ConnectionWriter};

use async_trait::async_trait;

pub struct Connection {
    reader: MockReader,
    writer: MockWriter,
}

impl Connection {
    /// Creates a new Mock Connection
    pub fn new() -> Self {
        Self {
            reader: MockReader::new(),
            writer: MockWriter::new(),
        }
    }

    /// Gives access to the underlying MockReader
    pub fn reader_mut(&mut self) -> &mut MockReader {
        &mut self.reader
    }
    /// Gives access to the underlying MockWriter
    pub fn writer_mut(&mut self) -> &mut MockWriter {
        &mut self.writer
    }
}

#[async_trait]
impl ConnectionReader for Connection {
    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.reader.read(buf).await
    }
    async fn read_full(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.reader.read_full(buf).await
    }
}

#[async_trait]
impl ConnectionWriter for Connection {
    async fn write_full(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.writer.write_full(buf).await
    }
}
