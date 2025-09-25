//! Bundle-related types and structures

use alloy::{
    primitives::{Bytes, TxHash, U256, B256},
    rpc::types::Transaction,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a bundle
pub type BundleId = Uuid;

/// State of a bundle in the processing pipeline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BundleState {
    /// Bundle has been queued for processing
    Queued,
    /// Bundle has been sent to relays
    Sent,
    /// Bundle has been included in a block
    Landed,
    /// Bundle has expired without inclusion
    Expired,
    /// Bundle processing failed
    Failed,
}

/// A complete bundle containing the original transaction and payment transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bundle {
    /// Unique bundle identifier
    pub id: BundleId,
    /// Original user transaction (priority fee = 0)
    pub tx1: Transaction,
    /// Payment transaction to builder (optional, created during processing)
    pub tx2: Option<Transaction>,
    /// Current bundle state
    pub state: BundleState,
    /// Payment amount in wei
    pub payment_amount_wei: U256,
    /// Target block numbers for inclusion
    pub target_blocks: Vec<u64>,
    /// Bundle creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Bundle expiration timestamp
    pub expires_at: DateTime<Utc>,
    /// Block hash if included
    pub block_hash: Option<B256>,
    /// Block number if included
    pub block_number: Option<u64>,
    /// Gas used by the bundle
    pub gas_used: Option<u64>,
}

/// Request to create a new bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleRequest {
    /// Raw signed transaction (EIP-1559 with priority_fee = 0)
    pub tx1: Bytes,
    /// Payment configuration
    pub payment: PaymentRequest,
    /// Optional single target block number for inclusion
    #[serde(default)]
    pub target_block: Option<u64>,
}

/// Payment configuration for a bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRequest {
    /// Payment mode (direct, permit, escrow)
    pub mode: String,
    /// Payment formula (flat, gas, basefee)
    pub formula: String,
    /// Maximum payment amount in wei
    #[serde(rename = "maxAmountWei")]
    pub max_amount_wei: String,
    /// Payment expiry timestamp
    pub expiry: DateTime<Utc>,
}
/// Response for bundle creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleResponse {
    /// Created bundle identifier
    #[serde(rename = "bundleId")]
    pub bundle_id: BundleId,
}

/// Bundle status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleStatus {
    /// Bundle identifier
    #[serde(rename = "bundleId")]
    pub bundle_id: BundleId,
    /// Current bundle state
    pub state: BundleState,
    /// Original transaction hash
    #[serde(rename = "tx1Hash")]
    pub tx1_hash: Option<TxHash>,
    /// Payment transaction hash
    #[serde(rename = "tx2Hash")]
    pub tx2_hash: Option<TxHash>,
    /// Block hash if included
    #[serde(rename = "blockHash")]
    pub block_hash: Option<B256>,
    /// Block number if included
    #[serde(rename = "blockNumber")]
    pub block_number: Option<u64>,
    /// Payment amount in wei
    #[serde(rename = "paymentAmount")]
    pub payment_amount: String,
    /// Bundle creation timestamp
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
    /// Bundle expiration timestamp
    #[serde(rename = "expiresAt")]
    pub expires_at: DateTime<Utc>,
    /// Relay submission information
    pub relays: Vec<RelaySubmissionInfo>,
    /// Additional metrics
    pub metrics: BundleMetrics,
}

/// Information about relay submissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelaySubmissionInfo {
    /// Relay name
    pub name: String,
    /// Submission status
    pub status: String,
    /// Submission timestamp
    #[serde(rename = "submittedAt")]
    pub submitted_at: Option<DateTime<Utc>>,
    /// Response from relay
    pub response: Option<String>,
}

/// Bundle metrics and statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleMetrics {
    /// Number of relays submitted to
    #[serde(rename = "relaysCount")]
    pub relays_count: u32,
    /// Gas used by transactions
    #[serde(rename = "gasUsed")]
    pub gas_used: Option<u64>,
    /// Time from submission to inclusion
    #[serde(rename = "inclusionTimeMs")]
    pub inclusion_time_ms: Option<u64>,
}

impl Bundle {
    /// Create a new bundle from a request
    pub fn new(
        tx1: Transaction,
        payment_amount_wei: U256,
        target_blocks: Vec<u64>,
        expires_at: DateTime<Utc>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            tx1,
            tx2: None,
            state: BundleState::Queued,
            payment_amount_wei,
            target_blocks,
            created_at: now,
            updated_at: now,
            expires_at,
            block_hash: None,
            block_number: None,
            gas_used: None,
        }
    }

    /// Check if the bundle has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Update the bundle state
    pub fn update_state(&mut self, new_state: BundleState) {
        self.state = new_state;
        self.updated_at = Utc::now();
    }

    /// Set the payment transaction
    pub fn set_payment_transaction(&mut self, tx2: Transaction) {
        self.tx2 = Some(tx2);
        self.updated_at = Utc::now();
    }

    /// Mark as landed in a block
    pub fn mark_landed(&mut self, block_hash: B256, block_number: u64, gas_used: u64) {
        self.state = BundleState::Landed;
        self.block_hash = Some(block_hash);
        self.block_number = Some(block_number);
        self.gas_used = Some(gas_used);
        self.updated_at = Utc::now();
    }
}

impl Default for BundleState {
    fn default() -> Self {
        BundleState::Queued
    }
}
