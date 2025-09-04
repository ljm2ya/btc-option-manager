# Oracle Integration Summary

## Overview

This document summarizes the complete integration of btc-option-manager with the oracle-node gRPC price service.

## Changes Made

### 1. Mutiny Wallet Integration (Completed)
- **File**: `src/mutiny_wallet.rs` (existing)
- **Changes**: Replaced mock pool API with real Mutiny wallet balance queries
- **Features**:
  - Fetches real Bitcoin balance from configured address
  - Supports mainnet, testnet, and signet networks
  - Converts satoshis to BTC automatically
  - Pool address fetched from `POOL_ADDRESS` environment variable

### 2. gRPC Price Oracle Integration (Completed)
- **File**: `src/price_oracle.rs` (completely rewritten)
- **Changes**: Replaced HTTP-based mock API with gRPC client
- **Key Components**:
  ```rust
  // Uses oracle-node's OracleService
  use oracle::oracle_service_client::OracleServiceClient;
  
  // Connects to aggregator
  let client = OracleServiceClient::connect(aggregator_url).await?;
  
  // Fetches aggregated price
  let response = client.get_aggregated_price(request).await?;
  ```
- **Features**:
  - Health check on initialization
  - 10-second price caching
  - Detailed error messages with setup instructions
  - Graceful connection handling

### 3. Proto File Configuration
- **File**: `proto/oracle.proto`
- **Content**: Includes both OracleService (used by oracle-node) and OracleAggregator (for compatibility)
- **Build**: `build.rs` compiles the proto file using tonic-build

### 4. Environment Configuration
- **New Variable**: `AGGREGATOR_URL` (required)
- **Default**: `http://localhost:50051`
- **Purpose**: Endpoint for gRPC oracle aggregator service

### 5. Documentation Updates
- **ORACLE_SETUP.md**: Complete setup guide for oracle system
- **CLAUDE.md**: Development guide with architecture details
- **EXTERNAL_API_TEST_REPORT.md**: Test results for external dependencies

### 6. Helper Scripts
- **start-oracle-system.sh**: Starts aggregator and multiple oracle nodes
- **run_with_oracle.sh**: Runs btc-option-manager with oracle checking
- **test-oracle-integration.sh**: Comprehensive integration test script

## Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│ Exchange APIs   │────▶│  Oracle Nodes    │────▶│  Aggregator     │
│ (Binance, etc)  │     │  (node1,2,3)     │     │  (:50051)       │
└─────────────────┘     └──────────────────┘     └─────────────────┘
                                                           │
                                                           ▼ gRPC
                                                  ┌─────────────────┐
                                                  │ BTC Options API │
                                                  │   (:8080)       │
                                                  └─────────────────┘
```

## Testing

### Unit Tests
All existing unit tests have been updated to work with the new architecture:
- Mock implementations for testing
- Integration tests marked with `#[ignore]`

### Integration Testing
Run the integration test:
```bash
./test-oracle-integration.sh
```

Or manually:
```bash
cargo test test_price_oracle_grpc_connection -- --ignored --nocapture
```

## Important Notes

1. **Oracle-node vs Oracle-node2**: We use `/home/zeno/projects/oracle-node` (NOT oracle-node2)
2. **Service Compatibility**: oracle-node provides `OracleService`, not `OracleAggregator`
3. **Required Services**: The aggregator must be running on port 50051 before starting btc-option-manager
4. **Graceful Degradation**: Only the price oracle requires the aggregator; IV data and pool data have fallbacks

## Quick Start

1. Start the oracle system:
   ```bash
   ./start-oracle-system.sh
   ```

2. Run btc-option-manager:
   ```bash
   ./run_with_oracle.sh
   ```

3. Test the integration:
   ```bash
   ./test-oracle-integration.sh
   ```

## Troubleshooting

### Connection Refused
If you see "connection refused" errors:
1. Ensure aggregator is running: `nc -z localhost 50051`
2. Check the aggregator logs for errors
3. Verify no firewall blocking port 50051

### Proto Compilation Errors
If you see proto compilation errors:
1. Clean the build: `cargo clean`
2. Ensure you're in nix-shell: `nix-shell`
3. Rebuild: `cargo build`

### Missing Dependencies
All dependencies should be available in the nix-shell environment. If not:
1. Exit shell: `exit`
2. Re-enter: `nix-shell`
3. Try again