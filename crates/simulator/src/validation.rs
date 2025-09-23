//! Transaction validation implementations

use crate::traits::{TransactionValidator, ValidationResult};
use alloy::rpc::types::Transaction;
use async_trait::async_trait;
use types::Result;

/// Basic transaction validator
#[derive(Debug, Clone)]
pub struct BasicTransactionValidator;

impl BasicTransactionValidator {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TransactionValidator for BasicTransactionValidator {
    async fn validate_format(&self, _tx: &Transaction) -> Result<ValidationResult> {
        // TODO: Implement format validation
        Ok(ValidationResult::valid())
    }

    async fn validate_signature(&self, _tx: &Transaction) -> Result<ValidationResult> {
        // TODO: Implement signature validation
        Ok(ValidationResult::valid())
    }

    async fn validate_nonce(&self, _tx: &Transaction) -> Result<ValidationResult> {
        // TODO: Implement nonce validation
        Ok(ValidationResult::valid())
    }

    async fn validate_gas(&self, _tx: &Transaction) -> Result<ValidationResult> {
        // TODO: Implement gas validation
        Ok(ValidationResult::valid())
    }
}

impl Default for BasicTransactionValidator {
    fn default() -> Self {
        Self::new()
    }
}
