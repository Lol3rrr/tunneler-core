use std::sync::Arc;

use super::{mpsc, Handler, QueueSender};
use crate::{message::Message, Details};

use async_trait::async_trait;

pub struct EmptyHandler;

impl EmptyHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Handler for EmptyHandler {
    async fn new_con(
        self: Arc<Self>,
        _id: u32,
        _details: Details,
        _rx: mpsc::StreamReader<Message>,
        _tx: QueueSender,
    ) {
    }
}
