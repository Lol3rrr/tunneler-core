use crate::{metrics, Destination};

use super::Client;

pub struct Empty;
pub struct BuilderDestination {
    dest: Destination,
}
pub struct BuilderExternalPort {
    prev: BuilderDestination,
    port: u16,
}
pub struct BuilderKey {
    prev: BuilderExternalPort,
    key: Vec<u8>,
}
pub struct BuilderMetrics<M> {
    prev: BuilderKey,
    metrics: M,
}

/// The Builder used to create a new Client in a compile-time checked way
pub struct ClientBuilder<S> {
    state: S,
}

impl ClientBuilder<Empty> {
    /// Creates a new Empty Builder as the starting Point
    pub fn new() -> Self {
        Self { state: Empty {} }
    }

    /// Sets the Destination for the Client
    pub fn destination(self, dest: Destination) -> ClientBuilder<BuilderDestination> {
        ClientBuilder {
            state: BuilderDestination { dest },
        }
    }
}

impl Default for ClientBuilder<Empty> {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientBuilder<BuilderDestination> {
    /// Sets the External Port for the Client
    pub fn external_port(self, port: u16) -> ClientBuilder<BuilderExternalPort> {
        ClientBuilder {
            state: BuilderExternalPort {
                prev: self.state,
                port,
            },
        }
    }
}

impl ClientBuilder<BuilderExternalPort> {
    /// Sets the Key for the Client
    pub fn key(self, key: Vec<u8>) -> ClientBuilder<BuilderKey> {
        ClientBuilder {
            state: BuilderKey {
                prev: self.state,
                key,
            },
        }
    }
}

impl ClientBuilder<BuilderKey> {
    /// Sets the Metrics for the Client
    pub fn metrics<M>(self, metrics: M) -> ClientBuilder<BuilderMetrics<M>> {
        ClientBuilder {
            state: BuilderMetrics {
                prev: self.state,
                metrics,
            },
        }
    }

    /// Uses the builtin Empty-Metrics Collector for the Client
    pub fn empty_metrics(self) -> ClientBuilder<BuilderMetrics<metrics::Empty>> {
        self.metrics(metrics::Empty::new())
    }
}

impl<M> ClientBuilder<BuilderMetrics<M>> {
    /// Actually builds the Client from the Configuration
    pub fn build(self) -> Client<M> {
        Client {
            server_destination: self.state.prev.prev.prev.dest,
            external_port: self.state.prev.prev.port,
            key: self.state.prev.key,
            metrics: std::sync::Arc::new(self.state.metrics),
        }
    }
}
