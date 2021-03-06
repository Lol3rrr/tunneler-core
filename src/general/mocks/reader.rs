use std::vec;

use crate::{general::ConnectionReader, message::Message};

#[cfg(test)]
use crate::message::{MessageHeader, MessageType};

use async_trait::async_trait;

#[derive(Debug)]
pub struct Reader {
    data: Vec<u8>,
    eof_returned: bool,
    close_on_eof: bool,
}

impl Reader {
    /// Creates a new empty Reader
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            eof_returned: false,
            close_on_eof: false,
        }
    }

    /// Adds the serialized Message to the internal chunks
    /// that will be returned when reading from it
    pub fn add_message(&mut self, msg: Message) {
        let mut head: [u8; 13] = [0; 13];
        let body = msg.serialize(&mut head);

        self.data.extend_from_slice(&head[..]);
        self.data.extend_from_slice(body);
    }

    /// Adds the raw bytes to the internal buffer
    pub fn add_bytes(&mut self, tmp: &[u8]) {
        self.data.extend_from_slice(tmp);
    }

    /// The last call to read when the buffer
    /// is empty and has already returned EOF,
    /// will return an error indicating that the
    /// connection has been closed
    pub fn close(&mut self) {
        self.close_on_eof = true;
    }
}

#[async_trait]
impl ConnectionReader for Reader {
    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let data_length = self.data.len();
        let buf_length = buf.len();
        let read_length = std::cmp::min(data_length, buf_length);
        if read_length == 0 && self.eof_returned && self.close_on_eof {
            return Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe));
        }
        if read_length == 0 {
            self.eof_returned = true;
        }

        for (index, tmp) in self.data.drain(0..read_length).enumerate() {
            buf[index] = tmp;
        }
        Ok(read_length)
    }

    async fn read_full(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let data_length = self.data.len();
        if data_length == 0 {
            return Ok(0);
        }

        let buf_length = buf.len();
        if buf_length > data_length {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Buffer has less Data than requested",
            ));
        }

        for (index, tmp) in self.data.drain(0..buf_length).enumerate() {
            buf[index] = tmp;
        }
        Ok(buf_length)
    }
}

#[test]
fn new_reader() {
    let tmp = Reader::new();

    assert_eq!(Vec::<u8>::new(), tmp.data);
}

#[test]
fn new_add_message() {
    let mut tmp = Reader::new();

    let msg = Message::new(MessageHeader::new(13, MessageType::Data, 13), vec![0; 13]);
    tmp.add_message(msg);

    let mut expected_data = vec![13u8, 0, 0, 0, 3, 13, 0, 0, 0, 0, 0, 0, 0];
    expected_data.append(&mut vec![0; 13]);
    assert_eq!(expected_data, tmp.data)
}

#[tokio::test]
async fn read_message() {
    let mut tmp = Reader::new();

    let msg = Message::new(MessageHeader::new(13, MessageType::Data, 10), vec![1; 10]);
    tmp.add_message(msg);

    let mut head_read = [0; 13];
    let head_result = tmp.read_full(&mut head_read).await;
    assert_eq!(true, head_result.is_ok());
    assert_eq!(13, head_result.unwrap());
    assert_eq!(
        &vec![13u8, 0, 0, 0, 3, 10, 0, 0, 0, 0, 0, 0, 0],
        &head_read[..]
    );

    let mut body_read = [0; 10];
    let body_result = tmp.read_full(&mut body_read).await;
    assert_eq!(true, body_result.is_ok());
    assert_eq!(10, body_result.unwrap());
    assert_eq!(&vec![1; 10], &body_read[..]);

    let mut empty = [0; 10];
    let eof_result = tmp.read_full(&mut empty).await;
    assert_eq!(true, eof_result.is_ok());
    assert_eq!(0, eof_result.unwrap());
}

#[tokio::test]
async fn read_fits() {
    let mut tmp = Reader::new();

    tmp.add_bytes(&vec![0, 1, 2, 3, 4, 5]);

    let mut buffer = [0u8; 6];
    let read_result = tmp.read(&mut buffer).await;
    assert_eq!(true, read_result.is_ok());
    assert_eq!(6, read_result.unwrap());
    assert_eq!(&vec![0, 1, 2, 3, 4, 5], &buffer);

    let eof_result = tmp.read(&mut buffer).await;
    assert_eq!(true, eof_result.is_ok());
    assert_eq!(0, eof_result.unwrap());
}
#[tokio::test]
async fn read_buffer_is_bigger() {
    let mut tmp = Reader::new();

    tmp.add_bytes(&vec![0, 1, 2, 3, 4, 5]);

    let mut buffer = [0u8; 8];
    let read_result = tmp.read(&mut buffer).await;
    assert_eq!(true, read_result.is_ok());
    assert_eq!(6, read_result.unwrap());
    assert_eq!(&vec![0, 1, 2, 3, 4, 5], &buffer[0..6]);

    let eof_result = tmp.read(&mut buffer).await;
    assert_eq!(true, eof_result.is_ok());
    assert_eq!(0, eof_result.unwrap());
}
#[tokio::test]
async fn read_buffer_is_smaller_multiple_reads() {
    let mut tmp = Reader::new();

    tmp.add_bytes(&vec![0, 1, 2, 3, 4, 5]);

    let mut buffer = [0u8; 3];
    let read_result = tmp.read(&mut buffer).await;
    assert_eq!(true, read_result.is_ok());
    assert_eq!(3, read_result.unwrap());
    assert_eq!(&vec![0, 1, 2], &buffer[0..3]);

    let read_result = tmp.read(&mut buffer).await;
    assert_eq!(true, read_result.is_ok());
    assert_eq!(3, read_result.unwrap());
    assert_eq!(&vec![3, 4, 5], &buffer[0..3]);

    let eof_result = tmp.read(&mut buffer).await;
    assert_eq!(true, eof_result.is_ok());
    assert_eq!(0, eof_result.unwrap());
}

#[tokio::test]
async fn read_fits_with_close() {
    let mut tmp = Reader::new();

    tmp.add_bytes(&vec![0, 1, 2, 3, 4, 5]);
    tmp.close();

    let mut buffer = [0u8; 6];
    let read_result = tmp.read(&mut buffer).await;
    assert_eq!(true, read_result.is_ok());
    assert_eq!(6, read_result.unwrap());
    assert_eq!(&vec![0, 1, 2, 3, 4, 5], &buffer);

    let eof_result = tmp.read(&mut buffer).await;
    assert_eq!(true, eof_result.is_ok());
    assert_eq!(0, eof_result.unwrap());

    let close_result = tmp.read(&mut buffer).await;
    assert_eq!(true, close_result.is_err());
}
