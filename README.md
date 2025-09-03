# BTC Options API

This project provides a web API for calculating and managing Bitcoin options contracts. It uses the Black-Scholes model for financial calculations and features a mock backend for simulating real-time financial data, making it a self-contained and easy-to-run application.

The application is built in Rust using the Actix web framework and stores contract data in a local SQLite database with comprehensive tracking of trading history and market metrics.

## Code Structure

The project is organized into several Rust modules:

-   `src/main.rs`: The core application that runs the main API server, defines data structures (like `Contract` and `OptionSide`), handles web requests, manages database operations, and performs financial calculations.
-   `src/mock_apis.rs`: Runs a mock API server on port 8081 to simulate real-world financial data services (BTC price, implied volatility, liquidity pool data).
-   `src/iv_oracle.rs`: Manages implied volatility data by fetching from Deribit API with caching and automatic updates every 15 seconds.
-   `src/price_oracle.rs`: Handles BTC price fetching with caching support and future gRPC integration capability.

## Environment Configuration

The application is configured using a `.env` file in the root of the project. This file allows you to easily change key parameters without modifying the code.

Create a file named `.env` and add the following variables:

```
RISK_FREE_RATE=0.05
COLLATERAL_RATE=0.5

# API URLs
# By default, these point to the local mock server.
# You can change them to point to real financial data APIs.
POOL_API_URL=http://127.0.0.1:8081/pool
PRICE_API_URL=http://127.0.0.1:8081/price
IV_API_URL=http://127.0.0.1:8081/iv
DERIBIT_API_URL=https://www.deribit.com/api/v2

# Oracle Configuration (for future gRPC integration)
AGGREGATOR_URL=http://localhost:50051
```

-   `RISK_FREE_RATE`: The risk-free interest rate, used in the Black-Scholes calculation. (e.g., `0.05` for 5%).
-   `COLLATERAL_RATE`: A factor used to determine the maximum tradeable quantity based on the available liquidity.
-   `POOL_API_URL`, `PRICE_API_URL`, `IV_API_URL`: These are the URLs the application will call to get financial data. If you don't set them, they will default to using the mock server running on port 8081.
-   `DERIBIT_API_URL`: URL for fetching real-time implied volatility data from Deribit exchange.
-   `AGGREGATOR_URL`: gRPC endpoint for the BTC oracle node (future integration).

## Requirements

To build and run this project, you will need:

-   [Rust](https://www.rust-lang.org/tools/install) (latest stable version recommended)
-   `curl` or a similar tool for testing the API from the command line

### System Dependencies

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install build-essential pkg-config libssl-dev
```

**macOS (with Homebrew):**
```bash
brew install pkg-config openssl
```

**Windows:**
- Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022) or Visual Studio
- OpenSSL is typically bundled with Rust on Windows

**NixOS/Nix Users:**
- Use the provided `shell.nix` for automatic dependency management

The project uses:
- Bundled SQLite (no separate installation needed)
- OpenSSL for secure connections (install via system package manager)
- gRPC dependencies (tonic, prost) for future oracle integration

## How to Run

1.  **Clone the repository:**
    ```bash
    git clone <repository-url>
    cd btc_options_api
    ```

2.  **(Optional) Create the `.env` file:**
    Create a `.env` file in the project root and customize the configuration variables as described above.

3.  **Build and Run the Server:**
    Use Cargo to build and run the application. This will start both the main API server (on port 8080) and the mock API server (on port 8081).
    
    **Standard method:**
    ```bash
    cargo run
    ```
    
    **For NixOS/Nix users:**
    ```bash
    nix-shell
    cargo run
    ```

## API Endpoints

Once the server is running, you can interact with the following endpoints.

### `GET /optionsTable`

Generates a table of available Call and Put options based on the provided strike prices and expiration dates.

**Query Parameters:**

-   `strike_prices`: A comma-separated list of strike prices (e.g., `100000,110000`).
-   `expires`: A comma-separated list of expiration durations (e.g., `1d,7d,30m`).

**Example Request:**

```bash
curl "http://127.0.0.1:8080/optionsTable?strike_prices=100000,110000&expires=1d,7d"
```

### `POST /contract`

Creates and saves a new options contract to the database.

**Request Body (JSON):**

```json
{
  "side": "Call",
  "strike_price": 110000.0,
  "quantity": 1.5,
  "expires": 1735689600,
  "premium": 5000.0
}
```

-   `expires` must be a future Unix timestamp.

**Example Request (Windows cmd):**

```bash
curl -X POST http://127.0.0.1:8080/contract -H "Content-Type: application/json" -d "{\"side\": \"Call\", \"strike_price\": 110000.0, \"quantity\": 1.5, \"expires\": 1735689600, \"premium\": 5000.0}"
```

### `GET /delta`

Calculates and returns the total delta for all non-expired contracts currently in the database.

**Example Request:**

```bash
curl http://127.0.0.1:8080/delta
```

### `GET /contracts`

A debugging endpoint that returns a list of all contracts currently stored in the database.

**Example Request:**

```bash
curl http://127.0.0.1:8080/contracts
```

### `GET /topBanner`

Returns key market statistics for the trading interface banner.

**Response Fields:**
- `volume_24hr`: Total trading volume in the last 24 hours (sum of all contract quantities)
- `open_interest_usd`: Total value of all open (non-expired) contracts in USD
- `contract_count`: Number of open contracts

**Example Request:**

```bash
curl http://127.0.0.1:8080/topBanner
```

### `GET /marketHighlights`

Returns the top 6 products by 24-hour trading volume with price movement data.

**Response Fields (array of items):**
- `product_symbol`: Product identifier (format: BTC-{expire}-{strike}-{side})
- `side`: Option type (Call/Put)
- `strike_price`: Strike price
- `expire`: Expiration time (e.g., "1d", "7h", "30m")
- `volume_24hr`: 24-hour trading volume
- `price_change_24hr_percent`: Premium price change percentage over 24 hours

**Example Request:**

```bash
curl http://127.0.0.1:8080/marketHighlights
```

### `GET /topGainers`

Returns the top 5 products by 24-hour percentage change.

**Response Fields (array of items):**
- `product_symbol`: Product identifier
- `side`: Option type
- `strike_price`: Strike price
- `expire`: Expiration time
- `change_24hr_percent`: 24-hour price change percentage
- `last_price`: Latest premium price

**Example Request:**

```bash
curl http://127.0.0.1:8080/topGainers
```

### `GET /topVolume`

Returns the top 5 products by 24-hour trading volume in USD.

**Response Fields (array of items):**
- `product_symbol`: Product identifier
- `side`: Option type
- `strike_price`: Strike price
- `expire`: Expiration time
- `volume_usd`: 24-hour trading volume in USD
- `last_price`: Latest premium price

**Example Request:**

```bash
curl http://127.0.0.1:8080/topVolume
```

## Database Schema

The application uses SQLite with two main tables:

### contracts
Stores all option contracts with:
- `id`: Primary key
- `side`: Option type (Call/Put)
- `strike_price`: Strike price
- `quantity`: Contract quantity
- `expires`: Expiration timestamp
- `premium`: Premium price
- `created_at`: Creation timestamp (for 24hr calculations)

### premium_history
Tracks premium price history for products:
- `id`: Primary key
- `product_key`: Unique product identifier
- `side`, `strike_price`, `expires`: Product details
- `premium`: Premium price at timestamp
- `timestamp`: Record timestamp 