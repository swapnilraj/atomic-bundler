#!/usr/bin/env python3
"""
Create and submit bundles directly to Titan builder on mainnet
Based on create_test_tx.py but adapted for mainnet Titan submission

Requires: pip install web3 requests python-dotenv eth-account
"""

import os
import requests
import time
import json
from datetime import datetime, timezone, timedelta
from web3 import Web3
from eth_account import Account
from decimal import Decimal

# Try to load .env file
try:
    from dotenv import load_dotenv
    load_dotenv()
    print("✅ Loaded environment variables from .env file")
except ImportError:
    print("⚠️  python-dotenv not installed. Using system environment variables only.")
    print("   Install with: pip install python-dotenv")
except Exception as e:
    print(f"⚠️  Could not load .env file: {e}")
    print("   Using system environment variables only.")


def normalize_key(k: str) -> str:
    """Normalize private key format"""
    if k.startswith('0x'):
        return k
    return '0x' + k


def to_wei_eth(amount_eth: str | float) -> int:
    """Convert ETH amount to wei"""
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
    """Sign an EIP-1559 transaction"""
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


def eth_send_bundle(relay_url: str, txs: list[str], block_number: int):
    """Submit bundle to Titan relay"""
    req = {
        'jsonrpc': '2.0',
        'id': 1,
        'method': 'eth_sendBundle',
        'params': [{
            'txs': txs,
            'blockNumber': hex(block_number),
        }]
    }
    print(f"🚀 Submitting bundle to Titan relay: {json.dumps(req, indent=2)}")
    resp = requests.post(relay_url, json=req, timeout=30)
    resp.raise_for_status()
    payload = resp.json()
    
    # Handle Titan response format
    if isinstance(payload.get('result'), dict) and 'bundleHash' in payload['result']:
        return payload['result']['bundleHash']
    if isinstance(payload.get('result'), str):
        return payload['result']
    
    if payload.get('error'):
        raise RuntimeError(f"Titan bundle submission error: {payload['error']}")
    
    raise RuntimeError(f"Unexpected Titan response: {payload}")


def eth_call_bundle(relay_url: str, txs: list[str], block_number: int):
    """Simulate bundle using eth_callBundle"""
    req = {
        'jsonrpc': '2.0',
        'id': 1,
        'method': 'eth_callBundle',
        'params': [{
            'txs': txs,
            'blockNumber': hex(block_number),
            'stateBlockNumber': hex(block_number - 1),
            'timestamp': 0
        }]
    }
    
    resp = requests.post(relay_url, json=req, timeout=30)
    resp.raise_for_status()
    payload = resp.json()
    
    if payload.get('error'):
        raise RuntimeError(f"Bundle simulation error: {payload['error']}")
    
    return payload.get('result', {})


def poll_titan_bundle_stats(stats_url: str, bundle_hash: str, total_secs: int = 300, interval_secs: int = 10):
    """Poll Titan bundle stats until completion or timeout"""
    attempts = max(1, (total_secs + max(1, interval_secs) - 1) // max(1, interval_secs))
    min_attempts = 5  # Always try at least 5 times before giving up on errors
    print(f"🛰  Polling Titan bundle stats up to {total_secs}s (~{attempts} attempts every {interval_secs}s)")
    print(f"    Will retry errors at least {min_attempts} times before stopping")
    
    error_count = 0
    
    for i in range(attempts):
        try:
            stats_req = {
                'jsonrpc': '2.0',
                'id': 1,
                'method': 'titan_getBundleStats',
                'params': [{'bundleHash': bundle_hash}]
            }
            
            stats_resp = requests.post(stats_url, json=stats_req, timeout=15)
            print(f"  • attempt {i+1}/{attempts}: HTTP {stats_resp.status_code}")
            
            if stats_resp.status_code == 200:
                stats_body = stats_resp.json()
                
                if stats_body.get('result'):
                    result = stats_body['result']
                    status = result.get('status', 'UNKNOWN')
                    block_num = result.get('blockNumber', 'N/A')
                    gas_used = result.get('simulatedGasUsed', 'N/A')
                    received_at = result.get('receivedAt', 'N/A')
                    
                    print(f"    📊 Status: {status}")
                    print(f"    📦 Block: {block_num}")
                    print(f"    ⛽ Gas Used: {gas_used}")
                    print(f"    🕐 Received: {received_at}")
                    
                    # Terminal statuses - only stop on definitive success/failure
                    if status.lower() in ['includedinblock', 'simulationfail', 'invalid', 'excludedfromblock']:
                        if status.lower() == 'includedinblock':
                            print(f"    🎉 BUNDLE INCLUDED IN BLOCK!")
                            return True
                        else:
                            print(f"    ❌ Bundle failed with status: {status}")
                            # Only stop on terminal failure if we've tried at least min_attempts
                            if i + 1 >= min_attempts:
                                return False
                            else:
                                print(f"    🔄 Retrying... ({i+1}/{min_attempts} minimum attempts)")
                    else:
                        print(f"    ⏳ Bundle status: {status} (continuing to poll...)")
                        
                elif stats_body.get('error'):
                    error = stats_body['error']
                    error_count += 1
                    print(f"    ❌ Error ({error_count}): {error.get('message', 'Unknown error')}")
                    
                    if 'not found' in error.get('message', '').lower() or 'failed to get stats' in error.get('message', '').lower():
                        print(f"    📝 Bundle may not be processed yet")
                    else:
                        print(f"    🚫 API error")
                    
                    # Only stop on errors if we've tried at least min_attempts
                    if i + 1 >= min_attempts and error_count >= min_attempts:
                        print(f"    🛑 Stopping after {min_attempts} consecutive errors")
                        return False
                    else:
                        print(f"    🔄 Retrying error... ({i+1}/{min_attempts} minimum attempts)")
                        
                else:
                    print(f"    ⚠️  Empty result: {stats_body}")
                    
            else:
                error_count += 1
                print(f"    ❌ HTTP {stats_resp.status_code} ({error_count}): {stats_resp.text}")
                
                # Only stop on HTTP errors if we've tried at least min_attempts
                if i + 1 >= min_attempts and error_count >= min_attempts:
                    print(f"    🛑 Stopping after {min_attempts} HTTP errors")
                    return False
                else:
                    print(f"    🔄 Retrying HTTP error... ({i+1}/{min_attempts} minimum attempts)")
                
        except Exception as e:
            error_count += 1
            print(f"    ❌ Exception ({error_count}): {e}")
            
            # Only stop on exceptions if we've tried at least min_attempts
            if i + 1 >= min_attempts and error_count >= min_attempts:
                print(f"    🛑 Stopping after {min_attempts} exceptions")
                return False
            else:
                print(f"    🔄 Retrying exception... ({i+1}/{min_attempts} minimum attempts)")
        
        if i < attempts - 1:
            time.sleep(interval_secs)
    
    print(f"    ⌛ Polling timeout after {total_secs}s")
    return False


def monitor_tx_inclusion(w3: Web3, tx_hashes: list[str], target_blocks: list[int], poll_interval: int = 5):
    """Monitor transaction inclusion on-chain"""
    print(f"\n⏳ Monitoring on-chain inclusion...")
    print(f"  • Target blocks: {target_blocks}")
    print(f"  • Transactions: {len(tx_hashes)}")
    
    max_target = max(target_blocks)
    poll_deadline_blocks = max_target + 3  # Grace period
    
    while True:
        try:
            current_block = w3.eth.block_number
            print(f"  • Current block: {current_block}, deadline: {poll_deadline_blocks}")
            
            # Check all transactions
            all_included = True
            included_txs = []
            
            for i, tx_hash in enumerate(tx_hashes, 1):
                try:
                    receipt = w3.eth.get_transaction_receipt(tx_hash)
                    if receipt:
                        status = receipt.get('status') if isinstance(receipt, dict) else receipt.status
                        block_num = receipt.get('blockNumber') if isinstance(receipt, dict) else receipt.blockNumber
                        gas_used = receipt.get('gasUsed') if isinstance(receipt, dict) else receipt.gasUsed
                        
                        print(f"    ✅ tx{i}: Block {block_num}, Status {status}, Gas {gas_used}")
                        included_txs.append({
                            'tx': i,
                            'hash': tx_hash,
                            'block': block_num,
                            'status': status,
                            'gasUsed': gas_used
                        })
                    else:
                        all_included = False
                        print(f"    ⏳ tx{i}: Not yet mined")
                        
                except Exception:
                    all_included = False
                    print(f"    ⏳ tx{i}: Not yet mined")
            
            if all_included and included_txs:
                print(f"  🎉 ALL TRANSACTIONS INCLUDED!")
                # Check if they're in the same block (atomic bundle)
                blocks = set(tx['block'] for tx in included_txs)
                if len(blocks) == 1:
                    print(f"  🎯 ATOMIC BUNDLE SUCCESS - All txs in block {list(blocks)[0]}")
                else:
                    print(f"  ⚠️  Transactions split across blocks: {sorted(blocks)}")
                return True
                
            if current_block >= poll_deadline_blocks:
                print(f"  ⌛ Deadline reached - not included by block {poll_deadline_blocks}")
                return False
                
        except Exception as e:
            print(f"  ❌ Error reading block: {e}")
        
        time.sleep(poll_interval)


def create_mainnet_titan_bundle():
    """Main function to create and submit bundle to Titan on mainnet"""
    
    # Configuration
    RPC_URL = os.getenv('ETH_RPC_URL')
    TEST_PRIVATE_KEY = os.getenv('TEST_PRIVATE_KEY', '')
    PAYMENT_SIGNER_PRIVATE_KEY = os.getenv('PAYMENT_SIGNER_PRIVATE_KEY', '')
    
    # Titan mainnet configuration
    TITAN_RELAY_URL = os.getenv('TITAN_RELAY_URL', 'https://rpc.titanbuilder.xyz')
    TITAN_STATS_URL = os.getenv('TITAN_STATS_URL', 'https://stats.titanbuilder.xyz') 
    TITAN_COINBASE = os.getenv('TITAN_COINBASE', '0x4838B106FCe9647Bdf1E7877BF73cE8B0BAD5f97')  # Titan mainnet coinbase
    
    print(f"\n🔧 Mainnet Titan Bundle Configuration:")
    print(f"  • RPC URL: {RPC_URL}")
    print(f"  • Titan Relay: {TITAN_RELAY_URL}")
    print(f"  • Titan Stats: {TITAN_STATS_URL}")
    print(f"  • Titan Coinbase: {TITAN_COINBASE}")
    print(f"  • Test Key: {'✅ Set' if TEST_PRIVATE_KEY else '❌ Not set'}")
    print(f"  • Payment Key: {'✅ Set' if PAYMENT_SIGNER_PRIVATE_KEY else '❌ Not set'}")
    
    if not TEST_PRIVATE_KEY or not PAYMENT_SIGNER_PRIVATE_KEY:
        print("\n❌ Required private keys not found!")
        print("Please set in your .env file:")
        print("TEST_PRIVATE_KEY=your_test_private_key")
        print("PAYMENT_SIGNER_PRIVATE_KEY=your_payment_signer_private_key")
        print("ETH_RPC_URL=https://mainnet.infura.io/v3/YOUR-API-KEY")
        return
    
    # Connect to mainnet
    w3 = Web3(Web3.HTTPProvider(RPC_URL))
    if not w3.is_connected():
        print(f"❌ Failed to connect to {RPC_URL}")
        return
    
    if w3.eth.chain_id != 1:
        print(f"⚠️  Warning: Not connected to mainnet (chain_id={w3.eth.chain_id})")
        confirm = input("Continue anyway? (y/N): ")
        if confirm.lower() != 'y':
            return
    
    print(f"✅ Connected to Ethereum (chain_id={w3.eth.chain_id})")
    
    # Normalize keys
    test_key = normalize_key(TEST_PRIVATE_KEY)
    payment_key = normalize_key(PAYMENT_SIGNER_PRIVATE_KEY)
    
    # Create accounts
    test_account = Account.from_key(test_key)
    payment_account = Account.from_key(payment_key)
    
    print(f"\n👤 Accounts:")
    print(f"  • Test Account: {test_account.address}")
    print(f"  • Payment Account: {payment_account.address}")
    
    # Check balances
    test_balance = w3.eth.get_balance(test_account.address)
    payment_balance = w3.eth.get_balance(payment_account.address)
    
    print(f"\n💰 Balances:")
    print(f"  • Test Account: {w3.from_wei(test_balance, 'ether'):.6f} ETH")
    print(f"  • Payment Account: {w3.from_wei(payment_balance, 'ether'):.6f} ETH")
    
    # Minimum balance check
    min_balance_wei = to_wei_eth('0.01')  # 0.01 ETH minimum
    if test_balance < min_balance_wei or payment_balance < min_balance_wei:
        print(f"⚠️  Warning: Low balances detected")
        print(f"   Recommended minimum: 0.01 ETH per account")
        confirm = input("Continue anyway? (y/N): ")
        if confirm.lower() != 'y':
            return
    
    # Get gas parameters
    latest = w3.eth.get_block('latest')
    base_fee = int(latest.get('baseFeePerGas', 20_000_000_000) * 1.5)
    # max_priority_fee = int(os.getenv('PRIORITY_FEE_WEI', '2000000000'))  # 2 Gwei default for mainnet
    max_priority_fee = 0
    max_fee = base_fee + max_priority_fee
    
    print(f"\n⛽ Gas Configuration:")
    print(f"  • Base Fee: {base_fee:,} wei ({w3.from_wei(base_fee, 'gwei'):.2f} Gwei)")
    print(f"  • Priority Fee: {max_priority_fee:,} wei ({w3.from_wei(max_priority_fee, 'gwei'):.2f} Gwei)")
    print(f"  • Max Fee: {max_fee:,} wei ({w3.from_wei(max_fee, 'gwei'):.2f} Gwei)")
    
    # Get nonces
    test_nonce = w3.eth.get_transaction_count(test_account.address)
    payment_nonce = w3.eth.get_transaction_count(payment_account.address)
    
    # Target blocks
    blocks_ahead = int(os.getenv('BLOCKS_AHEAD', '10'))
    target_block = latest.number + blocks_ahead
    
    print(f"\n🎯 Bundle Target:")
    print(f"  • Current Block: {latest.number}")
    print(f"  • Target Block: {target_block}")
    print(f"  • Blocks Ahead: {blocks_ahead}")
    
    # Create tx1: self-transfer
    tx1_value = to_wei_eth(os.getenv('TX1_VALUE_ETH', '0.001'))  # 0.001 ETH default
    tx1_hex, tx1_hash = sign_tx(
        w3,
        test_key,
        test_account.address,  # Self-transfer
        tx1_value,
        21_000,
        max_fee,
        max_priority_fee,
        test_nonce
    )
    
    # Create tx2: payment to Titan coinbase
    tx2_value = to_wei_eth(os.getenv('TX2_VALUE_ETH', '0.0001'))  # 0.0001 ETH default
    tx2_hex, tx2_hash = sign_tx(
        w3,
        payment_key,
        TITAN_COINBASE,
        tx2_value,
        21_000,
        max_fee,
        max_priority_fee,
        payment_nonce
    )
    
    print(f"\n📝 Bundle Transactions:")
    print(f"  • tx1 (self-transfer): {tx1_hash}")
    print(f"    Value: {w3.from_wei(tx1_value, 'ether'):.6f} ETH")
    print(f"    From/To: {test_account.address}")
    print(f"  • tx2 (builder payment): {tx2_hash}")
    print(f"    Value: {w3.from_wei(tx2_value, 'ether'):.6f} ETH")
    print(f"    From: {payment_account.address}")
    print(f"    To: {TITAN_COINBASE}")
    
    # Simulate bundle first (if supported)
    if not os.getenv('SKIP_SIMULATION', '').lower() in ('1', 'true', 'yes'):
        print(f"\n🔬 Simulating bundle...")
        try:
            simulation = eth_call_bundle(TITAN_RELAY_URL, [tx1_hex, tx2_hex], target_block)
            print(f"  📊 Simulation Results:")
            print(f"    Bundle Hash: {simulation.get('bundleHash', 'N/A')}")
            print(f"    Coinbase Diff: {simulation.get('coinbaseDiff', 'N/A')} wei")
            print(f"    Total Gas Used: {simulation.get('totalGasUsed', 'N/A')}")
            
            results = simulation.get('results', [])
            for i, result in enumerate(results, 1):
                gas_used = result.get('gasUsed', 'N/A')
                gas_price = result.get('gasPrice', 'N/A')
                print(f"    tx{i} - Gas Used: {gas_used}, Gas Price: {gas_price}")
                
                if result.get('error'):
                    print(f"    tx{i} - ❌ Error: {result['error']}")
                    print(f"    🚫 Bundle simulation failed - aborting")
                    return
                elif result.get('revert'):
                    print(f"    tx{i} - ❌ Revert: {result['revert']}")
                    print(f"    🚫 Bundle simulation failed - aborting")
                    return
            
            print(f"  ✅ Simulation successful!")
            
        except Exception as e:
            error_msg = str(e).lower()
            if 'method not found' in error_msg:
                print(f"  ℹ️  Simulation not supported by this relay")
            else:
                print(f"  ⚠️  Simulation failed: {e}")
            print(f"  📤 Proceeding with submission...")
    
    # Submit bundle
    print(f"\n🚀 Submitting bundle to Titan...")
    try:
        bundle_hash = eth_send_bundle(TITAN_RELAY_URL, [tx1_hex, tx2_hex], target_block)
        print(f"  ✅ Bundle submitted successfully!")
        print(f"  📦 Bundle Hash: {bundle_hash}")
        
    except Exception as e:
        print(f"  ❌ Bundle submission failed: {e}")
        return
    
    # Track bundle status
    stats_total_secs = int(os.getenv('TITAN_STATS_TOTAL_SECS', '300'))
    stats_interval_secs = int(os.getenv('TITAN_STATS_INTERVAL_SECS', '15'))
    
    bundle_included = poll_titan_bundle_stats(
        TITAN_STATS_URL, 
        bundle_hash, 
        stats_total_secs, 
        stats_interval_secs
    )
    
    # Monitor on-chain inclusion regardless of bundle stats
    tx_poll_interval = int(os.getenv('TX_POLL_INTERVAL_SECS', '6'))
    tx_hashes = [tx1_hash, tx2_hash]
    target_blocks = [target_block]
    
    chain_included = monitor_tx_inclusion(
        w3, 
        tx_hashes, 
        target_blocks, 
        tx_poll_interval
    )
    
    # Final summary
    print(f"\n📋 Final Summary:")
    print(f"  • Bundle Hash: {bundle_hash}")
    print(f"  • Target Block: {target_block}")
    print(f"  • Bundle Stats Result: {'✅ Included' if bundle_included else '❌ Not included/Unknown'}")
    print(f"  • On-chain Result: {'✅ Included' if chain_included else '❌ Not included'}")
    
    if bundle_included and chain_included:
        print(f"  🎉 SUCCESS: Bundle was included atomically!")
    elif chain_included:
        print(f"  ⚠️  PARTIAL: Transactions included but bundle status unclear")
    else:
        print(f"  ❌ FAILED: Bundle was not included")
    
    print(f"\n💡 Transaction Details:")
    print(f"  • tx1: {tx1_hash}")
    print(f"  • tx2: {tx2_hash}")


if __name__ == "__main__":
    create_mainnet_titan_bundle()
