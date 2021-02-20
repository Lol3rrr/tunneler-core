use crate::connections::Connections;
use crate::message::{Message, MessageHeader, MessageType};
use crate::objectpool::Pool;
use crate::server::client::ClientManager;
use crate::streams::mpsc;

use log::error;
use std::io::Read;
use tokio::io::AsyncReadExt;
use tokio::net::tcp;

pub fn handle(
    id: u32,
    con: &mut mio::net::TcpStream,
    event: &mio::event::Event,
    to_send_queue: &mut std::sync::mpsc::Receiver<Message>,
    user_cons: &Connections<mpsc::StreamWriter<Message>>,
    client_manager: &std::sync::Arc<ClientManager>,
    obj_pool: &Pool<Vec<u8>>,
) -> Result<(), ()> {
    if event.is_writable() {
        println!("Can write");
        while let Ok(msg) = to_send_queue.try_recv() {
            println!("Sending message: {:?}", msg);
        }
    }

    if event.is_readable() {
        println!("Can read");
        let mut head_buf = [0; 13];
        loop {
            let header = match con.read(&mut head_buf) {
                Ok(0) => {
                    println!("Closed");
                    continue;
                }
                Ok(n) => {
                    println!("Read: {}", n);

                    let h = MessageHeader::deserialize(head_buf);
                    if h.is_none() {
                        error!("[{}] Deserializing Header: {:?}", id, head_buf);
                        continue;
                    }
                    h.unwrap()
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // Socket is not ready anymore, stop reading
                    break;
                }
                e => panic!("err={:?}", e), // Unexpected error
            };

            match header.get_kind() {
                MessageType::Data => {}
                MessageType::Close => {
                    user_cons.remove(header.get_id());
                    return Ok(());
                }
                MessageType::Heartbeat => {
                    println!("Received Heartbeat");
                    continue;
                }
                _ => {
                    error!(
                        "[{}][{}] Unexpected Operation: {:?}",
                        id,
                        header.get_id(),
                        header.get_kind()
                    );
                    return Err(());
                }
            };

            let mut buf = vec![0; 4096];
            match con.read(&mut buf) {
                Ok(0) => {
                    println!("Closed");
                }
                Ok(n) => {
                    println!("Received: {:?}", &buf[0..n]);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // Socket is not ready anymore, stop reading
                    break;
                }
                e => panic!("err={:?}", e), // Unexpected error
            };
        }
    }

    Ok(())
}
