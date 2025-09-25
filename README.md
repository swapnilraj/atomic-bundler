# Atomic Bundler

> **Builder-Paid Atomic Bundles Middleware**

A production-ready Rust service that accepts EIP-1559 transactions with zero priority fees, computes builder payments, and submits atomic bundles to multiple MEV builder relays.

## 🚀 Quick Start

```bash
# Clone the repository
git clone https://github.com/atomic-bundler/atomic-bundler.git
cd atomic-bundler

# Set up environment variables
cp .env.example .env
# Edit .env with your private key and RPC URL

# Build and run
cargo build --release
cargo run --bin middleware
```

### Environment Setup

1. **Copy environment template:**
   ```bash
   cp .env.example .env
   ```

2. **Configure required variables in `.env`:**
   ```bash
   # Your private key for signing payment transactions (without 0x)
   PAYMENT_SIGNER_PRIVATE_KEY=your_private_key_here
   
   # Ethereum RPC endpoint
   ETH_RPC_URL=https://eth-mainnet.alchemyapi.io/v2/YOUR-API-KEY
   ```

3. **Configure builders in `config.yaml`:**
   ```yaml
   builders:
     - name: flashbots
       relay_url: "https://relay.flashbots.net"
       payment_address: "0xbuilder_payment_address"
       enabled: true
   ```

### Testing the API

Once running, test the service:

```bash
# Health check
curl -X GET http://localhost:8080/healthz

# Submit a bundle (example)
curl -X POST http://localhost:8080/bundles \
  -H "Content-Type: application/json" \
  -d '{
    "tx1": "0x02f87082013a8085174876e80085174876e80082520894...",
    "payment": {
      "mode": "direct",
      "formula": "flat",
      "maxAmountWei": "100000000000000000",
      "expiry": "2025-01-01T12:00:00Z"
    },
    "target_block": 19000010
  }'

# Get bundle status
curl -X GET http://localhost:8080/bundles/{bundle_id}
```

## 📋 Overview

The Atomic Bundler middleware:

1. **Accepts** signed EIP-1559 transactions with `priority_fee = 0`
2. **Computes** builder payment based on configurable formulas
3. **Forges** a second transaction (tx2) for builder payment
4. **Submits** atomic bundles `[tx1, tx2]` via `eth_sendBundle` to multiple builder relays
5. **Tracks** inclusion status and enforces spending caps
6. **Exposes** REST API and Prometheus metrics

## 🏗️ Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Client App    │───▶│  Atomic Bundler  │───▶│  Builder Relays │
│                 │    │   (Middleware)   │    │                 │
│ • Submit tx1    │    │ • Payment calc   │    │ • Flashbots     │
│ • Zero priority │    │ • Bundle forge   │    │ • BeaverBuild   │
│ • Get status    │    │ • Multi-relay    │    │ • Titan         │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

### Crate Structure

- **`middleware`** - HTTP API server and orchestration logic
- **`relay_client`** - Builder relay communication (`eth_sendBundle`)
- **`simulator`** - Transaction simulation and validation (pluggable)
- **`payment`** - Payment calculation and tx2 forging
- **`config`** - Configuration parsing and validation
- **`types`** - Shared domain types and data structures

## 🔧 Configuration

The service is configured via `config.yaml`:

```yaml
network: mainnet
targets:
  blocks_ahead: 3
  resubmit_max: 3
payment:
  formula: basefee      # flat|gas|basefee
  k1: 1.0               # multiplier for gas-based formulas
  k2: 200000000000000   # constant bribe
  max_amount_wei: 500000000000000
limits:
  per_bundle_cap_wei: 2000000000000000    # 0.002 ETH
  daily_cap_wei: 500000000000000000       # 0.5 ETH
builders:
  - name: flashbots
    relay_url: "https://relay.flashbots.net"
    status_url: null
    payment_address: "0xabc...abc"
    enabled: true
```

See `config.example.yaml` for full configuration options.

## 🌐 API Reference

### Submit Bundle
```http
POST /bundles
Content-Type: application/json

{
  "tx1": "0x02f86c0182...",
  "payment": {
    "mode": "direct",
    "formula": "basefee",
    "maxAmountWei": "500000000000000",
    "expiry": "2024-01-01T12:00:00Z"
  },
  "target_block": 18500005
}
```

**Response:**
```json
{
  "bundleId": "550e8400-e29b-41d4-a716-446655440000"
}
```

### Get Bundle Status
```http
GET /bundles/{bundleId}
```

**Response:**
```json
{
  "state": "sent",
  "blockHash": null,
  "tx1Hash": "0xabc123...",
  "tx2Hash": "0xdef456...",
  "metrics": {
    "submittedAt": "2024-01-01T12:00:00Z",
    "relaysCount": 3,
    "gasUsed": 21000
  }
}
```

### Health Check
```http
GET /healthz
```

### Admin Endpoints
```http
POST /config/reload    # Reload configuration
POST /killswitch       # Emergency stop
```

## 🚀 Deployment

### Docker

```bash
# Build image
make docker-build

# Run container
make docker-run
```

### Production

```bash
# Build release binary
make build-release

# Run with systemd, Docker, or Kubernetes
./target/release/middleware
```

## 📊 Monitoring

The service exposes Prometheus metrics on `:9090/metrics`:

- `atomic_bundler_bundles_total` - Total bundles processed
- `atomic_bundler_bundles_landed` - Successfully landed bundles
- `atomic_bundler_payment_amount_wei` - Payment amounts
- `atomic_bundler_relay_latency_seconds` - Relay response times

## 🔒 Security

- **Rate limiting** - Configurable per-minute limits
- **Spending caps** - Per-bundle and daily limits
- **Admin API** - Protected with API keys
- **Input validation** - Comprehensive transaction validation
- **Audit logging** - All operations logged

## 🛠️ Development

### Prerequisites

- Rust 1.75+
- SQLite 3
- Docker (optional)

### Commands

```bash
make help           # Show all available commands
make dev            # Run development checks
make test           # Run tests
make watch          # Watch for changes
make lint           # Run linting
make audit          # Security audit
```

### Testing

```bash
# Run all tests
make test

# Run with coverage
cargo llvm-cov --workspace --lcov --output-path lcov.info

# Integration tests
cargo test --test integration
```

## 📚 Documentation

- [Architecture Guide](ARCHITECTURE.md) - Detailed system design
- [Product Requirements](PRD.md) - Business requirements and specifications
- [API Documentation](https://docs.rs/atomic-bundler) - Generated API docs

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run `make pre-commit` to ensure quality
5. Submit a pull request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🆘 Support

- **Issues**: [GitHub Issues](https://github.com/atomic-bundler/atomic-bundler/issues)
- **Documentation**: [docs.rs](https://docs.rs/atomic-bundler)
- **Email**: support@atomicbundler.org

---

**Built with ❤️ by the Atomic Bundler community**
