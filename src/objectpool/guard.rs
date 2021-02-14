use crossbeam_queue::ArrayQueue;

/// A thin wrapper around the Data it is generic over
/// and returns the Data it owns over the given channel
/// once this Guard is dropped
#[derive(Debug)]
pub struct Guard<T>
where
    T: Default,
{
    data: T,
    queue: std::sync::Arc<ArrayQueue<T>>,
}

impl<T> Guard<T>
where
    T: Default,
{
    /// Returns a new Guard that will return the Data on the given Sender
    pub fn new(data: T, queue: std::sync::Arc<ArrayQueue<T>>) -> Self {
        Self { data, queue }
    }
}

impl<T> Drop for Guard<T>
where
    T: Default,
{
    fn drop(&mut self) {
        let inner_data = std::mem::take(&mut self.data);
        if !self.queue.is_full() {
            self.queue.push(inner_data);
        }
    }
}

impl<T> PartialEq for Guard<T>
where
    T: PartialEq + Default,
{
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl<T> std::ops::Deref for Guard<T>
where
    T: Default,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> std::ops::DerefMut for Guard<T>
where
    T: Default,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

#[test]
fn return_on_drop() {
    let queue = std::sync::Arc::new(ArrayQueue::new(1));
    let guard = Guard::new(5, queue.clone());
    drop(guard);

    assert_eq!(1, queue.len());
    assert_eq!(Some(5), queue.pop());
}

#[test]
fn deref_value_drop() {
    let queue = std::sync::Arc::new(ArrayQueue::new(1));
    let guard = Guard::new(5, queue.clone());
    assert_eq!(5, *guard);

    drop(guard);

    assert_eq!(1, queue.len());
    assert_eq!(Some(5), queue.pop());
}

#[test]
fn mut_deref_value_drop() {
    let queue = std::sync::Arc::new(ArrayQueue::new(1));
    let mut guard = Guard::new(5, queue.clone());
    assert_eq!(5, *guard);
    *guard = 3;
    assert_eq!(3, *guard);

    drop(guard);

    assert_eq!(1, queue.len());
    assert_eq!(Some(3), queue.pop());
}
