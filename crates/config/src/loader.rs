//! Configuration loader implementation

use crate::schema::Config;
use anyhow::{Context, Result};
use figment::{
    providers::{Env, Format, Yaml},
    Figment,
};
use std::path::Path;
use types::{AtomicBundlerError, ConfigError};

/// Configuration loader that handles YAML files and environment variables
pub struct ConfigLoader;

impl ConfigLoader {
    /// Load configuration from file and environment variables
    pub fn load<P: AsRef<Path>>(config_path: P) -> Result<Config> {
        let config_path = config_path.as_ref();
        
        // Check if config file exists
        if !config_path.exists() {
            return Err(AtomicBundlerError::Config(format!(
                "Configuration file not found: {}",
                config_path.display()
            )).into());
        }

        // Load configuration using Figment
        let config: Config = Figment::new()
            // Start with YAML file
            .merge(Yaml::file(config_path))
            // Override with environment variables (prefixed with ATOMIC_BUNDLER_)
            .merge(Env::prefixed("ATOMIC_BUNDLER_").split("_"))
            // Also support unprefixed environment variables for common settings
            .merge(Env::raw().only(&[
                "RUST_LOG",
                "DATABASE_URL",
                "HTTP_PORT",
                "HTTP_HOST",
                "ADMIN_API_KEY",
            ]))
            .extract()
            .context("Failed to parse configuration")?;

        // Validate the configuration
        Self::validate(&config)?;

        Ok(config)
    }

    /// Load configuration from string (for testing)
    pub fn load_from_str(yaml_content: &str) -> Result<Config> {
        let config: Config = Figment::new()
            .merge(Yaml::string(yaml_content))
            .extract()
            .context("Failed to parse configuration from string")?;

        Self::validate(&config)?;
        Ok(config)
    }

    /// Validate configuration
    fn validate(config: &Config) -> Result<()> {
        // Validate network
        if config.network.network.is_empty() {
            return Err(ConfigError::MissingField {
                field: "network.network".to_string(),
            }.into());
        }

        // Validate that at least one builder is enabled
        let enabled_builders: Vec<_> = config.builders.iter()
            .filter(|b| b.enabled)
            .collect();
        
        if enabled_builders.is_empty() {
            return Err(ConfigError::ValidationError {
                field: "builders".to_string(),
                message: "At least one builder must be enabled".to_string(),
            }.into());
        }

        // Validate builder configurations
        for builder in &config.builders {
            if builder.name.is_empty() {
                return Err(ConfigError::ValidationError {
                    field: "builders.name".to_string(),
                    message: "Builder name cannot be empty".to_string(),
                }.into());
            }

            if builder.relay_url.is_empty() {
                return Err(ConfigError::ValidationError {
                    field: "builders.relay_url".to_string(),
                    message: format!("Relay URL cannot be empty for builder {}", builder.name),
                }.into());
            }

            // Validate relay URL format
            if !builder.relay_url.starts_with("http://") && !builder.relay_url.starts_with("https://") {
                return Err(ConfigError::ValidationError {
                    field: "builders.relay_url".to_string(),
                    message: format!("Invalid relay URL format for builder {}: {}", builder.name, builder.relay_url),
                }.into());
            }

            if builder.payment_address.is_empty() {
                return Err(ConfigError::ValidationError {
                    field: "builders.payment_address".to_string(),
                    message: format!("Payment address cannot be empty for builder {}", builder.name),
                }.into());
            }

            // Validate payment address format
            if !types::utils::is_valid_address(&builder.payment_address) {
                return Err(ConfigError::ValidationError {
                    field: "builders.payment_address".to_string(),
                    message: format!("Invalid payment address format for builder {}: {}", builder.name, builder.payment_address),
                }.into());
            }

            // Validate timeout values
            if builder.timeout_seconds == 0 {
                return Err(ConfigError::ValidationError {
                    field: "builders.timeout_seconds".to_string(),
                    message: format!("Timeout must be greater than 0 for builder {}", builder.name),
                }.into());
            }

            if builder.timeout_seconds > 300 {
                return Err(ConfigError::ValidationError {
                    field: "builders.timeout_seconds".to_string(),
                    message: format!("Timeout too high for builder {} (max 300s)", builder.name),
                }.into());
            }
        }

        // Validate payment configuration
        if config.payment.k1 < 0.0 {
            return Err(ConfigError::ValidationError {
                field: "payment.k1".to_string(),
                message: "k1 coefficient cannot be negative".to_string(),
            }.into());
        }

        // Validate spending limits
        let limits = config.parse_limits()
            .map_err(|e| ConfigError::ValidationError {
                field: "limits".to_string(),
                message: e,
            })?;

        if limits.per_bundle_cap_wei > limits.daily_cap_wei {
            return Err(ConfigError::ValidationError {
                field: "limits".to_string(),
                message: "Per-bundle cap cannot be greater than daily cap".to_string(),
            }.into());
        }

        // Validate server configuration
        if config.server.port == 0 {
            return Err(ConfigError::ValidationError {
                field: "server.port".to_string(),
                message: "Server port cannot be 0".to_string(),
            }.into());
        }

        if config.server.max_body_size == 0 {
            return Err(ConfigError::ValidationError {
                field: "server.max_body_size".to_string(),
                message: "Max body size cannot be 0".to_string(),
            }.into());
        }

        // Validate database configuration
        if config.database.url.is_empty() {
            return Err(ConfigError::ValidationError {
                field: "database.url".to_string(),
                message: "Database URL cannot be empty".to_string(),
            }.into());
        }

        if config.database.max_connections == 0 {
            return Err(ConfigError::ValidationError {
                field: "database.max_connections".to_string(),
                message: "Max connections cannot be 0".to_string(),
            }.into());
        }

        // Validate logging configuration
        let valid_log_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_log_levels.contains(&config.logging.level.as_str()) {
            return Err(ConfigError::ValidationError {
                field: "logging.level".to_string(),
                message: format!("Invalid log level: {}. Valid levels: {:?}", config.logging.level, valid_log_levels),
            }.into());
        }

        let valid_log_formats = ["json", "pretty"];
        if !valid_log_formats.contains(&config.logging.format.as_str()) {
            return Err(ConfigError::ValidationError {
                field: "logging.format".to_string(),
                message: format!("Invalid log format: {}. Valid formats: {:?}", config.logging.format, valid_log_formats),
            }.into());
        }

        // Validate metrics configuration
        if config.metrics.port == 0 {
            return Err(ConfigError::ValidationError {
                field: "metrics.port".to_string(),
                message: "Metrics port cannot be 0".to_string(),
            }.into());
        }

        // Check for port conflicts
        if config.server.port == config.metrics.port {
            return Err(ConfigError::ValidationError {
                field: "ports".to_string(),
                message: "Server port and metrics port cannot be the same".to_string(),
            }.into());
        }

        // Validate security configuration
        if config.security.rate_limit_per_minute == 0 {
            return Err(ConfigError::ValidationError {
                field: "security.rate_limit_per_minute".to_string(),
                message: "Rate limit per minute cannot be 0".to_string(),
            }.into());
        }

        if config.security.rate_limit_burst == 0 {
            return Err(ConfigError::ValidationError {
                field: "security.rate_limit_burst".to_string(),
                message: "Rate limit burst cannot be 0".to_string(),
            }.into());
        }

        Ok(())
    }

    /// Get default configuration
    pub fn default() -> Config {
        Config::default()
    }

    /// Create example configuration file
    pub fn create_example<P: AsRef<Path>>(path: P) -> Result<()> {
        let config = Self::default();
        let yaml_content = serde_yaml::to_string(&config)
            .context("Failed to serialize default configuration")?;
        
        std::fs::write(path.as_ref(), yaml_content)
            .context("Failed to write example configuration file")?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_default_config() {
        let config = ConfigLoader::default();
        assert_eq!(config.network.network, "mainnet");
        assert_eq!(config.server.port, 8080);
        assert!(!config.builders.is_empty());
    }

    #[test]
    fn test_load_from_string() {
        let yaml_content = r#"
network:
  network: "testnet"
  chain_id: 5
targets:
  blocks_ahead: 2
  resubmit_max: 2
payment:
  formula: "flat"
  k1: 1.0
  k2: "100000000000000"
  max_amount_wei: "500000000000000"
  per_bundle_cap_wei: "1000000000000000"
  daily_cap_wei: "100000000000000000"
limits:
  per_bundle_cap_wei: "1000000000000000"
  daily_cap_wei: "100000000000000000"
builders:
  - name: "test_builder"
    relay_url: "https://test.relay.com"
    payment_address: "0x1234567890123456789012345678901234567890"
    enabled: true
"#;

        let config = ConfigLoader::load_from_str(yaml_content).unwrap();
        assert_eq!(config.network.network, "testnet");
        assert_eq!(config.targets.blocks_ahead, 2);
        assert_eq!(config.builders[0].name, "test_builder");
    }

    #[test]
    fn test_validation_errors() {
        // Test empty network
        let yaml_content = r#"
network:
  network: ""
builders: []
"#;
        let result = ConfigLoader::load_from_str(yaml_content);
        assert!(result.is_err());

        // Test no enabled builders
        let yaml_content = r#"
network:
  network: "mainnet"
builders:
  - name: "test"
    relay_url: "https://test.com"
    payment_address: "0x1234567890123456789012345678901234567890"
    enabled: false
"#;
        let result = ConfigLoader::load_from_str(yaml_content);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_example() {
        let temp_file = NamedTempFile::new().unwrap();
        let result = ConfigLoader::create_example(temp_file.path());
        assert!(result.is_ok());

        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        assert!(content.contains("network:"));
        assert!(content.contains("builders:"));
    }
}
