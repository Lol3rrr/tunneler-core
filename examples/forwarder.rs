use tunneler_core::client::queues;
use tunneler_core::message::Message;
use tunneler_core::streams::mpsc;

async fn handler(
    id: u32,
    reader: mpsc::StreamReader<Message>,
    sender: queues::Sender,
    data: Option<u64>,
) {
    println!("Handling: {}", id);
    sender.send(vec![b't', b'e', b's', b't'], 4).await;
}

fn main() {
    println!("Starting...");

    let key = vec![b'a', b'b', b'c', b'd', b'e'];
    let server = tunneler_core::server::Server::new(8080, 8081, key.clone());

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();

    rt.spawn(server.start());

    let destination = tunneler_core::Destination::new("localhost".to_owned(), 8081);
    let client = tunneler_core::client::Client::new(destination, key);

    rt.block_on(client.start(handler, None));
}
