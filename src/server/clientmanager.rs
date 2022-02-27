#[derive(Debug)]
pub struct ClientManager<C> {
    index: std::sync::atomic::AtomicU64,
    client_count: std::sync::atomic::AtomicU64,
    clients: std::sync::Mutex<Vec<C>>,
}

pub trait Client {
    fn id(&self) -> u32;
}

impl<C> Default for ClientManager<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C> ClientManager<C> {
    /// Creates a new empty Client-Manager
    pub fn new() -> Self {
        ClientManager {
            index: std::sync::atomic::AtomicU64::new(0),
            client_count: std::sync::atomic::AtomicU64::new(0),
            clients: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl<C> ClientManager<C>
where
    C: Clone + Client,
{
    /// Returns a random Client from all currently connected
    /// Clients
    pub fn get(&self) -> Option<C> {
        let clients_data = self.clients.lock().unwrap();
        if clients_data.is_empty() {
            return None;
        }

        let raw_index = self.index.load(std::sync::atomic::Ordering::Relaxed);
        let client_count = clients_data.len() as u64;

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
    pub fn add(&self, client: C) {
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
            if client.id() == id {
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

    #[cfg(test)]
    fn client_count(&self) -> u64 {
        let clients = self.clients.lock().unwrap();
        clients.len() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestClient {
        id: u32,
    }

    impl Client for TestClient {
        fn id(&self) -> u32 {
            self.id
        }
    }

    #[test]
    fn new_empty_manager() {
        let manager = ClientManager::<TestClient>::new();

        assert_eq!(0, manager.client_count());
    }

    #[test]
    fn add_client() {
        let manager = std::sync::Arc::new(ClientManager::<TestClient>::new());
        assert_eq!(0, manager.client_count());

        manager.add(TestClient { id: 13 });
        assert_eq!(1, manager.client_count());
    }

    #[test]
    fn add_remove_client() {
        let manager = std::sync::Arc::new(ClientManager::new());
        assert_eq!(0, manager.client_count());

        manager.add(TestClient { id: 123 });
        assert_eq!(1, manager.client_count());

        manager.remove(123);
        assert_eq!(0, manager.client_count());
    }

    #[test]
    fn get_client() {
        let manager = std::sync::Arc::new(ClientManager::new());
        assert_eq!(0, manager.client_count());

        manager.add(TestClient { id: 123 });
        assert_eq!(1, manager.client_count());

        let tmp_client = manager.get();
        assert_eq!(true, tmp_client.is_some());
        assert_eq!(123, tmp_client.unwrap().id());
    }
}
