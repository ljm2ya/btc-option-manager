#!/bin/bash

echo "Testing BTC Options API with Real Services"
echo "=========================================="
echo ""
echo "This test uses:"
echo "- Real Mutiny wallet for pool balance"
echo "- gRPC oracle aggregator for BTC prices"
echo "- Deribit API for implied volatility"
echo ""

# Example signet address (you'll need to replace with your actual pool address)
# This is just a demonstration address
EXAMPLE_POOL_ADDRESS="tb1qxyz123abc456def789ghi012jkl345mno678pqr"

# Create a test .env file
cat > .env << EOF
RISK_FREE_RATE=0.05
COLLATERAL_RATE=0.5

# Mutiny Wallet Configuration
POOL_ADDRESS=$EXAMPLE_POOL_ADDRESS
POOL_NETWORK=signet

# API URLs
PRICE_API_URL=http://127.0.0.1:8081/price
IV_API_URL=http://127.0.0.1:8081/iv
DERIBIT_API_URL=https://www.deribit.com/api/v2

# Oracle Configuration
AGGREGATOR_URL=http://localhost:50051
EOF

echo "Created .env file with example configuration"
echo ""
echo "IMPORTANT: Replace POOL_ADDRESS with your actual Bitcoin address!"
echo ""
echo "To test the API:"
echo "1. Update POOL_ADDRESS in .env with your real Bitcoin address"
echo "2. Run: cargo run"
echo "3. In another terminal, test the endpoints:"
echo ""
echo "# Get options table"
echo "curl 'http://127.0.0.1:8080/optionsTable?strike_prices=50000,55000,60000&expires=1d,7d,30d'"
echo ""
echo "# Create a contract"
echo 'curl -X POST http://127.0.0.1:8080/contract \
  -H "Content-Type: application/json" \
  -d '"'"'{
    "side": "Call",
    "strike_price": 50000,
    "quantity": 0.1,
    "expires": 1735689600,
    "premium": 0.05
  }'"'"''
echo ""
echo "# Get top banner statistics"
echo "curl http://127.0.0.1:8080/topBanner"
echo ""

# Make the script executable
chmod +x test_with_real_wallet.sh