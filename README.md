# BTC Options API

A web API for calculating and managing Bitcoin options contracts using the Black-Scholes model. Built with Rust and Actix-web, featuring real-time market data integration and comprehensive trading history tracking.

## Quick Start

1. **Clone the repository:**
   ```bash
   git clone <repository-url>
   cd btc-option-manager
   ```

2. **Set up environment:**
   ```bash
   cp .env.example .env
   # Edit .env and set your POOL_ADDRESS
   ```

3. **Run the application:**
   ```bash
   cargo run
   ```

The API will be available at:
- Main API: `http://localhost:8080`
- Mock IV Server: `http://localhost:8081`

## Documentation

- [📦 Installation Guide](docs/INSTALL.md) - Platform-specific setup instructions
- [🛠️ Development Guide](docs/CLAUDE.md) - Development workflow and architecture
- [📡 API Reference](docs/API_REFERENCE.md) - Complete endpoint documentation
- [🔮 Oracle Setup Guide](docs/ORACLE_SETUP.md) - gRPC oracle aggregator setup
- [🏦 Mutiny Wallet Guide](docs/MUTINY_WALLET_GUIDE.md) - Bitcoin wallet integration
- [🔍 External API Report](docs/EXTERNAL_API_TEST_REPORT.md) - Service dependencies & testing

## Core Features

- **Black-Scholes Options Pricing** - Real-time options valuation
- **Market Data Integration** - Live BTC price via gRPC oracle, Deribit IV data
- **Portfolio Management** - Track contracts, calculate delta, monitor positions
- **Market Analytics** - 24hr volume, top gainers, market highlights
- **Wallet Integration** - Real Bitcoin pool balance via Mutiny wallet

## API Endpoints

### Trading Endpoints
- `GET /optionsTable` - Generate available options with pricing
- `POST /contract` - Create new options contract
- `GET /delta` - Calculate total portfolio delta
- `GET /contracts` - List all contracts (debug)

### Market Analytics
- `GET /topBanner` - 24hr volume, open interest, contract count
- `GET /marketHighlights` - Top 6 products by volume
- `GET /topGainers` - Top 5 products by price change
- `GET /topVolume` - Top 5 products by USD volume

## Architecture

```
src/
├── main.rs         # API server and endpoints
├── price_oracle.rs # gRPC price oracle client
├── iv_oracle.rs    # Deribit IV data with caching
├── mutiny_wallet.rs # Bitcoin wallet integration
├── db.rs           # Database operations
└── mock_apis.rs    # Fallback IV data server
```

## External Dependencies

1. **BTC Price Oracle** (Required)
   - gRPC service on `localhost:50051`
   - Provides real-time BTC price data
   - See [External API Report](docs/EXTERNAL_API_TEST_REPORT.md) for details

2. **Mutiny Wallet API**
   - Fetches real Bitcoin pool balance
   - Supports mainnet/testnet/signet

3. **Deribit API**
   - Real-time implied volatility data
   - Falls back to mock API if unavailable

## Configuration

Create `.env` file with:

```env
# Core Settings
RISK_FREE_RATE=0.05      # Risk-free rate for Black-Scholes
COLLATERAL_RATE=0.5      # Max tradeable % of pool

# Wallet Configuration
POOL_ADDRESS=your_btc_address_here  # Required
POOL_NETWORK=signet                 # mainnet/testnet/signet

# External Services
AGGREGATOR_URL=http://localhost:50051  # gRPC price oracle
DERIBIT_API_URL=https://www.deribit.com/api/v2
IV_API_URL=http://127.0.0.1:8081/iv   # Fallback IV
```

## Testing

```bash
# Run all tests
cargo test

# Run integration tests (requires external services)
cargo test -- --ignored

# Test with real wallet
./test_with_real_wallet.sh
```

## License

MIT License - see LICENSE file for details