use crate::message::{Message, MessageHeader, MessageType};
use crate::objectpool;
use crate::streams::mpsc;
use crate::{client::queues, general::ConnectionReader};
use crate::{connections::Connections, general::Metrics};

use std::{future::Future, sync::Arc};

use log::{debug, error};

#[derive(Debug)]
enum ReceiveError {
    DeserializingHeader,
    ReceivingMessage(std::io::Error),
}

/// All the Options needed to receive a single Message
struct SingleOptions<'a, R, F, T> {
    /// The Connection to the external Server
    server_con: &'a mut R,
    /// The Queue to send Messages to the Server
    send_queue: &'a tokio::sync::mpsc::UnboundedSender<Message>,
    /// A Collection of all current Connections
    client_cons: &'a Arc<Connections<mpsc::StreamWriter<Message>>>,
    /// The Function used to start a Handler for a new Connection
    start_handler: &'a F,
    /// The Data that should be passed to the Handler on startup
    handler_data: Option<T>,
    /// The Buffer that should be used for Deserializing the Header
    /// into it
    head_buf: &'a mut [u8; 13],
    /// The Obeject Pool that should be used
    obj_pool: &'a objectpool::Pool<Vec<u8>>,
}

/// # Returns:
/// * Ok(_) if everything went alright and as expected
/// * Err(e) if any sort of Problem was encountered, more Details
/// are then provided using the Error-Type
async fn receive_single<F, Fut, T, R, M>(
    opts: SingleOptions<'_, R, F, T>,
    metrics: &M,
) -> Result<(), ReceiveError>
where
    F: Fn(u32, mpsc::StreamReader<Message>, queues::Sender, Option<T>) -> Fut,
    Fut: Future + Send + 'static,
    Fut::Output: Send,
    T: Sized + Send + Clone,
    R: ConnectionReader + Sized + Send + Sync,
    M: Metrics + Send + Sync,
{
    let header = match opts.server_con.read_full(opts.head_buf).await {
        Ok(_) => match MessageHeader::deserialize(&opts.head_buf) {
            Some(s) => s,
            None => return Err(ReceiveError::DeserializingHeader),
        },
        Err(e) => return Err(ReceiveError::ReceivingMessage(e)),
    };

    let id = header.get_id();
    let kind = header.get_kind();
    match kind {
        MessageType::Close => {
            opts.client_cons.remove(id);
            return Ok(());
        }
        MessageType::Data | MessageType::EOF => {}
        // A new connection should be established for the given ID
        MessageType::Connect => {
            // Setup the send channel for requests for this user
            let (tx, handle_rx) = mpsc::stream();
            // Add the Connection to the current map of user-connection
            opts.client_cons.set(id, tx);
            let handle_tx =
                queues::Sender::new(id, opts.send_queue.clone(), opts.client_cons.clone());
            let handler = opts.start_handler;
            tokio::task::spawn(handler(id, handle_rx, handle_tx, opts.handler_data));

            debug!("Established new Connection: {}", id);

            return Ok(());
        }
        _ => {
            error!("Unexpected Operation: {:?}", kind);
            return Ok(());
        }
    };

    metrics.received_msg();
    metrics.recv_bytes(header.get_length());

    let data_length = header.get_length() as usize;
    let mut buf = opts.obj_pool.get();
    buf.resize(data_length, 0);

    let msg = match opts.server_con.read_full(&mut buf).await {
        Ok(_) => Message::new_guarded(header, buf),
        Err(e) => {
            error!("Receiving Data: {}", e);
            opts.client_cons.remove(id);
            return Ok(());
        }
    };

    let con_queue = match opts.client_cons.get_clone(id) {
        Some(q) => q,
        // In case there is no matching user-connection, create a new one
        None => {
            error!("Received Data for non-existing Connection: {}", id);
            return Ok(());
        }
    };

    match con_queue.send(msg) {
        Ok(_) => {}
        Err(e) => {
            error!("Adding to Queue for {}: {}", id, e);
            return Ok(());
        }
    };

    Ok(())
}

/// Receives all the messages from the server
///
/// Then adds the message to the matching connection queue.
/// If there is no matching queue, it creates and starts a new client,
/// which will then be placed into the Connection Manager for further
/// requests
///
/// # Params:
/// * `server_con`: The Connection to the external Server
/// * `send_queue`: The Queue of messages that should be send to the Server
/// * `client_cons`: A Collection of Clients that are all listening on this Connection
/// * `start_handler`: The Function used to start a new Handler when a new Connection is received
/// * `handler_data`: The Data that will be passed to the `start_handler` function
pub async fn receiver<F, Fut, T, R, M>(
    mut server_con: R,
    send_queue: tokio::sync::mpsc::UnboundedSender<Message>,
    client_cons: std::sync::Arc<Connections<mpsc::StreamWriter<Message>>>,
    start_handler: &F,
    handler_data: &Option<T>,
    metrics: Arc<M>,
) where
    F: Fn(u32, mpsc::StreamReader<Message>, queues::Sender, Option<T>) -> Fut,
    Fut: Future + Send + 'static,
    Fut::Output: Send,
    T: Sized + Send + Clone,
    R: ConnectionReader + Sized + Send + Sync,
    M: Metrics + Send + Sync,
{
    let mut head_buf = [0; 13];
    let obj_pool: objectpool::Pool<Vec<u8>> = objectpool::Pool::new(50);

    loop {
        let opts = SingleOptions {
            server_con: &mut server_con,
            send_queue: &send_queue,
            client_cons: &client_cons,
            start_handler,
            handler_data: handler_data.clone(),
            head_buf: &mut head_buf,
            obj_pool: &obj_pool,
        };
        match receive_single(opts, metrics.as_ref()).await {
            Ok(_) => {}
            Err(e) => {
                log::error!("Receiving: {:?}", e);
                break;
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::general::mocks;
    use crate::metrics::Empty;

    async fn test_handler(
        id: u32,
        _reader: mpsc::StreamReader<Message>,
        _sender: queues::Sender,
        _data: Option<u64>,
    ) {
        println!("Started: {}", id);
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
            SingleOptions {
                server_con: &mut tmp_reader,
                send_queue: &queue_tx,
                client_cons: &client_cons,
                start_handler: &test_handler,
                handler_data: None,
                head_buf: &mut head_buf,
                obj_pool: &obj_pool,
            },
            &Empty::new(),
        )
        .await;

        assert_eq!(true, result.is_ok());

        assert_eq!(
            Ok(Message::new(
                MessageHeader::new(id, MessageType::Data, 10),
                vec![3; 10],
            )),
            client_rx.recv().await
        );
    }

    #[tokio::test]
    async fn valid_establish_connection() {
        let id = 13;

        let mut tmp_reader = mocks::MockReader::new();
        tmp_reader.add_message(Message::new(
            MessageHeader::new(id, MessageType::Connect, 0),
            vec![],
        ));

        let (queue_tx, _) = tokio::sync::mpsc::unbounded_channel();

        let client_cons = std::sync::Arc::new(Connections::<mpsc::StreamWriter<Message>>::new());

        let mut head_buf = [0; 13];
        let obj_pool = objectpool::Pool::new(2);

        let result = receive_single(
            SingleOptions {
                server_con: &mut tmp_reader,
                send_queue: &queue_tx,
                client_cons: &client_cons,
                start_handler: &test_handler,
                handler_data: None,
                head_buf: &mut head_buf,
                obj_pool: &obj_pool,
            },
            &Empty::new(),
        )
        .await;

        assert_eq!(true, result.is_ok());

        let connection_queue = client_cons.get_clone(id);
        assert_eq!(true, connection_queue.is_some());
    }
}
