//! Configuration schema definitions

use alloy::primitives::{Address, U256};
use serde::{Deserialize, Serialize};
use types::{BuilderRelay, PaymentConfig};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Ethereum network configuration
    pub network: NetworkConfig,
    /// Target block configuration
    pub targets: TargetConfig,
    /// Payment configuration
    pub payment: PaymentConfig,
    /// Spending limits configuration
    pub limits: LimitsConfig,
    /// Builder relay configurations
    pub builders: Vec<BuilderConfig>,
    /// HTTP server configuration
    #[serde(default)]
    pub server: ServerConfig,
    /// Database configuration
    #[serde(default)]
    pub database: DatabaseConfig,
    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,
    /// Metrics configuration
    #[serde(default)]
    pub metrics: MetricsConfig,
    /// Security configuration
    #[serde(default)]
    pub security: SecurityConfig,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Network name (mainnet, goerli, sepolia)
    pub network: String,
    /// Ethereum RPC URL (optional, for simulation)
    pub rpc_url: Option<String>,
    /// Chain ID
    pub chain_id: Option<u64>,
}

/// Target block configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetConfig {
    /// Number of blocks ahead to target
    pub blocks_ahead: u32,
    /// Maximum number of resubmission attempts
    pub resubmit_max: u32,
    /// Bundle expiry time in seconds
    #[serde(default = "default_bundle_expiry_seconds")]
    pub bundle_expiry_seconds: u64,
}

/// Spending limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitsConfig {
    /// Per-bundle spending cap in wei
    pub per_bundle_cap_wei: String,
    /// Daily spending cap in wei
    pub daily_cap_wei: String,
    /// Monthly spending cap in wei (optional)
    pub monthly_cap_wei: Option<String>,
    /// Emergency stop enabled
    #[serde(default = "default_true")]
    pub emergency_stop_enabled: bool,
    /// Emergency stop threshold in wei
    #[serde(default = "default_emergency_threshold")]
    pub emergency_stop_threshold_wei: String,
}

/// Builder configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuilderConfig {
    /// Builder name
    pub name: String,
    /// Relay URL
    pub relay_url: String,
    /// Payment address for this builder
    pub payment_address: String,
    /// Whether this builder is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Connection timeout in seconds
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
    /// Maximum retries
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// Health check interval in seconds
    #[serde(default = "default_health_check_interval")]
    pub health_check_interval_seconds: u64,
}

/// HTTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server host
    #[serde(default = "default_host")]
    pub host: String,
    /// Server port
    #[serde(default = "default_port")]
    pub port: u16,
    /// Request timeout in seconds
    #[serde(default = "default_request_timeout")]
    pub request_timeout_seconds: u64,
    /// Maximum request body size in bytes
    #[serde(default = "default_max_body_size")]
    pub max_body_size: usize,
    /// Enable CORS
    #[serde(default = "default_true")]
    pub cors_enabled: bool,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database URL
    #[serde(default = "default_database_url")]
    pub url: String,
    /// Maximum number of connections
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    /// Connection timeout in seconds
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout_seconds: u64,
    /// Enable WAL mode for SQLite
    #[serde(default = "default_true")]
    pub wal_mode: bool,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    #[serde(default = "default_log_level")]
    pub level: String,
    /// Log format (json, pretty)
    #[serde(default = "default_log_format")]
    pub format: String,
    /// Log file path (optional)
    pub file_path: Option<String>,
    /// Enable request logging
    #[serde(default = "default_true")]
    pub request_logging: bool,
    /// Enable SQL query logging
    #[serde(default = "default_false")]
    pub sql_logging: bool,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics collection
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Metrics server port
    #[serde(default = "default_metrics_port")]
    pub port: u16,
    /// Prometheus namespace
    #[serde(default = "default_metrics_namespace")]
    pub namespace: String,
    /// Metrics collection interval in seconds
    #[serde(default = "default_metrics_interval")]
    pub collection_interval_seconds: u64,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Admin API key
    pub admin_api_key: Option<String>,
    /// Enable rate limiting
    #[serde(default = "default_true")]
    pub rate_limiting_enabled: bool,
    /// Rate limit per minute
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_minute: u32,
    /// Rate limit burst size
    #[serde(default = "default_rate_limit_burst")]
    pub rate_limit_burst: u32,
    /// Enable killswitch
    #[serde(default = "default_true")]
    pub killswitch_enabled: bool,
}

// Default value functions
fn default_bundle_expiry_seconds() -> u64 {
    300 // 5 minutes
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_emergency_threshold() -> String {
    "100000000000000000".to_string() // 0.1 ETH
}

fn default_timeout_seconds() -> u64 {
    30
}

fn default_max_retries() -> u32 {
    3
}

fn default_health_check_interval() -> u64 {
    60
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_request_timeout() -> u64 {
    30
}

fn default_max_body_size() -> usize {
    1024 * 1024 // 1MB
}

fn default_database_url() -> String {
    "sqlite:data/atomic_bundler.db".to_string()
}

fn default_max_connections() -> u32 {
    10
}

fn default_connection_timeout() -> u64 {
    30
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "json".to_string()
}

fn default_metrics_port() -> u16 {
    9090
}

fn default_metrics_namespace() -> String {
    "atomic_bundler".to_string()
}

fn default_metrics_interval() -> u64 {
    30
}

fn default_rate_limit() -> u32 {
    100
}

fn default_rate_limit_burst() -> u32 {
    20
}

impl Config {
    /// Convert builder configs to BuilderRelay instances
    pub fn to_builder_relays(&self) -> Result<Vec<BuilderRelay>, String> {
        let mut relays = Vec::new();
        
        for builder in &self.builders {
            let payment_address = builder.payment_address.parse::<Address>()
                .map_err(|e| format!("Invalid payment address for builder {}: {}", builder.name, e))?;
            
            relays.push(BuilderRelay {
                name: builder.name.clone(),
                relay_url: builder.relay_url.clone(),
                payment_address,
                enabled: builder.enabled,
                timeout_seconds: builder.timeout_seconds,
                max_retries: builder.max_retries,
                health_check_interval_seconds: builder.health_check_interval_seconds,
            });
        }
        
        Ok(relays)
    }

    /// Convert limits config to U256 values
    pub fn parse_limits(&self) -> Result<ParsedLimits, String> {
        let per_bundle_cap_wei = self.limits.per_bundle_cap_wei.parse::<U256>()
            .map_err(|e| format!("Invalid per_bundle_cap_wei: {}", e))?;
        
        let daily_cap_wei = self.limits.daily_cap_wei.parse::<U256>()
            .map_err(|e| format!("Invalid daily_cap_wei: {}", e))?;
        
        let monthly_cap_wei = if let Some(ref monthly) = self.limits.monthly_cap_wei {
            Some(monthly.parse::<U256>()
                .map_err(|e| format!("Invalid monthly_cap_wei: {}", e))?)
        } else {
            None
        };
        
        let emergency_stop_threshold_wei = self.limits.emergency_stop_threshold_wei.parse::<U256>()
            .map_err(|e| format!("Invalid emergency_stop_threshold_wei: {}", e))?;
        
        Ok(ParsedLimits {
            per_bundle_cap_wei,
            daily_cap_wei,
            monthly_cap_wei,
            emergency_stop_enabled: self.limits.emergency_stop_enabled,
            emergency_stop_threshold_wei,
        })
    }
}

/// Parsed limits with U256 values
#[derive(Debug, Clone)]
pub struct ParsedLimits {
    pub per_bundle_cap_wei: U256,
    pub daily_cap_wei: U256,
    pub monthly_cap_wei: Option<U256>,
    pub emergency_stop_enabled: bool,
    pub emergency_stop_threshold_wei: U256,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            network: NetworkConfig {
                network: "mainnet".to_string(),
                rpc_url: None,
                chain_id: Some(1),
            },
            targets: TargetConfig {
                blocks_ahead: 3,
                resubmit_max: 3,
                bundle_expiry_seconds: default_bundle_expiry_seconds(),
            },
            payment: PaymentConfig::default(),
            limits: LimitsConfig {
                per_bundle_cap_wei: "2000000000000000".to_string(), // 0.002 ETH
                daily_cap_wei: "500000000000000000".to_string(), // 0.5 ETH
                monthly_cap_wei: None,
                emergency_stop_enabled: default_true(),
                emergency_stop_threshold_wei: default_emergency_threshold(),
            },
            builders: vec![
                BuilderConfig {
                    name: "flashbots".to_string(),
                    relay_url: "https://relay.flashbots.net".to_string(),
                    payment_address: "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5".to_string(),
                    enabled: true,
                    timeout_seconds: default_timeout_seconds(),
                    max_retries: default_max_retries(),
                    health_check_interval_seconds: default_health_check_interval(),
                },
            ],
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            logging: LoggingConfig::default(),
            metrics: MetricsConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            request_timeout_seconds: default_request_timeout(),
            max_body_size: default_max_body_size(),
            cors_enabled: default_true(),
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: default_database_url(),
            max_connections: default_max_connections(),
            connection_timeout_seconds: default_connection_timeout(),
            wal_mode: default_true(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
            file_path: None,
            request_logging: default_true(),
            sql_logging: default_false(),
        }
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            port: default_metrics_port(),
            namespace: default_metrics_namespace(),
            collection_interval_seconds: default_metrics_interval(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            admin_api_key: None,
            rate_limiting_enabled: default_true(),
            rate_limit_per_minute: default_rate_limit(),
            rate_limit_burst: default_rate_limit_burst(),
            killswitch_enabled: default_true(),
        }
    }
}
