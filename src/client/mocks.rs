use std::sync::Arc;

use super::{connections::UserCon, Handler};
use crate::Details;

use async_trait::async_trait;

pub struct EmptyHandler;

impl EmptyHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Handler for EmptyHandler {
    async fn new_con(self: Arc<Self>, _id: u32, _details: Details, _con: UserCon) {}
}
