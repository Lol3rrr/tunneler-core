use crate::client::connections::UserCon;
use crate::client::user_con;
use crate::general::ConnectionReader;
use crate::streams::mpsc;
use crate::Details;
use crate::{
    client::Handler,
    message::{Message, MessageHeader, MessageType},
};
use crate::{connections::Connections, metrics::Metrics};

use std::sync::Arc;

#[derive(Debug)]
enum ReceiveError {
    DeserializingHeader,
    ReceivingMessage(std::io::Error),
}

/// All the Options needed to receive a single Message
struct SingleOptions<'a, R> {
    /// The Connection to the external Server
    server_con: &'a mut R,
    /// The Queue to send Messages to the Server
    send_queue: &'a tokio::sync::mpsc::UnboundedSender<Message>,
    /// A Collection of all current Connections
    client_cons: &'a Arc<Connections<mpsc::StreamWriter<Message>>>,
    /// The Buffer that should be used for Deserializing the Header
    /// into it
    head_buf: &'a mut [u8; 13],
}

/// Receives a single Message from the external Server and processes
/// it accordingly
///
/// # Returns:
/// * Ok(_) if everything went alright and as expected
/// * Err(e) if any sort of Problem was encountered, more Details
/// are then provided using the Error-Type
async fn receive_single<R, H, M>(
    opts: SingleOptions<'_, R>,
    handler: Arc<H>,
    metrics: &M,
) -> Result<(), ReceiveError>
where
    R: ConnectionReader + Sized + Send + Sync,
    H: Handler + Send + Sync + 'static,
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
            debug!("Closing Connection: {}", id);

            return Ok(());
        }
        MessageType::Data | MessageType::EOF => {}
        // A new connection should be established for the given ID
        MessageType::Connect => {
            let mut details_buf = vec![0; header.get_length() as usize];
            let details = match opts.server_con.read_full(&mut details_buf).await {
                Ok(_) => match Details::deserialize(&mut details_buf) {
                    Ok(d) => d,
                    Err(e) => {
                        error!("Parsing Connection-Details: {:?}", e);
                        return Ok(());
                    }
                },
                Err(e) => {
                    error!("Reading Connection-Details: {:?}", e);
                    return Ok(());
                }
            };

            // Setup the send channel for requests for this user
            let (tx, stream_rx) = mpsc::stream();
            // Add the Connection to the current map of user-connection
            opts.client_cons.set(id, tx);

            let handle_rx = user_con::OwnedReceiver::new(stream_rx);
            let handle_tx =
                user_con::OwnedSender::new(id, opts.send_queue.clone(), opts.client_cons.clone());

            let handle_con = UserCon::new(handle_rx, handle_tx);
            tokio::task::spawn(H::new_con(handler, id, details, handle_con));

            debug!("Established new Connection: {}", id);

            return Ok(());
        }
        _ => {
            error!("Unexpected Message-Type: {:?}", kind);
            return Ok(());
        }
    };

    // Handle all the metrics related stuff
    metrics.received_msg();
    metrics.recv_bytes(header.get_length());

    let data_length = header.get_length() as usize;
    let mut buf = vec![0; data_length];

    let msg = match opts.server_con.read_full(&mut buf).await {
        Ok(_) => Message::new(header, buf),
        Err(e) => {
            error!("Receiving Data: {}", e);
            opts.client_cons.remove(id);
            return Ok(());
        }
    };

    let con_queue = match opts.client_cons.get_clone(id) {
        Some(q) => q,
        None => {
            error!("Received Data for non-existing Connection: {}", id);
            return Ok(());
        }
    };

    if let Err(e) = con_queue.send(msg) {
        error!("Adding to Queue for {}: {}", id, e);
        return Ok(());
    }

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
pub async fn receiver<R, H, M>(
    mut server_con: R,
    send_queue: tokio::sync::mpsc::UnboundedSender<Message>,
    client_cons: std::sync::Arc<Connections<mpsc::StreamWriter<Message>>>,
    handler: Arc<H>,
    metrics: Arc<M>,
) where
    R: ConnectionReader + Sized + Send + Sync,
    H: Handler + Send + Sync + 'static,
    M: Metrics + Send + Sync,
{
    let mut head_buf = [0; 13];

    loop {
        let opts = SingleOptions {
            server_con: &mut server_con,
            send_queue: &send_queue,
            client_cons: &client_cons,
            head_buf: &mut head_buf,
        };
        if let Err(e) = receive_single(opts, handler.clone(), metrics.as_ref()).await {
            error!("Receiving: {:?}", e);
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use super::*;
    use crate::client::mocks as client_mocks;
    use crate::general::mocks;
    use crate::metrics::Empty;

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

        let result = receive_single(
            SingleOptions {
                server_con: &mut tmp_reader,
                send_queue: &queue_tx,
                client_cons: &client_cons,
                head_buf: &mut head_buf,
            },
            Arc::new(client_mocks::EmptyHandler::new()),
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

        let details = Details::new(IpAddr::V4(Ipv4Addr::from([0, 0, 0, 0]))).serialize();

        let mut tmp_reader = mocks::MockReader::new();
        tmp_reader.add_message(Message::new(
            MessageHeader::new(id, MessageType::Connect, details.len() as u64),
            details,
        ));

        let (queue_tx, _) = tokio::sync::mpsc::unbounded_channel();

        let client_cons = std::sync::Arc::new(Connections::<mpsc::StreamWriter<Message>>::new());

        let mut head_buf = [0; 13];

        let result = receive_single(
            SingleOptions {
                server_con: &mut tmp_reader,
                send_queue: &queue_tx,
                client_cons: &client_cons,
                head_buf: &mut head_buf,
            },
            Arc::new(client_mocks::EmptyHandler::new()),
            &Empty::new(),
        )
        .await;

        assert_eq!(true, result.is_ok());

        let connection_queue = client_cons.get_clone(id);
        assert_eq!(true, connection_queue.is_some());
    }
}
