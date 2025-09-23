//! Error types for the Atomic Bundler system

use thiserror::Error;

/// Main error type for the atomic bundler system
#[derive(Error, Debug)]
pub enum AtomicBundlerError {
    /// Configuration related errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// Transaction validation errors
    #[error("Transaction validation error: {0}")]
    TransactionValidation(String),

    /// Payment calculation errors
    #[error("Payment calculation error: {0}")]
    PaymentCalculation(String),

    /// Bundle processing errors
    #[error("Bundle processing error: {0}")]
    BundleProcessing(String),

    /// Relay communication errors
    #[error("Relay communication error: {relay}: {message}")]
    RelayCommunication { relay: String, message: String },

    /// Database operation errors
    #[error("Database error: {0}")]
    Database(String),

    /// Simulation errors
    #[error("Simulation error: {0}")]
    Simulation(String),

    /// Rate limiting errors
    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    /// Authentication/authorization errors
    #[error("Authentication error: {0}")]
    Authentication(String),

    /// Spending limit errors
    #[error("Spending limit exceeded: {0}")]
    SpendingLimit(String),

    /// Bundle expiration errors
    #[error("Bundle expired: {bundle_id}")]
    BundleExpired { bundle_id: String },

    /// Not found errors
    #[error("Resource not found: {resource}")]
    NotFound { resource: String },

    /// Internal server errors
    #[error("Internal error: {0}")]
    Internal(String),

    /// External service errors
    #[error("External service error: {service}: {message}")]
    ExternalService { service: String, message: String },
}

/// Result type alias for atomic bundler operations
pub type Result<T> = std::result::Result<T, AtomicBundlerError>;

/// Transaction validation specific errors
#[derive(Error, Debug)]
pub enum TransactionError {
    /// Invalid transaction format
    #[error("Invalid transaction format: {0}")]
    InvalidFormat(String),

    /// Non-zero priority fee detected
    #[error("Non-zero priority fee detected: {fee}")]
    NonZeroPriorityFee { fee: String },

    /// Invalid signature
    #[error("Invalid transaction signature")]
    InvalidSignature,

    /// Gas limit too high
    #[error("Gas limit too high: {limit}")]
    GasLimitTooHigh { limit: u64 },

    /// Gas limit too low
    #[error("Gas limit too low: {limit}")]
    GasLimitTooLow { limit: u64 },

    /// Invalid nonce
    #[error("Invalid nonce: {nonce}")]
    InvalidNonce { nonce: u64 },

    /// Insufficient balance
    #[error("Insufficient balance for transaction")]
    InsufficientBalance,
}

/// Payment calculation specific errors
#[derive(Error, Debug)]
pub enum PaymentError {
    /// Unknown payment formula
    #[error("Unknown payment formula: {formula}")]
    UnknownFormula { formula: String },

    /// Payment amount exceeds cap
    #[error("Payment amount {amount} exceeds cap {cap}")]
    ExceedsCap { amount: String, cap: String },

    /// Daily spending limit exceeded
    #[error("Daily spending limit exceeded: {spent}/{limit}")]
    DailyLimitExceeded { spent: String, limit: String },

    /// Invalid payment parameters
    #[error("Invalid payment parameters: {0}")]
    InvalidParameters(String),

    /// Payment calculation overflow
    #[error("Payment calculation overflow")]
    CalculationOverflow,
}

/// Relay communication specific errors
#[derive(Error, Debug)]
pub enum RelayError {
    /// Connection timeout
    #[error("Connection timeout to relay: {relay}")]
    ConnectionTimeout { relay: String },

    /// HTTP error
    #[error("HTTP error from relay {relay}: {status}")]
    HttpError { relay: String, status: u16 },

    /// Invalid response format
    #[error("Invalid response format from relay {relay}: {message}")]
    InvalidResponse { relay: String, message: String },

    /// Bundle rejected by relay
    #[error("Bundle rejected by relay {relay}: {reason}")]
    BundleRejected { relay: String, reason: String },

    /// Relay unavailable
    #[error("Relay unavailable: {relay}")]
    RelayUnavailable { relay: String },

    /// Rate limited by relay
    #[error("Rate limited by relay: {relay}")]
    RateLimited { relay: String },
}

/// Database specific errors
#[derive(Error, Debug)]
pub enum DatabaseError {
    /// Connection failed
    #[error("Database connection failed: {0}")]
    ConnectionFailed(String),

    /// Query execution failed
    #[error("Query execution failed: {0}")]
    QueryFailed(String),

    /// Transaction failed
    #[error("Database transaction failed: {0}")]
    TransactionFailed(String),

    /// Constraint violation
    #[error("Database constraint violation: {0}")]
    ConstraintViolation(String),

    /// Record not found
    #[error("Record not found: {table}")]
    RecordNotFound { table: String },

    /// Migration failed
    #[error("Database migration failed: {0}")]
    MigrationFailed(String),
}

/// Configuration specific errors
#[derive(Error, Debug)]
pub enum ConfigError {
    /// File not found
    #[error("Configuration file not found: {path}")]
    FileNotFound { path: String },

    /// Parse error
    #[error("Configuration parse error: {0}")]
    ParseError(String),

    /// Validation error
    #[error("Configuration validation error: {field}: {message}")]
    ValidationError { field: String, message: String },

    /// Missing required field
    #[error("Missing required configuration field: {field}")]
    MissingField { field: String },

    /// Invalid value
    #[error("Invalid configuration value for {field}: {value}")]
    InvalidValue { field: String, value: String },
}

// Conversion implementations for common error types

impl From<TransactionError> for AtomicBundlerError {
    fn from(err: TransactionError) -> Self {
        AtomicBundlerError::TransactionValidation(err.to_string())
    }
}

impl From<PaymentError> for AtomicBundlerError {
    fn from(err: PaymentError) -> Self {
        AtomicBundlerError::PaymentCalculation(err.to_string())
    }
}

impl From<RelayError> for AtomicBundlerError {
    fn from(err: RelayError) -> Self {
        match err {
            RelayError::ConnectionTimeout { relay } => AtomicBundlerError::RelayCommunication {
                relay,
                message: "Connection timeout".to_string(),
            },
            RelayError::HttpError { relay, status } => AtomicBundlerError::RelayCommunication {
                relay,
                message: format!("HTTP error: {}", status),
            },
            RelayError::InvalidResponse { relay, message } => {
                AtomicBundlerError::RelayCommunication { relay, message }
            }
            RelayError::BundleRejected { relay, reason } => {
                AtomicBundlerError::RelayCommunication {
                    relay,
                    message: format!("Bundle rejected: {}", reason),
                }
            }
            RelayError::RelayUnavailable { relay } => AtomicBundlerError::RelayCommunication {
                relay,
                message: "Relay unavailable".to_string(),
            },
            RelayError::RateLimited { relay } => AtomicBundlerError::RelayCommunication {
                relay,
                message: "Rate limited".to_string(),
            },
        }
    }
}

impl From<DatabaseError> for AtomicBundlerError {
    fn from(err: DatabaseError) -> Self {
        AtomicBundlerError::Database(err.to_string())
    }
}

impl From<ConfigError> for AtomicBundlerError {
    fn from(err: ConfigError) -> Self {
        AtomicBundlerError::Config(err.to_string())
    }
}
