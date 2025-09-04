# Development Guide

This file provides guidance for development with Claude Code (claude.ai/code) and general development workflows.

## Project Overview

The BTC Options API is a Rust-based web service implementing Black-Scholes options pricing with real-world data integration:

- **Core Technologies**: Rust, Actix-web, SQLite, gRPC (Tonic)
- **External Integrations**: Mutiny Wallet (Bitcoin), Deribit (IV data), gRPC Oracle (BTC price)
- **Architecture Pattern**: Modular with dependency injection via AppState

## Development Workflow

### Initial Setup

For installation instructions, see [INSTALL.md](INSTALL.md).

### Common Tasks

```bash
# Build for development
cargo build

# Build optimized release
cargo build --release

# Run with environment checking
./run_with_oracle.sh

# Run tests
cargo test

# Check code without building
cargo check

# Run linter
cargo clippy
```

### NixOS Development

The project includes `shell.nix` for reproducible development:

```bash
nix-shell  # Enter development environment
cargo run  # All dependencies automatically available
```

## Architecture Details

### Core Components

1. **Main API Server** (`main.rs`)
   - Actix-web server on port 8080
   - Handles all trading and analytics endpoints
   - Manages AppState with shared resources

2. **Price Oracle** (`price_oracle.rs`)
   - gRPC client for BTC price data
   - 10-second cache for efficiency
   - Health check on initialization

3. **IV Oracle** (`iv_oracle.rs`)
   - Fetches from Deribit API
   - 15-second auto-refresh
   - Falls back to mock API

4. **Mutiny Wallet** (`mutiny_wallet.rs`)
   - Real Bitcoin balance queries
   - Network-aware (mainnet/testnet/signet)
   - Replaces mock pool data

### Database Schema

```sql
-- contracts table
id INTEGER PRIMARY KEY
side TEXT NOT NULL              -- 'Call' or 'Put'
strike_price REAL NOT NULL
quantity REAL NOT NULL
expires INTEGER NOT NULL        -- Unix timestamp
premium REAL NOT NULL
created_at INTEGER NOT NULL     -- For 24hr analytics

-- premium_history table  
id INTEGER PRIMARY KEY
product_key TEXT NOT NULL       -- Unique product ID
side TEXT NOT NULL
strike_price REAL NOT NULL
expires INTEGER NOT NULL
premium REAL NOT NULL
timestamp INTEGER NOT NULL
UNIQUE(product_key, timestamp)
```

### Key Patterns

1. **Error Handling**
   - Custom `ApiError` type for consistent responses
   - Graceful fallbacks for external services
   - Detailed error messages with remediation steps

2. **Caching Strategy**
   - Price data: 10-second cache
   - IV data: 15-second cache with background refresh
   - Database: Connection pooling with r2d2

3. **Testing Approach**
   - Unit tests: Pure business logic
   - Integration tests: External API interactions (use `#[ignore]`)
   - See [EXTERNAL_API_TEST_REPORT.md](EXTERNAL_API_TEST_REPORT.md)

## Recent Architecture Updates

### gRPC Integration (Latest)
- Replaced HTTP price API with gRPC oracle client
- Added protobuf compilation via `build.rs`
- Comprehensive error handling with setup instructions

### Mutiny Wallet Integration
- Replaced mock pool API with real wallet queries
- Added network configuration support
- Satoshi to BTC conversion handling

### Database Enhancements
- Added timestamps for market analytics
- Premium history tracking for price movements
- Optimized indexes for time-based queries

## Development Guidelines

1. **Always Check Git Status First**
   ```bash
   git status
   git branch  # Never work on main
   ```

2. **Follow Existing Patterns**
   - Check `Cargo.toml` before adding dependencies
   - Match existing code style and error handling
   - Use dependency injection via AppState

3. **External Service Integration**
   - Always implement fallbacks
   - Add detailed error messages
   - Include setup instructions in errors

4. **Testing Philosophy**
   - Unit test business logic
   - Integration test with `#[ignore]` flag
   - Document external dependencies clearly

## Debugging Tips

- Check `.env` configuration first for issues
- Verify external services are running:
  ```bash
  nc -z localhost 50051  # gRPC oracle
  ```
- Use `RUST_LOG=debug` for verbose logging
- Database is at `contracts.db` (SQLite)

## Important Notes

- The oracle-node at `/home/zeno/projects/oracle-node` is a client, NOT the aggregator server
- gRPC aggregator must be running separately on port 50051
- All external APIs have graceful fallbacks except BTC price (required)