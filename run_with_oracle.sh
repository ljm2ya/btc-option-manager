#!/bin/bash

echo "BTC Options API with Oracle Integration"
echo "======================================="
echo ""

# Check if .env exists
if [ ! -f .env ]; then
    echo "Creating .env file from template..."
    cp .env.example .env
    echo ""
    echo "IMPORTANT: Edit .env and set POOL_ADDRESS to your Bitcoin address!"
    echo ""
fi

# Check if oracle is running
echo "Checking Oracle Aggregator connection..."
if ! nc -z localhost 50051 2>/dev/null; then
    echo ""
    echo "❌ Oracle Aggregator is not running on localhost:50051"
    echo ""
    echo "To start the oracle system:"
    echo ""
    echo "1. Start the aggregator server (Terminal 1):"
    echo "   cd /home/zeno/projects/oracle-node/aggregator-server"
    echo "   nix-shell"
    echo "   cargo run"
    echo ""
    echo "2. Start oracle nodes (Terminal 2, 3, 4):"
    echo "   cd /home/zeno/projects/oracle-node"
    echo "   nix-shell"
    echo "   cargo run -- --node-id node1 --aggregator-url http://localhost:50051"
    echo ""
    echo "For detailed setup instructions, see: docs/ORACLE_SETUP.md"
    echo ""
    exit 1
fi

echo "✅ Oracle Aggregator is running!"
echo ""

# Check if POOL_ADDRESS is set
if grep -q "YOUR_POOL_ADDRESS_HERE" .env; then
    echo "❌ Please update POOL_ADDRESS in .env with your actual Bitcoin address"
    exit 1
fi

echo "Starting BTC Options API..."
echo ""
echo "Main API Server: http://localhost:8080"
echo "Mock IV Server:  http://localhost:8081"
echo ""

# Run the application
cargo run