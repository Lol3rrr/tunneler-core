use crate::connections::Connections;
use crate::message::{Message, MessageHeader, MessageType};
use crate::objectpool;
use crate::streams::mpsc;
use crate::{client::queues, general::ConnectionReader};

#[cfg(test)]
use crate::general::mocks;

use std::future::Future;

use log::error;

/// Returns
/// * True if everything went alright and there are more
/// to come
/// * False if there was an error and it should be stopped
async fn receive_single<F, Fut, T, R>(
    server_con: &mut R,
    send_queue: &tokio::sync::mpsc::UnboundedSender<Message>,
    client_cons: &std::sync::Arc<Connections<mpsc::StreamWriter<Message>>>,
    start_handler: &F,
    handler_data: Option<T>,
    head_buf: &mut [u8; 13],
    obj_pool: &objectpool::Pool<Vec<u8>>,
) -> bool
where
    F: Fn(u32, mpsc::StreamReader<Message>, queues::Sender, Option<T>) -> Fut,
    Fut: Future + Send + 'static,
    Fut::Output: Send,
    T: Sized + Send + Clone,
    R: ConnectionReader + Sized + Send + Sync,
{
    let header = match server_con.read_full(head_buf).await {
        Ok(_) => match MessageHeader::deserialize(&head_buf) {
            Some(s) => s,
            None => {
                error!("Deserializing Header: {:?}", head_buf);
                return false;
            }
        },
        Err(e) => {
            error!("Reading Data: {}", e);
            return false;
        }
    };

    let id = header.get_id();
    let kind = header.get_kind();
    match kind {
        MessageType::Close => {
            client_cons.remove(id);
            return true;
        }
        MessageType::Data | MessageType::EOF => {}
        _ => {
            error!("Unexpected Operation: {:?}", kind);
            return true;
        }
    };

    let data_length = header.get_length() as usize;
    let mut buf = obj_pool.get();
    buf.resize(data_length, 0);

    let msg = match server_con.read_full(&mut buf).await {
        Ok(_) => Message::new_guarded(header, buf),
        Err(e) => {
            error!("Receiving Data: {}", e);
            client_cons.remove(id);
            return true;
        }
    };

    let con_queue = match client_cons.get_clone(id) {
        Some(q) => q,
        // In case there is no matching user-connection, create a new one
        None => {
            // Setup the send channel for requests for this user
            let (tx, handle_rx) = mpsc::stream();
            // Add the Connection to the current map of user-connection
            client_cons.set(id, tx.clone());

            let handle_tx = queues::Sender::new(id, send_queue.clone(), client_cons.clone());

            tokio::task::spawn(start_handler(id, handle_rx, handle_tx, handler_data));
            tx
        }
    };

    match con_queue.send(msg) {
        Ok(_) => {}
        Err(e) => {
            error!("Adding to Queue for {}: {}", id, e);
            return true;
        }
    };

    return true;
}

/// Receives all the messages from the server
///
/// Then adds the message to the matching connection queue.
/// If there is no matching queue, it creates and starts a new client,
/// which will then be placed into the Connection Manager for further
/// requests
pub async fn receiver<F, Fut, T, R>(
    mut server_con: R,
    send_queue: tokio::sync::mpsc::UnboundedSender<Message>,
    client_cons: std::sync::Arc<Connections<mpsc::StreamWriter<Message>>>,
    start_handler: &F,
    handler_data: &Option<T>,
) where
    F: Fn(u32, mpsc::StreamReader<Message>, queues::Sender, Option<T>) -> Fut,
    Fut: Future + Send + 'static,
    Fut::Output: Send,
    T: Sized + Send + Clone,
    R: ConnectionReader + Sized + Send + Sync,
{
    let mut head_buf = [0; 13];
    let obj_pool: objectpool::Pool<Vec<u8>> = objectpool::Pool::new(50);

    loop {
        if !receive_single(
            &mut server_con,
            &send_queue,
            &client_cons,
            start_handler,
            handler_data.clone(),
            &mut head_buf,
            &obj_pool,
        )
        .await
        {
            break;
        }
    }
}

#[cfg(test)]
async fn test_handler(
    id: u32,
    _reader: mpsc::StreamReader<Message>,
    _sender: queues::Sender,
    _data: Option<u64>,
) {
    println!("Started: {}", id);
}

#[tokio::test]
async fn valid_starts_handler() {
    let id = 13;

    let mut tmp_reader = mocks::MockReader::new();
    tmp_reader.add_message(Message::new(
        MessageHeader::new(id, MessageType::Data, 10),
        vec![3; 10],
    ));

    let (queue_tx, _) = tokio::sync::mpsc::unbounded_channel();

    let client_cons = std::sync::Arc::new(Connections::<mpsc::StreamWriter<Message>>::new());

    let mut head_buf = [0; 13];
    let obj_pool = objectpool::Pool::new(2);

    let result = receive_single(
        &mut tmp_reader,
        &queue_tx,
        &client_cons,
        &test_handler,
        None,
        &mut head_buf,
        &obj_pool,
    )
    .await;

    assert_eq!(true, result);

    let connection_queue = client_cons.get_clone(id);
    assert_eq!(true, connection_queue.is_some());
}

#[tokio::test]
async fn valid_sends_data_to_correct_handler() {
    let id = 13;

    let mut tmp_reader = mocks::MockReader::new();
    tmp_reader.add_message(Message::new(
        MessageHeader::new(id, MessageType::Data, 10),
        vec![3; 10],
    ));

    let (queue_tx, _) = tokio::sync::mpsc::unbounded_channel();

    let client_cons = std::sync::Arc::new(Connections::<mpsc::StreamWriter<Message>>::new());

    let (client_tx, mut client_rx) = mpsc::stream();
    client_cons.set(id, client_tx);

    let mut head_buf = [0; 13];
    let obj_pool = objectpool::Pool::new(2);

    let result = receive_single(
        &mut tmp_reader,
        &queue_tx,
        &client_cons,
        &test_handler,
        None,
        &mut head_buf,
        &obj_pool,
    )
    .await;

    assert_eq!(true, result);

    assert_eq!(
        Ok(Message::new(
            MessageHeader::new(id, MessageType::Data, 10),
            vec![3; 10],
        )),
        client_rx.recv().await
    );
}
