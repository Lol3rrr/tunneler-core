use crate::objectpool::Guard;

/// A Generic Pool that recovers the Data it hands out
/// for later use again
pub struct Pool<T>
where
    T: Default,
{
    objs: std::sync::Mutex<Vec<T>>,
    tx: tokio::sync::mpsc::UnboundedSender<T>,
    rx: tokio::sync::Mutex<tokio::sync::mpsc::UnboundedReceiver<T>>,
}

impl<T> Pool<T>
where
    T: Default,
{
    /// Creates a new Pool with size amount of Elements
    pub fn new(size: usize) -> Self {
        let mut objs = Vec::with_capacity(size);
        for _ in 0..size {
            objs.push(T::default());
        }

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        Self {
            objs: std::sync::Mutex::new(objs),
            tx,
            rx: tokio::sync::Mutex::new(rx),
        }
    }

    /// Creates a new Pool just like `new` but directly wraps it
    /// in an Arc to allow for easy sharing
    pub fn new_arc(size: usize) -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self::new(size))
    }

    /// Returns the currently available Objects in the pool
    pub fn available_objs(&self) -> usize {
        let tmp = self.objs.lock().unwrap();
        tmp.len()
    }

    /// Tries to return an entry
    pub fn try_get(&self) -> Option<Guard<T>> {
        let mut tmp_objs = self.objs.lock().unwrap();
        match tmp_objs.pop() {
            Some(s) => Some(Guard::new(s, self.tx.clone())),
            None => None,
        }
    }

    /// Always returns an Instance of the Pool-Datatype
    pub fn get(&self) -> Guard<T> {
        match self.try_get() {
            Some(s) => s,
            None => {
                let n_obj = T::default();
                Guard::new(n_obj, self.tx.clone())
            }
        }
    }

    /// Actually attempts to recover a single new piece of Data
    /// received on the Channel
    async fn recover(&self) {
        let mut rx = self.rx.lock().await;

        let tmp = match rx.recv().await {
            Some(s) => s,
            None => {
                return;
            }
        };
        drop(rx);

        let mut underlying_data = self.objs.lock().unwrap();
        underlying_data.push(tmp);
    }

    /// This should run in its own seperate loop as it will block the
    /// current task forever
    pub async fn recover_loop(pool: std::sync::Arc<Self>) {
        loop {
            pool.recover().await;
        }
    }
}

#[test]
fn new_available() {
    let pool: Pool<u32> = Pool::new(5);
    assert_eq!(5, pool.available_objs());
}

#[test]
fn try_get_not_empty_without_recover() {
    let pool: Pool<u32> = Pool::new(5);
    assert_eq!(5, pool.available_objs());

    let returned = pool.try_get();
    assert_eq!(true, returned.is_some());
    let value = returned.unwrap();
    assert_eq!(0, *value);

    drop(value);

    assert_eq!(4, pool.available_objs());
}
#[test]
fn try_get_empty_without_recover() {
    let pool: Pool<u32> = Pool::new(0);
    assert_eq!(0, pool.available_objs());

    let returned = pool.try_get();
    assert_eq!(false, returned.is_some());

    assert_eq!(0, pool.available_objs());
}
#[tokio::test]
async fn try_get_not_empty_with_recover() {
    let pool: Pool<u32> = Pool::new(5);
    assert_eq!(5, pool.available_objs());

    let returned = pool.try_get();
    assert_eq!(true, returned.is_some());
    let value = returned.unwrap();
    assert_eq!(0, *value);

    assert_eq!(4, pool.available_objs());
    drop(value);

    pool.recover().await;

    assert_eq!(5, pool.available_objs());
}

#[test]
fn get_empty_without_recover() {
    let pool: Pool<u32> = Pool::new(0);
    assert_eq!(0, pool.available_objs());

    let value = pool.get();
    assert_eq!(0, *value);

    drop(value);

    assert_eq!(0, pool.available_objs());
}
#[tokio::test]
async fn get_empty_with_recover() {
    let pool: Pool<u32> = Pool::new(0);
    assert_eq!(0, pool.available_objs());

    let value = pool.get();
    assert_eq!(0, *value);

    drop(value);
    assert_eq!(0, pool.available_objs());

    pool.recover().await;

    assert_eq!(1, pool.available_objs());
}
#[tokio::test]
async fn get_with_recover() {
    let pool: Pool<u32> = Pool::new(1);
    assert_eq!(1, pool.available_objs());

    let value = pool.get();
    assert_eq!(0, *value);

    drop(value);
    assert_eq!(0, pool.available_objs());

    pool.recover().await;

    assert_eq!(1, pool.available_objs());
}
