# BTC Options API

This project provides a web API for calculating and managing Bitcoin options contracts. It uses the Black-Scholes model for financial calculations and features a mock backend for simulating real-time financial data, making it a self-contained and easy-to-run application.

The application is built in Rust using the Actix web framework and stores contract data in a local SQLite database.

## Code Structure

The project is organized into two main Rust files:

-   `src/main.rs`: This is the core of the application. It runs the main API server, defines the data structures (like `Contract` and `OptionSide`), handles incoming web requests, interacts with the database, and performs all the financial calculations.
-   `src/mock_apis.rs`: This file runs a secondary, mock API server on a different port. This server's job is to simulate real-world financial data services, such as providing the current price of Bitcoin, the implied volatility for an option, and the size of the liquidity pool. This allows the main application to be developed and tested without needing access to live, expensive financial data feeds.

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
```

-   `RISK_FREE_RATE`: The risk-free interest rate, used in the Black-Scholes calculation. (e.g., `0.05` for 5%).
-   `COLLATERAL_RATE`: A factor used to determine the maximum tradeable quantity based on the available liquidity.
-   `POOL_API_URL`, `PRICE_API_URL`, `IV_API_URL`: These are the URLs the application will call to get financial data. If you don't set them, they will default to using the mock server running on port 8081.

## Requirements

To build and run this project, you will need:

-   [Rust](https://www.rust-lang.org/tools/install) (latest stable version recommended)
-   `curl` or a similar tool for testing the API from the command line.

The project uses a bundled version of SQLite, so you do not need to install it separately.

## How to Run

1.  **Clone the repository:**
    ```bash
    git clone <repository-url>
    cd btc_options_api
    ```

2.  **(Optional) Create the `.env` file:**
    Create a `.env` file in the project root and customize the configuration variables as described above.

3.  **Build and Run the Server:**
    Use the Cargo command to build and run the application. This will start both the main API server (on port 8080) and the mock API server (on port 8081).
    ```bash
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