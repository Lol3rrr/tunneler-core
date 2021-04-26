/// The Type/Kind of Message
#[derive(Debug, PartialEq, Clone)]
pub enum MessageType {
    /// Indicates a new Connection should be established
    /// for the provided id
    Connect,
    /// Indicates the Connection with the given id should
    /// be closed
    Close,
    /// Indicates that this message contains Data that should
    /// be handled accordingly
    Data,
    /// A simple Heartbeat connection to signal that the
    /// other side is still there and ready to accept messages
    /// as well as making sure that the connection is not closed
    /// by the OS or other factors
    Heartbeat,
    /// This is the 1. message a Client sends when establishing
    /// a connection with the Server
    Establish,
    /// This is the 2. message of the Handshake where the the Server
    /// sends it public key over to the Client
    Key,
    /// This is the 3. message of the Handshake where the Client
    /// sends the key/password to the Server after it was encrypted
    /// with the previously received Public-Key
    Verify,
    /// This is the 4. and last message of the Handshake which is simply
    /// send by the Server to acknowledge the new connection established
    Acknowledge,
    /// Signals the EOF from a request that is not yet closed
    EOF,
    /// This is the 5. message in the Handshake and is send by
    /// the client along with an unsigned 16-bit number to
    /// tell the server on which port it should listen for
    /// user requests
    Port,
}

impl MessageType {
    /// Deserializes the Kind/Type from a single Byte/u8
    ///
    /// Returns:
    /// * None if the Byte was not a valid type/kind
    /// * Some with the type/kind of the byte
    pub fn deserialize(data: u8) -> Option<MessageType> {
        match data {
            1 => Some(MessageType::Connect),
            2 => Some(MessageType::Close),
            3 => Some(MessageType::Data),
            4 => Some(MessageType::Heartbeat),
            5 => Some(MessageType::Establish),
            6 => Some(MessageType::Key),
            7 => Some(MessageType::Verify),
            8 => Some(MessageType::Acknowledge),
            9 => Some(MessageType::EOF),
            10 => Some(MessageType::Port),
            _ => None,
        }
    }

    /// Serializes the Type/Kind into a single Byte that can
    /// then be send to a Client or Server
    pub fn serialize(&self) -> u8 {
        match *self {
            MessageType::Connect => 1,
            MessageType::Close => 2,
            MessageType::Data => 3,
            MessageType::Heartbeat => 4,
            MessageType::Establish => 5,
            MessageType::Key => 6,
            MessageType::Verify => 7,
            MessageType::Acknowledge => 8,
            MessageType::EOF => 9,
            MessageType::Port => 10,
        }
    }
}

#[test]
fn message_type_deserialize_connect() {
    assert_eq!(Some(MessageType::Connect), MessageType::deserialize(1));
}
#[test]
fn message_type_deserialize_close() {
    assert_eq!(Some(MessageType::Close), MessageType::deserialize(2));
}
#[test]
fn message_type_deserialize_data() {
    assert_eq!(Some(MessageType::Data), MessageType::deserialize(3));
}
#[test]
fn message_type_deserialize_heartbeat() {
    assert_eq!(Some(MessageType::Heartbeat), MessageType::deserialize(4));
}
#[test]
fn message_type_deserialize_establish() {
    assert_eq!(Some(MessageType::Establish), MessageType::deserialize(5));
}
#[test]
fn message_type_deserialize_key() {
    assert_eq!(Some(MessageType::Key), MessageType::deserialize(6));
}
#[test]
fn message_type_deserialize_verify() {
    assert_eq!(Some(MessageType::Verify), MessageType::deserialize(7));
}
#[test]
fn message_type_deserialize_acknowledge() {
    assert_eq!(Some(MessageType::Acknowledge), MessageType::deserialize(8));
}
#[test]
fn message_type_deserialize_eof() {
    assert_eq!(Some(MessageType::EOF), MessageType::deserialize(9));
}
#[test]
fn message_type_deserialize_invalid() {
    assert_eq!(None, MessageType::deserialize(123));
}

#[test]
fn message_type_serialize_connect() {
    assert_eq!(1, MessageType::Connect.serialize());
}
#[test]
fn message_type_serialize_close() {
    assert_eq!(2, MessageType::Close.serialize());
}
#[test]
fn message_type_serialize_data() {
    assert_eq!(3, MessageType::Data.serialize());
}
#[test]
fn message_type_serialize_heartbeat() {
    assert_eq!(4, MessageType::Heartbeat.serialize());
}
#[test]
fn message_type_serialize_establish() {
    assert_eq!(5, MessageType::Establish.serialize());
}
#[test]
fn message_type_serialize_key() {
    assert_eq!(6, MessageType::Key.serialize());
}
#[test]
fn message_type_serialize_verify() {
    assert_eq!(7, MessageType::Verify.serialize());
}
#[test]
fn message_type_serialize_acknowledge() {
    assert_eq!(8, MessageType::Acknowledge.serialize());
}
#[test]
fn message_type_serialize_eof() {
    assert_eq!(9, MessageType::EOF.serialize());
}
