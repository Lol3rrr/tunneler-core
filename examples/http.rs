use tunneler_core::client;
use tunneler_core::message;
use tunneler_core::server;
use tunneler_core::streams;
use tunneler_core::Destination;

/// This will be called for every new connection forwarded to the client
async fn client_handler(
    _id: u32,
    _reader: streams::mpsc::StreamReader<message::Message>,
    sender: client::queues::Sender,
    raw_extra: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
) {
    let extra = raw_extra.unwrap();
    extra.store(true, std::sync::atomic::Ordering::SeqCst);

    let resp_data = "HTTP/1.1 200 OK\r\nServer: test\r\n\r\ntest-body"
        .as_bytes()
        .to_vec();
    let length = resp_data.len() as u64;
    sender.send(resp_data, length).await;
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let req_port = 30123;
    let listen_port = 30124;
    let key = vec![0, 1, 2, 3, 4];

    let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    println!("Setting up the Server");
    let server_instance = server::Server::new(req_port, listen_port, key.clone());
    tokio::task::spawn(server_instance.start());

    println!("Setting up the Client");
    let server_dest = Destination::new("localhost".to_owned(), listen_port);
    let client_instance = client::Client::new(server_dest, key.clone());
    tokio::task::spawn(client_instance.start(client_handler, Some(called.clone())));

    // Wait some time for everything to setup
    println!("Waiting for setup to finish");
    tokio::time::sleep(std::time::Duration::new(15, 0)).await;
    // Send test request
    let req_url = format!("http://localhost:{}/", req_port);

    let resp = reqwest::get(&req_url).await.unwrap();

    assert_eq!(reqwest::StatusCode::OK, resp.status());
    assert_eq!(true, called.load(std::sync::atomic::Ordering::SeqCst));
}
