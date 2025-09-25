//! HTTP API request handlers

use crate::app::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde_json::{json, Value};
use std::sync::Arc;
use types::BundleRequest;
use alloy::primitives::keccak256;
use uuid::Uuid;
use payment::{PaymentCalculator, PaymentTransactionForger};
use alloy::primitives::{Address, U256};
use alloy::providers::{Provider, ProviderBuilder};
use std::str::FromStr;
use types::{PaymentParams, PaymentFormula};
use relay_client;

/// Submit a new bundle for processing
pub async fn submit_bundle(
    State(state): State<Arc<AppState>>,
    Json(request): Json<BundleRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    // Check killswitch
    if state.is_killswitch_active().await {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": "Service temporarily unavailable - killswitch active"
            })),
        ));
    }

    // Minimal happy path:
    // - compute flat payment from config (k2)
    // - forge a signed tx2 via payment::forger
    // - assemble [tx1, tx2] bundle and log
    let bundle_id = Uuid::new_v4();

    // Get all enabled builders
    let enabled_builders: Vec<_> = state.config.builders.iter().filter(|b| b.enabled).collect();
    if enabled_builders.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "No enabled builders configured" })),
        ));
    }

    // tx1 as provided
    let tx1_hex = format!("{}", request.tx1);

    // Get signer key from env (this is still needed for signing)
    let signer_key = std::env::var("PAYMENT_SIGNER_PRIVATE_KEY")
        .map_err(|_| (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "PAYMENT_SIGNER_PRIVATE_KEY missing" }))
        ))?;

    let chain_id = state.config.network.chain_id.unwrap_or(1);

    // Create RPC provider to get current network conditions
    let rpc_url = std::env::var("ETH_RPC_URL")
        .unwrap_or_else(|_| "http://localhost:8545".to_string());
    let provider = ProviderBuilder::new()
        .on_http(rpc_url.parse().map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Invalid RPC URL" }))
        ))?);

    // Get current base fee and suggested max fee from latest block
    let latest_block = provider.get_block_by_number(alloy::rpc::types::BlockNumberOrTag::Latest, false)
        .await
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to get latest block: {}", e) }))
        ))?
        .ok_or_else(|| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Latest block not found" }))
        ))?;

    let base_fee_per_gas = U256::from(
        latest_block.header.base_fee_per_gas
            .unwrap_or(20_000_000_000u64) // 20 gwei fallback
    );

    // Estimate gas for tx1 using simulator helper (decode + eth_estimateGas)
    let estimated_gas_used: u64 = match simulator::estimate_gas_from_raw(&rpc_url, &tx1_hex).await {
        // Add 21_000 to the estimated gas used to account for the tx2
        Ok(g) => g + 21_000u64,
        Err(e) => {
            tracing::warn!(error = %e, "tx1 gas estimation failed; defaulting to 21000");
            21_000u64
        }
    };

    tracing::info!(estimated_gas_used = estimated_gas_used, "Estimated gas used for tx1");

    // Calculate payment using PaymentCalculator to get priority fee
    let calculator = PaymentCalculator::new();
    let payment_params = PaymentParams {
        gas_used: estimated_gas_used,
        base_fee_per_gas,
        max_priority_fee_per_gas: U256::from(0u64), // 0 gwei default, will be calculated
        formula: PaymentFormula::Flat,
        k1: state.config.payment.k1,
        k2: state.config.payment.k2,
        max_amount: U256::from_str(&state.config.payment.max_amount_wei.to_string())
            .unwrap_or(U256::from(500_000_000_000_000_000u64)), // 0.5 ETH fallback
    };

    let payment_result = calculator.calculate_payment(&payment_params)
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Payment calculation failed: {}", e) }))
        ))?;

    let flat_amount_wei = payment_result.amount_wei;

    let max_priority_fee_per_gas: u128 = 0;
    let max_fee_per_gas: u128 = (((base_fee_per_gas * U256::from(3)) / U256::from(2))
        + U256::from(max_priority_fee_per_gas))
        .try_into()
        .unwrap_or(2_000_000_000u128);

    let gas_limit: u64 = 21_000; // Standard ETH transfer

    // Get nonce for payment signer
    let signer_addr = alloy::signers::local::PrivateKeySigner::from_str(&signer_key)
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Invalid signer key format" }))
        ))?
        .address();

    let base_nonce: u64 = provider.get_transaction_count(signer_addr)
        .await
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to get nonce: {}", e) }))
        ))?
        .try_into()
        .unwrap_or(0);

    // Ensure payment signer has enough balance for value + max gas cost
    let signer_balance = provider.get_balance(signer_addr)
        .await
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to get balance: {}", e) }))
        ))?;

    let required_wei = U256::from(gas_limit)
        .checked_mul(U256::from(max_fee_per_gas))
        .unwrap_or(U256::MAX)
        .saturating_add(flat_amount_wei);

    if signer_balance < required_wei {
        tracing::warn!(
            signer = %format!("0x{:x}", signer_addr),
            balance_wei = %signer_balance,
            required_wei = %required_wei,
            gas_limit = gas_limit,
            max_fee_per_gas = max_fee_per_gas,
            payment_wei = %flat_amount_wei,
            "Insufficient balance for tx2 (value + max gas). Consider lowering payment or max fee"
        );
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Insufficient balance for tx2 (value + max gas)",
                "balanceWei": format!("{}", signer_balance),
                "requiredWei": format!("{}", required_wei)
            }))
        ));
    }

    let forger = PaymentTransactionForger::new();
    // Optional single target block accepted at API level
    let requested_target_block = request.target_block;
    
    // Compute tx1 hash for diagnostics (keccak256 of raw signed RLP)
    let tx1_hash = {
        let raw = tx1_hex.trim_start_matches("0x");
        match alloy::hex::decode(raw) {
            Ok(bytes) => format!("0x{}", alloy::hex::encode(keccak256(&bytes))),
            Err(_) => "0x".to_string(),
        }
    };

    // Create a bundle for each enabled builder
    let mut bundles = Vec::new();
    
    for builder in enabled_builders.iter() {
        // Parse builder payment address
        let builder_addr = Address::from_str(builder.payment_address.as_str())
            .map_err(|_| (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("Invalid builder payment address for {}", builder.name) }))
            ))?;

        let (tx2_hex, tx2_hash) = forger
            .forge_flat_transfer_hex(
                builder_addr,
                flat_amount_wei,
                chain_id,
                base_nonce,
                max_fee_per_gas,
                max_priority_fee_per_gas,
                gas_limit,
                &signer_key,
            )
            .await
            .map_err(|e| (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("failed to forge tx2 for {}: {}", builder.name, e) }))
            ))?;

        // Log the tx2 hash for this builder
        tracing::info!(
            builder = %builder.name,
            tx2_hash = %tx2_hash,
            tx2_to = %builder_addr,
            tx2_value_wei = %flat_amount_wei,
            tx1_hash = %tx1_hash,
            "Forged tx2 payment transaction for builder"
        );

        let txs = vec![tx1_hex.clone(), tx2_hex.clone()];
        bundles.push((builder.name.clone(), txs));
    }

    // Submit bundles to relays individually (each builder gets their specific bundle)
    let mut submission_results = Vec::new();
    for (i, (builder_name, txs)) in bundles.iter().enumerate() {
        let builder_config = &enabled_builders[i];
        
        // Create BuilderRelay from BuilderConfig
        let payment_address = Address::from_str(builder_config.payment_address.as_str())
            .map_err(|_| (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("Invalid payment address for builder {}", builder_config.name) }))
            ))?;
            
        let builder_relay = types::BuilderRelay {
            name: builder_config.name.clone(),
            relay_url: builder_config.relay_url.clone(),
            status_url: builder_config.status_url.clone(),
            payment_address,
            enabled: builder_config.enabled,
            timeout_seconds: builder_config.timeout_seconds,
            max_retries: builder_config.max_retries,
            health_check_interval_seconds: builder_config.health_check_interval_seconds,
        };
        
        let relay_client = relay_client::RelayClient::new(builder_relay);
        
        // If API provided a target block, include it; otherwise omit blockNumber
        let chosen_target_opt = requested_target_block;
        tracing::info!(relay = %builder_name, target = ?chosen_target_opt, "Preparing to submit bundle");

        match relay_client.submit_bundle(txs.clone(), chosen_target_opt).await {
            Ok(response) => {
                tracing::info!(
                    bundle_id = %bundle_id,
                    builder = %builder_name,
                    relay_response = %response,
                    "Bundle submitted successfully"
                );
                submission_results.push(json!({
                    "builder": builder_name,
                    "status": "submitted",
                    "response": response
                }));
            }
            Err(e) => {
                tracing::error!(
                    bundle_id = %bundle_id,
                    builder = %builder_name,
                    error = %e,
                    "Bundle submission failed"
                );
                submission_results.push(json!({
                    "builder": builder_name,
                    "status": "failed",
                    "error": e.to_string()
                }));
            }
        }
    }

    tracing::info!(
        bundle_id = %bundle_id,
        builders = ?enabled_builders.iter().map(|b| &b.name).collect::<Vec<_>>(),
        payment_wei = %flat_amount_wei,
        tx1_len = tx1_hex.len(),
        bundles_count = bundles.len(),
        "Created and submitted bundles for all enabled builders"
    );

    Ok((StatusCode::OK, Json(json!({ 
        "bundleId": bundle_id,
        "submissions": submission_results
    }))))
}

/// Get bundle status by ID
pub async fn get_bundle_status(
    State(_state): State<Arc<AppState>>,
    Path(bundle_id): Path<String>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    // TODO: Implement bundle status lookup
    tracing::info!("Bundle status request for ID: {}", bundle_id);
    
    // Validate bundle ID format
    if Uuid::parse_str(&bundle_id).is_err() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Invalid bundle ID format"
            })),
        ));
    }

    // Placeholder response
    Ok((
        StatusCode::OK,
        Json(json!({
            "bundleId": bundle_id,
            "state": "queued",
            "createdAt": "2024-01-01T12:00:00Z",
            "updatedAt": "2024-01-01T12:00:00Z"
        })),
    ))
}

/// Health check endpoint
pub async fn health_check(
    State(state): State<Arc<AppState>>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    // Check database connectivity
    let db_healthy = state.database.health_check().await.is_ok();
    
    let status = if db_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    Ok((
        status,
        Json(json!({
            "status": if db_healthy { "healthy" } else { "unhealthy" },
            "version": env!("CARGO_PKG_VERSION"),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "components": {
                "database": if db_healthy { "healthy" } else { "unhealthy" },
                "killswitch": if state.is_killswitch_active().await { "active" } else { "inactive" }
            }
        })),
    ))
}

/// System status endpoint with more detailed information
pub async fn system_status(
    State(state): State<Arc<AppState>>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let db_healthy = state.database.health_check().await.is_ok();
    let killswitch_active = state.is_killswitch_active().await;
    
    // TODO: Add more status checks (relays, etc.)
    
    Ok((
        StatusCode::OK,
        Json(json!({
            "service": "atomic-bundler",
            "version": env!("CARGO_PKG_VERSION"),
            "status": if db_healthy && !killswitch_active { "operational" } else { "degraded" },
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "components": {
                "database": {
                    "status": if db_healthy { "healthy" } else { "unhealthy" }
                },
                "killswitch": {
                    "active": killswitch_active
                },
                "configuration": {
                    "network": state.config.network.network,
                    "enabled_builders": state.config.builders.iter()
                        .filter(|b| b.enabled)
                        .map(|b| &b.name)
                        .collect::<Vec<_>>()
                }
            }
        })),
    ))
}

/// Reload configuration (admin endpoint)
pub async fn reload_config(
    State(_state): State<Arc<AppState>>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    // TODO: Implement config reloading
    tracing::info!("Configuration reload requested");
    
    Ok((
        StatusCode::OK,
        Json(json!({
            "message": "Configuration reload initiated",
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    ))
}

/// Toggle killswitch (admin endpoint)
pub async fn toggle_killswitch(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let activate = payload
        .get("activate")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    if activate {
        state.activate_killswitch().await;
    } else {
        state.deactivate_killswitch().await;
    }

    Ok((
        StatusCode::OK,
        Json(json!({
            "killswitch": if activate { "activated" } else { "deactivated" },
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    ))
}

/// Admin metrics endpoint
pub async fn admin_metrics(
    State(_state): State<Arc<AppState>>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    // TODO: Implement metrics collection
    Ok((
        StatusCode::OK,
        Json(json!({
            "metrics": {
                "bundles_submitted_total": 0,
                "bundles_landed_total": 0,
                "uptime_seconds": 0
            },
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    ))
}
