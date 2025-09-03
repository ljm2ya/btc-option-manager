# Installation Guide

This guide provides step-by-step instructions for installing and running the BTC Options API on different operating systems.

## Prerequisites

- Rust (latest stable version)
- Git
- System dependencies (varies by OS)

## Platform-Specific Instructions

### Ubuntu/Debian Linux

1. **Install system dependencies:**
   ```bash
   sudo apt update
   sudo apt install build-essential pkg-config libssl-dev curl git
   ```

2. **Install Rust:**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source "$HOME/.cargo/env"
   ```

3. **Clone the repository:**
   ```bash
   git clone <repository-url>
   cd btc-option-manager
   ```

4. **Build and run:**
   ```bash
   cargo build --release
   cargo run
   ```

### macOS

1. **Install Homebrew (if not already installed):**
   ```bash
   /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
   ```

2. **Install system dependencies:**
   ```bash
   brew install pkg-config openssl
   ```

3. **Install Rust:**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source "$HOME/.cargo/env"
   ```

4. **Clone and run:**
   ```bash
   git clone <repository-url>
   cd btc-option-manager
   cargo build --release
   cargo run
   ```

### Windows

1. **Install Visual Studio Build Tools:**
   - Download from [Visual Studio Downloads](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022)
   - Install "Desktop development with C++" workload

2. **Install Rust:**
   - Download and run [rustup-init.exe](https://rustup.rs/)
   - Follow the installation prompts

3. **Clone and run:**
   ```powershell
   git clone <repository-url>
   cd btc-option-manager
   cargo build --release
   cargo run
   ```

### NixOS / Nix Package Manager

For NixOS users or those with Nix package manager installed:

1. **Clone the repository:**
   ```bash
   git clone <repository-url>
   cd btc-option-manager
   ```

2. **Enter Nix shell and run:**
   ```bash
   nix-shell
   cargo run
   ```

## Troubleshooting

### OpenSSL Errors

If you encounter OpenSSL-related errors during compilation:

**Ubuntu/Debian:**
```bash
sudo apt install libssl-dev pkg-config
```

**macOS:**
```bash
brew install openssl
export PKG_CONFIG_PATH="/usr/local/opt/openssl/lib/pkgconfig"
```

**Windows:**
- OpenSSL should be bundled with Rust on Windows
- If issues persist, install OpenSSL manually from [slproweb.com](https://slproweb.com/products/Win32OpenSSL.html)

### Permission Errors

If you get permission errors when running cargo:
```bash
chmod +x ~/.cargo/bin/cargo
```

### Build Performance

For faster builds, you can use:
```bash
cargo build --release  # Optimized build
cargo run --release    # Run optimized version
```

## Verifying Installation

After successful installation, verify the application is running:

1. Check main API server:
   ```bash
   curl http://127.0.0.1:8080/contracts
   ```

2. Check mock API server:
   ```bash
   curl http://127.0.0.1:8081/price
   ```

Both should return responses indicating the servers are running.