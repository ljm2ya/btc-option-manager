# Installation Guide

Platform-specific installation instructions for the BTC Options API.

## Prerequisites

- Rust (latest stable)
- Git
- System dependencies (see platform sections)
- gRPC Oracle Aggregator running on port 50051

## Platform Instructions

### Ubuntu/Debian Linux

```bash
# Install system dependencies
sudo apt update
sudo apt install build-essential pkg-config libssl-dev curl git

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Clone and run
git clone <repository-url>
cd btc-option-manager
cp .env.example .env  # Configure your settings
cargo run
```

### macOS

```bash
# Install Homebrew (if needed)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install dependencies
brew install pkg-config openssl

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Clone and run
git clone <repository-url>
cd btc-option-manager
cp .env.example .env  # Configure your settings
cargo run
```

### Windows

1. **Install Visual Studio Build Tools**
   - Download from [Visual Studio Downloads](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022)
   - Select "Desktop development with C++" workload

2. **Install Rust**
   - Download and run [rustup-init.exe](https://rustup.rs/)

3. **Clone and run**
   ```powershell
   git clone <repository-url>
   cd btc-option-manager
   copy .env.example .env  # Configure your settings
   cargo run
   ```

### NixOS / Nix

```bash
# Clone repository
git clone <repository-url>
cd btc-option-manager

# Enter Nix shell (installs all dependencies)
nix-shell

# Configure and run
cp .env.example .env  # Configure your settings
cargo run
```

## Troubleshooting

### OpenSSL Errors

**Linux**: `sudo apt install libssl-dev pkg-config`

**macOS**: 
```bash
brew install openssl
export PKG_CONFIG_PATH="/usr/local/opt/openssl/lib/pkgconfig"
```

**Windows**: OpenSSL is bundled with Rust. If issues persist, install from [slproweb.com](https://slproweb.com/products/Win32OpenSSL.html)

### Build Performance

```bash
cargo build --release  # Optimized build
cargo run --release    # Run optimized
```

### Permission Errors

```bash
chmod +x ~/.cargo/bin/cargo
```

## Verification

After installation, verify both servers are running:

```bash
# Main API (should return empty array or contracts)
curl http://127.0.0.1:8080/contracts

# Mock IV server (should return IV data)
curl http://127.0.0.1:8081/iv?symbol=BTCUSD
```

## Next Steps

1. Configure `.env` file with your settings
2. Ensure gRPC Oracle is running on port 50051
3. See [Development Guide](CLAUDE.md) for development workflow