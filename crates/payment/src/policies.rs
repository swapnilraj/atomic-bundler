//! Payment policies and limit enforcement

use alloy::primitives::U256;
use chrono::Utc;
use types::{DailySpending, PaymentPolicy, PaymentResult, Result};

/// Payment policy enforcer
#[derive(Debug, Clone)]
pub struct PaymentPolicyEnforcer {
    policy: PaymentPolicy,
}

impl PaymentPolicyEnforcer {
    /// Create a new policy enforcer
    pub fn new(policy: PaymentPolicy) -> Self {
        Self { policy }
    }

    /// Check if a payment is allowed under the current policy
    pub async fn check_payment_allowed(
        &self,
        payment_result: &PaymentResult,
        current_daily_spending: &DailySpending,
    ) -> Result<bool> {
        // Check per-bundle cap
        if payment_result.amount_wei > self.policy.per_bundle_cap_wei {
            return Ok(false);
        }

        // Check daily cap
        let new_daily_total = current_daily_spending
            .total_amount_wei
            .checked_add(payment_result.amount_wei)
            .unwrap_or(U256::MAX);

        if new_daily_total > self.policy.daily_cap_wei {
            return Ok(false);
        }

        // Check emergency stop
        if self.policy.emergency_stop_enabled
            && payment_result.amount_wei > self.policy.emergency_stop_threshold_wei
        {
            return Ok(false);
        }

        Ok(true)
    }

    /// Update daily spending record
    pub async fn update_daily_spending(
        &self,
        mut daily_spending: DailySpending,
        payment_amount: U256,
    ) -> Result<DailySpending> {
        daily_spending.total_amount_wei = daily_spending
            .total_amount_wei
            .checked_add(payment_amount)
            .unwrap_or(U256::MAX);
        daily_spending.bundle_count += 1;
        daily_spending.updated_at = Utc::now();

        Ok(daily_spending)
    }

    /// Get or create daily spending record for today
    pub fn get_or_create_daily_spending(&self) -> DailySpending {
        let today = Utc::now().date_naive();
        DailySpending {
            date: today,
            total_amount_wei: U256::ZERO,
            bundle_count: 0,
            updated_at: Utc::now(),
        }
    }

    /// Get the policy
    pub fn policy(&self) -> &PaymentPolicy {
        &self.policy
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::{PaymentFormula, PaymentResult};

    #[tokio::test]
    async fn test_payment_allowed() {
        let policy = PaymentPolicy::default();
        let enforcer = PaymentPolicyEnforcer::new(policy);

        let payment_result = PaymentResult::new(
            U256::from(1_000_000_000_000_000u64), // 0.001 ETH
            PaymentFormula::Flat,
            21000,
            None,
            false,
        );

        let daily_spending = enforcer.get_or_create_daily_spending();

        let allowed = enforcer
            .check_payment_allowed(&payment_result, &daily_spending)
            .await
            .unwrap();

        assert!(allowed);
    }

    #[tokio::test]
    async fn test_payment_exceeds_daily_cap() {
        let policy = PaymentPolicy {
            daily_cap_wei: U256::from(1_000_000_000_000_000u64), // 0.001 ETH cap
            ..PaymentPolicy::default()
        };
        let enforcer = PaymentPolicyEnforcer::new(policy);

        let payment_result = PaymentResult::new(
            U256::from(2_000_000_000_000_000u64), // 0.002 ETH (exceeds cap)
            PaymentFormula::Flat,
            21000,
            None,
            false,
        );

        let daily_spending = enforcer.get_or_create_daily_spending();

        let allowed = enforcer
            .check_payment_allowed(&payment_result, &daily_spending)
            .await
            .unwrap();

        assert!(!allowed);
    }
}
