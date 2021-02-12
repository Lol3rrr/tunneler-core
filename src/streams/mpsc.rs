use crate::streams::error::{RecvError, SendError};

/// The Reader Part of a simple Message-Stream that
/// can be used to quickly pass messages around
pub struct StreamReader<T> {
    reader: tokio::sync::mpsc::Receiver<T>,
}

impl<T> StreamReader<T>
where
    T: Send,
{
    fn new(rx: tokio::sync::mpsc::Receiver<T>) -> Self {
        Self { reader: rx }
    }

    /// Receives data that has been queued up
    pub async fn recv(&mut self) -> Result<T, RecvError> {
        match self.reader.recv().await {
            Some(s) => Ok(s),
            None => Err(RecvError::Closed),
        }
    }
}

/// The Writer Part of a simple Message-Stream that
/// can be used to quickly pass messages around
#[derive(Debug)]
pub struct StreamWriter<T> {
    sender: tokio::sync::mpsc::Sender<T>,
}

impl<T> Clone for StreamWriter<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl<T> StreamWriter<T>
where
    T: Send,
{
    /// Creates a new Writer
    fn new(tx: tokio::sync::mpsc::Sender<T>) -> Self {
        Self { sender: tx }
    }

    /// Adds the Data to the Queue/Stream
    pub async fn send(&self, data: T) -> Result<(), SendError> {
        match self.sender.send(data).await {
            Ok(_) => Ok(()),
            Err(e) => Err(SendError::from(e)),
        }
    }

    /// Blocks until the message could be send
    ///
    /// Can only be called outside of the Tokio runtime
    pub fn blocking_send(&self, data: T) -> Result<(), SendError> {
        match self.sender.blocking_send(data) {
            Ok(_) => Ok(()),
            Err(e) => Err(SendError::from(e)),
        }
    }
}

/// Creates a new Stream pair
pub fn stream<T>() -> (StreamWriter<T>, StreamReader<T>)
where
    T: Send,
{
    let (tx, rx) = tokio::sync::mpsc::channel(25);
    (StreamWriter::new(tx), StreamReader::new(rx))
}
