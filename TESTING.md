# Testing Guide

## 🧪 Creating Test Transactions (tx1)

The easiest way to test the atomic bundler without managing nonces manually.

### Using the Python Test Script

```bash
# Install dependencies
pip install web3 requests python-dotenv

# Option 1: Use .env file (recommended)
cp .env.example .env
# Edit .env and add:
# TEST_PRIVATE_KEY=your_test_private_key_here
# ETH_RPC_URL=your_rpc_url_here

# Option 2: Set environment variables manually
export TEST_PRIVATE_KEY=your_test_private_key_here
export ETH_RPC_URL=your_rpc_url_here

# Run script (automatically reads .env)
python3 scripts/create_test_tx.py
```

The script will automatically:
- ✅ Get the current nonce from the network
- ✅ Set priority fee to 0 (as required by atomic bundler)
- ✅ Create a simple 0.001 ETH self-transfer transaction
- ✅ Sign the transaction and output the raw hex
- ✅ Generate a complete curl command for testing
- ✅ Save the bundle request to `test_bundle_request.json`

## 🚀 Testing the Complete Flow

### 1. Start the Service

```bash
# Set up environment
cp .env.example .env
# Edit .env with your keys

# Run the service
cargo run --bin middleware
```

### 2. Health Check

```bash
curl -X GET http://localhost:8080/healthz
```

Expected response:
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "timestamp": "2024-01-01T12:00:00Z",
  "components": {
    "database": "healthy",
    "killswitch": "inactive"
  }
}
```

### 3. Submit Bundle

Using the generated transaction:

```bash
curl -X POST http://localhost:8080/bundles \
  -H "Content-Type: application/json" \
  -d @test_bundle_request.json
```

Expected response:
```json
{
  "bundleId": "550e8400-e29b-41d4-a716-446655440000",
  "submissions": [
    {
      "builder": "flashbots",
      "status": "submitted", 
      "response": "0x123..."
    }
  ]
}
```

### 4. Check Bundle Status

```bash
curl -X GET http://localhost:8080/bundles/550e8400-e29b-41d4-a716-446655440000
```

## 🔧 Test Configuration

### Required Environment Variables

```bash
# Payment signer (for tx2)
PAYMENT_SIGNER_PRIVATE_KEY=your_payment_signer_key

# RPC endpoint
ETH_RPC_URL=https://eth-sepolia.g.alchemy.com/v2/YOUR-API-KEY

# Test transaction creator (optional)
TEST_PRIVATE_KEY=your_test_transaction_key
```

### Test Network Setup (Sepolia)

1. Get Sepolia ETH from faucets:
   - https://sepoliafaucet.com/
   - https://faucet.sepolia.dev/

2. Use Sepolia RPC:
   ```
   ETH_RPC_URL=https://eth-sepolia.g.alchemy.com/v2/YOUR-API-KEY
   ```

3. Configure builders for Sepolia in `config.yaml`:
   ```yaml
   network:
     network: sepolia
     chain_id: 11155111
   
   builders:
     - name: test-builder
       relay_url: "https://sepolia-relay.example.com"
       payment_address: "0x742d35Cc6635C0532925a3b8D5C70d3f"
       enabled: true
   ```

## 🐛 Troubleshooting

### Common Issues

1. **"PAYMENT_SIGNER_PRIVATE_KEY missing"**
   - Set the environment variable in `.env`

2. **"Failed to get latest block"**
   - Check your `ETH_RPC_URL` is correct
   - Verify RPC endpoint is accessible

3. **"Invalid builder payment address"**
   - Ensure addresses in `config.yaml` are valid hex

4. **"Bundle submission failed"**
   - Check relay URLs are accessible
   - Verify network configuration matches

### Debug Mode

Run with detailed logging:
```bash
RUST_LOG=debug cargo run --bin middleware
```

### Mock Testing

For testing without real relays, add to `.env`:
```bash
TEST_MODE=true
MOCK_RELAY_RESPONSES=true
```
