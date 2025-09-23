//! Individual relay client implementation

use reqwest::Client;
use std::time::Duration;
use tokio::time::timeout;
use types::{
    BuilderRelay, RelayBundleRequest, RelayBundleResponse, RelayResult, Result,
};
use serde_json::Value;
use uuid::Uuid;

/// HTTP client for a single relay
#[derive(Debug, Clone)]
pub struct RelayClient {
    relay: BuilderRelay,
    http_client: Client,
}

impl RelayClient {
    /// Create a new relay client
    pub fn new(relay: BuilderRelay) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(relay.timeout_seconds))
            .user_agent("atomic-bundler/0.1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            relay,
            http_client,
        }
    }

    /// Submit a bundle to the relay
    pub async fn submit_bundle(
        &self,
        transactions: Vec<String>,
        target_block: u64,
    ) -> Result<String> {
        let request_id = self.generate_request_id();
        let request = RelayBundleRequest::new(request_id, transactions, target_block);

        tracing::info!(
            relay = %self.relay.name,
            target_block = target_block,
            tx_count = request.params[0].txs.len(),
            "Submitting bundle to relay"
        );

        let response = timeout(
            Duration::from_secs(self.relay.timeout_seconds),
            self.http_client
                .post(&self.relay.relay_url)
                .json(&request)
                .send(),
        )
        .await
        .map_err(|_| types::error::RelayError::ConnectionTimeout {
            relay: self.relay.name.clone(),
        })?
        .map_err(|e| types::error::RelayError::HttpError {
            relay: self.relay.name.clone(),
            status: e.status().map(|s| s.as_u16()).unwrap_or(0),
        })?;

        if !response.status().is_success() {
            return Err(types::error::RelayError::HttpError {
                relay: self.relay.name.clone(),
                status: response.status().as_u16(),
            }
            .into());
        }

        let raw_text = response.text().await.map_err(|e| types::error::RelayError::InvalidResponse {
            relay: self.relay.name.clone(),
            message: format!("error reading response body: {}", e),
        })?;

        match parse_bundle_submit_response(&self.relay.name, &raw_text) {
            Ok(hash) => {
                tracing::info!(relay = %self.relay.name, bundle_hash = %hash, "Bundle submitted");
                Ok(hash)
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Perform health check on the relay
    pub async fn health_check(&self) -> Result<Duration> {
        let start = std::time::Instant::now();

        // Simple JSON-RPC call to check connectivity
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": self.generate_request_id(),
            "method": "eth_blockNumber",
            "params": []
        });

        let response = timeout(
            Duration::from_secs(10), // Shorter timeout for health checks
            self.http_client
                .post(&self.relay.relay_url)
                .json(&request)
                .send(),
        )
        .await
        .map_err(|_| types::error::RelayError::ConnectionTimeout {
            relay: self.relay.name.clone(),
        })?
        .map_err(|e| types::error::RelayError::HttpError {
            relay: self.relay.name.clone(),
            status: e.status().map(|s| s.as_u16()).unwrap_or(0),
        })?;

        let elapsed = start.elapsed();

        if response.status().is_success() {
            Ok(elapsed)
        } else {
            Err(types::error::RelayError::HttpError {
                relay: self.relay.name.clone(),
                status: response.status().as_u16(),
            }
            .into())
        }
    }

    /// Get relay configuration
    pub fn relay(&self) -> &BuilderRelay {
        &self.relay
    }

    /// Generate a unique request ID
    fn generate_request_id(&self) -> u64 {
        // Use timestamp and random component for uniqueness
        let uuid = Uuid::new_v4();
        let bytes = uuid.as_bytes();
        u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ])
    }
}

/// Parse builder response into bundle hash with robust fallbacks
fn parse_bundle_submit_response(relay_name: &str, raw_text: &str) -> std::result::Result<String, types::error::RelayError> {
    // 1) Try strict schema
    if let Ok(resp) = serde_json::from_str::<RelayBundleResponse>(raw_text) {
        return match resp.result {
            RelayResult::Success { result } => Ok(result),
            RelayResult::Error { error } => Err(types::error::RelayError::BundleRejected {
                relay: relay_name.to_string(),
                reason: error.message,
            }),
        };
    }

    // 2) Loose parsing
    let value: Value = serde_json::from_str(raw_text).map_err(|e| types::error::RelayError::InvalidResponse {
        relay: relay_name.to_string(),
        message: format!("invalid JSON response: {} | raw: {}", e, raw_text),
    })?;

    // { "result": "0x..." }
    if let Some(result) = value.get("result").and_then(|v| v.as_str()) {
        return Ok(result.to_string());
    }
    // { "result": { "bundleHash": "0x..." } }
    if let Some(result) = value.get("result").and_then(|r| r.get("bundleHash")).and_then(|v| v.as_str()) {
        return Ok(result.to_string());
    }

    // error path
    let (code, message) = if let Some(err) = value.get("error") {
        (
            err.get("code").and_then(|c| c.as_i64()).unwrap_or(0) as i32,
            err.get("message").and_then(|m| m.as_str()).unwrap_or("unknown error").to_string(),
        )
    } else {
        (
            value.get("code").and_then(|c| c.as_i64()).unwrap_or(0) as i32,
            value.get("message").and_then(|m| m.as_str()).unwrap_or("invalid response").to_string(),
        )
    };

    Err(types::error::RelayError::InvalidResponse {
        relay: relay_name.to_string(),
        message: format!("unexpected response (code {}): {} | raw: {}", code, message, raw_text),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::Address;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    #[tokio::test]
    async fn test_successful_bundle_submission() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": "0x1234567890abcdef"
            })))
            .mount(&mock_server)
            .await;

        let relay = BuilderRelay {
            name: "test".to_string(),
            relay_url: mock_server.uri(),
            payment_address: Address::ZERO,
            enabled: true,
            timeout_seconds: 30,
            max_retries: 3,
            health_check_interval_seconds: 60,
        };

        let client = RelayClient::new(relay);
        let result = client
            .submit_bundle(vec!["0x123".to_string()], 12345)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "0x1234567890abcdef");
    }

    #[tokio::test]
    async fn test_bundle_submission_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "error": {
                    "code": -32000,
                    "message": "Bundle rejected"
                }
            })))
            .mount(&mock_server)
            .await;

        let relay = BuilderRelay {
            name: "test".to_string(),
            relay_url: mock_server.uri(),
            payment_address: Address::ZERO,
            enabled: true,
            timeout_seconds: 30,
            max_retries: 3,
            health_check_interval_seconds: 60,
        };

        let client = RelayClient::new(relay);
        let result = client
            .submit_bundle(vec!["0x123".to_string()], 12345)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_health_check_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": "0x123456"
            })))
            .mount(&mock_server)
            .await;

        let relay = BuilderRelay {
            name: "test".to_string(),
            relay_url: mock_server.uri(),
            payment_address: Address::ZERO,
            enabled: true,
            timeout_seconds: 30,
            max_retries: 3,
            health_check_interval_seconds: 60,
        };

        let client = RelayClient::new(relay);
        let result = client.health_check().await;

        assert!(result.is_ok());
        assert!(result.unwrap().as_millis() > 0);
    }
}
