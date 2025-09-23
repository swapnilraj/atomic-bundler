//! Payment-related types and structures

use alloy::primitives::{Address, U256};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Payment formula types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PaymentFormula {
    /// Fixed payment amount: payment = k2
    Flat,
    /// Gas-based payment: payment = k1 * gas_used + k2
    Gas,
    /// Base fee-based payment: payment = k1 * gas_used * (base_fee + tip) + k2
    Basefee,
}

/// Payment mode types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PaymentMode {
    /// Direct ETH transfer to builder
    Direct,
    /// ERC-20 permit-based payment (future)
    Permit,
    /// Escrow-based payment (future)
    Escrow,
}

/// Payment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentConfig {
    /// Payment formula to use
    pub formula: PaymentFormula,
    /// Multiplier coefficient (used for gas and basefee formulas)
    pub k1: f64,
    /// Base amount in wei
    pub k2: U256,
    /// Maximum payment amount in wei
    pub max_amount_wei: U256,
    /// Per-bundle payment cap in wei
    pub per_bundle_cap_wei: U256,
    /// Daily spending cap in wei
    pub daily_cap_wei: U256,
}

/// Payment calculation parameters
#[derive(Debug, Clone)]
pub struct PaymentParams {
    /// Gas used by the transaction
    pub gas_used: u64,
    /// Base fee per gas in wei
    pub base_fee_per_gas: U256,
    /// Max priority fee per gas in wei (tip)
    pub max_priority_fee_per_gas: U256,
    /// Payment formula to use
    pub formula: PaymentFormula,
    /// Formula parameters
    pub k1: f64,
    pub k2: U256,
    /// Maximum allowed payment
    pub max_amount: U256,
}

/// Payment calculation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentResult {
    /// Calculated payment amount in wei
    pub amount_wei: U256,
    /// Payment formula used
    pub formula: PaymentFormula,
    /// Gas used in calculation
    pub gas_used: u64,
    /// Base fee used in calculation
    pub base_fee_per_gas: Option<U256>,
    /// Whether the payment was capped
    pub was_capped: bool,
    /// Calculation timestamp
    pub calculated_at: DateTime<Utc>,
}

/// Payment policy for spending limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentPolicy {
    /// Per-bundle spending cap in wei
    pub per_bundle_cap_wei: U256,
    /// Daily spending cap in wei
    pub daily_cap_wei: U256,
    /// Monthly spending cap in wei (optional)
    pub monthly_cap_wei: Option<U256>,
    /// Whether emergency stop is enabled
    pub emergency_stop_enabled: bool,
    /// Emergency stop threshold in wei
    pub emergency_stop_threshold_wei: U256,
}

/// Daily spending tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySpending {
    /// Date for this spending record
    pub date: chrono::NaiveDate,
    /// Total amount spent in wei
    pub total_amount_wei: U256,
    /// Number of bundles processed
    pub bundle_count: u32,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Payment transaction details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentTransaction {
    /// Recipient address (builder payment address)
    pub to: Address,
    /// Payment amount in wei
    pub amount_wei: U256,
    /// Gas limit for the payment transaction
    pub gas_limit: u64,
    /// Gas price for the payment transaction
    pub gas_price: U256,
    /// Transaction data (empty for ETH transfers)
    pub data: Vec<u8>,
    /// Nonce for the payment transaction
    pub nonce: u64,
}

impl PaymentFormula {
    /// Parse payment formula from string
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "flat" => Ok(PaymentFormula::Flat),
            "gas" => Ok(PaymentFormula::Gas),
            "basefee" => Ok(PaymentFormula::Basefee),
            _ => Err(format!("Unknown payment formula: {}", s)),
        }
    }

    /// Convert payment formula to string
    pub fn as_str(&self) -> &'static str {
        match self {
            PaymentFormula::Flat => "flat",
            PaymentFormula::Gas => "gas",
            PaymentFormula::Basefee => "basefee",
        }
    }
}

impl PaymentMode {
    /// Parse payment mode from string
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "direct" => Ok(PaymentMode::Direct),
            "permit" => Ok(PaymentMode::Permit),
            "escrow" => Ok(PaymentMode::Escrow),
            _ => Err(format!("Unknown payment mode: {}", s)),
        }
    }

    /// Convert payment mode to string
    pub fn as_str(&self) -> &'static str {
        match self {
            PaymentMode::Direct => "direct",
            PaymentMode::Permit => "permit",
            PaymentMode::Escrow => "escrow",
        }
    }
}

impl PaymentResult {
    /// Create a new payment result
    pub fn new(
        amount_wei: U256,
        formula: PaymentFormula,
        gas_used: u64,
        base_fee_per_gas: Option<U256>,
        was_capped: bool,
    ) -> Self {
        Self {
            amount_wei,
            formula,
            gas_used,
            base_fee_per_gas,
            was_capped,
            calculated_at: Utc::now(),
        }
    }
}

impl Default for PaymentFormula {
    fn default() -> Self {
        PaymentFormula::Basefee
    }
}

impl Default for PaymentMode {
    fn default() -> Self {
        PaymentMode::Direct
    }
}

impl Default for PaymentConfig {
    fn default() -> Self {
        Self {
            formula: PaymentFormula::Basefee,
            k1: 1.0,
            k2: U256::from(200_000_000_000_000u64), // 0.0002 ETH
            max_amount_wei: U256::from(500_000_000_000_000u64), // 0.0005 ETH
            per_bundle_cap_wei: U256::from(2_000_000_000_000_000u64), // 0.002 ETH
            daily_cap_wei: U256::from(500_000_000_000_000_000u64), // 0.5 ETH
        }
    }
}

impl Default for PaymentPolicy {
    fn default() -> Self {
        Self {
            per_bundle_cap_wei: U256::from(2_000_000_000_000_000u64), // 0.002 ETH
            daily_cap_wei: U256::from(500_000_000_000_000_000u64), // 0.5 ETH
            monthly_cap_wei: None,
            emergency_stop_enabled: true,
            emergency_stop_threshold_wei: U256::from(100_000_000_000_000_000u64), // 0.1 ETH
        }
    }
}
