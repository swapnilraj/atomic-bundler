//! Relay manager for coordinating multiple relays

use crate::{RelayClient, RelayHealthMonitor};
use std::collections::HashMap;
use types::{BuilderRelay, Result};

/// Manager for multiple relay clients
#[derive(Debug)]
pub struct RelayManager {
    clients: HashMap<String, RelayClient>,
    health_monitor: RelayHealthMonitor,
}

impl RelayManager {
    /// Create a new relay manager
    pub fn new(relays: Vec<BuilderRelay>) -> Self {
        let mut clients = HashMap::new();
        
        for relay in &relays {
            if relay.enabled {
                clients.insert(relay.name.clone(), RelayClient::new(relay.clone()));
            }
        }

        let health_monitor = RelayHealthMonitor::new(relays);

        Self {
            clients,
            health_monitor,
        }
    }

    /// Submit bundle to all enabled relays
    pub async fn submit_bundle_to_all(
        &self,
        transactions: Vec<String>,
        target_block: u64,
    ) -> HashMap<String, Result<String>> {
        let mut results = HashMap::new();
        
        for (name, client) in &self.clients {
            let result = client.submit_bundle(transactions.clone(), target_block).await;
            results.insert(name.clone(), result);
        }

        results
    }

    /// Get a specific relay client
    pub fn get_client(&self, relay_name: &str) -> Option<&RelayClient> {
        self.clients.get(relay_name)
    }

    /// Get all relay names
    pub fn relay_names(&self) -> Vec<String> {
        self.clients.keys().cloned().collect()
    }

    /// Get health monitor
    pub fn health_monitor(&self) -> &RelayHealthMonitor {
        &self.health_monitor
    }
}
