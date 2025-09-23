//! Relay health monitoring

use std::time::Duration;
use types::{BuilderRelay, RelayHealth, RelayHealthCheck};

/// Health monitor for tracking relay status
#[derive(Debug)]
pub struct RelayHealthMonitor {
    relays: Vec<RelayHealthCheck>,
}

impl RelayHealthMonitor {
    /// Create a new health monitor
    pub fn new(relays: Vec<BuilderRelay>) -> Self {
        let health_checks = relays
            .into_iter()
            .map(|relay| RelayHealthCheck::new(relay.name, RelayHealth::Unknown))
            .collect();

        Self {
            relays: health_checks,
        }
    }

    /// Get health status for all relays
    pub fn get_all_health(&self) -> &[RelayHealthCheck] {
        &self.relays
    }

    /// Update health status for a relay
    pub fn update_health(&mut self, relay_name: &str, _health: RelayHealth, response_time: Option<Duration>) {
        if let Some(check) = self.relays.iter_mut().find(|r| r.name == relay_name) {
            if let Some(duration) = response_time {
                check.mark_healthy(duration.as_millis() as u64);
            } else {
                check.mark_unhealthy("No response".to_string());
            }
        }
    }
}
