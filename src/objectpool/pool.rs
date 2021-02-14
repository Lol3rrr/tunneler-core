use crate::objectpool::Guard;

use crossbeam_queue::ArrayQueue;

/// A Generic Pool that recovers the Data it hands out
/// for later use again
pub struct Pool<T>
where
    T: Default,
{
    objs: std::sync::Arc<ArrayQueue<T>>,
}

impl<T> Pool<T>
where
    T: Default,
{
    /// Creates a new Pool with size amount of Elements
    pub fn new(size: usize) -> Self {
        let objs = std::sync::Arc::new(ArrayQueue::new(size));
        for _ in 0..size {
            // This should not fail as we only fill this up to the max
            // size so it will never exceed its given capacity
            objs.push(T::default());
        }

        Self { objs }
    }

    /// Tries to return an entry
    pub fn try_get(&self) -> Option<Guard<T>> {
        match self.objs.pop() {
            Some(s) => Some(Guard::new(s, self.objs.clone())),
            None => None,
        }
    }

    /// Always returns an Instance of the Pool-Datatype
    pub fn get(&self) -> Guard<T> {
        match self.try_get() {
            Some(s) => s,
            None => {
                let n_obj = T::default();
                Guard::new(n_obj, self.objs.clone())
            }
        }
    }
}

impl<T> Clone for Pool<T>
where
    T: Default,
{
    fn clone(&self) -> Self {
        Self {
            objs: self.objs.clone(),
        }
    }
}

#[test]
fn try_get_not_empty() {
    let pool: Pool<u32> = Pool::<u32>::new(5);

    let returned = pool.try_get();
    assert_eq!(true, returned.is_some());
    let value = returned.unwrap();
    assert_eq!(0, *value);

    drop(value);
}
#[test]
fn try_get_empty() {
    let pool: Pool<u32> = Pool::<u32>::new(1);

    let first = pool.try_get().unwrap();

    let returned = pool.try_get();
    assert_eq!(false, returned.is_some());

    drop(first);
}

#[test]
fn get_empty() {
    let pool: Pool<u32> = Pool::<u32>::new(1);

    let value = pool.get();
    assert_eq!(0, *value);

    drop(value);
}
