//! Relay-related types and structures

use alloy::primitives::{Address, TxHash};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Builder relay configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuilderRelay {
    /// Unique name for the builder
    pub name: String,
    /// Relay URL endpoint
    pub relay_url: String,
    /// Builder's payment address
    pub payment_address: Address,
    /// Whether this relay is enabled
    pub enabled: bool,
    /// Connection timeout in seconds
    pub timeout_seconds: u64,
    /// Maximum retries for failed requests
    pub max_retries: u32,
    /// Health check interval in seconds
    pub health_check_interval_seconds: u64,
}

/// Bundle submission request to relay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayBundleRequest {
    /// JSON-RPC version
    pub jsonrpc: String,
    /// Request ID
    pub id: u64,
    /// Method name (eth_sendBundle)
    pub method: String,
    /// Request parameters
    pub params: Vec<RelayBundleParams>,
}

/// Parameters for eth_sendBundle request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayBundleParams {
    /// Array of signed transaction hex strings
    pub txs: Vec<String>,
    /// Target block number (hex)
    #[serde(rename = "blockNumber")]
    pub block_number: String,
    /// Minimum timestamp for inclusion (optional)
    #[serde(rename = "minTimestamp", skip_serializing_if = "Option::is_none")]
    pub min_timestamp: Option<u64>,
    /// Maximum timestamp for inclusion (optional)
    #[serde(rename = "maxTimestamp", skip_serializing_if = "Option::is_none")]
    pub max_timestamp: Option<u64>,
    /// Reverting transaction hashes (optional)
    #[serde(rename = "revertingTxHashes", skip_serializing_if = "Option::is_none")]
    pub reverting_tx_hashes: Option<Vec<TxHash>>,
}

/// Response from relay bundle submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayBundleResponse {
    /// JSON-RPC version
    pub jsonrpc: String,
    /// Request ID
    pub id: u64,
    /// Result (bundle hash) or error
    #[serde(flatten)]
    pub result: RelayResult,
}

/// Relay response result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RelayResult {
    /// Successful response with bundle hash
    Success { result: String },
    /// Error response
    Error { error: RelayError },
}

/// Relay error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayError {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Additional error data
    pub data: Option<serde_json::Value>,
}

/// Health status of a relay
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RelayHealth {
    /// Relay is healthy and responding
    Healthy,
    /// Relay is responding but with issues
    Degraded,
    /// Relay is not responding
    Unhealthy,
    /// Relay health is unknown
    Unknown,
}

/// Relay health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayHealthCheck {
    /// Relay name
    pub name: String,
    /// Current health status
    pub status: RelayHealth,
    /// Response time in milliseconds
    pub response_time_ms: Option<u64>,
    /// Last check timestamp
    pub last_check: DateTime<Utc>,
    /// Error message if unhealthy
    pub error_message: Option<String>,
    /// Number of consecutive failures
    pub consecutive_failures: u32,
}

/// Bundle submission status to a specific relay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelaySubmissionStatus {
    /// Relay name
    pub relay_name: String,
    /// Bundle ID
    pub bundle_id: String,
    /// Submission status
    pub status: SubmissionStatus,
    /// Submission timestamp
    pub submitted_at: Option<DateTime<Utc>>,
    /// Response from relay
    pub response: Option<String>,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Number of retry attempts
    pub retry_count: u32,
    /// Last retry timestamp
    pub last_retry_at: Option<DateTime<Utc>>,
}

/// Status of bundle submission to relay
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SubmissionStatus {
    /// Submission pending
    Pending,
    /// Successfully submitted to relay
    Submitted,
    /// Submission failed
    Failed,
    /// Bundle was included in a block
    Included,
    /// Bundle was rejected by relay
    Rejected,
    /// Submission timed out
    TimedOut,
}

/// Relay metrics and statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayMetrics {
    /// Relay name
    pub name: String,
    /// Total requests sent
    pub total_requests: u64,
    /// Successful responses
    pub successful_responses: u64,
    /// Failed responses
    pub failed_responses: u64,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// Current health status
    pub health_status: RelayHealth,
    /// Last successful request timestamp
    pub last_success_at: Option<DateTime<Utc>>,
    /// Last failure timestamp
    pub last_failure_at: Option<DateTime<Utc>>,
    /// Uptime percentage (last 24 hours)
    pub uptime_percentage: f64,
}

impl RelayBundleRequest {
    /// Create a new bundle request
    pub fn new(id: u64, txs: Vec<String>, block_number: u64) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: "eth_sendBundle".to_string(),
            params: vec![RelayBundleParams {
                txs,
                block_number: format!("0x{:x}", block_number),
                min_timestamp: None,
                max_timestamp: None,
                reverting_tx_hashes: None,
            }],
        }
    }
}

impl RelayHealthCheck {
    /// Create a new health check result
    pub fn new(name: String, status: RelayHealth) -> Self {
        Self {
            name,
            status,
            response_time_ms: None,
            last_check: Utc::now(),
            error_message: None,
            consecutive_failures: 0,
        }
    }

    /// Mark as healthy with response time
    pub fn mark_healthy(&mut self, response_time_ms: u64) {
        self.status = RelayHealth::Healthy;
        self.response_time_ms = Some(response_time_ms);
        self.last_check = Utc::now();
        self.error_message = None;
        self.consecutive_failures = 0;
    }

    /// Mark as unhealthy with error message
    pub fn mark_unhealthy(&mut self, error_message: String) {
        self.status = RelayHealth::Unhealthy;
        self.response_time_ms = None;
        self.last_check = Utc::now();
        self.error_message = Some(error_message);
        self.consecutive_failures += 1;
    }
}

impl Default for BuilderRelay {
    fn default() -> Self {
        Self {
            name: "unknown".to_string(),
            relay_url: "https://relay.example.com".to_string(),
            payment_address: Address::ZERO,
            enabled: true,
            timeout_seconds: 30,
            max_retries: 3,
            health_check_interval_seconds: 60,
        }
    }
}

impl Default for RelayHealth {
    fn default() -> Self {
        RelayHealth::Unknown
    }
}

impl Default for SubmissionStatus {
    fn default() -> Self {
        SubmissionStatus::Pending
    }
}
