# Oracle Node Setup Guide

This guide explains how to set up and run the gRPC price oracle aggregator required by the BTC Options API.

## Overview

The BTC Options API requires a gRPC oracle aggregator service running on `localhost:50051` to provide real-time BTC price data. The oracle system consists of:

1. **Aggregator Server** - Collects and aggregates price data from multiple oracle nodes
2. **Oracle Nodes** - Fetch price data from exchanges (Binance, Coinbase, Kraken) and submit to aggregator

## Prerequisites

- NixOS or Nix package manager installed
- Access to oracle-node repository at `/home/zeno/projects/oracle-node`
- Multiple terminal windows/tabs

## Setup Instructions

### Step 1: Start the Aggregator Server

Open a terminal and run:

```bash
cd /home/zeno/projects/oracle-node/aggregator-server
nix-shell
cargo build
cargo run
```

The aggregator will start listening on `localhost:50051` for:
- Price data submissions from oracle nodes
- Price queries from client applications (like btc-option-manager)

### Step 2: Start Oracle Nodes

You need at least one oracle node running, but for better reliability, run 3 nodes in separate terminals:

**Terminal 1 - Node 1:**
```bash
cd /home/zeno/projects/oracle-node
nix-shell
cargo run -- --node-id node1 --aggregator-url http://localhost:50051
```

**Terminal 2 - Node 2:**
```bash
cd /home/zeno/projects/oracle-node
nix-shell
cargo run -- --node-id node2 --aggregator-url http://localhost:50051
```

**Terminal 3 - Node 3:**
```bash
cd /home/zeno/projects/oracle-node
nix-shell
cargo run -- --node-id node3 --aggregator-url http://localhost:50051
```

Or use the helper script:
```bash
cd /home/zeno/projects/oracle-node
./scripts/run_multi_nodes.sh
```

Each node will:
- Fetch BTC price from its assigned exchange
- Submit price data to the aggregator
- Participate in consensus for accurate pricing

### Step 3: Verify Setup

Check that everything is running correctly:

```bash
# Check aggregator is listening
nc -z localhost 50051 && echo "✅ Aggregator is running" || echo "❌ Aggregator not found"

# Check processes
ps aux | grep -E "(aggregator|oracle)" | grep -v grep
```

### Step 4: Run BTC Options API

Once the oracle system is running, you can start the BTC Options API:

```bash
cd /home/zeno/projects/btc-option-manager
./run_with_oracle.sh
```

## Testing the Connection

To test the gRPC connection from the BTC Options API:

```bash
cd /home/zeno/projects/btc-option-manager
nix-shell
cargo test test_price_oracle_grpc_connection -- --ignored --nocapture
```

## Architecture Diagram

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│ Exchange APIs   │────▶│  Oracle Nodes    │────▶│  Aggregator     │
│ (Binance, etc)  │     │  (node1,2,3)     │     │  (:50051)       │
└─────────────────┘     └──────────────────┘     └─────────────────┘
                                                           │
                                                           ▼
                                                  ┌─────────────────┐
                                                  │ BTC Options API │
                                                  │   (:8080)       │
                                                  └─────────────────┘
```

## Troubleshooting

### Build Error: "protoc failed"

If you get an error like:
```
Error: Custom { kind: Other, error: "protoc failed: Could not make proto path relative: ../-oracle-node/proto/oracle.proto: No such file or directory\n" }
```

This has been fixed in the latest version. The aggregator-server now correctly references the proto file at `../proto/oracle.proto`.

### Port Already in Use

If port 50051 is already in use:
```bash
# Find process using the port
lsof -i :50051

# Kill the process if needed
kill -9 <PID>
```

### Connection Refused

If you get "connection refused" errors:
1. Ensure aggregator is running first
2. Check firewall settings
3. Verify localhost resolves to 127.0.0.1

### Build Errors

If you encounter build errors:
1. Make sure you're in a nix-shell
2. Run `cargo clean` and rebuild
3. Check that all dependencies are installed
4. Run the test script: `./test-oracle-build.sh`

## Development Tips

- Use `RUST_LOG=debug` for verbose logging
- Monitor aggregator logs for incoming price submissions
- Each oracle node can be configured with different exchanges
- The aggregator uses consensus to determine final price

## Quick Start Script

Create a script to start everything:

```bash
#!/bin/bash
# save as start-oracle-system.sh

echo "Starting Oracle System..."

# Start aggregator
echo "Starting aggregator server..."
gnome-terminal --tab --title="Aggregator" -- bash -c "cd /home/zeno/projects/oracle-node2/aggregator-server && nix-shell --run 'cargo run'; read"

# Wait for aggregator to start
sleep 5

# Start oracle nodes
echo "Starting oracle nodes..."
gnome-terminal --tab --title="Oracle Node 1" -- bash -c "cd /home/zeno/projects/oracle-node2 && nix-shell --run 'cargo run -- --node-id node1 --aggregator-url http://localhost:50051'; read"
gnome-terminal --tab --title="Oracle Node 2" -- bash -c "cd /home/zeno/projects/oracle-node2 && nix-shell --run 'cargo run -- --node-id node2 --aggregator-url http://localhost:50051'; read"
gnome-terminal --tab --title="Oracle Node 3" -- bash -c "cd /home/zeno/projects/oracle-node2 && nix-shell --run 'cargo run -- --node-id node3 --aggregator-url http://localhost:50051'; read"

echo "Oracle system started!"
```

Make it executable: `chmod +x start-oracle-system.sh`