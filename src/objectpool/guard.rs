use log::error;

/// A thin wrapper around the Data it is generic over
/// and returns the Data it owns over the given channel
/// once this Guard is dropped
#[derive(Debug)]
pub struct Guard<T>
where
    T: Default,
{
    data: T,
    ret_tx: tokio::sync::mpsc::UnboundedSender<T>,
}

impl<T> Guard<T>
where
    T: Default,
{
    /// Returns a new Guard that will return the Data on the given Sender
    pub fn new(data: T, return_tx: tokio::sync::mpsc::UnboundedSender<T>) -> Self {
        Self {
            data,
            ret_tx: return_tx,
        }
    }
}

impl<T> Drop for Guard<T>
where
    T: Default,
{
    fn drop(&mut self) {
        let inner_data = std::mem::take(&mut self.data);
        if let Err(e) = self.ret_tx.send(inner_data) {
            error!("Returning Data: {}", e);
        };
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
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let guard = Guard::new(5, tx);
    drop(guard);

    assert_eq!(Some(5), rx.blocking_recv());
}

#[test]
fn deref_value_drop() {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let guard = Guard::new(5, tx);
    assert_eq!(5, *guard);

    drop(guard);

    assert_eq!(Some(5), rx.blocking_recv());
}

#[test]
fn mut_deref_value_drop() {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let mut guard = Guard::new(5, tx);
    assert_eq!(5, *guard);
    *guard = 3;
    assert_eq!(3, *guard);

    drop(guard);

    assert_eq!(Some(3), rx.blocking_recv());
}
