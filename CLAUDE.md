# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Environment

This project uses Nix for environment management. All commands should be run inside nix-shell:

```bash
nix-shell  # Enter development environment
cargo build --bin btc_options_api  # Build the main API server
cargo run --bin btc_options_api    # Run the API server
cargo test                          # Run all tests
cargo test -- --ignored             # Run integration tests (requires external services)
```

## Architecture Overview

This is a Bitcoin options trading API that acts as an **options seller**. The system manages a pool of BTC collateral and allows selling options contracts with proper risk management.

### Core Components

1. **main.rs** - API server with two key responsibilities:
   - Options pricing and risk calculation endpoints
   - Contract creation with collateral validation
   
2. **External Oracle Integration**:
   - **price_oracle.rs** - gRPC client connecting to `localhost:50051` for real-time BTC prices from aggregated exchange data
   - **iv_oracle.rs** - Fetches implied volatility from Deribit API with in-memory caching, handles both standard (19SEP25) and daily (6SEP25) expiry formats
   
3. **Risk Management** (risk_manager.rs):
   - Calculates position-specific risk using Black-Scholes probabilities
   - Enforces portfolio-wide collateral limits with safety margins
   - Max quantity = Available collateral ÷ Position margin requirement
   
4. **Wallet Integration** (mutiny_wallet.rs):
   - Fetches real BTC balance from blockchain via Mutiny API
   - Supports mainnet/testnet/signet networks

### Key Business Logic

The system operates as an **options seller**, meaning:
- Pool balance represents collateral for underwriting options
- Risk calculations focus on maximum potential losses
- Put options: Max loss = Strike - Premium (if BTC → 0)
- Call options: Max loss capped at 3x spot price movement
- 20% safety margin (configurable via RISK_MARGIN)

### Options Table Response

The `/optionsTable` endpoint returns an array of options with:
- `side`: Call or Put
- `strike_price`: Strike price in USD
- `expire`: Expiry duration (1d, 2d, 3d, 5d, 7d)
- `premium`: Option premium in BTC
- `max_quantity`: Risk-based maximum tradeable quantity
- `iv`: Implied volatility from Deribit oracle
- `delta`: Option delta calculated using Black-Scholes

### External Dependencies

1. **Oracle Aggregator** (REQUIRED) - Must be running on `localhost:50051`
   - Located in `../oracle-node/` repository
   - Aggregates prices from Binance, Coinbase, Kraken
   - Provides median price within 60-second window

2. **Deribit API** - For implied volatility data
   - Falls back to mock API if unavailable

### Database

SQLite database stores:
- Active contracts
- Premium history for price tracking
- Database pool managed by r2d2

### gRPC Protocol

The project uses Protocol Buffers for oracle communication:
- `proto/oracle.proto` - Defines price aggregation service
- Auto-generated via `build.rs` during compilation

## Configuration

Required `.env` settings:
```
POOL_ADDRESS=<your_btc_address>    # REQUIRED - Bitcoin address with pool funds
POOL_NETWORK=signet                # Network selection
COLLATERAL_RATE=0.5                # 50% of pool available for trading
RISK_MARGIN=1.2                    # 20% safety margin
```

## Important Implementation Details

1. **Timestamp Handling**: IV oracle expects milliseconds, but contract expiries are stored as seconds
2. **Date Parsing**: Supports both single-digit (6SEP25) and double-digit (19SEP25) Deribit expiry formats
3. **Risk Calculation**: Uses cumulative normal distribution for ITM probability calculations
4. **Collateral Check**: Every new contract validates against total portfolio risk, not just individual position
5. **Quantity Validation**: POST /contract checks requested quantity against calculated max_quantity before accepting the contract. Returns error if quantity exceeds risk-based limits

## Testing Approach

- Unit tests: Standard Rust testing
- Integration tests: Marked with `#[ignore]`, require external services running
- Manual testing scripts exist but should only be created upon request

## Documentation and Test Script Guidelines

**DO NOT** automatically generate:
- Test scripts (*.sh files) for every implementation
- Documentation files (*.md) for every feature addition
- Example files or demo scripts unless specifically requested

**ONLY** create documentation/scripts when:
- User explicitly requests documentation or test scripts
- Comprehensive refactoring requires updated documentation
- Major architectural changes necessitate new guides
- User asks questions that indicate documentation would help

Focus on implementing the requested functionality without creating supplementary files unless asked.