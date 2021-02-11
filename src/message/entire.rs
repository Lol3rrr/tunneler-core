use crate::message::MessageHeader;

#[cfg(test)]
use crate::message::MessageType;

/// A single Message that is send between the Server and Client
#[derive(Debug, PartialEq, Clone)]
pub struct Message {
    header: MessageHeader,
    data: Vec<u8>, // X bytes
}

impl Message {
    /// Creates a new message with the provided Metadata
    pub fn new(header: MessageHeader, data: Vec<u8>) -> Message {
        Message { header, data }
    }

    /// Serializes the Message into a Vector of Bytes that can
    /// then be send over to the other side (Server or Client)
    pub fn serialize(&self) -> ([u8; 13], &[u8]) {
        let data = &self.data;
        let data_length = self.header.get_length() as usize;

        (self.header.serialize(), &data[0..data_length])
    }

    /// The Header of this message
    pub fn get_header(&self) -> &MessageHeader {
        &self.header
    }
    /// The Raw underlying data that belong to do this message
    pub fn get_data(&self) -> &[u8] {
        self.data.as_slice()
    }
}

#[test]
fn message_serialize_connect() {
    let mut inner_data = vec![0; 4092];
    inner_data[0] = 1;
    inner_data[1] = 1;
    let msg = Message {
        header: MessageHeader {
            id: 13,
            kind: MessageType::Connect,
            length: 2,
        },
        data: inner_data,
    };

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
    let inner_data = vec![0; 4092];
    let mut msg = Message {
        header: MessageHeader {
            id: 13,
            kind: MessageType::Data,
            length: 12,
        },
        data: inner_data,
    };
    msg.data[2] = 33;
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
