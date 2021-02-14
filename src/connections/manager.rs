use dashmap::DashMap;

#[derive(Debug)]
pub struct Connections<T>
where
    T: Clone,
{
    connections: std::sync::Arc<DashMap<u32, T, fnv::FnvBuildHasher>>,
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
            connections: std::sync::Arc::new(DashMap::with_hasher(fnv::FnvBuildHasher::default())),
        }
    }

    #[inline(always)]
    pub fn get_clone(&self, id: u32) -> Option<T> {
        match self.connections.get(&id) {
            None => None,
            Some(c) => Some(c.clone()),
        }
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
