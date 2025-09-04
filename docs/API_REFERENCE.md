# API Reference

Complete API documentation for the BTC Options Trading API.

## Base URLs

- **Main API**: `http://localhost:8080`
- **Mock IV Server**: `http://localhost:8081`

## Trading Endpoints

### GET /optionsTable

Generate available options with Black-Scholes pricing.

**Query Parameters:**
- `strike_prices` (required): Comma-separated strike prices (e.g., `100000,110000`)
- `expires` (required): Comma-separated expiration periods (e.g., `1d,7d,30m`)

**Example:**
```bash
curl "http://localhost:8080/optionsTable?strike_prices=100000,110000&expires=1d,7d"
```

**Response:**
```json
{
  "options": [{
    "side": "Call",
    "strike_price": 100000.0,
    "quantity": 1.5,
    "expires": 1735689600,
    "premium": 5234.56
  }]
}
```

### POST /contract

Create a new options contract.

**Request Body:**
```json
{
  "side": "Call",        // "Call" or "Put"
  "strike_price": 110000.0,
  "quantity": 1.5,
  "expires": 1735689600,  // Unix timestamp (must be future)
  "premium": 5000.0
}
```

**Example:**
```bash
curl -X POST http://localhost:8080/contract \
  -H "Content-Type: application/json" \
  -d '{"side":"Call","strike_price":110000,"quantity":1.5,"expires":1735689600,"premium":5000}'
```

### GET /delta

Calculate total portfolio delta (sum of all contract deltas).

**Example:**
```bash
curl http://localhost:8080/delta
```

**Response:**
```json
{
  "total_delta": 0.3456
}
```

## Market Analytics Endpoints

### GET /topBanner

Market overview statistics for the last 24 hours.

**Response:**
```json
{
  "volume_24hr": 125.5,           // Total BTC volume
  "open_interest_usd": 5234567.89,// Value of open contracts
  "contract_count": 42            // Number of open contracts
}
```

### GET /marketHighlights

Top 6 products by 24-hour trading volume.

**Response:**
```json
[{
  "product_symbol": "BTC-1d-100000-Call",
  "side": "Call",
  "strike_price": 100000.0,
  "expire": "1d",
  "volume_24hr": 25.5,
  "price_change_24hr_percent": 12.34
}]
```

### GET /topGainers

Top 5 products by 24-hour price change percentage.

**Response:**
```json
[{
  "product_symbol": "BTC-7d-110000-Put",
  "side": "Put",
  "strike_price": 110000.0,
  "expire": "7d",
  "change_24hr_percent": 45.67,
  "last_price": 8900.0
}]
```

### GET /topVolume

Top 5 products by 24-hour USD volume.

**Response:**
```json
[{
  "product_symbol": "BTC-30d-95000-Call",
  "side": "Call",
  "strike_price": 95000.0,
  "expire": "30d",
  "volume_usd": 1234567.89,
  "last_price": 12000.0
}]
```

## Debug Endpoints

### GET /contracts

List all contracts in the database (development only).

**Response:**
```json
[{
  "id": 1,
  "side": "Call",
  "strike_price": 100000.0,
  "quantity": 1.0,
  "expires": 1735689600,
  "premium": 5000.0,
  "created_at": 1735603200
}]
```

## Mock API Endpoints (Port 8081)

### GET /iv

Get implied volatility for a symbol (fallback when Deribit unavailable).

**Query Parameters:**
- `symbol` (required): Trading pair (e.g., `BTCUSD`)

**Example:**
```bash
curl "http://localhost:8081/iv?symbol=BTCUSD"
```

**Response:**
```json
{
  "implied_volatility": 0.65
}
```

## Error Responses

All endpoints return consistent error format:

```json
{
  "error": "Error message",
  "details": "Additional context if available"
}
```

**Common Status Codes:**
- `200`: Success
- `400`: Bad Request (invalid parameters)
- `404`: Not Found
- `500`: Internal Server Error
- `503`: Service Unavailable (external service down)