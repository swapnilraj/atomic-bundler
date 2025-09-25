#!/usr/bin/env python3
"""
Simple script to create test transactions for atomic bundler
Requires: pip install web3 requests python-dotenv
"""

import os
import requests
import time
from datetime import datetime, timezone, timedelta
from web3 import Web3
from eth_account import Account
import json

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

def create_test_tx():
    # Configuration
    RPC_URL = os.getenv('ETH_RPC_URL', 'http://localhost:8545')
    PRIVATE_KEY = os.getenv('TEST_PRIVATE_KEY', '')
    
    print(f"\n🔧 Configuration:")
    print(f"  • RPC URL: {RPC_URL}")
    print(f"  • Private Key: {'✅ Set' if PRIVATE_KEY else '❌ Not set'}")
    
    if not PRIVATE_KEY:
        print("\n❌ TEST_PRIVATE_KEY not found!")
        print("Please add TEST_PRIVATE_KEY to your .env file or set as environment variable")
        print("\nExample .env file:")
        print("TEST_PRIVATE_KEY=your_private_key_without_0x_prefix")
        print("ETH_RPC_URL=https://eth-mainnet.alchemyapi.io/v2/YOUR-API-KEY")
        return
    
    # Remove 0x prefix if present
    if PRIVATE_KEY.startswith('0x'):
        PRIVATE_KEY = PRIVATE_KEY[2:]
    
    # Connect to RPC
    w3 = Web3(Web3.HTTPProvider(RPC_URL))
    if not w3.is_connected():
        print(f"Failed to connect to {RPC_URL}")
        return
    
    # Get account
    account = Account.from_key(PRIVATE_KEY)
    address = account.address
    
    print(f"Creating test transaction from: {address}")
    
    # Get current nonce
    nonce = w3.eth.get_transaction_count(address)
    
    # Get current gas prices
    latest_block = w3.eth.get_block('latest')
    base_fee = latest_block.baseFeePerGas if hasattr(latest_block, 'baseFeePerGas') else 20_000_000_000
    
    # Create transaction (send 0.0001 ETH to self)
    tx = {
        'type': 2,  # EIP-1559
        'chainId': w3.eth.chain_id,
        'nonce': nonce,
        'to': address,  # Send to self
        'value': w3.to_wei(0.0002, 'ether'),
        'gas': 21000,
        'maxFeePerGas': base_fee * 2,  # 2x base fee
        'maxPriorityFeePerGas': 0,  # Zero priority fee as required
        'data': b'',
    }
    
    # Sign transaction
    signed_tx = w3.eth.account.sign_transaction(tx, PRIVATE_KEY)
    
    print(f"\nTransaction details:")
    print(f"- From: {address}")
    print(f"- To: {address} (self)")
    print(f"- Value: 0.0001 ETH")
    print(f"- Nonce: {nonce}")
    print(f"- Chain ID: {w3.eth.chain_id}")
    print(f"- Max Fee: {tx['maxFeePerGas']} wei")
    print(f"- Priority Fee: {tx['maxPriorityFeePerGas']} wei")
    
    print(f"\nRaw transaction (use this as tx1):")
    print(signed_tx.rawTransaction.hex())
    
    # Create bundle request with expiry (5 minutes from now)
    expiry_time = datetime.now(timezone.utc) + timedelta(minutes=5)
    
    bundle_request = {
        "tx1": signed_tx.rawTransaction.hex(),
        "payment": {
            "mode": "direct",
            "formula": "flat",
            "maxAmountWei": "100000000000000000",  # 0.1 ETH
            "expiry": expiry_time.isoformat()
        },
        "targets": {
            "blocks": [latest_block.number + 1, latest_block.number + 10]
        }
    }
    
    # Save bundle request to file
    with open('test_bundle_request.json', 'w') as f:
        json.dump(bundle_request, f, indent=2)
    print(f"\nBundle request saved to: test_bundle_request.json")
    
    # Submit bundle to middleware
    print(f"\n🚀 Submitting bundle to atomic bundler...")
    try:
        response = requests.post(
            'http://localhost:8080/bundles',
            headers={'Content-Type': 'application/json'},
            json=bundle_request,
            timeout=30
        )
        
        if response.status_code == 200:
            result = response.json()
            bundle_id = result.get('bundleId')
            submissions = result.get('submissions', [])
            
            print(f"✅ Bundle submitted successfully!")
            print(f"📦 Bundle ID: {bundle_id}")
            print(f"\n📊 Submission Results:")
            
            bundle_hash = None
            titan_bundle_hash = None
            for submission in submissions:
                builder = submission.get('builder')
                status = submission.get('status')
                if status == 'submitted':
                    relay_response = submission.get('response')
                    print(f"  • {builder}: ✅ {status} (response: {relay_response})")
                    bundle_hash = bundle_hash or relay_response
                    if builder and builder.lower() == 'titan':
                        titan_bundle_hash = relay_response
                else:
                    error = submission.get('error', 'Unknown error')
                    print(f"  • {builder}: ❌ {status} ({error})")
            
            # Extract transaction hashes
            print(f"\n🔍 Transaction Details:")
            print(f"  • tx1 (user transaction): {signed_tx.hash.hex()}")
            
            # Try to decode tx2 hash from the bundle (this would need to be calculated)
            # For now, we'll show the raw transaction hex
            print(f"  • tx1 raw: {signed_tx.rawTransaction.hex()}")
            print(f"  • tx2 will be generated by the middleware for each builder")

            # ---- Titan bundle tracing per docs: https://docs.titanbuilder.xyz/bundle-tracing ----
            # Endpoint: https://stats.titanbuilder.xyz
            # Method: titan_getBundleStats
            # Params: [{ "bundleHash": "0x..." }]
            stats_url = os.getenv('TITAN_STATS_URL', 'https://stats.titanbuilder.xyz')
            if titan_bundle_hash or bundle_hash:
                bh = titan_bundle_hash or bundle_hash
                print("\n🛰  Querying Titan bundle stats (titan_getBundleStats)...")

                # Poll up to ~5 minutes by default (docs say trace ready ~5m)
                total_secs = int(os.getenv('STATS_POLL_TOTAL_SECS', '300'))
                interval_secs = int(os.getenv('STATS_POLL_INTERVAL_SECS', '10'))
                attempts = max(1, (total_secs + max(1, interval_secs) - 1) // max(1, interval_secs))
                print(f"  • polling up to {total_secs}s (~{attempts} attempts every {interval_secs}s)")

                for i in range(attempts):
                    try:
                        stats_req = {
                            'jsonrpc': '2.0',
                            'id': 1,
                            'method': 'titan_getBundleStats',
                            'params': [ { 'bundleHash': bh } ]
                        }
                        stats_resp = requests.post(stats_url, json=stats_req, timeout=15)
                        if stats_resp.status_code == 200:
                            payload = stats_resp.json()
                            if 'result' in payload and payload['result'] is not None:
                                result = payload['result']
                                status = result.get('status')
                                builder_payment = result.get('builderPayment')
                                err = result.get('error')
                                print(f"  • attempt {i+1}/{attempts}: status={status}, builderPayment={builder_payment}, error={err}")
                                # Only stop on terminal statuses; keep polling if status is Received/SimulationPass for richer trace
                                terminal_statuses = { 'SimulationFail', 'ExcludedFromBlock', 'IncludedInBlock', 'Submitted', 'Invalid' }
                                if status in terminal_statuses:
                                    break
                            else:
                                # If the service returns an error like "Failed to get stats for bundle ..." keep polling until timeout
                                rpc_err = payload.get('error', {})
                                msg = rpc_err.get('message', '')
                                print(f"  • attempt {i+1}/{attempts}: waiting (response error='{msg}')")
                        else:
                            print(f"  • attempt {i+1}/{attempts}: HTTP {stats_resp.status_code}")
                    except Exception as e:
                        print(f"  • attempt {i+1}/{attempts}: error: {e}")

                    if i < attempts - 1:
                        time.sleep(interval_secs)

            # ---- Poll on-chain inclusion for tx1 ----
            print("\n⏳ Monitoring on-chain inclusion for tx1...")
            best_target = max(bundle_request['targets']['blocks'])
            poll_deadline_blocks = best_target + 1
            poll_interval = int(os.getenv('POLL_INTERVAL_SECS', '5'))
            landed = False

            while True:
                try:
                    current_block = w3.eth.block_number
                except Exception as e:
                    print(f"  • error reading block number: {e}")
                    time.sleep(poll_interval)
                    continue

                try:
                    receipt = w3.eth.get_transaction_receipt(signed_tx.hash)
                    if receipt:
                        status_hex = receipt.get('status') if isinstance(receipt, dict) else receipt.status
                        block_num = receipt.get('blockNumber') if isinstance(receipt, dict) else receipt.blockNumber
                        print(f"  • Landed in block {block_num}, status={status_hex}")
                        landed = True
                        break
                except Exception:
                    # Not yet mined
                    pass

                if current_block >= poll_deadline_blocks:
                    print(f"  • Expired: not included by block {poll_deadline_blocks}")
                    break

                time.sleep(poll_interval)

            if landed:
                print("✅ Bundle landed (tx1 observed on-chain)")
            else:
                print("⌛ Bundle not observed on-chain within target window")
            
        else:
            print(f"❌ Bundle submission failed!")
            print(f"Status: {response.status_code}")
            print(f"Response: {response.text}")
            
    except requests.exceptions.ConnectionError:
        print(f"❌ Failed to connect to middleware server at http://localhost:8080")
        print(f"Make sure the server is running with: cargo run --bin middleware")
    except requests.exceptions.Timeout:
        print(f"❌ Request timed out after 30 seconds")
    except Exception as e:
        print(f"❌ Unexpected error: {e}")
        
    print(f"\n💡 Manual curl command (if needed):")
    print(f"curl -X POST http://localhost:8080/bundles \\")
    print(f"  -H 'Content-Type: application/json' \\")
    print(f"  -d @test_bundle_request.json")

if __name__ == "__main__":
    create_test_tx()
