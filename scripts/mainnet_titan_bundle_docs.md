# Mainnet Titan Bundle Script

This script (`create_mainnet_titan_bundle.py`) creates and submits atomic bundles directly to Titan builder on Ethereum mainnet, then tracks their status.

## ⚠️ MAINNET WARNING

**This script operates on ETHEREUM MAINNET with real ETH. Use with caution!**

## Features

- 🚀 **Direct Titan Submission**: Bypasses middleware, submits directly to Titan relay
- 🔬 **Bundle Simulation**: Uses `eth_callBundle` to validate before submission (if supported)
- 📊 **Comprehensive Tracking**: Monitors both Titan bundle stats and on-chain inclusion
- 💰 **Balance Checking**: Validates sufficient funds before submission
- 🎯 **Atomic Verification**: Confirms transactions are included in the same block

## Setup

### 1. Environment Configuration

Create a `.env` file in the scripts directory:

```bash
# REQUIRED: Ethereum mainnet RPC URL
ETH_RPC_URL=https://mainnet.infura.io/v3/YOUR-API-KEY

# REQUIRED: Private keys (without 0x prefix)
TEST_PRIVATE_KEY=your_test_account_private_key_here
PAYMENT_SIGNER_PRIVATE_KEY=your_payment_signer_private_key_here

# OPTIONAL: Transaction amounts
TX1_VALUE_ETH=0.001          # Self-transfer amount
TX2_VALUE_ETH=0.01           # Builder payment amount
PRIORITY_FEE_WEI=2000000000  # 2 Gwei priority fee

# OPTIONAL: Titan configuration (defaults provided)
TITAN_RELAY_URL=https://rpc.titanbuilder.xyz
TITAN_STATS_URL=https://stats.titanbuilder.xyz
TITAN_COINBASE=0x4838B106FCe9647Bdf1E7877BF73cE8B0BAD5f97

# OPTIONAL: Monitoring
TITAN_STATS_TOTAL_SECS=300   # 5 minutes of stats polling
BLOCKS_AHEAD=3               # Target blocks ahead
```

### 2. Account Requirements

- **Test Account**: For tx1 (self-transfer), needs ~0.01+ ETH
- **Payment Account**: For tx2 (builder payment), needs ~0.02+ ETH
- Both accounts need sufficient ETH for gas fees

### 3. Dependencies

All dependencies are already included in `pyproject.toml`:
- `web3>=6.0.0`
- `eth-account>=0.10.0` 
- `requests>=2.32.0`
- `python-dotenv>=1.0.0`

## Usage

```bash
cd scripts/
uv run create_mainnet_titan_bundle.py
```

## Script Flow

1. **🔧 Configuration**: Loads environment variables and validates setup
2. **🌐 Connection**: Connects to Ethereum mainnet RPC
3. **👤 Accounts**: Creates accounts from private keys and checks balances
4. **⛽ Gas Setup**: Fetches current base fee and calculates gas prices
5. **📝 Transaction Creation**:
   - `tx1`: Self-transfer from test account (configurable amount)
   - `tx2`: Payment to Titan coinbase from payment account
6. **🔬 Simulation**: Validates bundle with `eth_callBundle` (if supported)
7. **🚀 Submission**: Submits bundle to Titan relay
8. **📊 Tracking**: Monitors bundle status via:
   - Titan bundle stats API (`titan_getBundleStats`) with retry logic (minimum 5 attempts)
   - On-chain transaction receipt monitoring
9. **📋 Summary**: Reports final results

## Sample Output

```
🔧 Mainnet Titan Bundle Configuration:
  • RPC URL: https://mainnet.infura.io/v3/...
  • Titan Relay: https://rpc.titanbuilder.xyz
  • Test Key: ✅ Set
  • Payment Key: ✅ Set

✅ Connected to Ethereum (chain_id=1)

👤 Accounts:
  • Test Account: 0x742d35Cc6...
  • Payment Account: 0x8ba1f109E...

💰 Balances:
  • Test Account: 0.050000 ETH
  • Payment Account: 0.100000 ETH

🚀 Submitting bundle to Titan...
  ✅ Bundle submitted successfully!
  📦 Bundle Hash: 0xabc123...

🛰  Polling Titan bundle stats up to 300s (~20 attempts every 15s)
    Will retry errors at least 5 times before stopping
  • attempt 1/20: HTTP 200
    📊 Status: SimulationPass
    📦 Block: 18500000
    ⛽ Gas Used: 42000
    🕐 Received: 2024-01-01T12:00:00Z
    ⏳ Bundle status: SimulationPass (continuing to poll...)

⏳ Monitoring on-chain inclusion...
  • Current block: 18499998, deadline: 18500003
    ✅ tx1: Block 18500000, Status 1, Gas 21000
    ✅ tx2: Block 18500000, Status 1, Gas 21000
  🎉 ALL TRANSACTIONS INCLUDED!
  🎯 ATOMIC BUNDLE SUCCESS - All txs in block 18500000

📋 Final Summary:
  • Bundle Hash: 0xabc123...
  • Target Block: 18500000
  • Bundle Stats Result: ✅ Included
  • On-chain Result: ✅ Included
  🎉 SUCCESS: Bundle was included atomically!
```

## Configuration Options

| Variable | Default | Description |
|----------|---------|-------------|
| `TX1_VALUE_ETH` | 0.001 | Self-transfer amount in ETH |
| `TX2_VALUE_ETH` | 0.01 | Builder payment in ETH |
| `BLOCKS_AHEAD` | 3 | Target blocks ahead of current |
| `PRIORITY_FEE_WEI` | 2000000000 | Priority fee (2 Gwei for mainnet) |
| `SKIP_SIMULATION` | false | Skip bundle simulation |
| `TITAN_STATS_TOTAL_SECS` | 300 | Stats polling duration |
| `TITAN_STATS_INTERVAL_SECS` | 15 | Stats polling interval |
| `TX_POLL_INTERVAL_SECS` | 5 | On-chain polling interval |

## Safety Features

- **Balance Validation**: Checks minimum balances before proceeding
- **Mainnet Confirmation**: Warns if not connected to mainnet
- **Simulation Validation**: Aborts if bundle simulation fails
- **Error Handling**: Graceful handling of API failures with retry logic
- **Persistent Polling**: Retries Titan stats at least 5 times before giving up
- **Comprehensive Logging**: Detailed status reporting throughout

## Troubleshooting

### Common Issues

1. **"Failed to connect to RPC"**
   - Check `ETH_RPC_URL` is valid mainnet endpoint
   - Verify API key if using Infura/Alchemy

2. **"Low balances detected"**
   - Ensure both accounts have sufficient ETH
   - Account for gas costs (~0.001 ETH per tx)

3. **"Bundle simulation failed"**
   - Check transaction parameters
   - Verify account nonces aren't stale
   - Ensure sufficient gas fees

4. **"Bundle not included"**
   - Normal on mainnet due to competition
   - Try higher priority fees
   - Target multiple blocks ahead

### Mainnet Considerations

- **Gas Prices**: Mainnet gas is expensive, monitor current prices
- **Competition**: Many searchers compete for block inclusion
- **Timing**: Blocks are ~12 seconds apart, plan accordingly
- **Costs**: Each failed attempt costs real ETH

## Related Scripts

- `create_test_tx.py`: Original testnet version with middleware
- `create_bundle_two_txs.py`: Multi-builder testnet version
