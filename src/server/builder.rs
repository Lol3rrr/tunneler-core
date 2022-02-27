use crate::metrics;

use super::{Server, Strategy};

pub struct BuilderEmpty;
pub struct BuilderListenPort {
    port: u32,
}
pub struct BuilderPortStrategy {
    prev: BuilderListenPort,
    strategy: Strategy,
}
pub struct BuilderKey {
    prev: BuilderPortStrategy,
    key: Vec<u8>,
}
pub struct BuilderMetrics<M> {
    prev: BuilderKey,
    metrics: M,
}

/// The Builder used for creating a new Instance of the Server
pub struct ServerBuilder<S> {
    state: S,
}

impl ServerBuilder<BuilderEmpty> {
    /// Creates a new Empty Builder used as the starting Point of constructing a new Server
    pub fn new() -> Self {
        Self {
            state: BuilderEmpty {},
        }
    }

    /// Sets the listen Port for the Server
    ///
    /// This will be the Port where Clients bind to *NOT* the Port for User Connections
    pub fn listen_port(self, port: u32) -> ServerBuilder<BuilderListenPort> {
        ServerBuilder {
            state: BuilderListenPort { port },
        }
    }
}

impl Default for ServerBuilder<BuilderEmpty> {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerBuilder<BuilderListenPort> {
    /// Sets the Port Strategy for the Server
    ///
    /// This will be used to determine which Ports should be opened for User Connections
    pub fn port_strategy(self, strategy: Strategy) -> ServerBuilder<BuilderPortStrategy> {
        ServerBuilder {
            state: BuilderPortStrategy {
                prev: self.state,
                strategy,
            },
        }
    }
}

impl ServerBuilder<BuilderPortStrategy> {
    /// Sets the Key for the Server
    ///
    /// This is used for "authenticating" User
    pub fn key(self, key: Vec<u8>) -> ServerBuilder<BuilderKey> {
        ServerBuilder {
            state: BuilderKey {
                prev: self.state,
                key,
            },
        }
    }
}

impl ServerBuilder<BuilderKey> {
    /// Sets the Metrics for the Server
    pub fn metrics<M>(self, metrics: M) -> ServerBuilder<BuilderMetrics<M>> {
        ServerBuilder {
            state: BuilderMetrics {
                prev: self.state,
                metrics,
            },
        }
    }

    /// Sets the Metrics to the Empty Metrics Collector
    pub fn empty_metrics(self) -> ServerBuilder<BuilderMetrics<metrics::Empty>> {
        self.metrics(metrics::Empty::new())
    }
}

impl<M> ServerBuilder<BuilderMetrics<M>> {
    /// Actually creates the Server based on the Configuration
    pub fn build(self) -> Server<M> {
        Server {
            listen_port: self.state.prev.prev.prev.port,
            port_strategy: self.state.prev.prev.strategy,
            key: self.state.prev.key,
            metrics: std::sync::Arc::new(self.state.metrics),
        }
    }
}
