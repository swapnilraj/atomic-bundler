//! Configuration validation utilities

use crate::schema::Config;
use alloy::primitives::U256;
use types::Result;

/// Configuration validator
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validate complete configuration
    pub fn validate(config: &Config) -> Result<ValidationReport> {
        let mut report = ValidationReport::new();

        // Validate network configuration
        Self::validate_network(config, &mut report);

        // Validate payment configuration
        Self::validate_payment(config, &mut report);

        // Validate builders configuration
        Self::validate_builders(config, &mut report);

        // Validate limits configuration
        Self::validate_limits(config, &mut report);

        // Validate server configuration
        Self::validate_server(config, &mut report);

        // Validate database configuration
        Self::validate_database(config, &mut report);

        // Validate logging configuration
        Self::validate_logging(config, &mut report);

        // Validate metrics configuration
        Self::validate_metrics(config, &mut report);

        // Validate security configuration
        Self::validate_security(config, &mut report);

        // Cross-validation checks
        Self::validate_cross_dependencies(config, &mut report);

        Ok(report)
    }

    fn validate_network(config: &Config, report: &mut ValidationReport) {
        if config.network.network.is_empty() {
            report.add_error("network.network", "Network name cannot be empty");
        }

        let valid_networks = ["mainnet", "goerli", "sepolia", "holesky", "testnet"];
        if !valid_networks.contains(&config.network.network.as_str()) {
            report.add_warning(
                "network.network",
                &format!("Unknown network '{}'. Supported networks: {:?}", config.network.network, valid_networks)
            );
        }

        if let Some(chain_id) = config.network.chain_id {
            match config.network.network.as_str() {
                "mainnet" if chain_id != 1 => {
                    report.add_warning("network.chain_id", "Chain ID 1 expected for mainnet");
                }
                "goerli" if chain_id != 5 => {
                    report.add_warning("network.chain_id", "Chain ID 5 expected for goerli");
                }
                "sepolia" if chain_id != 11155111 => {
                    report.add_warning("network.chain_id", "Chain ID 11155111 expected for sepolia");
                }
                _ => {}
            }
        }

        if let Some(ref rpc_url) = config.network.rpc_url {
            if !rpc_url.starts_with("http://") && !rpc_url.starts_with("https://") && !rpc_url.starts_with("ws://") && !rpc_url.starts_with("wss://") {
                report.add_error("network.rpc_url", "RPC URL must start with http://, https://, ws://, or wss://");
            }
        }
    }

    fn validate_payment(config: &Config, report: &mut ValidationReport) {
        if config.payment.k1 < 0.0 {
            report.add_error("payment.k1", "k1 coefficient cannot be negative");
        }

        if config.payment.k1 > 10.0 {
            report.add_warning("payment.k1", "k1 coefficient is very high, this may result in expensive payments");
        }

        if config.payment.k2 == U256::ZERO && matches!(config.payment.formula, types::PaymentFormula::Flat) {
            report.add_warning("payment.k2", "k2 is zero for flat payment formula, payments will be zero");
        }

        if config.payment.max_amount_wei == U256::ZERO {
            report.add_error("payment.max_amount_wei", "Maximum payment amount cannot be zero");
        }

        if config.payment.per_bundle_cap_wei > config.payment.max_amount_wei {
            report.add_error("payment", "Per-bundle cap cannot be greater than maximum payment amount");
        }
    }

    fn validate_builders(config: &Config, report: &mut ValidationReport) {
        if config.builders.is_empty() {
            report.add_error("builders", "At least one builder must be configured");
            return;
        }

        let enabled_count = config.builders.iter().filter(|b| b.enabled).count();
        if enabled_count == 0 {
            report.add_error("builders", "At least one builder must be enabled");
        }

        if enabled_count == 1 {
            report.add_warning("builders", "Only one builder is enabled, consider enabling multiple builders for redundancy");
        }

        let mut names = std::collections::HashSet::new();
        for builder in &config.builders {
            // Check for duplicate names
            if !names.insert(&builder.name) {
                report.add_error("builders", &format!("Duplicate builder name: {}", builder.name));
            }

            // Validate individual builder
            Self::validate_builder(builder, report);
        }
    }

    fn validate_builder(builder: &crate::schema::BuilderConfig, report: &mut ValidationReport) {
        if builder.name.is_empty() {
            report.add_error("builders.name", "Builder name cannot be empty");
        }

        if builder.relay_url.is_empty() {
            report.add_error("builders.relay_url", &format!("Relay URL cannot be empty for builder {}", builder.name));
        } else if !builder.relay_url.starts_with("https://") {
            report.add_warning("builders.relay_url", &format!("Relay URL for {} should use HTTPS", builder.name));
        }

        if !types::utils::is_valid_address(&builder.payment_address) {
            report.add_error("builders.payment_address", &format!("Invalid payment address for builder {}", builder.name));
        }

        if builder.timeout_seconds == 0 {
            report.add_error("builders.timeout_seconds", &format!("Timeout cannot be zero for builder {}", builder.name));
        } else if builder.timeout_seconds > 300 {
            report.add_warning("builders.timeout_seconds", &format!("Timeout is very high for builder {} ({}s)", builder.name, builder.timeout_seconds));
        }

        if builder.max_retries > 10 {
            report.add_warning("builders.max_retries", &format!("Max retries is very high for builder {} ({})", builder.name, builder.max_retries));
        }

        if builder.health_check_interval_seconds < 10 {
            report.add_warning("builders.health_check_interval_seconds", &format!("Health check interval is very low for builder {} ({}s)", builder.name, builder.health_check_interval_seconds));
        }
    }

    fn validate_limits(config: &Config, report: &mut ValidationReport) {
        match config.parse_limits() {
            Ok(limits) => {
                if limits.per_bundle_cap_wei > limits.daily_cap_wei {
                    report.add_error("limits", "Per-bundle cap cannot be greater than daily cap");
                }

                if let Some(monthly_cap) = limits.monthly_cap_wei {
                    if limits.daily_cap_wei * U256::from(31) > monthly_cap {
                        report.add_warning("limits", "Daily cap * 31 is greater than monthly cap");
                    }
                }

                // Check for reasonable limits
                let one_eth = U256::from(10u64.pow(18));
                if limits.per_bundle_cap_wei > one_eth {
                    report.add_warning("limits.per_bundle_cap_wei", "Per-bundle cap is greater than 1 ETH");
                }

                if limits.daily_cap_wei > one_eth * U256::from(10) {
                    report.add_warning("limits.daily_cap_wei", "Daily cap is greater than 10 ETH");
                }
            }
            Err(e) => {
                report.add_error("limits", &format!("Failed to parse limits: {}", e));
            }
        }
    }

    fn validate_server(config: &Config, report: &mut ValidationReport) {
        if config.server.port == 0 {
            report.add_error("server.port", "Server port cannot be 0");
        } else if config.server.port < 1024 {
            report.add_warning("server.port", "Server port is below 1024, may require elevated privileges");
        }

        if config.server.request_timeout_seconds == 0 {
            report.add_error("server.request_timeout_seconds", "Request timeout cannot be 0");
        } else if config.server.request_timeout_seconds > 300 {
            report.add_warning("server.request_timeout_seconds", "Request timeout is very high");
        }

        if config.server.max_body_size == 0 {
            report.add_error("server.max_body_size", "Max body size cannot be 0");
        } else if config.server.max_body_size > 10 * 1024 * 1024 {
            report.add_warning("server.max_body_size", "Max body size is greater than 10MB");
        }

        if config.server.host.is_empty() {
            report.add_error("server.host", "Server host cannot be empty");
        }
    }

    fn validate_database(config: &Config, report: &mut ValidationReport) {
        if config.database.url.is_empty() {
            report.add_error("database.url", "Database URL cannot be empty");
        }

        if !config.database.url.starts_with("sqlite:") {
            report.add_warning("database.url", "Only SQLite is currently supported");
        }

        if config.database.max_connections == 0 {
            report.add_error("database.max_connections", "Max connections cannot be 0");
        } else if config.database.max_connections > 100 {
            report.add_warning("database.max_connections", "Max connections is very high");
        }

        if config.database.connection_timeout_seconds == 0 {
            report.add_error("database.connection_timeout_seconds", "Connection timeout cannot be 0");
        }
    }

    fn validate_logging(config: &Config, report: &mut ValidationReport) {
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&config.logging.level.as_str()) {
            report.add_error("logging.level", &format!("Invalid log level: {}. Valid levels: {:?}", config.logging.level, valid_levels));
        }

        let valid_formats = ["json", "pretty"];
        if !valid_formats.contains(&config.logging.format.as_str()) {
            report.add_error("logging.format", &format!("Invalid log format: {}. Valid formats: {:?}", config.logging.format, valid_formats));
        }

        if config.logging.level == "trace" || config.logging.level == "debug" {
            report.add_warning("logging.level", "Debug/trace logging may impact performance in production");
        }

        if let Some(ref file_path) = config.logging.file_path {
            if let Some(parent) = std::path::Path::new(file_path).parent() {
                if !parent.exists() {
                    report.add_warning("logging.file_path", "Log file directory does not exist");
                }
            }
        }
    }

    fn validate_metrics(config: &Config, report: &mut ValidationReport) {
        if config.metrics.port == 0 {
            report.add_error("metrics.port", "Metrics port cannot be 0");
        } else if config.metrics.port < 1024 {
            report.add_warning("metrics.port", "Metrics port is below 1024, may require elevated privileges");
        }

        if config.metrics.namespace.is_empty() {
            report.add_error("metrics.namespace", "Metrics namespace cannot be empty");
        }

        if config.metrics.collection_interval_seconds == 0 {
            report.add_error("metrics.collection_interval_seconds", "Metrics collection interval cannot be 0");
        } else if config.metrics.collection_interval_seconds < 5 {
            report.add_warning("metrics.collection_interval_seconds", "Metrics collection interval is very low, may impact performance");
        }
    }

    fn validate_security(config: &Config, report: &mut ValidationReport) {
        if config.security.admin_api_key.is_none() {
            report.add_warning("security.admin_api_key", "No admin API key configured, admin endpoints will be unprotected");
        } else if let Some(ref key) = config.security.admin_api_key {
            if key.len() < 16 {
                report.add_warning("security.admin_api_key", "Admin API key is short, consider using a longer key");
            }
        }

        if config.security.rate_limit_per_minute == 0 {
            report.add_error("security.rate_limit_per_minute", "Rate limit per minute cannot be 0");
        } else if config.security.rate_limit_per_minute > 10000 {
            report.add_warning("security.rate_limit_per_minute", "Rate limit per minute is very high");
        }

        if config.security.rate_limit_burst == 0 {
            report.add_error("security.rate_limit_burst", "Rate limit burst cannot be 0");
        } else if config.security.rate_limit_burst > config.security.rate_limit_per_minute {
            report.add_warning("security.rate_limit_burst", "Rate limit burst is greater than per-minute limit");
        }
    }

    fn validate_cross_dependencies(config: &Config, report: &mut ValidationReport) {
        // Check for port conflicts
        if config.server.port == config.metrics.port {
            report.add_error("ports", "Server port and metrics port cannot be the same");
        }

        // Check targets vs builders
        if config.targets.blocks_ahead == 0 {
            report.add_error("targets.blocks_ahead", "Blocks ahead cannot be 0");
        } else if config.targets.blocks_ahead > 10 {
            report.add_warning("targets.blocks_ahead", "Targeting many blocks ahead may reduce inclusion probability");
        }

        if config.targets.resubmit_max == 0 {
            report.add_error("targets.resubmit_max", "Resubmit max cannot be 0");
        } else if config.targets.resubmit_max > 10 {
            report.add_warning("targets.resubmit_max", "High resubmit max may cause excessive relay load");
        }

        // Check bundle expiry
        if config.targets.bundle_expiry_seconds < 60 {
            report.add_warning("targets.bundle_expiry_seconds", "Bundle expiry is very short (< 1 minute)");
        } else if config.targets.bundle_expiry_seconds > 3600 {
            report.add_warning("targets.bundle_expiry_seconds", "Bundle expiry is very long (> 1 hour)");
        }
    }
}

/// Validation report containing errors and warnings
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub errors: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
}

/// A validation issue (error or warning)
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub field: String,
    pub message: String,
}

impl ValidationReport {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn add_error(&mut self, field: &str, message: &str) {
        self.errors.push(ValidationIssue {
            field: field.to_string(),
            message: message.to_string(),
        });
    }

    pub fn add_warning(&mut self, field: &str, message: &str) {
        self.warnings.push(ValidationIssue {
            field: field.to_string(),
            message: message.to_string(),
        });
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn is_valid(&self) -> bool {
        !self.has_errors()
    }

    pub fn summary(&self) -> String {
        format!("Validation: {} errors, {} warnings", self.errors.len(), self.warnings.len())
    }
}

impl Default for ValidationReport {
    fn default() -> Self {
        Self::new()
    }
}
