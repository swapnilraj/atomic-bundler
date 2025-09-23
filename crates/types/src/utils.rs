//! Utility functions and helpers

use alloy::primitives::U256;
use chrono::{DateTime, Utc};

/// Convert U256 to string for JSON serialization
pub fn u256_to_string(value: &U256) -> String {
    value.to_string()
}

/// Parse U256 from string
pub fn string_to_u256(s: &str) -> Result<U256, String> {
    s.parse().map_err(|e| format!("Failed to parse U256: {}", e))
}

/// Convert wei to ETH (as f64)
pub fn wei_to_eth(wei: U256) -> f64 {
    if wei == U256::ZERO {
        return 0.0;
    }
    
    // Convert to f64 with precision loss for display purposes
    // Use string conversion for better precision
    let wei_str = wei.to_string();
    let wei_f64: f64 = wei_str.parse().unwrap_or(0.0);
    wei_f64 / 1e18
}

/// Convert ETH to wei
pub fn eth_to_wei(eth: f64) -> U256 {
    let wei_f64 = eth * 1e18;
    // Convert to string first to avoid precision issues
    let wei_str = format!("{:.0}", wei_f64);
    wei_str.parse().unwrap_or(U256::ZERO)
}

/// Format wei amount for display
pub fn format_wei(wei: U256) -> String {
    if wei == U256::ZERO {
        return "0 wei".to_string();
    }

    let eth_amount = wei_to_eth(wei);
    if eth_amount >= 1.0 {
        format!("{:.6} ETH", eth_amount)
    } else if eth_amount >= 0.001 {
        format!("{:.6} ETH", eth_amount)
    } else {
        format!("{} wei", wei)
    }
}

/// Generate a correlation ID for request tracing
pub fn generate_correlation_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Calculate time difference in milliseconds
pub fn time_diff_ms(start: DateTime<Utc>, end: DateTime<Utc>) -> i64 {
    end.timestamp_millis() - start.timestamp_millis()
}

/// Validate Ethereum address format
pub fn is_valid_address(address: &str) -> bool {
    if !address.starts_with("0x") {
        return false;
    }
    
    if address.len() != 42 {
        return false;
    }
    
    address[2..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Validate transaction hash format
pub fn is_valid_tx_hash(hash: &str) -> bool {
    if !hash.starts_with("0x") {
        return false;
    }
    
    if hash.len() != 66 {
        return false;
    }
    
    hash[2..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Sanitize string for logging (remove sensitive data)
pub fn sanitize_for_logging(s: &str) -> String {
    if s.len() <= 10 {
        return s.to_string();
    }
    
    // Show first 6 and last 4 characters for hashes/addresses
    if s.starts_with("0x") && s.len() > 20 {
        format!("{}...{}", &s[..6], &s[s.len()-4..])
    } else {
        // For other strings, show first 10 characters
        format!("{}...", &s[..10])
    }
}

/// Calculate percentage
pub fn calculate_percentage(part: u64, total: u64) -> f64 {
    if total == 0 {
        return 0.0;
    }
    (part as f64 / total as f64) * 100.0
}

/// Round to specified decimal places
pub fn round_to_decimal_places(value: f64, places: u32) -> f64 {
    let multiplier = 10f64.powi(places as i32);
    (value * multiplier).round() / multiplier
}

/// Check if timestamp is within the last N seconds
pub fn is_recent(timestamp: DateTime<Utc>, seconds: i64) -> bool {
    let now = Utc::now();
    let diff = now.timestamp() - timestamp.timestamp();
    diff <= seconds
}

/// Generate a random delay for jitter (in milliseconds)
pub fn random_jitter_ms(max_ms: u64) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    Utc::now().timestamp_nanos_opt().unwrap_or(0).hash(&mut hasher);
    let hash = hasher.finish();
    
    hash % max_ms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wei_eth_conversion() {
        let one_eth = U256::from(10u64.pow(18));
        assert_eq!(wei_to_eth(one_eth), 1.0);
        assert_eq!(eth_to_wei(1.0), one_eth);
    }

    #[test]
    fn test_address_validation() {
        assert!(is_valid_address("0x1234567890123456789012345678901234567890"));
        assert!(!is_valid_address("1234567890123456789012345678901234567890"));
        assert!(!is_valid_address("0x123"));
        assert!(!is_valid_address("0xGGGG567890123456789012345678901234567890"));
    }

    #[test]
    fn test_tx_hash_validation() {
        assert!(is_valid_tx_hash("0x1234567890123456789012345678901234567890123456789012345678901234"));
        assert!(!is_valid_tx_hash("1234567890123456789012345678901234567890123456789012345678901234"));
        assert!(!is_valid_tx_hash("0x123"));
    }

    #[test]
    fn test_sanitize_for_logging() {
        assert_eq!(
            sanitize_for_logging("0x1234567890123456789012345678901234567890"),
            "0x1234...7890"
        );
        assert_eq!(sanitize_for_logging("short"), "short");
        assert_eq!(sanitize_for_logging("verylongstring"), "verylongst...");
    }

    #[test]
    fn test_percentage_calculation() {
        assert_eq!(calculate_percentage(50, 100), 50.0);
        assert_eq!(calculate_percentage(0, 100), 0.0);
        assert_eq!(calculate_percentage(100, 0), 0.0);
    }
}
