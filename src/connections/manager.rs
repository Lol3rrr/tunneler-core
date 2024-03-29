use dashmap::DashMap;

#[derive(Debug)]
pub struct Connections<T>
where
    T: Clone,
{
    connections: std::sync::Arc<DashMap<u32, T, ahash::RandomState>>,
}

impl<T> Default for Connections<T>
where
    T: Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Connections<T>
where
    T: Clone,
{
    pub fn new() -> Connections<T> {
        Connections {
            connections: std::sync::Arc::new(DashMap::default()),
        }
    }

    #[inline(always)]
    pub fn get_clone(&self, id: u32) -> Option<T> {
        self.connections.get(&id).map(|c| c.clone())
    }

    #[inline(always)]
    pub fn set(&self, id: u32, con: T) {
        self.connections.insert(id, con);
    }

    #[inline(always)]
    pub fn remove(&self, id: u32) -> Option<(u32, T)> {
        self.connections.remove(&id)
    }
}

impl<T> Clone for Connections<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            connections: self.connections.clone(),
        }
    }
}
