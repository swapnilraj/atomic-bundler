//! Shared types for the Atomic Bundler system
//!
//! This crate contains all the shared domain types used across the atomic bundler
//! middleware components.

pub mod bundle;
pub mod error;
pub mod payment;
pub mod relay;
pub mod utils;

// Re-export commonly used types
pub use bundle::*;
pub use error::{AtomicBundlerError, Result, TransactionError, PaymentError, DatabaseError, ConfigError};
pub use payment::*;
pub use relay::{BuilderRelay, RelayBundleRequest, RelayBundleResponse, RelayHealth, RelayHealthCheck, RelayError, RelayResult};
