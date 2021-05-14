use crate::message::{Message, MessageHeader, MessageType};

pub async fn keep_alive(
    send_queue: tokio::sync::mpsc::UnboundedSender<Message>,
    wait_time: std::time::Duration,
) {
    loop {
        let msg_header = MessageHeader::new(0, MessageType::Heartbeat, 0);
        let msg = Message::new(msg_header, Vec::new());
        if let Err(e) = send_queue.send(msg) {
            log::error!("Sending Heartbeat: {}", e);
            return;
        };
        tokio::time::sleep(wait_time).await;
    }
}
