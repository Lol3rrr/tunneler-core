use crate::handshake;

#[derive(Debug)]
pub enum EstablishConnectionError {
    Connection(std::io::Error),
    Handshake(handshake::HandshakeError),
}

impl From<std::io::Error> for EstablishConnectionError {
    fn from(other: std::io::Error) -> Self {
        Self::Connection(other)
    }
}

impl From<handshake::HandshakeError> for EstablishConnectionError {
    fn from(other: handshake::HandshakeError) -> Self {
        Self::Handshake(other)
    }
}

/// Establishes a new Connection to the external Server
///
/// Params:
/// * adr: The Address to connect to
/// * key: The Key used to authenticate
/// * port: The External Port on the Server
pub async fn establish_connection(
    adr: &str,
    key: &[u8],
    port: u16,
) -> Result<tokio::net::TcpStream, EstablishConnectionError> {
    let mut connection = tokio::net::TcpStream::connect(&adr).await?;
    handshake::client::perform(&mut connection, key, port).await?;

    Ok(connection)
}
