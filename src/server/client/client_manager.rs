use crate::server::client::Client;

#[derive(Debug)]
pub struct ClientManager {
    index: std::sync::atomic::AtomicU64,
    client_count: std::sync::atomic::AtomicU64,
    clients: std::sync::Mutex<Vec<Client>>,
}

impl Default for ClientManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientManager {
    /// Creates a new empty Client-Manager
    pub fn new() -> ClientManager {
        ClientManager {
            index: std::sync::atomic::AtomicU64::new(0),
            client_count: std::sync::atomic::AtomicU64::new(0),
            clients: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Returns a random Client from all currently connected
    /// Clients
    pub fn get(&self) -> Option<Client> {
        if self.client_count() == 0 {
            return None;
        }

        let clients_data = self.clients.lock().unwrap();
        let raw_index = self.index.load(std::sync::atomic::Ordering::SeqCst);
        let client_count = self.client_count.load(std::sync::atomic::Ordering::SeqCst);

        let index = raw_index % client_count;
        let client = clients_data.get(index as usize)?;
        self.index
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Some(client.clone())
    }

    /// Adds a new client connection to the List of connections
    ///
    /// Params:
    /// * client: The Client to add
    ///
    /// Returns:
    /// This function returns the new number of clients managed
    /// by this
    pub fn add(&self, client: Client) {
        let mut clients_data = self.clients.lock().unwrap();
        clients_data.push(client);
        drop(clients_data);
        self.client_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    /// This is used to remove a client connection again
    ///
    /// Params:
    /// * id: The ID of the Client-Connection to remove
    ///
    /// Returns:
    /// This function returns the new number of clients managed
    /// by this
    pub fn remove(&self, id: u32) {
        let mut client_data = self.clients.lock().unwrap();
        let mut remove_index: Option<usize> = None;
        for (index, client) in client_data.iter().enumerate() {
            if client.get_id() == id {
                remove_index = Some(index);
                break;
            }
        }

        match remove_index {
            None => {}
            Some(i) => {
                client_data.remove(i);
                self.client_count
                    .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            }
        };

        drop(client_data);
    }

    /// Returns the number of currently connected clients
    pub fn client_count(&self) -> u64 {
        self.client_count.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[test]
fn new_empty_manager() {
    let manager = ClientManager::new();

    assert_eq!(0, manager.client_count());
}

#[test]
fn add_client() {
    let manager = std::sync::Arc::new(ClientManager::new());
    assert_eq!(0, manager.client_count());

    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    manager.add(Client::new(123, manager.clone(), tx));
    assert_eq!(1, manager.client_count());
}

#[test]
fn add_remove_client() {
    let manager = std::sync::Arc::new(ClientManager::new());
    assert_eq!(0, manager.client_count());

    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    manager.add(Client::new(123, manager.clone(), tx));
    assert_eq!(1, manager.client_count());

    manager.remove(123);
    assert_eq!(0, manager.client_count());
}

#[test]
fn get_client() {
    let manager = std::sync::Arc::new(ClientManager::new());
    assert_eq!(0, manager.client_count());

    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    manager.add(Client::new(123, manager.clone(), tx.clone()));
    assert_eq!(1, manager.client_count());

    let tmp_client = manager.get();
    assert_eq!(true, tmp_client.is_some());
    assert_eq!(123, tmp_client.unwrap().get_id());
}
