//! Payment transaction forging

use alloy::consensus::{Signed, TxEip1559, TxEnvelope};
use alloy::eips::eip2718::Encodable2718;
use alloy::network::TxSignerSync;
use alloy::primitives::{Address, Bytes, TxKind, U256, keccak256};
use alloy::signers::local::PrivateKeySigner;
use std::str::FromStr;
use types::{PaymentTransaction, Result};

/// Transaction forger for creating payment transactions
#[derive(Debug, Clone)]
pub struct PaymentTransactionForger;

impl PaymentTransactionForger {
    /// Create a new payment transaction forger
    pub fn new() -> Self {
        Self
    }

    /// Forge a payment transaction
    pub async fn forge_payment_transaction(
        &self,
        recipient: Address,
        amount_wei: U256,
        gas_price: U256,
        nonce: u64,
    ) -> Result<PaymentTransaction> {
        // TODO: Implement actual transaction forging with signing
        Ok(PaymentTransaction {
            to: recipient,
            amount_wei,
            gas_limit: 21000, // Standard ETH transfer
            gas_price,
            data: Vec::new(), // Empty for ETH transfers
            nonce,
        })
    }

    /// Forge and sign an EIP-1559 ETH transfer and return raw signed tx hex.
    pub async fn forge_flat_transfer_hex(
        &self,
        to: Address,
        amount_wei: U256,
        chain_id: u64,
        nonce: u64,
        max_fee_per_gas: u128,
        max_priority_fee_per_gas: u128,
        gas_limit: u64,
        signer_key_hex: &str,
    ) -> Result<String> {
        // Build an EIP-1559 transaction envelope
        let mut tx = TxEip1559 {
            chain_id,
            nonce,
            max_fee_per_gas,
            max_priority_fee_per_gas,
            gas_limit,
            to: TxKind::Call(to),
            value: amount_wei,
            input: Bytes::new(),
            access_list: Default::default(),
        };

        // Sign
        let signer = PrivateKeySigner::from_str(signer_key_hex)
            .map_err(|e| types::AtomicBundlerError::Internal(format!("invalid signer key: {}", e)))?;

        let signature = signer
            .sign_transaction_sync(&mut tx)
            .map_err(|e| types::AtomicBundlerError::Internal(format!("signing failed: {}", e)))?;

        // Calculate the transaction hash for the signed transaction
        let tx_hash = keccak256(alloy::rlp::encode(&tx));
        let signed = Signed::new_unchecked(tx, signature, tx_hash);
        let envelope: TxEnvelope = signed.into();

        let encoded = envelope.encoded_2718();
        Ok(format!("0x{}", alloy::hex::encode(encoded)))
    }
}

impl Default for PaymentTransactionForger {
    fn default() -> Self {
        Self::new()
    }
}
