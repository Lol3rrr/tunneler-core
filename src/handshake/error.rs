use super::ConfigError;

/// The Errors that could be encountered during the Validation
/// Phase of establishing a Connection
#[derive(Debug)]
pub enum HandshakeError {
    /// The Public-Key could not be send to the Client
    SendingKey(std::io::Error),
    /// The Public-Key could not be received from the Server
    ReceivingKey(std::io::Error),
    /// The Message could not be send
    SendingMessage(std::io::Error),
    /// The next Message could not be received
    ReceivingMessage(std::io::Error),
    /// The received Message was malformed
    DeserializeMessage,
    /// Received the wrong message
    WrongResponseType,
    /// The encryption Key could not be generated
    GeneratingKey(rsa::errors::Error),
    /// The received Key could not be parsed
    ParseKey(rsa::errors::Error),
    /// There was an error while encrypting the password/key
    Encrypting(rsa::errors::Error),
    /// There was an error while decrypting the password/key
    /// send by the Client
    Decrypting(rsa::errors::Error),
    /// The Client-Key and Server-Key don't match
    MismatchedKeys,
    /// The Acknowledge-Message could not be send
    SendingAcknowledge(std::io::Error),
    /// The Config-Message was malformed in some way
    MalformedConfig(ConfigError),
    /// The received Port is not considered Valid
    InvalidPort,
}
