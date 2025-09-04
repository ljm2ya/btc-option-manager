#!/bin/bash

# Test Oracle Integration Script
# This script verifies that btc-option-manager can connect to oracle-node

echo "Oracle Integration Test"
echo "======================"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to check if a port is open
check_port() {
    local port=$1
    if nc -z localhost $port 2>/dev/null; then
        echo -e "${GREEN}✓${NC} Port $port is open"
        return 0
    else
        echo -e "${RED}✗${NC} Port $port is not open"
        return 1
    fi
}

# Step 1: Check prerequisites
echo "1. Checking prerequisites..."
echo ""

# Check if oracle-node directory exists
if [ -d "/home/zeno/projects/oracle-node" ]; then
    echo -e "${GREEN}✓${NC} oracle-node directory found"
else
    echo -e "${RED}✗${NC} oracle-node directory not found at /home/zeno/projects/oracle-node"
    exit 1
fi

# Check if aggregator-server exists
if [ -d "/home/zeno/projects/oracle-node/aggregator-server" ]; then
    echo -e "${GREEN}✓${NC} aggregator-server directory found"
else
    echo -e "${RED}✗${NC} aggregator-server directory not found"
    exit 1
fi

# Step 2: Check if services are running
echo ""
echo "2. Checking if oracle services are running..."
echo ""

aggregator_running=false
if check_port 50051; then
    aggregator_running=true
    echo -e "${GREEN}✓${NC} Oracle aggregator appears to be running"
else
    echo -e "${YELLOW}!${NC} Oracle aggregator not running on port 50051"
    echo ""
    echo "To start the oracle system, run:"
    echo "  ./start-oracle-system.sh"
    echo ""
    echo "Or manually start:"
    echo "  1. cd /home/zeno/projects/oracle-node/aggregator-server && nix-shell && cargo run"
    echo "  2. cd /home/zeno/projects/oracle-node && nix-shell && cargo run -- --node-id node1"
fi

# Step 3: Check btc-option-manager configuration
echo ""
echo "3. Checking btc-option-manager configuration..."
echo ""

# Check .env file
if [ -f ".env" ]; then
    echo -e "${GREEN}✓${NC} .env file found"
    
    # Check for AGGREGATOR_URL
    if grep -q "AGGREGATOR_URL" .env; then
        AGGREGATOR_URL=$(grep "AGGREGATOR_URL" .env | cut -d'=' -f2 | tr -d ' ')
        echo -e "${GREEN}✓${NC} AGGREGATOR_URL configured: $AGGREGATOR_URL"
    else
        echo -e "${YELLOW}!${NC} AGGREGATOR_URL not found in .env"
        echo "  Adding default AGGREGATOR_URL..."
        echo "AGGREGATOR_URL=http://localhost:50051" >> .env
    fi
else
    echo -e "${RED}✗${NC} .env file not found"
    echo "  Creating .env from template..."
    if [ -f ".env.example" ]; then
        cp .env.example .env
        echo "AGGREGATOR_URL=http://localhost:50051" >> .env
        echo -e "${GREEN}✓${NC} Created .env file"
    else
        echo -e "${RED}✗${NC} .env.example not found"
        exit 1
    fi
fi

# Step 4: Build btc-option-manager
echo ""
echo "4. Building btc-option-manager..."
echo ""

# Enter nix-shell and build
if command -v nix-shell &> /dev/null; then
    echo "Building with nix-shell..."
    nix-shell --run "cargo build" 2>&1 | grep -E "(Compiling|Finished|error)"
    
    if [ ${PIPESTATUS[0]} -eq 0 ]; then
        echo -e "${GREEN}✓${NC} Build successful"
    else
        echo -e "${RED}✗${NC} Build failed"
        echo "  Try running: nix-shell --run 'cargo clean && cargo build'"
        exit 1
    fi
else
    echo "Building with cargo..."
    cargo build 2>&1 | grep -E "(Compiling|Finished|error)"
    
    if [ ${PIPESTATUS[0]} -eq 0 ]; then
        echo -e "${GREEN}✓${NC} Build successful"
    else
        echo -e "${RED}✗${NC} Build failed"
        exit 1
    fi
fi

# Step 5: Run integration test
echo ""
echo "5. Running gRPC connection test..."
echo ""

if [ "$aggregator_running" = true ]; then
    if command -v nix-shell &> /dev/null; then
        nix-shell --run "cargo test test_price_oracle_grpc_connection -- --ignored --nocapture" 2>&1 | tail -20
    else
        cargo test test_price_oracle_grpc_connection -- --ignored --nocapture 2>&1 | tail -20
    fi
    
    if [ ${PIPESTATUS[0]} -eq 0 ]; then
        echo -e "${GREEN}✓${NC} Integration test passed!"
    else
        echo -e "${RED}✗${NC} Integration test failed"
    fi
else
    echo -e "${YELLOW}!${NC} Skipping test - oracle aggregator not running"
fi

# Summary
echo ""
echo "Summary"
echo "======="
echo ""

if [ "$aggregator_running" = true ]; then
    echo -e "${GREEN}✓${NC} Oracle system is ready for use!"
    echo ""
    echo "You can now run btc-option-manager:"
    echo "  ./run_with_oracle.sh"
else
    echo -e "${YELLOW}!${NC} Oracle system needs to be started"
    echo ""
    echo "Run ./start-oracle-system.sh to start all components"
fi