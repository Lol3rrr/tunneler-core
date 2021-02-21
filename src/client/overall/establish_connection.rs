use crate::message::{Message, MessageHeader, MessageType};

use log::error;
use rsa::{BigUint, PaddingScheme, PublicKey, RSAPublicKey};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn establish_connection(adr: &str, key: &[u8]) -> Option<tokio::net::TcpStream> {
    let mut connection = match tokio::net::TcpStream::connect(&adr).await {
        Ok(c) => c,
        Err(e) => {
            error!("Establishing-Connection: {}", e);
            return None;
        }
    };

    // Step 2 - Receive
    let mut head_buf = [0; 13];
    let header = match connection.read_exact(&mut head_buf).await {
        Ok(_) => {
            let msg = MessageHeader::deserialize(&head_buf);
            msg.as_ref()?;
            msg.unwrap()
        }
        Err(e) => {
            error!("Reading Message-Header: {}", e);
            return None;
        }
    };
    if *header.get_kind() != MessageType::Key {
        return None;
    }

    let key_length = header.get_length() as usize;
    let mut key_buf = vec![0; key_length];
    match connection.read_exact(&mut key_buf).await {
        Ok(_) => {}
        Err(e) => {
            error!("Reading Public-Key from Server: {}", e);
            return None;
        }
    };

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
    let data = msg.serialize(&mut h_data);
    match connection.write_all(&h_data).await {
        Ok(_) => {}
        Err(e) => {
            error!("Sending Encrypted Key/Password: {}", e);
            return None;
        }
    };
    match connection.write_all(&data).await {
        Ok(_) => {}
        Err(e) => {
            error!("Sending Encrypted Key/Password: {}", e);
            return None;
        }
    };

    let mut buf = [0; 13];
    let header = match connection.read_exact(&mut buf).await {
        Ok(_) => match MessageHeader::deserialize(&buf) {
            Some(c) => c,
            None => {
                return None;
            }
        },
        Err(e) => {
            error!("Reading response: {}", e);
            return None;
        }
    };

    if *header.get_kind() != MessageType::Acknowledge {
        return None;
    }

    Some(connection)
}
