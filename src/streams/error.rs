/// The Error that could be returned when trying to send something
#[derive(Debug, PartialEq)]
pub enum SendError {
    /// The Streams buffer was already full
    Full,
    /// The Stream was closed, either by shutting it down or dropping all
    /// receivers
    Closed,
}

impl std::fmt::Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            SendError::Full => write!(f, "The Channel is full"),
            SendError::Closed => write!(f, "The Channel has been closed"),
        }
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for SendError {
    fn from(raw: tokio::sync::mpsc::error::SendError<T>) -> SendError {
        let try_error = tokio::sync::mpsc::error::TrySendError::from(raw);
        match try_error {
            tokio::sync::mpsc::error::TrySendError::Full(_) => SendError::Full,
            tokio::sync::mpsc::error::TrySendError::Closed(_) => SendError::Closed,
        }
    }
}

/// The Error that could be returned when trying to read from a stream
#[derive(Debug, PartialEq)]
pub enum RecvError {
    /// The Stream was already closed
    Closed,
}

impl std::fmt::Display for RecvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            RecvError::Closed => write!(f, "The Channel has been closed"),
        }
    }
}
