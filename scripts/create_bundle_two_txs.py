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
from eth_account.messages import encode_defunct
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
    return signed.rawTransaction.hex(), signed.hash.hex()


def eth_call_bundle(relay_url: str, txs: list[str], block_number: int, auth_headers: dict = None):
    """Simulate a bundle using eth_callBundle"""
    req = {
        'jsonrpc': '2.0',
        'id': 1,
        'method': 'eth_callBundle',
        'params': [{
            'txs': txs,
            'blockNumber': hex(block_number),
            'stateBlockNumber': hex(block_number - 1),  # Use previous block as state
            'timestamp': 0  # Use current timestamp
        }]
    }
    
    headers = {'Content-Type': 'application/json'}
    if auth_headers:
        headers.update(auth_headers)
    
    resp = requests.post(relay_url, headers=headers, json=req, timeout=30)
    resp.raise_for_status()
    payload = resp.json()
    
    if payload.get('error'):
        raise RuntimeError(f"Bundle simulation error: {payload['error']}")
    
    return payload.get('result', {})


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

    # Priority fee defaults to 1 gwei; allow override
    max_prio = max(1, int(os.getenv('PRIORITY_FEE_WEI', '1000000000')))
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
    blocks_ahead = int(os.getenv('BLOCKS_AHEAD', '30'))
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

    # Flashbots mode (Sepolia/Mainnet)
    if os.getenv('FLASHBOTS', '').lower() in ('1', 'true', 'yes'):
        fb_relay = os.getenv('FLASHBOTS_RELAY_URL', 'https://relay-sepolia.flashbots.net')
        payload = {
            'jsonrpc': '2.0',
            'id': 1,
            'method': 'eth_sendBundle',
            'params': [{
                'txs': [tx1_hex, tx2_hex],
                'blockNumber': hex(target_block),
            }]
        }
        # Serialize once and sign EXACTLY what we send
        body_str = json.dumps(payload, separators=(',', ':'), ensure_ascii=False)
        headers = {'Content-Type': 'application/json'}
        auth_key = os.getenv('FLASHBOTS_AUTH_PRIVATE_KEY')
        if auth_key:
            auth_key = normalize_key(auth_key)
            auth_acct = Account.from_key(auth_key)
            # Flashbots requires signing the keccak256 hash of the body as hex string with EIP-191
            # This matches the official flashbots library implementation
            body_hash_hex = Web3.keccak(text=body_str).hex()
            message = encode_defunct(text=body_hash_hex)
            signed_message = auth_acct.sign_message(message)
            sig = signed_message.signature.hex()
            headers['X-Flashbots-Signature'] = f"{auth_acct.address.lower()}:{sig}"

        print(f"\nSimulating bundle with Flashbots: {fb_relay} block {target_block}...")
        
        # First simulate the bundle
        try:
            auth_headers = {}
            if auth_key:
                auth_key = normalize_key(auth_key)
                auth_acct = Account.from_key(auth_key)
                # Create auth headers for simulation
                sim_body = json.dumps({
                    'jsonrpc': '2.0',
                    'id': 1,
                    'method': 'eth_callBundle',
                    'params': [{
                        'txs': [tx1_hex, tx2_hex],
                        'blockNumber': hex(target_block),
                        'stateBlockNumber': hex(target_block - 1),
                        'timestamp': 0
                    }]
                }, separators=(',', ':'), ensure_ascii=False)
                
                body_hash_hex = Web3.keccak(text=sim_body).hex()
                message = encode_defunct(text=body_hash_hex)
                signed_message = auth_acct.sign_message(message)
                sig = signed_message.signature.hex()
                auth_headers['X-Flashbots-Signature'] = f"{auth_acct.address.lower()}:{sig}"
            
            simulation = eth_call_bundle(fb_relay, [tx1_hex, tx2_hex], target_block, auth_headers)
            print(f"  📊 Simulation Results:")
            print(f"    Bundle Hash: {simulation.get('bundleHash', 'N/A')}")
            print(f"    Coinbase Diff: {simulation.get('coinbaseDiff', 'N/A')} wei")
            print(f"    Gas Used: {simulation.get('totalGasUsed', 'N/A')}")
            
            results = simulation.get('results', [])
            for i, result in enumerate(results, 1):
                print(f"    tx{i} - Gas Used: {result.get('gasUsed', 'N/A')}, Gas Price: {result.get('gasPrice', 'N/A')}")
                if result.get('error'):
                    print(f"    tx{i} - ❌ Error: {result['error']}")
                    print(f"    ⚠️  Bundle simulation failed - not submitting")
                    return
                elif result.get('revert'):
                    print(f"    tx{i} - ❌ Revert: {result['revert']}")
                    print(f"    ⚠️  Bundle simulation failed - not submitting")
                    return
            
            print(f"  ✅ Simulation successful - proceeding with submission")
            
        except Exception as e:
            print(f"  ⚠️  Simulation failed: {e}")
            print(f"  📤 Proceeding with submission anyway...")
        
        print(f"\nSubmitting bundle to Flashbots: {fb_relay} block {target_block}...")
        resp = requests.post(fb_relay, headers=headers, data=body_str.encode('utf-8'), timeout=30)
        try:
            resp.raise_for_status()
            body = resp.json()
        except Exception as e:
            raise SystemExit(f"flashbots submission error: HTTP {resp.status_code} {resp.text} ({e})")

        bundle_hash = None
        if isinstance(body.get('result'), dict) and 'bundleHash' in body['result']:
            bundle_hash = body['result']['bundleHash']
        elif isinstance(body.get('result'), str):
            bundle_hash = body['result']
        else:
            err = body.get('error')
            raise SystemExit(f"flashbots unexpected response: {body} (error={err})")

        print(f"  bundleHash: {bundle_hash}")

        # Monitor bundle status using Flashbots Transaction Status API
        # Note: Bundle stats APIs were deprecated, but we can check individual transactions
        total_secs = int(os.getenv('FLASHBOTS_STATS_TOTAL_SECS', '300'))
        interval_secs = int(os.getenv('FLASHBOTS_STATS_INTERVAL_SECS', '10'))
        attempts = max(1, (total_secs + max(1, interval_secs) - 1) // max(1, interval_secs))
        print(f"  monitoring bundle via transaction status up to {total_secs}s (~{attempts} attempts every {interval_secs}s)...")
        print(f"  target block: {target_block}")
        print(f"  bundle hash: {bundle_hash}")
        print(f"  tx1 hash: {tx1_hash}")
        print(f"  tx2 hash: {tx2_hash}")
        
        # Try to use Flashbots Transaction Status API
        for i in range(attempts):
            try:
                current_block = w3.eth.block_number
                print(f"    attempt {i+1}/{attempts}: current block {current_block}, target {target_block}")
                
                # Check individual transaction status via Flashbots API
                tx1_status = None
                tx2_status = None
                
                try:
                    # Check tx1 status
                    tx1_url = f"https://protect.flashbots.net/tx/{tx1_hash}"
                    tx1_resp = requests.get(tx1_url, timeout=10)
                    if tx1_resp.status_code == 200:
                        tx1_status = tx1_resp.json().get('status', 'UNKNOWN')
                        print(f"    📊 tx1 status: {tx1_resp.json()}")
                        print(f"    📊 tx1 status: {tx1_status}")
                except Exception as e:
                    print(f"    ⚠️  Could not check tx1 status: {e}")
                
                try:
                    # Check tx2 status
                    tx2_url = f"https://protect.flashbots.net/tx/{tx2_hash}"
                    tx2_resp = requests.get(tx2_url, timeout=10)
                    if tx2_resp.status_code == 200:
                        tx2_status = tx2_resp.json().get('status', 'UNKNOWN')
                        print(f"    📊 tx2 status: {tx2_status}")
                except Exception as e:
                    print(f"    ⚠️  Could not check tx2 status: {e}")
                
                # Check if both transactions are included
                if tx1_status == 'INCLUDED' and tx2_status == 'INCLUDED':
                    print(f"    🎉 BOTH TRANSACTIONS INCLUDED!")
                    break
                elif tx1_status == 'FAILED' or tx2_status == 'FAILED':
                    print(f"    ❌ One or both transactions failed")
                    break
                elif tx1_status == 'CANCELLED' or tx2_status == 'CANCELLED':
                    print(f"    ❌ One or both transactions cancelled")
                    break
                
                # Check if we've passed the target block significantly
                if current_block > target_block + 5:
                    print(f"    ⏰ Bundle likely missed - current block {current_block} > target {target_block}")
                    break
                    
            except Exception as e:
                print(f"    attempt {i+1}/{attempts}: error: {e}")
            
            if i < attempts - 1:
                import time; time.sleep(interval_secs)
        
        return

    # Submit bundle to Titan (default)
    print(f"\nSubmitting bundle to {relay_url} for block {target_block}...")
    bundle_hash = eth_send_bundle(relay_url, [tx1_hex, tx2_hex], target_block)
    print(f'  bundleHash: {bundle_hash}')

    # Poll titan_getBundleStats for bundle status
    stats_url = os.getenv('TITAN_STATS_URL', 'https://stats.titanbuilder.xyz')
    total_secs = int(os.getenv('TITAN_STATS_TOTAL_SECS', '300'))
    interval_secs = int(os.getenv('TITAN_STATS_INTERVAL_SECS', '10'))
    attempts = max(1, (total_secs + max(1, interval_secs) - 1) // max(1, interval_secs))
    print(f"  polling titan_getBundleStats up to {total_secs}s (~{attempts} attempts every {interval_secs}s)...")
    print(f"  target block: {target_block}")
    print(f"  bundle hash: {bundle_hash}")
    print(f"  tx1 hash: {tx1_hash}")
    print(f"  tx2 hash: {tx2_hash}")
    
    # Wait a bit before first attempt to let bundle propagate
    print(f"  waiting 30s before first stats check...")
    import time; time.sleep(30)
    
    for i in range(attempts):
        try:
            current_block = w3.eth.block_number
            print(f"    attempt {i+1}/{attempts}: current block {current_block}, target {target_block}")
            
            stats_req = {
                'jsonrpc': '2.0',
                'id': 1,
                'method': 'titan_getBundleStats',
                'params': [{'bundleHash': bundle_hash}]
            }
            
            stats_resp = requests.post(stats_url, json=stats_req, timeout=15)
            print(f"    HTTP {stats_resp.status_code}")
            
            if stats_resp.status_code == 200:
                try:
                    stats_body = stats_resp.json()
                    print(f"    Raw response: {json.dumps(stats_body, indent=2)}")
                    
                    if stats_body.get('result'):
                        result = stats_body['result']
                        print(f"    📊 Titan Bundle Stats:")
                        print(f"      Status: {result.get('status', 'N/A')}")
                        print(f"      Block Number: {result.get('blockNumber', 'N/A')}")
                        print(f"      Simulated Gas Used: {result.get('simulatedGasUsed', 'N/A')}")
                        print(f"      Received At: {result.get('receivedAt', 'N/A')}")
                        
                        status = result.get('status', '').lower()
                        if status == 'includedinblock':
                            print(f"    🎉 BUNDLE INCLUDED IN BLOCK!")
                            break
                        elif status == 'simulationfail':
                            print(f"    ❌ Bundle simulation failed")
                            break
                        elif status == 'invalid':
                            print(f"    ❌ Bundle marked as invalid")
                            break
                        elif status in ['received', 'simulationpass', 'submitted']:
                            print(f"    ⏳ Bundle status: {result.get('status', 'N/A')}")
                        else:
                            print(f"    📋 Bundle status: {result.get('status', 'UNKNOWN')}")
                            
                    elif stats_body.get('error'):
                        error = stats_body['error']
                        print(f"    ❌ Error: {error.get('message', 'Unknown error')}")
                        if 'not found' in error.get('message', '').lower() or 'failed to get stats' in error.get('message', '').lower():
                            print(f"    Note: Bundle may not have been processed yet (try waiting longer)")
                        elif error.get('code') == -32601:  # Method not found
                            print(f"    Note: titan_getBundleStats may not be available")
                            break
                    else:
                        print(f"    ⚠️  No result in response: {stats_body}")
                        
                except json.JSONDecodeError:
                    print(f"    ❌ Invalid JSON response: {stats_resp.text}")
            else:
                print(f"    ❌ HTTP {stats_resp.status_code}: {stats_resp.text}")
            
            # Check if we've passed the target block significantly
            if current_block > target_block + 5:
                print(f"    ⏰ Bundle likely missed - current block {current_block} > target {target_block}")
                break
                
        except Exception as e:
            print(f"    attempt {i+1}/{attempts}: error: {e}")
        
        if i < attempts - 1:
            time.sleep(interval_secs)


if __name__ == '__main__':
    main()


