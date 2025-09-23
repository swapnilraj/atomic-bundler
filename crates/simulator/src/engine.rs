//! Simulation engine implementations

use crate::traits::{GasEstimate, SimulationEngine, SimulationResult, ValidationResult};
use alloy::rpc::types::Transaction;
use async_trait::async_trait;
use types::Result;

/// Stub simulation engine for development
#[derive(Debug, Clone)]
pub struct StubSimulationEngine {
    name: String,
}

impl StubSimulationEngine {
    /// Create a new stub simulation engine
    pub fn new() -> Self {
        Self {
            name: "stub".to_string(),
        }
    }
}

#[async_trait]
impl SimulationEngine for StubSimulationEngine {
    async fn simulate_transaction(&self, _tx: &Transaction) -> Result<SimulationResult> {
        // TODO: Implement actual simulation
        Ok(SimulationResult::success(21000))
    }

    async fn simulate_bundle(&self, txs: &[Transaction]) -> Result<Vec<SimulationResult>> {
        let mut results = Vec::new();
        for tx in txs {
            results.push(self.simulate_transaction(tx).await?);
        }
        Ok(results)
    }

    async fn estimate_gas(&self, _tx: &Transaction) -> Result<GasEstimate> {
        // TODO: Implement actual gas estimation
        Ok(GasEstimate {
            gas_limit: 21000,
            gas_price: alloy::primitives::U256::from(20_000_000_000u64), // 20 gwei
            base_fee_per_gas: alloy::primitives::U256::from(15_000_000_000u64), // 15 gwei
            max_priority_fee_per_gas: alloy::primitives::U256::from(2_000_000_000u64), // 2 gwei
        })
    }

    async fn validate_transaction(&self, _tx: &Transaction) -> Result<ValidationResult> {
        // TODO: Implement actual validation
        Ok(ValidationResult::valid())
    }

    async fn is_available(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl Default for StubSimulationEngine {
    fn default() -> Self {
        Self::new()
    }
}
