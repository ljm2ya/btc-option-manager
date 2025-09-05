# BTC Options Trading API

A Bitcoin options seller API built with Rust and Actix-web, implementing Black-Scholes pricing with real-time market data integration and comprehensive risk management.

## 🚀 Quick Start

1. **Clone and setup:**
   ```bash
   git clone <repository-url>
   cd btc-option-manager
   cp .env.example .env
   # Configure your POOL_ADDRESS in .env
   ```

2. **Run with Nix (Recommended):**
   ```bash
   nix-shell
   cargo run --bin btc_options_api
   ```

3. **Or install dependencies manually:**
   - See [Installation Guide](docs/INSTALL.md) for platform-specific setup

The API will be available at:
- Local: `http://localhost:8080`
- External: `http://<your-ip>:8080`

**Note**: For external access, ensure firewall allows ports 8080 and 8081

## 📊 Core Features

### Options Selling Platform
- **Risk-Based Position Sizing** - Intelligent collateral management with 20% safety margins
- **Real-Time Options Table** - 110 options (Call/Put × 11 strikes × 5 expiries) auto-generated
- **Black-Scholes Pricing** - Professional options valuation with Greeks calculation
- **Portfolio Management** - Track positions, calculate delta, monitor risk exposure

### Market Data Integration
- **BTC Price Oracle** - Real-time prices via gRPC aggregator (3+ exchange sources)
- **Implied Volatility** - Live IV data from Deribit with intelligent caching
- **Wallet Integration** - Real Bitcoin pool balance via Mutiny API

### Market Analytics
- **Trading Dashboard** - 24hr volume, open interest, contract counts
- **Market Intelligence** - Top gainers, volume leaders, market highlights
- **Portfolio Analytics** - Real-time delta, risk exposure, collateral utilization

## 🔧 API Endpoints

### Core Trading
```bash
GET  /health              # Server health check
GET  /optionsTable        # 110 options with risk-based quantities
POST /contract           # Create options contract with validation
GET  /contracts          # List all contracts
GET  /delta              # Portfolio delta calculation
```

### Market Analytics
```bash
GET  /topBanner          # 24hr volume, open interest, contract count
GET  /marketHighlights   # Top 6 products by volume
GET  /topGainers         # Top 5 products by price change
GET  /topVolume          # Top 5 products by USD volume
```

See [API Reference](docs/API_REFERENCE.md) for detailed documentation.

## 🏗️ Architecture

```
src/
├── main.rs              # API server & endpoints
├── price_oracle.rs      # gRPC BTC price client
├── iv_oracle.rs         # Deribit IV with caching
├── risk_manager.rs      # Risk-based position sizing
├── mutiny_wallet.rs     # Bitcoin wallet integration
├── db.rs                # SQLite operations
└── utils.rs             # Helper functions
```

### Risk Management System
- **Position-Specific Risk**: Max loss = (Strike - Premium) × Quantity for puts
- **Portfolio-Wide Limits**: Available collateral = Total - Existing exposure
- **Configurable Margins**: 20% safety buffer (configurable via `RISK_MARGIN`)
- **Max Quantity Calculation**: Risk-aware position limits per option

### Options Table Generation
- **Dynamic Strike Prices**: 11 strikes centered around current BTC price (±$5k steps)
- **Fixed Expiries**: 1d, 2d, 3d, 5d, 7d from current time
- **Real-Time Data**: Live BTC prices + Deribit IV data
- **Risk Integration**: Max quantities calculated per option

## ⚙️ Configuration

Create `.env` file:

```env
# Required - Bitcoin Pool Settings
POOL_ADDRESS=your_btc_address_here    # Your Bitcoin address with pool funds
POOL_NETWORK=signet                   # Network: mainnet/testnet/signet

# Risk Management
COLLATERAL_RATE=0.5                   # 50% of pool available for trading
RISK_MARGIN=1.2                       # 20% safety margin
RISK_FREE_RATE=0.05                   # 5% risk-free rate for Black-Scholes

# External Services (Optional - good defaults provided)
AGGREGATOR_URL=http://localhost:50051  # gRPC price oracle
DERIBIT_API_URL=https://www.deribit.com/api/v2
IV_API_URL=http://127.0.0.1:8081/iv   # Fallback IV server
```

## 🔗 External Dependencies

### Required
1. **gRPC Price Oracle** - Must run on `localhost:50051`
   - Aggregates BTC prices from multiple exchanges
   - Provides median price within 60-second window
   - **Critical**: API won't start without this service

### Optional (with fallbacks)
2. **Deribit API** - Real-time implied volatility
   - Falls back to mock IV server if unavailable
3. **Mutiny Wallet API** - Real Bitcoin balance queries
   - Falls back to mock data if unavailable

## 🧪 Testing

### Unit & Integration Tests
```bash
# Run all tests
cargo test

# Integration tests (requires external services)
cargo test -- --ignored
```

### API Testing Scripts
```bash
# Comprehensive API test suite
bash test_api.sh

# Market analytics validation
bash verify_analytics.sh

# Generate test data (10 contracts with varied dates)
bash generate_test_contracts.sh
```

### Test Data Generation
The `generate_test_contracts.sh` script creates realistic test data:
- Fetches real options from `/optionsTable` endpoint
- Creates 10 contracts with varied parameters:
  - Mix of Call and Put options
  - Quantities: 5-30% of max allowed
  - Creation dates: Spread over past 7 days
  - Real strike prices and premiums
- Useful for testing analytics endpoints with historical data

**Note**: Requires server running with price oracle available

## 💡 Key Improvements (Recent)

### Risk Management Implementation
- Replaced simple collateral allocation with proper risk-based position sizing
- Added Black-Scholes probability calculations for expected loss
- Portfolio-wide risk tracking with safety margins
- Real-time collateral utilization display

### Options Table Automation  
- Auto-generates 110 options based on current market conditions
- No manual parameter entry required
- Integrated with live price and IV oracles
- Risk-aware maximum quantities per option

### Market Analytics Suite
- Added comprehensive market analytics endpoints
- 24hr volume tracking, price change analysis
- Real-time portfolio delta calculation
- Professional trading dashboard data

### Enhanced Testing
- Created comprehensive test suite with adaptive quantities
- Added market analytics verification scripts
- Improved error handling and validation testing

## 📈 Business Model

**Options Seller**: The system acts as an options seller, managing a Bitcoin collateral pool:
- Pool balance represents available collateral for underwriting options
- Risk calculations focus on maximum potential losses
- Put options: Max loss = Strike - Premium (if BTC → 0)
- Call options: Max loss capped at 3x spot price movement
- All positions subject to 20% safety margin

## 🛠️ Development

```bash
# Development with Nix (recommended)
nix-shell
cargo run --bin btc_options_api

# Manual development
cargo build
cargo test
cargo clippy
```

## 📝 License

MIT License - see LICENSE file for details