use crate::client::queues;
use crate::connections::{Connection, Connections};
use crate::message::{Message, MessageHeader, MessageType};
use crate::objectpool;
use crate::streams::mpsc;

use std::future::Future;

use log::error;

/// Receives all the messages from the server
///
/// Then adds the message to the matching connection queue.
/// If there is no matching queue, it creates and starts a new client,
/// which will then be placed into the Connection Manager for further
/// requests
pub async fn receiver<F, Fut, T>(
    server_con: std::sync::Arc<Connection>,
    send_queue: tokio::sync::mpsc::UnboundedSender<Message>,
    client_cons: std::sync::Arc<Connections<mpsc::StreamWriter<Message>>>,
    start_handler: &F,
    handler_data: &Option<T>,
) where
    F: Fn(u32, mpsc::StreamReader<Message>, queues::Sender, Option<T>) -> Fut,
    Fut: Future<Output = ()>,
    T: Sized + Send + Clone,
{
    let mut head_buf = [0; 13];
    let obj_pool: objectpool::Pool<Vec<u8>> = objectpool::Pool::new(50);

    loop {
        let header = match server_con.read_total(&mut head_buf, 13).await {
            Ok(_) => match MessageHeader::deserialize(&head_buf) {
                Some(s) => s,
                None => {
                    error!("Deserializing Header: {:?}", head_buf);
                    return;
                }
            },
            Err(e) => {
                error!("Reading Data: {}", e);
                return;
            }
        };

        let id = header.get_id();
        let kind = header.get_kind();
        match kind {
            MessageType::Close => {
                client_cons.remove(id);
                continue;
            }
            MessageType::Data => {}
            _ => {
                error!("Unexpected Operation: {:?}", kind);
                continue;
            }
        };

        let data_length = header.get_length() as usize;
        let mut buf = obj_pool.get();
        buf.resize(data_length, 0);

        let msg = match server_con.read_total(&mut buf, data_length).await {
            Ok(_) => Message::new_guarded(header, buf),
            Err(e) => {
                error!("Receiving Data: {}", e);
                client_cons.remove(id);
                continue;
            }
        };

        let con_queue = match client_cons.get_clone(id) {
            Some(q) => q.clone(),
            // In case there is no matching user-connection, create a new one
            None => {
                // Setup the send channel for requests for this user
                let (tx, handle_rx) = mpsc::stream();
                // Add the Connection to the current map of user-connection
                client_cons.set(id, tx.clone());

                let handle_tx = queues::Sender::new(id, send_queue.clone(), client_cons.clone());

                start_handler(id, handle_rx, handle_tx, handler_data.clone()).await;
                tx
            }
        };

        match con_queue.send(msg) {
            Ok(_) => {}
            Err(e) => {
                error!("Adding to Queue for {}: {}", id, e);
                continue;
            }
        };
    }
}
