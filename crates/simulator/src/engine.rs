//! Simulation engine implementations

use crate::traits::{GasEstimate, SimulationEngine, SimulationResult, ValidationResult};
use alloy::rpc::types::Transaction;
use async_trait::async_trait;
use types::Result;
use alloy::consensus::TxEnvelope;
use alloy::rlp::Decodable;
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::{TransactionInput, TransactionRequest};
use alloy::primitives::{Bytes, TxKind, U256};
use alloy::consensus::Transaction as ConsensusTransaction;

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

/// Estimate gas for a raw signed transaction hex by decoding it and calling eth_estimateGas.
pub async fn estimate_gas_from_raw(rpc_url: &str, raw_tx_hex: &str) -> Result<u64> {
    let raw = raw_tx_hex.trim_start_matches("0x");
    let mut bytes = alloy::hex::decode(raw)
        .map_err(|e| types::AtomicBundlerError::Internal(format!("invalid tx1 hex: {}", e)))?;

    let envelope = TxEnvelope::decode(&mut bytes.as_slice())
        .map_err(|e| types::AtomicBundlerError::Internal(format!("failed to decode tx1: {}", e)))?;

    // Build TransactionRequest from as many fields as possible
    let mut req = TransactionRequest::default();

    match envelope.to() {
        TxKind::Call(addr) => {
            req = req.to(addr);
        }
        TxKind::Create => {}
    }

    let value: U256 = envelope.value();
    if value > U256::from(0u64) {
        req = req.value(value);
    }

    let input_bytes = envelope.input();
    if !input_bytes.is_empty() {
        req = req.input(TransactionInput::from(Bytes::copy_from_slice(input_bytes)));
    }

    // Fee fields / gas config
    let gas_limit = envelope.gas_limit();
    if gas_limit > 0 { req.gas = Some(gas_limit); }

    if let Some(gas_price) = envelope.gas_price() {
        req.gas_price = Some(gas_price);
    }

    let max_fee = envelope.max_fee_per_gas();
    if max_fee > 0 { req.max_fee_per_gas = Some(max_fee); }

    if let Some(prio) = envelope.max_priority_fee_per_gas() {
        req.max_priority_fee_per_gas = Some(prio);
    }

    if let Some(max_blob_fee) = envelope.max_fee_per_blob_gas() {
        req.max_fee_per_blob_gas = Some(max_blob_fee);
    }

    // Access list (EIP-2930 / 1559)
    if let Some(al) = envelope.access_list() {
        req.access_list = Some(al.clone());
    }

    // Blob hashes (EIP-4844)
    if let Some(hashes) = envelope.blob_versioned_hashes() {
        req.blob_versioned_hashes = Some(hashes.to_vec());
    }

    // Authorization list (EIP-7702)
    if let Some(auth) = envelope.authorization_list() {
        req.authorization_list = Some(auth.to_vec());
    }

    // Chain id, nonce, and tx type
    if let Some(chain_id) = envelope.chain_id() {
        req.chain_id = Some(chain_id);
    }
    req.nonce = Some(envelope.nonce());
    req.transaction_type = Some(envelope.ty());

    // Recover signer for `from` if available (requires alloy-consensus feature `k256`)
    #[cfg(feature = "k256")]
    {
        if let Ok(from_addr) = envelope.recover_signer() {
            req.from = Some(from_addr);
        }
    }

    // Trim conflicting keys based on preferred type
    req.trim_conflicting_keys();

    let provider = ProviderBuilder::new()
        .on_http(rpc_url.parse().map_err(|_| types::AtomicBundlerError::Internal("Invalid RPC URL".to_string()))?);

    let gas = provider
        .estimate_gas(&req)
        .await
        .map_err(|e| types::AtomicBundlerError::Internal(format!("eth_estimateGas failed: {}", e)))?;

    Ok(gas.try_into().unwrap_or(21_000u64))
}
