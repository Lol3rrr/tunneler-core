use std::vec;

use crate::{general::ConnectionReader, message::Message};

#[cfg(test)]
use crate::message::{MessageHeader, MessageType};

use async_trait::async_trait;

#[derive(Debug)]
pub struct Reader {
    data: Vec<u8>,
}

impl Reader {
    /// Creates a new empty Reader
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Adds the serialized Message to the internal chunks
    /// that will be returned when reading from it
    pub fn add_message(&mut self, msg: Message) {
        let mut head: [u8; 13] = [0; 13];
        let body = msg.serialize(&mut head);

        self.data.extend_from_slice(&head[..]);
        self.data.extend_from_slice(body);
    }
}

#[async_trait]
impl ConnectionReader for Reader {
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
