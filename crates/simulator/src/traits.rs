//! Simulation engine traits and interfaces

use alloy::rpc::types::Transaction;
use async_trait::async_trait;
use types::Result;

/// Simulation result containing execution details
#[derive(Debug, Clone)]
pub struct SimulationResult {
    /// Whether the simulation was successful
    pub success: bool,
    /// Gas used by the transaction
    pub gas_used: u64,
    /// Error message if simulation failed
    pub error: Option<String>,
    /// Return data from the transaction
    pub return_data: Option<Vec<u8>>,
    /// State changes caused by the transaction
    pub state_changes: Vec<StateChange>,
}

/// Represents a state change caused by transaction execution
#[derive(Debug, Clone)]
pub struct StateChange {
    /// Address that was modified
    pub address: alloy::primitives::Address,
    /// Storage slot that was modified
    pub slot: alloy::primitives::U256,
    /// Previous value
    pub previous_value: alloy::primitives::U256,
    /// New value
    pub new_value: alloy::primitives::U256,
}

/// Gas estimation result
#[derive(Debug, Clone)]
pub struct GasEstimate {
    /// Estimated gas limit
    pub gas_limit: u64,
    /// Estimated gas price
    pub gas_price: alloy::primitives::U256,
    /// Base fee per gas
    pub base_fee_per_gas: alloy::primitives::U256,
    /// Max priority fee per gas
    pub max_priority_fee_per_gas: alloy::primitives::U256,
}

/// Transaction validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the transaction is valid
    pub is_valid: bool,
    /// List of validation errors
    pub errors: Vec<String>,
    /// List of validation warnings
    pub warnings: Vec<String>,
}

/// Trait for transaction simulation engines
#[async_trait]
pub trait SimulationEngine: Send + Sync {
    /// Simulate a single transaction
    async fn simulate_transaction(&self, tx: &Transaction) -> Result<SimulationResult>;

    /// Simulate multiple transactions as a bundle
    async fn simulate_bundle(&self, txs: &[Transaction]) -> Result<Vec<SimulationResult>>;

    /// Estimate gas for a transaction
    async fn estimate_gas(&self, tx: &Transaction) -> Result<GasEstimate>;

    /// Validate a transaction
    async fn validate_transaction(&self, tx: &Transaction) -> Result<ValidationResult>;

    /// Check if the simulation engine is available
    async fn is_available(&self) -> bool;

    /// Get the name of the simulation engine
    fn name(&self) -> &str;
}

/// Trait for transaction validators
#[async_trait]
pub trait TransactionValidator: Send + Sync {
    /// Validate transaction format and basic properties
    async fn validate_format(&self, tx: &Transaction) -> Result<ValidationResult>;

    /// Validate transaction signature
    async fn validate_signature(&self, tx: &Transaction) -> Result<ValidationResult>;

    /// Validate transaction nonce
    async fn validate_nonce(&self, tx: &Transaction) -> Result<ValidationResult>;

    /// Validate transaction gas parameters
    async fn validate_gas(&self, tx: &Transaction) -> Result<ValidationResult>;

    /// Perform complete validation
    async fn validate_complete(&self, tx: &Transaction) -> Result<ValidationResult> {
        let mut combined_result = ValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        // Run all validations
        let validations = vec![
            self.validate_format(tx).await?,
            self.validate_signature(tx).await?,
            self.validate_nonce(tx).await?,
            self.validate_gas(tx).await?,
        ];

        // Combine results
        for result in validations {
            if !result.is_valid {
                combined_result.is_valid = false;
            }
            combined_result.errors.extend(result.errors);
            combined_result.warnings.extend(result.warnings);
        }

        Ok(combined_result)
    }
}

impl SimulationResult {
    /// Create a successful simulation result
    pub fn success(gas_used: u64) -> Self {
        Self {
            success: true,
            gas_used,
            error: None,
            return_data: None,
            state_changes: Vec::new(),
        }
    }

    /// Create a failed simulation result
    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            gas_used: 0,
            error: Some(error),
            return_data: None,
            state_changes: Vec::new(),
        }
    }

    /// Check if the simulation was successful
    pub fn is_success(&self) -> bool {
        self.success
    }

    /// Get the error message if simulation failed
    pub fn error_message(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

impl ValidationResult {
    /// Create a valid result
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Create an invalid result with errors
    pub fn invalid(errors: Vec<String>) -> Self {
        Self {
            is_valid: false,
            errors,
            warnings: Vec::new(),
        }
    }

    /// Add an error to the result
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
        self.is_valid = false;
    }

    /// Add a warning to the result
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Check if there are any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}
