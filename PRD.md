# Product Requirements Document (PRD)

## Executive Summary

**Product**: Atomic Bundler - Builder-Paid Atomic Bundles Middleware

**Vision**: Enable users to submit Ethereum transactions without paying priority fees while ensuring builders receive fair compensation through atomic bundle mechanisms.

**Mission**: Provide a production-ready middleware service that accepts zero-priority-fee transactions, calculates appropriate builder payments, and submits atomic bundles to multiple MEV builder relays.

## Problem Statement

### Current Challenges

1. **High Transaction Costs**: Users face high priority fees during network congestion
2. **MEV Complexity**: Direct interaction with MEV infrastructure is complex for applications
3. **Builder Relationships**: Establishing relationships with multiple builders is challenging
4. **Payment Uncertainty**: Unclear payment models for builder services

### Target Users

- **DeFi Applications**: DEXs, lending protocols, yield farming platforms
- **Wallet Providers**: MetaMask, Coinbase Wallet, hardware wallet integrations  
- **Trading Bots**: Arbitrage and MEV-aware trading systems
- **Enterprise Users**: Institutions requiring predictable transaction costs

## Product Goals

### Primary Goals

1. **Cost Reduction**: Reduce user transaction costs by eliminating priority fees
2. **Reliability**: Ensure high transaction inclusion rates across multiple builders
3. **Transparency**: Provide clear pricing and payment models
4. **Scalability**: Support high transaction volumes with low latency

### Secondary Goals

1. **Developer Experience**: Simple API integration for applications
2. **Monitoring**: Comprehensive metrics and observability
3. **Flexibility**: Configurable payment models and builder selection
4. **Security**: Robust validation and spending controls

## Functional Requirements

### Core Features

#### FR1: Transaction Acceptance
- **FR1.1**: Accept signed EIP-1559 transactions with `priority_fee = 0`
- **FR1.2**: Validate transaction format and signature
- **FR1.3**: Reject transactions with non-zero priority fees
- **FR1.4**: Support standard Ethereum transaction types

#### FR2: Payment Calculation
- **FR2.1**: Support multiple payment formulas:
  - Flat: Fixed payment amount
  - Gas: `k1 * gas_used + k2`
  - Base Fee: `k1 * gas_used * (base_fee + tip) + k2`
- **FR2.2**: Enforce per-bundle payment caps
- **FR2.3**: Track and enforce daily spending limits
- **FR2.4**: Support configurable payment parameters

#### FR3: Bundle Creation
- **FR3.1**: Forge payment transaction (tx2) to builder
- **FR3.2**: Create atomic bundle `[tx1, tx2]`
- **FR3.3**: Generate unique bundle identifiers
- **FR3.4**: Set appropriate bundle targeting (block numbers)

#### FR4: Multi-Relay Submission
- **FR4.1**: Submit bundles to multiple builder relays simultaneously
- **FR4.2**: Support major builders (Flashbots, BeaverBuild, Titan)
- **FR4.3**: Handle relay failures gracefully
- **FR4.4**: Track submission status per relay

#### FR5: Bundle Tracking
- **FR5.1**: Provide real-time bundle status updates
- **FR5.2**: Track bundle inclusion in blocks
- **FR5.3**: Handle bundle expiration
- **FR5.4**: Maintain audit trail of all operations

#### FR6: API Interface
- **FR6.1**: RESTful HTTP API
- **FR6.2**: Bundle submission endpoint
- **FR6.3**: Bundle status query endpoint
- **FR6.4**: Health check endpoint
- **FR6.5**: Admin endpoints (config reload, killswitch)

### Advanced Features

#### FR7: Simulation
- **FR7.1**: Pre-execution transaction simulation
- **FR7.2**: Gas estimation for payment calculation
- **FR7.3**: Conflict detection between transactions
- **FR7.4**: Pluggable simulation backends

#### FR8: Payment Modes
- **FR8.1**: Direct ETH transfer payments
- **FR8.2**: ERC-20 permit-based payments (future)
- **FR8.3**: Escrow-based payments (future)

## API Specification

### Bundle Submission

```http
POST /bundles
Content-Type: application/json

{
  "tx1": "0x02f86c01...",
  "payment": {
    "mode": "direct",
    "formula": "basefee",
    "maxAmountWei": "500000000000000",
    "expiry": "2024-01-01T12:00:00Z"
  },
  "targets": {
    "blocks": [18500000, 18500001, 18500002]
  }
}
```

**Response Codes:**
- `200 OK`: Bundle accepted
- `400 Bad Request`: Invalid transaction or parameters
- `429 Too Many Requests`: Rate limit exceeded
- `500 Internal Server Error`: Service error

### Bundle Status

```http
GET /bundles/{bundleId}
```

**Response:**
```json
{
  "bundleId": "550e8400-e29b-41d4-a716-446655440000",
  "state": "sent|landed|expired|failed",
  "tx1Hash": "0xabc123...",
  "tx2Hash": "0xdef456...",
  "blockHash": "0x789abc...",
  "blockNumber": 18500001,
  "paymentAmount": "200000000000000",
  "createdAt": "2024-01-01T12:00:00Z",
  "updatedAt": "2024-01-01T12:00:15Z",
  "expiresAt": "2024-01-01T12:05:00Z",
  "relays": [
    {
      "name": "flashbots",
      "status": "submitted",
      "submittedAt": "2024-01-01T12:00:01Z"
    }
  ]
}
```

## Configuration Requirements

### Payment Configuration
- Support for multiple payment formulas
- Configurable formula parameters (k1, k2)
- Per-bundle and daily spending caps
- Payment validation rules

### Builder Configuration
- Multiple builder relay endpoints
- Builder-specific payment addresses
- Enable/disable individual builders
- Builder-specific timeout settings

### Operational Configuration
- Target block configuration (blocks ahead)
- Resubmission limits
- Rate limiting parameters
- Database connection settings

### Security Configuration
- Admin API key management
- Killswitch configuration
- Audit logging settings
- Input validation rules

## Success Metrics

### Business Metrics
- **Bundle Submission Rate**: Bundles submitted per hour
- **Inclusion Rate**: Percentage of bundles included in blocks
- **Cost Savings**: Average priority fee savings per transaction
- **Builder Coverage**: Number of active builder relationships

### Technical Metrics
- **API Latency**: P95 response time for API endpoints
- **Relay Success Rate**: Percentage of successful relay submissions
- **System Uptime**: Service availability percentage
- **Error Rate**: Percentage of failed operations

### User Experience Metrics
- **Time to Inclusion**: Average time from submission to block inclusion
- **API Response Time**: Average API response time
- **Documentation Usage**: API documentation engagement
- **Support Ticket Volume**: Number of user support requests

## Risk Assessment

### Technical Risks

**Risk**: Relay Failures
- **Impact**: High - Reduced bundle inclusion rates
- **Mitigation**: Multi-relay submission, circuit breakers, health monitoring

**Risk**: Database Performance
- **Impact**: Medium - API latency degradation
- **Mitigation**: Connection pooling, query optimization, caching

**Risk**: Payment Calculation Errors
- **Impact**: High - Financial losses or disputes
- **Mitigation**: Extensive testing, spending caps, audit logging

### Business Risks

**Risk**: Builder Relationship Changes
- **Impact**: Medium - Service disruption
- **Mitigation**: Multiple builder relationships, configuration flexibility

**Risk**: Network Congestion
- **Impact**: Medium - Reduced inclusion rates
- **Mitigation**: Dynamic payment adjustments, multiple block targeting

### Security Risks

**Risk**: API Abuse
- **Impact**: High - Service degradation, financial losses
- **Mitigation**: Rate limiting, authentication, spending caps

**Risk**: Transaction Manipulation
- **Impact**: High - Financial losses, reputation damage
- **Mitigation**: Input validation, signature verification, simulation

## Implementation Phases

### Phase 1: Core Infrastructure (Weeks 1-4)
- Basic crate structure and build system
- Configuration system implementation
- Database schema and operations
- Basic HTTP API framework

### Phase 2: Bundle Processing (Weeks 5-8)
- Transaction validation and parsing
- Payment calculation engine
- Bundle creation and storage
- Basic relay client implementation

### Phase 3: Multi-Relay Integration (Weeks 9-12)
- Multiple builder relay support
- Bundle submission orchestration
- Status tracking and updates
- Error handling and retry logic

### Phase 4: Advanced Features (Weeks 13-16)
- Transaction simulation integration
- Advanced payment modes
- Comprehensive monitoring
- Performance optimizations

### Phase 5: Production Readiness (Weeks 17-20)
- Security hardening
- Load testing and optimization
- Documentation completion
- Deployment automation

## Acceptance Criteria

### Minimum Viable Product (MVP)
- [ ] Accept EIP-1559 transactions with priority_fee = 0
- [ ] Calculate payments using configurable formulas
- [ ] Submit atomic bundles to at least 2 builder relays
- [ ] Provide bundle status tracking via API
- [ ] Enforce spending caps and limits
- [ ] Basic monitoring and health checks

### Production Ready
- [ ] Support all major builder relays (3+)
- [ ] Handle 1000+ bundles per minute
- [ ] 99.9% uptime over 30-day period
- [ ] Comprehensive security testing passed
- [ ] Full documentation and runbooks complete
- [ ] Automated deployment pipeline operational

## Future Enhancements

### Short Term (3-6 months)
- Advanced payment modes (ERC-20, escrow)
- Enhanced simulation capabilities
- Mobile SDK development
- Advanced analytics dashboard

### Medium Term (6-12 months)
- Cross-chain bundle support
- MEV protection features
- Enterprise SLA tiers
- Third-party integrations

### Long Term (12+ months)
- Decentralized relay network
- Advanced MEV strategies
- Institutional custody integration
- Regulatory compliance features

## Conclusion

The Atomic Bundler represents a significant advancement in Ethereum transaction processing, providing users with cost-effective transaction submission while ensuring builders receive fair compensation. The product addresses real market needs while maintaining high standards for security, reliability, and performance.

Success will be measured through user adoption, transaction volume growth, and the establishment of a robust ecosystem of builder relationships. The phased implementation approach ensures steady progress while allowing for market feedback and iterative improvements.
