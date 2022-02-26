use crate::{
    general::{ConnectionReader, ConnectionWriter},
    handshake::HandshakeError,
    message::{Message, MessageHeader, MessageType},
    PROTOCOL_VERSION,
};

use rand::rngs::OsRng;
use rsa::{PaddingScheme, PublicKeyParts, RSAPrivateKey, RSAPublicKey};

use super::Config;

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
pub async fn perform<C, V>(
    con: &mut C,
    key: &[u8],
    is_port_valid: V,
) -> Result<Config, HandshakeError>
where
    C: ConnectionReader + ConnectionWriter + Send,
    V: FnOnce(u16) -> bool,
{
    // Step 2
    let mut rng = OsRng;
    let priv_key = match RSAPrivateKey::new(&mut rng, 2048) {
        Ok(k) => k,
        Err(e) => return Err(HandshakeError::GeneratingKey(e)),
    };
    let pub_key = RSAPublicKey::from(&priv_key);

    let pub_n_bytes = pub_key.n().to_bytes_le();
    let mut pub_e_bytes = pub_key.e().to_bytes_le();

    let mut data = pub_n_bytes;
    data.append(&mut pub_e_bytes);

    let msg_header = MessageHeader::new(0, MessageType::Key, data.len() as u64);
    let msg = Message::new(msg_header, data);

    let mut h_data = [0; 13];
    if let Err(e) = con.write_msg(&msg, &mut h_data).await {
        return Err(HandshakeError::SendingKey(e));
    }

    // Step 4
    let mut head_buf = [0; 13];
    let header = match con.read_full(&mut head_buf).await {
        Ok(_) => match MessageHeader::deserialize(&head_buf) {
            Some(m) => m,
            None => return Err(HandshakeError::DeserializeMessage),
        },
        Err(e) => return Err(HandshakeError::ReceivingMessage(e)),
    };
    if *header.get_kind() != MessageType::Verify {
        return Err(HandshakeError::WrongResponseType);
    }

    let key_length = header.get_length() as usize;
    let mut recv_encrypted_key = vec![0; key_length];
    if let Err(e) = con.read_full(&mut recv_encrypted_key).await {
        return Err(HandshakeError::ReceivingMessage(e));
    }

    let recv_key = match priv_key.decrypt(PaddingScheme::PKCS1v15Encrypt, &recv_encrypted_key) {
        Ok(raw_key) => raw_key,
        Err(e) => return Err(HandshakeError::Decrypting(e)),
    };

    // Step 5
    if recv_key != key {
        // Step 5a
        return Err(HandshakeError::MismatchedKeys);
    }

    // Step 5b
    let ack_header = MessageHeader::new(0, MessageType::Acknowledge, 0);
    let ack_msg = Message::new(ack_header, vec![]);
    let mut ack_data = [0; 13];
    if let Err(e) = con.write_msg(&ack_msg, &mut ack_data).await {
        return Err(HandshakeError::SendingAcknowledge(e));
    }

    // Step 6
    let header = match con.read_full(&mut head_buf).await {
        Ok(_) => match MessageHeader::deserialize(&head_buf) {
            Some(h) => h,
            None => return Err(HandshakeError::DeserializeMessage),
        },
        Err(e) => return Err(HandshakeError::ReceivingMessage(e)),
    };
    if *header.get_kind() != MessageType::Config {
        return Err(HandshakeError::WrongResponseType);
    }

    let mut recv_buffer = vec![0; header.get_length() as usize];
    if let Err(e) = con.read_full(&mut recv_buffer).await {
        return Err(HandshakeError::ReceivingMessage(e));
    }

    let config = match Config::from_bytes(&recv_buffer) {
        Ok(c) => c,
        Err(e) => return Err(HandshakeError::MalformedConfig(e)),
    };

    //  Step 7
    if is_port_valid(config.port()) {
        // Step 7a
        let ack_header = MessageHeader::new(0, MessageType::Acknowledge, 0);
        let ack_msg = Message::new(ack_header, vec![]);
        let mut ack_data = [0; 13];
        if let Err(e) = con.write_msg(&ack_msg, &mut ack_data).await {
            return Err(HandshakeError::SendingAcknowledge(e));
        }
    } else {
        // Step  7b
        return Err(HandshakeError::InvalidPort);
    }

    // Check the Protocol Version for Compatibility
    if config.protocol_version() > PROTOCOL_VERSION + 1 {
        return Err(HandshakeError::MismatchedProtocol {
            current: PROTOCOL_VERSION,
            other: config.protocol_version(),
        });
    }

    Ok(config)
}
