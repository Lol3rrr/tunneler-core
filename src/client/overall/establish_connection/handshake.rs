use crate::general::{ConnectionReader, ConnectionWriter};
use crate::message::{Message, MessageHeader, MessageType};

use log::error;
use rsa::{BigUint, PaddingScheme, PublicKey, RSAPublicKey};

pub async fn perform<C>(connection: &mut C, key: &[u8], port: u16) -> bool
where
    C: ConnectionWriter + ConnectionReader + Send,
{
    // Step 2 - Receive
    let mut head_buf = [0; 13];
    let header = match connection.read_full(&mut head_buf).await {
        Ok(_) => {
            let msg = MessageHeader::deserialize(&head_buf);
            msg.as_ref().unwrap();
            msg.unwrap()
        }
        Err(e) => {
            error!("Reading Message-Header: {}", e);
            return false;
        }
    };
    if *header.get_kind() != MessageType::Key {
        return false;
    }

    let key_length = header.get_length() as usize;
    let mut key_buf = vec![0; key_length];
    if let Err(e) = connection.read_full(&mut key_buf).await {
        error!("Reading Public-Key from Server: {}", e);
        return false;
    }

    let e_bytes = key_buf.split_off(256);
    let n_bytes = key_buf;

    let pub_key = RSAPublicKey::new(
        BigUint::from_bytes_le(&n_bytes),
        BigUint::from_bytes_le(&e_bytes),
    )
    .expect("Could not create Public-Key");

    let encrypted_key = pub_key
        .encrypt(&mut rand::rngs::OsRng, PaddingScheme::PKCS1v15Encrypt, key)
        .expect("Could not encrypt Key");

    let msg_header = MessageHeader::new(0, MessageType::Verify, encrypted_key.len() as u64);
    let msg = Message::new(msg_header, encrypted_key);

    let mut h_data = [0; 13];
    if let Err(e) = connection.write_msg(&msg, &mut h_data).await {
        error!("Sending Encrypted Key/Password: {}", e);
        return false;
    }

    let mut buf = [0; 13];
    let header = match connection.read_full(&mut buf).await {
        Ok(_) => match MessageHeader::deserialize(&buf) {
            Some(c) => c,
            None => {
                return false;
            }
        },
        Err(e) => {
            error!("Reading response: {}", e);
            return false;
        }
    };

    if *header.get_kind() != MessageType::Acknowledge {
        return false;
    }

    let port_msg_header = MessageHeader::new(0, MessageType::Port, 2);
    let port_msg = Message::new(port_msg_header, port.to_be_bytes().to_vec());

    if let Err(e) = connection.write_msg(&port_msg, &mut h_data).await {
        error!("Sending desired Port: {}", e);
        return false;
    }

    let mut buf = [0; 13];
    let header = match connection.read_full(&mut buf).await {
        Ok(_) => match MessageHeader::deserialize(&buf) {
            Some(c) => c,
            None => {
                return false;
            }
        },
        Err(e) => {
            error!("Reading response: {}", e);
            return false;
        }
    };

    if *header.get_kind() != MessageType::Acknowledge {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use rand::rngs::OsRng;
    use rsa::{PublicKeyParts, RSAPrivateKey};

    use super::*;

    use crate::general::mocks::MockConnection;

    /// Returns
    /// (PubKey-Message, Private Key)
    fn setup_key() -> (Message, RSAPrivateKey) {
        let mut rng = OsRng;
        let priv_key = RSAPrivateKey::new(&mut rng, 2048).expect("Failed to generate private key");
        let pub_key = RSAPublicKey::from(&priv_key);

        let pub_n_bytes = pub_key.n().to_bytes_le();
        let mut pub_e_bytes = pub_key.e().to_bytes_le();

        let mut data = pub_n_bytes;
        data.append(&mut pub_e_bytes);

        let key_msg = Message::new(
            MessageHeader::new(0, MessageType::Key, data.len() as u64),
            data,
        );

        (key_msg, priv_key)
    }

    #[tokio::test]
    async fn valid_handshake() {
        let mut connection = MockConnection::new();

        let (key_msg, priv_key) = setup_key();

        // All the Messages the Server will send to the Client
        connection.reader_mut().add_message(key_msg);
        connection.reader_mut().add_message(Message::new(
            MessageHeader::new(0, MessageType::Acknowledge, 0),
            Vec::new(),
        ));
        connection.reader_mut().add_message(Message::new(
            MessageHeader::new(0, MessageType::Acknowledge, 0),
            Vec::new(),
        ));

        let key_password = "test".as_bytes();
        let port = 13;

        perform(&mut connection, key_password, port).await;

        let chunks = connection.writer_mut().chunks();
        assert_eq!(4, chunks.len());

        //let encrypted_chunk_header = chunks.get(0).unwrap();
        let encrypted_chunk_body = chunks.get(1).unwrap();
        let recv_key = priv_key
            .decrypt(PaddingScheme::PKCS1v15Encrypt, &encrypted_chunk_body)
            .unwrap();
        assert_eq!(key_password, recv_key);

        //let port_chunk_header = chunks.get(2).unwrap();
        let port_chunk_body = chunks.get(3).unwrap();
        let recv_port = u16::from_be_bytes([
            *port_chunk_body.get(0).unwrap(),
            *port_chunk_body.get(1).unwrap(),
        ]);
        assert_eq!(port, recv_port);
    }
}
