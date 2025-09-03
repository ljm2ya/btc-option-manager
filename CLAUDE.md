# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Environment

### Standard Setup (Ubuntu/Debian/macOS)

1. Install system dependencies:
   ```bash
   # Ubuntu/Debian
   sudo apt update
   sudo apt install build-essential pkg-config libssl-dev
   
   # macOS
   brew install pkg-config openssl
   ```

2. Install Rust if not already installed:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

3. Run the project:
   ```bash
   cargo run
   ```

### NixOS/Nix Setup

For NixOS users or those preferring Nix, use the provided shell environment:

```bash
nix-shell
```

The nix shell provides:
- rustup for managing Rust toolchain
- clang/llvm for compilation
- pkg-config and openssl for dependencies
- Proper environment variables for Rust development

## Common Development Tasks

### Building the Project
```bash
cargo build
```

### Running the Application
```bash
cargo run
```
This starts both servers:
- Main API server on port 8080
- Mock API server on port 8081

### Running Tests
```bash
cargo test
```

### Checking Code
```bash
cargo check
cargo clippy
```

**Note for NixOS users:** Prefix all cargo commands with `nix-shell` or run `nix-shell` first.

## Architecture Overview

The application is a Bitcoin options trading API that implements the Black-Scholes model for option pricing with comprehensive market data tracking.

### Core Components

1. **Main API Server** (`src/main.rs`)
   - Runs on port 8080
   - Handles options contract management
   - Implements Black-Scholes pricing calculations
   - Uses SQLite database for contract storage and premium history tracking
   - Validates contracts against available collateral
   - Provides market analytics endpoints

2. **Mock API Server** (`src/mock_apis.rs`)
   - Runs on port 8081
   - Simulates external financial data services
   - Provides endpoints for: price, pool liquidity, implied volatility

3. **IV Oracle** (`src/iv_oracle.rs`)
   - Fetches real-time implied volatility data from Deribit API
   - Caches IV data with automatic updates every 15 seconds
   - Falls back to mock API when data unavailable
   - Uses custom StrikePrice wrapper for HashMap compatibility

4. **Price Oracle** (`src/price_oracle.rs`)
   - Manages BTC price fetching with 10-second caching
   - Currently uses HTTP with future gRPC support planned
   - Provides centralized price data for all endpoints

### API Endpoints

**Core Trading Endpoints:**
- `GET /optionsTable` - Generate available options with pricing
- `POST /contract` - Create new options contract with validation
- `GET /delta` - Calculate total portfolio delta
- `GET /contracts` - List all stored contracts (debug)

**Market Analytics Endpoints:**
- `GET /topBanner` - 24hr volume, open interest (USD), contract count
- `GET /marketHighlights` - Top 6 products by volume with 24hr price movement
- `GET /topGainers` - Top 5 products by 24hr percentage change
- `GET /topVolume` - Top 5 products by 24hr trading volume in USD

### Data Flow

1. User requests options table → IV Oracle fetches market data → Black-Scholes calculates premium
2. User creates contract → Validate against collateral → Store in SQLite → Update premium history
3. Delta calculation → Fetch active contracts → Calculate individual deltas → Sum total exposure
4. Market analytics → Query 24hr data → Aggregate metrics → Calculate price movements

### Environment Configuration

The application uses `.env` file for configuration:
- `RISK_FREE_RATE` - Risk-free interest rate for Black-Scholes
- `COLLATERAL_RATE` - Maximum tradeable percentage of pool
- `POOL_API_URL` - Liquidity pool API endpoint
- `PRICE_API_URL` - BTC price API endpoint
- `IV_API_URL` - Implied volatility API endpoint
- `DERIBIT_API_URL` - Deribit API for real IV data (defaults to production: https://www.deribit.com/api/v2)
- `AGGREGATOR_URL` - gRPC endpoint for BTC oracle node (future integration)

### Database Schema

SQLite database `contracts.db` with tables:

**contracts** - Stores all option contracts:
```sql
contracts (
    id INTEGER PRIMARY KEY,
    side TEXT NOT NULL,        -- 'Call' or 'Put'
    strike_price REAL NOT NULL,
    quantity REAL NOT NULL,
    expires INTEGER NOT NULL,  -- Unix timestamp
    premium REAL NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
)
```

**premium_history** - Tracks premium price changes:
```sql
premium_history (
    id INTEGER PRIMARY KEY,
    product_key TEXT NOT NULL,
    side TEXT NOT NULL,
    strike_price REAL NOT NULL,
    expires INTEGER NOT NULL,
    premium REAL NOT NULL,
    timestamp INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    UNIQUE(product_key, timestamp)
)
```

## Recent Updates

### New Features (Latest Implementation)
- **Market Analytics Endpoints**: Added comprehensive market data tracking with 24-hour metrics
- **Price Oracle Module**: Centralized BTC price management with caching
- **Database Enhancements**: Added timestamps and premium history tracking
- **gRPC Preparation**: Added dependencies for future oracle node integration

### Technical Improvements
- Fixed HashMap compatibility issues in IV Oracle using custom StrikePrice wrapper
- Integrated price oracle across all endpoints for consistent pricing
- Added indexes to database for efficient time-based queries
- Updated Nix shell configuration with OpenSSL and pkg-config dependencies

# important-instruction-reminders
Do what has been asked; nothing more, nothing less.
NEVER create files unless they're absolutely necessary for achieving your goal.
ALWAYS prefer editing an existing file to creating a new one.
NEVER proactively create documentation files (*.md) or README files. Only create documentation files if explicitly requested by the User.

      
      IMPORTANT: this context may or may not be relevant to your tasks. You should not respond to this context or otherwise consider it in your response unless it is highly relevant to your task. Most of the time, it is not relevant.