use crate::message::{Data, MessageHeader};
use crate::objectpool::Guard;

#[cfg(test)]
use crate::message::MessageType;

/// A single Message that is send between the Server and Client
#[derive(Debug, PartialEq)]
pub struct Message {
    header: MessageHeader,
    data: Data<Vec<u8>>, // X bytes
}

impl Message {
    /// Creates a new Message with the given "raw" Data
    pub fn new(header: MessageHeader, data: Vec<u8>) -> Self {
        Self {
            header,
            data: Data::Raw(data),
        }
    }

    /// Creates a new Message with the given pooled/guarded Data
    pub fn new_guarded(header: MessageHeader, data: Guard<Vec<u8>>) -> Self {
        Self {
            header,
            data: Data::Pooled(data),
        }
    }

    /// Serializes the Message into a Vector of Bytes that can
    /// then be send over to the other side (Server or Client)
    pub fn serialize(&self) -> ([u8; 13], &[u8]) {
        let data = match self.data {
            Data::Raw(ref r) => r,
            Data::Pooled(ref p) => p,
        };

        (self.header.serialize(), data)
    }

    /// The Header of this message
    pub fn get_header(&self) -> &MessageHeader {
        &self.header
    }
    /// The Raw underlying data that belong to do this message
    pub fn get_data(&self) -> &[u8] {
        match self.data {
            Data::Raw(ref r) => &r,
            Data::Pooled(ref p) => &p,
        }
    }
}

#[test]
fn message_serialize_connect() {
    let mut inner_data = vec![0; 2];
    inner_data[0] = 1;
    inner_data[1] = 1;
    let msg = Message::new(
        MessageHeader {
            id: 13,
            kind: MessageType::Connect,
            length: 2,
        },
        inner_data,
    );

    let (h_output, d_output) = msg.serialize();

    let mut expected_header = [0; 13];
    expected_header[0] = 13;
    expected_header[5] = 2;
    assert_eq!(expected_header, h_output);

    let mut expected_data = vec![0; 2];
    expected_data[0] = 1;
    expected_data[1] = 1;
    assert_eq!(expected_data, d_output);
}
#[test]
fn message_serialize_data() {
    let mut inner_data = vec![0; 12];
    inner_data[2] = 33;
    let msg = Message::new(
        MessageHeader {
            id: 13,
            kind: MessageType::Data,
            length: 12,
        },
        inner_data,
    );
    let (h_output, d_output) = msg.serialize();

    let mut header_expect = [0; 13];
    header_expect[0] = 13;
    header_expect[4] = 2;
    header_expect[5] = 12;
    assert_eq!(header_expect, h_output);

    let mut data_expect = vec![0; 12];
    data_expect[2] = 33;
    assert_eq!(&data_expect, d_output);
}
