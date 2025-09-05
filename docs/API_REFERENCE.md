# API Reference

Complete API documentation for the BTC Options Trading API.

## Base URL

- **Main API**: `http://localhost:8080`

## Authentication

Currently no authentication required. All endpoints are publicly accessible.

## Core Trading Endpoints

### GET /health

Health check endpoint for monitoring server status.

**Response:**
```json
{
  "status": "healthy",
  "service": "BTC Options API",
  "version": "1.0.0"
}
```

### GET /optionsTable

Generate 110 available options with Black-Scholes pricing and risk-based quantities.

**Features:**
- Auto-generates 11 strike prices centered around current BTC price (±$5k increments)
- 5 expiry periods: 1d, 2d, 3d, 5d, 7d from current time  
- Both Call and Put sides for each combination
- Real-time pricing with live BTC price and Deribit IV data
- Risk-aware maximum quantities per option

**Example:**
```bash
curl "http://localhost:8080/optionsTable"
```

**Response:**
```json
[
  {
    "side": "Call",
    "strike_price": 110000.0,
    "expire": "1d",
    "premium": 0.001234,
    "max_quantity": 15.67890123,
    "iv": 0.4234,
    "delta": 0.1234
  },
  {
    "side": "Put", 
    "strike_price": 110000.0,
    "expire": "1d",
    "premium": 0.000567,
    "max_quantity": 8.12345678,
    "iv": 0.4234,
    "delta": -0.0987
  }
]
```

**Response Fields:**
- `side`: "Call" or "Put"
- `strike_price`: Strike price in USD
- `expire`: Expiry period (1d, 2d, 3d, 5d, 7d)
- `premium`: Option premium in BTC
- `max_quantity`: Risk-based maximum tradeable quantity in BTC
- `iv`: Implied volatility from Deribit
- `delta`: Option delta calculated using Black-Scholes

### POST /contract

Create a new options contract with risk validation.

**Request Body:**
```json
{
  "side": "Put",
  "strike_price": 110000.0,
  "quantity": 0.5,
  "expires": 1735689600,
  "premium": 0.001234
}
```

**Request Fields:**
- `side`: "Call" or "Put" (required)
- `strike_price`: Strike price in USD (required)
- `quantity`: Quantity in BTC (required, must not exceed max_quantity)
- `expires`: Unix timestamp in seconds (required, must be future date)
- `premium`: Premium in BTC (required)

**Success Response (200):**
```json
{
  "message": "Contract created successfully",
  "id": 123
}
```

**Error Response (400):**
```json
{
  "error": "Contract risk exceeds available collateral. New position margin required: $15,000.00, Total portfolio margin would be: $97,340.00, Available collateral: $84,945.00"
}
```

### GET /contracts

List all created contracts (primarily for debugging).

**Response:**
```json
[
  {
    "id": 1,
    "side": "Put",
    "strike_price": 110000.0,
    "quantity": 0.5,
    "expires": 1735689600,
    "premium": 0.001234,
    "created_at": 1735000000
  }
]
```

### GET /delta

Calculate total portfolio delta across all positions.

**Response:**
```json
-0.1234567890123456
```

Returns a single number representing the portfolio's sensitivity to BTC price changes.

## Market Analytics Endpoints

### GET /topBanner

Market overview statistics for dashboard display.

**Response:**
```json
{
  "volume_24hr": 1.2345,
  "open_interest_usd": 45678.90,
  "contract_count": 42
}
```

**Response Fields:**
- `volume_24hr`: Total trading volume in last 24 hours (BTC)
- `open_interest_usd`: Total open interest in USD
- `contract_count`: Number of active contracts

### GET /marketHighlights

Top 6 products by 24-hour volume.

**Response:**
```json
[
  {
    "product_symbol": "BTC-23h-110000-Put",
    "side": "Put",
    "strike_price": 110000.0,
    "expire": "23h",
    "volume_24hr": 0.5678,
    "price_change_24hr_percent": 12.34
  }
]
```

### GET /topGainers  

Top 5 products by price change percentage.

**Response:**
```json
[
  {
    "product_symbol": "BTC-2d-115000-Call", 
    "side": "Call",
    "strike_price": 115000.0,
    "expire": "2d",
    "change_24hr_percent": 25.67,
    "last_price": 0.002345
  }
]
```

### GET /topVolume

Top 5 products by USD trading volume.

**Response:**
```json
[
  {
    "product_symbol": "BTC-1d-108000-Put",
    "side": "Put", 
    "strike_price": 108000.0,
    "expire": "1d",
    "volume_usd": 12345.67,
    "last_price": 0.001234
  }
]
```

## Error Responses

All endpoints return consistent error format:

**4xx Client Errors:**
```json
{
  "error": "Descriptive error message with remediation guidance"
}
```

**5xx Server Errors:**
```json
{
  "error": "Internal server error",
  "details": "Additional technical details (development mode only)"
}
```

## Rate Limits

Currently no rate limiting implemented. For production deployment, consider implementing rate limiting based on:
- IP address
- Endpoint type (trading vs analytics)
- Resource usage

## Data Freshness

- **BTC Prices**: Updated every 10 seconds via gRPC oracle
- **Implied Volatility**: Updated every 15 seconds from Deribit
- **Pool Balance**: Queried from blockchain on startup and demand
- **Market Analytics**: Calculated in real-time from database

## External Dependencies

The API integrates with several external services:

1. **gRPC Price Oracle** (Required)
   - Endpoint: `localhost:50051`
   - Purpose: Real-time BTC price aggregation
   - Fallback: None - API will not start without this

2. **Deribit API** (Optional)
   - Endpoint: `https://www.deribit.com/api/v2`
   - Purpose: Implied volatility data
   - Fallback: Mock IV server on port 8081

3. **Mutiny Wallet API** (Optional)
   - Purpose: Real Bitcoin pool balance
   - Fallback: Configuration-based mock data

## Development Notes

- All timestamps are Unix timestamps in seconds (except IV oracle which uses milliseconds internally)
- Premiums are stored and returned as BTC amounts with 8 decimal precision
- Strike prices are always in USD
- Quantities are in BTC with up to 8 decimal places
- Maximum 1000 contracts per individual position (sanity limit)