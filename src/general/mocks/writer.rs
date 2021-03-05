use async_trait::async_trait;

use crate::general::ConnectionWriter;

pub struct Writer {
    chunks: Vec<Vec<u8>>,
}

impl Writer {
    pub fn new() -> Self {
        Self { chunks: Vec::new() }
    }

    pub fn chunks(&self) -> &[Vec<u8>] {
        &self.chunks
    }
}

#[async_trait]
impl ConnectionWriter for Writer {
    async fn write_full(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.chunks.push(buf.to_vec());
        Ok(())
    }
}

#[test]
fn new_writer() {
    let tmp = Writer::new();
    assert_eq!(Vec::<Vec<u8>>::new(), tmp.chunks);
}

#[tokio::test]
async fn valid_write() {
    let mut tmp = Writer::new();

    assert_eq!(true, tmp.write_full(&vec![1, 2, 3, 4]).await.is_ok());
    assert_eq!(vec![vec![1, 2, 3, 4]], tmp.chunks());
}
