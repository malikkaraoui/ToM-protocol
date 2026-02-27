//! tom-relay — ToM relay server (forked from iroh-relay)
//!
//! Phase R7.2: Skeleton only. Full relay copy happens in R7.3.

/// Relay server placeholder.
/// Will contain DERP-like relay logic in R7.3.
pub struct RelayServer;

impl RelayServer {
    /// Create a new relay server (placeholder).
    pub fn new() -> Self {
        Self
    }
}

impl Default for RelayServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relay_server_creation() {
        let _server = RelayServer::new();
    }
}
