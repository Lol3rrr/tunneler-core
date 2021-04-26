use log::error;

use crate::handshake;

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
            error!("Establishing-Connection: {}", e);
            return None;
        }
    };

    if !handshake::client::perform(&mut connection, key, port).await {
        return None;
    }

    Some(connection)
}
