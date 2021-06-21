use crate::message::{MessageHeader, MessageType};

/// A single Message that is send between the Server and Client
#[derive(Debug)]
pub struct Message {
    header: MessageHeader,
    data: Vec<u8>, // X bytes
}

impl Message {
    /// Creates a new Message with the given "raw" Data
    pub fn new(header: MessageHeader, data: Vec<u8>) -> Self {
        Self { header, data }
    }

    /// Serializes the Message into a Vector of Bytes that can
    /// then be send over to the other side (Server or Client)
    pub fn serialize(&self, header: &mut [u8; 13]) -> &[u8] {
        let data = &self.data;

        self.header.serialize(header);

        &data[0..self.header.length as usize]
    }

    /// The Header of this message
    pub fn get_header(&self) -> &MessageHeader {
        &self.header
    }
    /// The Raw underlying data that belong to do this message
    pub fn get_data(&self) -> &[u8] {
        &self.data
    }

    /// Checks if the messsage is marked as an EOF(End-Of-File)
    ///
    /// This is useful for cases where the requests may be over, but the
    /// connections are not actually closed
    pub fn is_eof(&self) -> bool {
        self.header.kind == MessageType::EOF
    }
}

impl PartialEq for Message {
    fn eq(&self, other: &Self) -> bool {
        self.header == other.header && self.get_data() == other.get_data()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let mut h_output = [0; 13];
        let d_output = msg.serialize(&mut h_output);

        let mut expected_header = [0; 13];
        expected_header[0] = 13;
        expected_header[4] = 1;
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
        let mut h_output = [0; 13];
        let d_output = msg.serialize(&mut h_output);

        let mut header_expect = [0; 13];
        header_expect[0] = 13;
        header_expect[4] = 3;
        header_expect[5] = 12;
        assert_eq!(header_expect, h_output);

        let mut data_expect = vec![0; 12];
        data_expect[2] = 33;
        assert_eq!(&data_expect, d_output);
    }

    #[test]
    fn message_serialize_length_less_than_vec_size() {
        let mut inner_data = vec![0; 20];
        inner_data[2] = 33;
        let msg = Message::new(
            MessageHeader {
                id: 13,
                kind: MessageType::Data,
                length: 12,
            },
            inner_data,
        );
        let mut h_output = [0; 13];
        let d_output = msg.serialize(&mut h_output);

        let mut header_expect = [0; 13];
        header_expect[0] = 13;
        header_expect[4] = 3;
        header_expect[5] = 12;
        assert_eq!(header_expect, h_output);

        let mut data_expect = vec![0; 12];
        data_expect[2] = 33;
        assert_eq!(&data_expect, d_output);
    }

    #[test]
    fn message_is_eof() {
        let header = MessageHeader::new(0, MessageType::EOF, 0);
        let msg = Message::new(header, vec![]);

        assert_eq!(true, msg.is_eof());
    }
    #[test]
    fn message_is_not_eof() {
        let header = MessageHeader::new(0, MessageType::Data, 0);
        let msg = Message::new(header, vec![]);

        assert_eq!(false, msg.is_eof());
    }
}
