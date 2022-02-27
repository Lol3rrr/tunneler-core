use std::sync::Arc;

use tunneler_core::client::{Handler, Sender, UserCon};
use tunneler_core::{server::Strategy, Details};

use async_trait::async_trait;

pub struct ExampleHandler;

#[async_trait]
impl Handler for ExampleHandler {
    async fn new_con(self: Arc<Self>, id: u32, _details: Details, con: UserCon) {
        println!("Handling: {}", id);
        con.send_msg(vec![b't', b'e', b's', b't'], 4).await.unwrap();
    }
}

fn main() {
    println!("Starting...");

    let key = vec![b'a', b'b', b'c', b'd', b'e'];
    let server = tunneler_core::server::builder()
        .listen_port(8081)
        .port_strategy(Strategy::Single(8080))
        .key(key.clone())
        .empty_metrics()
        .build();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();

    rt.spawn(server.listen());

    let destination = tunneler_core::Destination::new("localhost".to_owned(), 8081);
    let client = tunneler_core::client::builder()
        .destination(destination)
        .external_port(8080)
        .key(key)
        .empty_metrics()
        .build();

    rt.block_on(client.start(Arc::new(ExampleHandler {})));
}
