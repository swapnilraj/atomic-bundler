//! Payment calculation engine

use alloy::primitives::U256;
use types::{PaymentFormula, PaymentParams, PaymentResult, Result};

/// Payment calculator that implements various payment formulas
#[derive(Debug, Clone)]
pub struct PaymentCalculator;

impl PaymentCalculator {
    /// Create a new payment calculator
    pub fn new() -> Self {
        Self
    }

    /// Calculate payment amount based on the given parameters
    pub fn calculate_payment(&self, params: &PaymentParams) -> Result<PaymentResult> {
        let amount_wei = match params.formula {
            PaymentFormula::Flat => self.calculate_flat(&params)?,
            PaymentFormula::Gas => self.calculate_gas_based(&params)?,
            PaymentFormula::Basefee => self.calculate_basefee_based(&params)?,
        };

        let was_capped = amount_wei > params.max_amount;
        let final_amount = if was_capped {
            params.max_amount
        } else {
            amount_wei
        };

        Ok(PaymentResult::new(
            final_amount,
            params.formula.clone(),
            params.gas_used,
            Some(params.base_fee_per_gas),
            was_capped,
        ))
    }

    /// Calculate flat payment: payment = k2
    fn calculate_flat(&self, params: &PaymentParams) -> Result<U256> {
        Ok(U256::from(200000000000000u64))
        // Ok(params.k2)
    }

    /// Calculate gas-based payment: payment = k1 * gas_used + k2
    fn calculate_gas_based(&self, params: &PaymentParams) -> Result<U256> {
        let gas_component = U256::from(params.gas_used)
            .checked_mul(U256::from((params.k1 * 1e18) as u64))
            .and_then(|v| v.checked_div(U256::from(1e18 as u64)))
            .ok_or_else(|| types::PaymentError::CalculationOverflow)?;

        let total = gas_component
            .checked_add(params.k2)
            .ok_or_else(|| types::PaymentError::CalculationOverflow)?;

        Ok(total)
    }

    /// Calculate base fee-based payment: payment = k1 * gas_used * (base_fee + tip) + k2
    fn calculate_basefee_based(&self, params: &PaymentParams) -> Result<U256> {
        let effective_gas_price = params
            .base_fee_per_gas
            .checked_add(params.max_priority_fee_per_gas)
            .ok_or_else(|| types::PaymentError::CalculationOverflow)?;

        let gas_cost = U256::from(params.gas_used)
            .checked_mul(effective_gas_price)
            .ok_or_else(|| types::PaymentError::CalculationOverflow)?;

        let gas_component = gas_cost
            .checked_mul(U256::from((params.k1 * 1e18) as u64))
            .and_then(|v| v.checked_div(U256::from(1e18 as u64)))
            .ok_or_else(|| types::PaymentError::CalculationOverflow)?;

        let total = gas_component
            .checked_add(params.k2)
            .ok_or_else(|| types::PaymentError::CalculationOverflow)?;

        Ok(total)
    }

    /// Validate payment parameters
    pub fn validate_params(&self, params: &PaymentParams) -> Result<()> {
        if params.gas_used == 0 {
            return Err(types::PaymentError::InvalidParameters(
                "Gas used cannot be zero".to_string(),
            )
            .into());
        }

        if params.k1 < 0.0 {
            return Err(types::PaymentError::InvalidParameters(
                "k1 coefficient cannot be negative".to_string(),
            )
            .into());
        }

        if params.max_amount == U256::ZERO {
            return Err(types::PaymentError::InvalidParameters(
                "Maximum amount cannot be zero".to_string(),
            )
            .into());
        }

        Ok(())
    }
}

impl Default for PaymentCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::PaymentFormula;

    #[test]
    fn test_flat_payment_calculation() {
        let calculator = PaymentCalculator::new();
        let params = PaymentParams {
            gas_used: 21000,
            base_fee_per_gas: U256::from(20_000_000_000u64), // 20 gwei
            max_priority_fee_per_gas: U256::from(1_000_000_000u64), // 1 gwei
            formula: PaymentFormula::Flat,
            k1: 1.0,
            k2: U256::from(100_000_000_000_000u64), // 0.0001 ETH
            max_amount: U256::from(1_000_000_000_000_000u64), // 0.001 ETH
        };

        let result = calculator.calculate_payment(&params).unwrap();
        assert_eq!(result.amount_wei, U256::from(100_000_000_000_000u64));
        assert!(!result.was_capped);
    }

    #[test]
    fn test_gas_based_payment_calculation() {
        let calculator = PaymentCalculator::new();
        let params = PaymentParams {
            gas_used: 21000,
            base_fee_per_gas: U256::from(20_000_000_000u64),
            max_priority_fee_per_gas: U256::from(1_000_000_000u64),
            formula: PaymentFormula::Gas,
            k1: 1.5,
            k2: U256::from(100_000_000_000_000u64),
            max_amount: U256::from(1_000_000_000_000_000u64),
        };

        let result = calculator.calculate_payment(&params).unwrap();
        // Should be k1 * gas_used + k2 = 1.5 * 21000 + 100_000_000_000_000
        assert!(result.amount_wei > params.k2);
        assert!(!result.was_capped);
    }

    #[test]
    fn test_payment_capping() {
        let calculator = PaymentCalculator::new();
        let params = PaymentParams {
            gas_used: 21000,
            base_fee_per_gas: U256::from(20_000_000_000u64),
            max_priority_fee_per_gas: U256::from(1_000_000_000u64),
            formula: PaymentFormula::Flat,
            k1: 1.0,
            k2: U256::from(2_000_000_000_000_000u64), // 0.002 ETH
            max_amount: U256::from(1_000_000_000_000_000u64), // 0.001 ETH (lower cap)
        };

        let result = calculator.calculate_payment(&params).unwrap();
        assert_eq!(result.amount_wei, params.max_amount);
        assert!(result.was_capped);
    }

    #[test]
    fn test_invalid_parameters() {
        let calculator = PaymentCalculator::new();
        let params = PaymentParams {
            gas_used: 0, // Invalid: zero gas
            base_fee_per_gas: U256::from(20_000_000_000u64),
            max_priority_fee_per_gas: U256::from(1_000_000_000u64),
            formula: PaymentFormula::Flat,
            k1: 1.0,
            k2: U256::from(100_000_000_000_000u64),
            max_amount: U256::from(1_000_000_000_000_000u64),
        };

        let result = calculator.validate_params(&params);
        assert!(result.is_err());
    }
}
