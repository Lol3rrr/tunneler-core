use crate::handshake;

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
) -> Option<tokio::net::TcpStream> {
    let mut connection = match tokio::net::TcpStream::connect(&adr).await {
        Ok(c) => c,
        Err(e) => {
            log::error!("Establishing-Connection: {}", e);
            return None;
        }
    };

    if let Err(e) = handshake::client::perform(&mut connection, key, port).await {
        log::error!("Performing Handshake: {:?}", e);
        return None;
    }

    Some(connection)
}
