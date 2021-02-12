use crate::objectpool::Guard;

/// The Data enum that is used to store the actual content
/// inside a message
#[derive(Debug, PartialEq)]
pub enum Data<T>
where
    T: Default,
{
    /// Represents Data from a Object-Pool
    Pooled(Guard<T>),
    /// Represents raw Data from a newly allocated
    /// vector
    Raw(T),
}
