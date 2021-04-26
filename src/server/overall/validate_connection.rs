use crate::message::{Message, MessageHeader, MessageType};

use rand::rngs::OsRng;
use rsa::{PaddingScheme, PublicKeyParts, RSAPrivateKey, RSAPublicKey};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use log::error;

/// The Errors that could be encountered during the Validation
/// Phase of establishing a Connection
#[derive(Debug)]
pub enum ValidateError {
    /// The Public-Key could not be send to the Client
    SendingKey(std::io::Error),
    /// The next Message could not be received
    ReceivingMessage(std::io::Error),
    /// The received Message was malformed
    DeserializeMessage,
    /// Received the wrong message
    WrongResponseType,
    /// There was an error while decrypting the password/key
    /// send by the Client
    Decrypting(rsa::errors::Error),
    /// The Client-Key and Server-Key don't match
    MismatchedKeys,
    /// The Acknowledge-Message could not be send
    SendingAcknowledge(std::io::Error),
    /// The Port-Message was malformed in some way
    MalformedPort,
    /// The received Port is not considered Valid
    InvalidPort,
}

// The validation flow is like this
//
// 1. Client connects
// 2. Server generates and sends public key
// 3. Client sends encrypted password/key
// 4. Server decrypts the message and checks if the password/key is valid
// 5a. If valid: Server sends an Acknowledge message and its done
// 5b. If invalid: Server closes the connection
// 6. Client sends the Port-Packet
// 7. Server validates the given Port
// 7a. Valid: Sends ACK-Message back
// 7b. Invalid: Closes the Connection
pub async fn validate_connection<V>(
    con: &mut tokio::net::TcpStream,
    key: &[u8],
    is_port_valid: V,
) -> Result<u16, ValidateError>
where
    V: FnOnce(u16) -> bool,
{
    // Step 2
    let mut rng = OsRng;
    let priv_key = RSAPrivateKey::new(&mut rng, 2048).expect("Failed to generate private key");
    let pub_key = RSAPublicKey::from(&priv_key);

    let pub_n_bytes = pub_key.n().to_bytes_le();
    let mut pub_e_bytes = pub_key.e().to_bytes_le();

    let mut data = pub_n_bytes;
    data.append(&mut pub_e_bytes);

    let msg_header = MessageHeader::new(0, MessageType::Key, data.len() as u64);
    let msg = Message::new(msg_header, data);

    let mut h_data = [0; 13];
    let data = msg.serialize(&mut h_data);
    if let Err(e) = con.write_all(&h_data).await {
        return Err(ValidateError::SendingKey(e));
    }
    if let Err(e) = con.write_all(&data).await {
        error!("Sending Key-Data: {}", e);
        return Err(ValidateError::SendingKey(e));
    }

    // Step 4
    let mut head_buf = [0; 13];
    let header = match con.read_exact(&mut head_buf).await {
        Ok(_) => match MessageHeader::deserialize(&head_buf) {
            Some(m) => m,
            None => return Err(ValidateError::DeserializeMessage),
        },
        Err(e) => {
            return Err(ValidateError::ReceivingMessage(e));
        }
    };
    if *header.get_kind() != MessageType::Verify {
        return Err(ValidateError::WrongResponseType);
    }

    let key_length = header.get_length() as usize;
    let mut recv_encrypted_key = vec![0; key_length];
    if let Err(e) = con.read_exact(&mut recv_encrypted_key).await {
        return Err(ValidateError::ReceivingMessage(e));
    }

    let recv_key = match priv_key.decrypt(PaddingScheme::PKCS1v15Encrypt, &recv_encrypted_key) {
        Ok(raw_key) => raw_key,
        Err(e) => {
            return Err(ValidateError::Decrypting(e));
        }
    };

    // Step 5
    if recv_key != key {
        // Step 5a
        return Err(ValidateError::MismatchedKeys);
    }

    // Step 5b
    let ack_header = MessageHeader::new(0, MessageType::Acknowledge, 0);
    let mut ack_data = [0; 13];
    ack_header.serialize(&mut ack_data);
    if let Err(e) = con.write_all(&ack_data).await {
        return Err(ValidateError::SendingAcknowledge(e));
    }

    // Step 6
    let header = match con.read_exact(&mut head_buf).await {
        Ok(_) => match MessageHeader::deserialize(&head_buf) {
            Some(h) => h,
            None => return Err(ValidateError::DeserializeMessage),
        },
        Err(e) => return Err(ValidateError::ReceivingMessage(e)),
    };
    if *header.get_kind() != MessageType::Port {
        return Err(ValidateError::WrongResponseType);
    }

    let port_length = header.get_length() as usize;
    if port_length != 2 {
        return Err(ValidateError::MalformedPort);
    }

    let mut recv_port: [u8; 2] = [0, 0];
    if let Err(e) = con.read_exact(&mut recv_port).await {
        return Err(ValidateError::ReceivingMessage(e));
    }

    let port = u16::from_be_bytes(recv_port);

    //  Step 7
    if is_port_valid(port) {
        // Step 7a
        let ack_header = MessageHeader::new(0, MessageType::Acknowledge, 0);
        let mut ack_data = [0; 13];
        ack_header.serialize(&mut ack_data);
        if let Err(e) = con.write_all(&ack_data).await {
            return Err(ValidateError::SendingAcknowledge(e));
        }
    } else {
        // Step  7b
        return Err(ValidateError::InvalidPort);
    }

    Ok(port)
}
