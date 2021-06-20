use super::Metrics;

/// An empty Metrics-Collector that does not keep track
/// of any metrics it receives and simply discards them.
///
/// This is mainly intended for when metrics are not desired
/// and as it does nothing, all the metrics calls should resolve
/// to a NOP and be removed entirely
#[derive(Debug, PartialEq)]
pub struct Empty;

impl Empty {
    /// Creates a new empty Instance of the Empty-Metrics-Collector
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for Empty {
    fn default() -> Self {
        Self::new()
    }
}

impl Metrics for Empty {}
