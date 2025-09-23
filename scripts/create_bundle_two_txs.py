#!/usr/bin/env python3
"""
Create two signed EIP-1559 transactions and submit as a bundle to Titan Hoodi:
 - tx1: self-transfer (from TEST_PRIVATE_KEY) for a small amount
 - tx2: payment (0.1 ETH) to Titan coinbase (builder payment address from config.yaml)

Reads env:
 - ETH_RPC_URL: RPC endpoint
 - TEST_PRIVATE_KEY: hex key (0x... or raw) for tx1 sender
 - PAYMENT_SIGNER_PRIVATE_KEY: hex key for tx2 sender (payment signer)
 - TEST_PRIVATE_ADDRESS (optional): address assertion for TEST_PRIVATE_KEY
 - PAYMENT_SIGNER_ADDRESS (optional): address assertion for SIGNER_PRIVATE_KEY

Reads config.yaml for builder relay_url and payment_address.
"""

import os
import json
import yaml
from decimal import Decimal
from dotenv import load_dotenv
from web3 import Web3
from eth_account import Account
import requests
from datetime import datetime, timezone, timedelta


def load_env():
    try:
        load_dotenv()
    except Exception:
        pass


def read_config_builder(path: str):
    with open(path, 'r') as f:
        cfg = yaml.safe_load(f)
    builders = cfg.get('builders', [])
    titan = next((b for b in builders if b.get('name', '').lower() == 'titan' and b.get('enabled')), None)
    if not titan:
        raise RuntimeError('Enabled titan builder not found in config.yaml')
    relay_url = titan['relay_url'].rstrip('/')
    builder_payment_address = titan['payment_address']
    return relay_url, builder_payment_address


def normalize_key(k: str) -> str:
    if k.startswith('0x'):
        return k
    return '0x' + k


def to_wei_eth(amount_eth: str | float) -> int:
    return int(Decimal(str(amount_eth)) * Decimal(10**18))


def sign_tx(
    w3: Web3,
    privkey: str,
    to: str,
    value_wei: int,
    gas_limit: int,
    max_fee_wei: int,
    max_prio_wei: int,
    nonce: int,
) -> tuple[str, str]:
    # Always EIP-1559
    tx = {
        'type': 2,
        'chainId': w3.eth.chain_id,
        'nonce': nonce,
        'to': Web3.to_checksum_address(to),
        'value': value_wei,
        'gas': gas_limit,
        'maxFeePerGas': max_fee_wei,
        'maxPriorityFeePerGas': max_prio_wei,
        'data': b'',
    }
    signed = w3.eth.account.sign_transaction(tx, privkey)
    return signed.raw_transaction.hex(), signed.hash.hex()


def eth_send_bundle(relay_url: str, txs: list[str], block_number: int):
    req = {
        'jsonrpc': '2.0',
        'id': 1,
        'method': 'eth_sendBundle',
        'params': [{
            'txs': txs,
            'blockNumber': hex(block_number),
        }]
    }
    resp = requests.post(relay_url, json=req, timeout=30)
    resp.raise_for_status()
    payload = resp.json()
    # Accept both Titan nested and plain result shapes
    if isinstance(payload.get('result'), dict) and 'bundleHash' in payload['result']:
        return payload['result']['bundleHash']
    if isinstance(payload.get('result'), str):
        return payload['result']
    raise RuntimeError(f"Unexpected builder response: {payload}")


def main():
    load_env()

    rpc_url = os.getenv('ETH_RPC_URL', 'http://localhost:8545')
    test_key = os.getenv('TEST_PRIVATE_KEY')
    signer_key = os.getenv('PAYMENT_SIGNER_PRIVATE_KEY')
    if not test_key or not signer_key:
        raise SystemExit('TEST_PRIVATE_KEY and PAYMENT_SIGNER_PRIVATE_KEY must be set in env')

    # Config
    relay_url, builder_coinbase = read_config_builder(os.getenv('CONFIG_PATH', '../config.yaml'))

    w3 = Web3(Web3.HTTPProvider(rpc_url))
    if not w3.is_connected():
        raise SystemExit(f'Failed to connect to {rpc_url}')

    # Fees and nonces
    latest = w3.eth.get_block('latest')
    base_fee = latest.get('baseFeePerGas', 20_000_000_000)

    max_prio = 1
    max_fee = base_fee + max_prio

    test_key = normalize_key(test_key)
    signer_key = normalize_key(signer_key)

    test_acct = Account.from_key(test_key)
    signer_acct = Account.from_key(signer_key)

    nonce1 = w3.eth.get_transaction_count(test_acct.address)
    nonce2 = w3.eth.get_transaction_count(signer_acct.address)

    # tx1: self transfer 0.001 ETH
    tx1_hex, tx1_hash = sign_tx(
        w3,
        test_key,
        test_acct.address,
        to_wei_eth('0.001'),
        gas_limit=21_000,
        max_fee_wei=max_fee,
        max_prio_wei=max_prio,
        nonce=nonce1,
    )

    # tx2: pay builder coinbase 0.1 ETH
    tx2_hex, tx2_hash = sign_tx(
        w3,
        signer_key,
        builder_coinbase,
        to_wei_eth('0.1'),
        gas_limit=21_000,
        max_fee_wei=max_fee,
        max_prio_wei=max_prio,
        nonce=nonce2,
    )

    # Targets
    blocks_ahead = int(os.getenv('BLOCKS_AHEAD', '3'))
    target_block = latest.number + blocks_ahead

    print('\nTransactions prepared:')
    print(f'  tx1: {tx1_hash} (self-transfer)')
    print(f'  tx2: {tx2_hash} (builder payment to {builder_coinbase})')

    # Option: send directly to RPC (skip bundler)
    if os.getenv('DIRECT_TO_RPC', '').lower() in ('1', 'true', 'yes'):
        print('\nDIRECT_TO_RPC enabled - submitting txs directly to RPC...')
        submit_rpc = os.getenv('SUBMIT_RPC_URL', rpc_url)
        w3_submit = w3 if submit_rpc == rpc_url else Web3(Web3.HTTPProvider(submit_rpc))
        if submit_rpc != rpc_url:
            print(f'  using SUBMIT_RPC_URL: {submit_rpc}')
        try:
            sent_tx1 = w3_submit.eth.send_raw_transaction(Web3.to_bytes(hexstr=tx1_hex)).hex()
            print(f'  sent tx1: {sent_tx1}')
        except Exception as e:
            print(f'  error sending tx1: {e}')
        try:
            sent_tx2 = w3_submit.eth.send_raw_transaction(Web3.to_bytes(hexstr=tx2_hex)).hex()
            print(f'  sent tx2: {sent_tx2}')
        except Exception as e:
            print(f'  error sending tx2: {e}')
        return

    # Submit bundle to Titan
    print(f"\nSubmitting bundle to {relay_url} for block {target_block}...")
    bundle_hash = eth_send_bundle(relay_url, [tx1_hex, tx2_hex], target_block)
    print(f'  bundleHash: {bundle_hash}')

    # Optional: Titan stats
    stats_url = os.getenv('TITAN_STATS_URL', 'https://stats.titanbuilder.xyz')
    expiry_time = datetime.now(timezone.utc) + timedelta(minutes=5)
    print(f'  Titan stats available around: {expiry_time.isoformat()}')

    stats_req = {
        'jsonrpc': '2.0', 'id': 1, 'method': 'titan_getBundleStats', 'params': [ { 'bundleHash': bundle_hash } ]
    }
    try:
        stats_resp = requests.post(stats_url, json=stats_req, timeout=10)
        print(f'  immediate stats probe: HTTP {stats_resp.status_code} -> {stats_resp.text}')
    except Exception as e:
        print(f'  immediate stats probe error: {e}')


if __name__ == '__main__':
    main()


