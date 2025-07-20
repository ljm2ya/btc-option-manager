// Actix-web for web server functionality.
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
// Serde for JSON serialization and deserialization.
use serde::{Deserialize, Serialize};
// Chrono for handling timestamps.
use chrono::Utc;
// Reqwest for making HTTP requests.
use reqwest::Client;
// Standard library for formatting.
use std::fmt;
// Standard library for environment variables.
use std::env;
// Dotenv for loading .env files.
use dotenv::dotenv;
// Rusqlite for SQLite database interaction.
use rusqlite::{Connection, Result, params};
use rusqlite::types::{ToSql, FromSql, ToSqlOutput, FromSqlError, ValueRef};

// Module for the mock API server.
mod mock_apis;

// Represents the side of an option: Call or Put.
#[derive(Serialize, Deserialize, Clone, Debug)]
enum OptionSide {
    Call,
    Put,
}

// Trait implementation to allow 'OptionSide' to be stored in the database.
impl ToSql for OptionSide {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        Ok(self.to_string().into())
    }
}

// Trait implementation to allow 'OptionSide' to be read from the database.
impl FromSql for OptionSide {
    fn column_result(value: ValueRef<'_>) -> std::result::Result<Self, FromSqlError> {
        value.as_str()?.parse()
    }
}

// Trait implementation to parse 'OptionSide' from a string.
impl std::str::FromStr for OptionSide {
    type Err = FromSqlError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "Call" => Ok(OptionSide::Call),
            "Put" => Ok(OptionSide::Put),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}


// Trait implementation to format 'OptionSide' as a string.
impl fmt::Display for OptionSide {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            OptionSide::Call => write!(f, "Call"),
            OptionSide::Put => write!(f, "Put"),
        }
    }
}

// Defines the structure for an options contract.
#[derive(Serialize, Deserialize, Clone)]
struct Contract {
    side: OptionSide,
    strike_price: f64,
    quantity: f64,
    expires: i64,
    premium: f64,
}

// Defines the request structure for the '/optionsTable' endpoint.
#[derive(Deserialize)]
struct OptionsTableRequest {
    strike_prices: String,
    expires: String,
}

// Defines the response structure for the '/optionsTable' endpoint.
#[derive(Serialize)]
struct OptionsTableResponse {
    side: OptionSide,
    strike_price: f64,
    expire: String,
    premium: f64,
    max_quantity: f64,
    iv: f64,
}

// Initializes the SQLite database and creates the 'contracts' table if it doesn't exist.
fn init_db() -> Result<Connection> {
    let conn = Connection::open("contracts.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS contracts (
            id INTEGER PRIMARY KEY,
            side TEXT NOT NULL,
            strike_price REAL NOT NULL,
            quantity REAL NOT NULL,
            expires INTEGER NOT NULL,
            premium REAL NOT NULL
        )",
        [],
    )?;
    Ok(conn)
}

// Main application entry point.
#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables from .env file.
    dotenv().ok();

    // Initialize the database on startup.
    init_db().unwrap();

    // Configure and start the main API server on port 8080.
    let server1 = HttpServer::new(move || {
        App::new()
            // Register API endpoints and their handlers.
            .service(web::resource("/contract").route(web::post().to(post_contract)))
            .service(web::resource("/contracts").route(web::get().to(get_contracts)))
            .service(web::resource("/optionsTable").route(web::get().to(get_options_table)))
            .service(web::resource("/delta").route(web::get().to(get_delta)))
    })
    .bind("127.0.0.1:8080")?
    .run();

    // Start the mock API server on port 8081.
    let server2 = mock_apis::mock_server();

    // Run both servers concurrently.
    let _ = tokio::join!(server1, server2);

    Ok(())
}

// Handles POST requests to create and save a new contract.
async fn post_contract(contract: web::Json<Contract>) -> impl Responder {
    // --- Validation Step 1: Check Expiration ---
    let now = Utc::now().timestamp();
    if contract.expires <= now {
        return HttpResponse::BadRequest().body("Contract expiration date must be in the future.");
    }

    // --- Validation Step 2: Check Collateral Risk ---
    let client = Client::new();

    // Get API URLs from environment variables, with mock fallbacks.
    let pool_api_url = env::var("POOL_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8081/pool".to_string());
    let price_api_url = env::var("PRICE_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8081/price".to_string());
    
    // Get collateral rate from environment variables, with a default.
    let collateral_rate: f64 = env::var("COLLATERAL_RATE")
        .unwrap_or_else(|_| "0.5".to_string())
        .parse()
        .unwrap_or(0.5);

    // Fetch pool quantity and current price from APIs.
    let pool_qty: f64 = match client.get(&pool_api_url).send().await {
        Ok(res) => res.json().await.unwrap_or(0.0),
        Err(_) => return HttpResponse::InternalServerError().body("Error: Could not fetch pool quantity from API."),
    };
    let btc_price: f64 = match client.get(&price_api_url).send().await {
        Ok(res) => res.json().await.unwrap_or(0.0),
        Err(_) => return HttpResponse::InternalServerError().body("Error: Could not fetch BTC price from API."),
    };

    // Calculate total available collateral.
    let max_collateral_usd = pool_qty * btc_price * collateral_rate;
    
    // Open database connection to check existing contract risk.
    let conn = Connection::open("contracts.db").unwrap();
    let mut stmt = conn
        .prepare("SELECT premium, quantity FROM contracts")
        .unwrap();

    // Calculate the value of each existing contract.
    let contracts_iter = stmt.query_map([], |row| {
        let premium: f64 = row.get(0)?;
        let quantity: f64 = row.get(1)?;
        Ok(premium * quantity)
    }).unwrap();

    // Sum the risk of all existing contracts.
    let current_total_risk: f64 = contracts_iter.map(|r| r.unwrap_or(0.0)).sum();
    // Calculate the risk of the new contract.
    let new_contract_risk = contract.premium * contract.quantity;

    // Reject if new contract exceeds available collateral.
    if current_total_risk + new_contract_risk > max_collateral_usd {
        return HttpResponse::BadRequest().body("Contract risk exceeds available collateral.");
    }

    // --- Save to Database ---
    // If validation passes, insert the new contract into the database.
    conn.execute(
        "INSERT INTO contracts (side, strike_price, quantity, expires, premium) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            contract.side,
            contract.strike_price,
            contract.quantity,
            contract.expires,
            contract.premium
        ],
    )
    .unwrap();

    HttpResponse::Ok().finish()
}

// Handles GET requests to fetch all contracts for debugging.
async fn get_contracts() -> impl Responder {
    let conn = Connection::open("contracts.db").unwrap();
    let mut stmt = conn
        .prepare("SELECT side, strike_price, quantity, expires, premium FROM contracts")
        .unwrap();

    // Map database rows to Contract structs.
    let contracts_iter = stmt.query_map([], |row| {
        Ok(Contract {
            side: row.get(0)?,
            strike_price: row.get(1)?,
            quantity: row.get(2)?,
            expires: row.get(3)?,
            premium: row.get(4)?,
        })
    }).unwrap();

    let contracts: Vec<Contract> = contracts_iter.map(|c| c.unwrap()).collect();
    // Return the list of contracts as JSON.
    HttpResponse::Ok().json(contracts)
}


// Handles GET requests to generate a table of available options.
async fn get_options_table(req: web::Query<OptionsTableRequest>) -> impl Responder {
    let client = Client::new();

    // Get API URLs from environment variables, with mock fallbacks.
    let pool_api_url = env::var("POOL_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8081/pool".to_string());
    let price_api_url = env::var("PRICE_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8081/price".to_string());
    let iv_api_url = env::var("IV_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8081/iv".to_string());

    // Get financial rates from environment variables, with defaults.
    let risk_free_rate: f64 = env::var("RISK_FREE_RATE")
        .unwrap_or_else(|_| "0.0".to_string())
        .parse()
        .unwrap_or(0.0);
    let collateral_rate: f64 = env::var("COLLATERAL_RATE")
        .unwrap_or_else(|_| "0.5".to_string())
        .parse()
        .unwrap_or(0.5);

    // Fetch financial data from APIs.
    let pool_qty: f64 = client
        .get(&pool_api_url)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let btc_price: f64 = client
        .get(&price_api_url)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let mut table = Vec::new();

    // Parse comma-separated request parameters.
    let strike_prices: Vec<f64> = req
        .strike_prices
        .split(',')
        .filter_map(|s| s.parse().ok())
        .collect();
    let expires: Vec<String> = req.expires.split(',').map(|s| s.to_string()).collect();

    // Generate options for both Call and Put sides.
    let sides = [OptionSide::Call, OptionSide::Put];

    // Loop through each combination of strike, expiration, and side.
    for strike_price in &strike_prices {
        for expire in &expires {
            for side in &sides {
                // Fetch Implied Volatility (IV) from the API.
                let iv: f64 = client
                    .get(&format!(
                        "{}?side={}&strike_price={}&expire={}",
                        iv_api_url, side, strike_price, expire
                    ))
                    .send()
                    .await
                    .unwrap()
                    .json()
                    .await
                    .unwrap();

                // Convert duration string to a fraction of a year.
                let t = parse_duration(expire);

                // Calculate the premium using the Black-Scholes model.
                let premium = match side {
                    OptionSide::Call => black_scholes::call(
                        btc_price,
                        *strike_price,
                        risk_free_rate,
                        iv,
                        t,
                    ),
                    OptionSide::Put => black_scholes::put(
                        btc_price,
                        *strike_price,
                        risk_free_rate,
                        iv,
                        t,
                    ),
                };
                // Calculate the maximum tradeable quantity.
                let max_quantity = pool_qty * collateral_rate / (premium * btc_price);

                // Add the calculated option to the response table.
                table.push(OptionsTableResponse {
                    side: side.clone(),
                    strike_price: *strike_price,
                    expire: expire.clone(),
                    premium,
                    max_quantity,
                    iv,
                });
            }
        }
    }

    HttpResponse::Ok().json(table)
}

// Helper function to parse duration strings (e.g., "30m", "1d") into a year fraction.
fn parse_duration(duration: &str) -> f64 {
    let d = duration.trim();
    let (num_str, unit) = d.split_at(d.len() - 1);
    let num: f64 = num_str.parse().unwrap();
    match unit {
        "m" => num / (365.0 * 24.0 * 60.0),
        "h" => num / (365.0 * 24.0),
        "d" => num / 365.0,
        _ => 0.0,
    }
}

// Handles GET requests to calculate the total delta of all active contracts.
async fn get_delta() -> impl Responder {
    let now = Utc::now().timestamp();
    let conn = Connection::open("contracts.db").unwrap();

    // Select all non-expired contracts from the database.
    let mut stmt = conn
        .prepare("SELECT side, strike_price, quantity, expires FROM contracts WHERE expires > ?1")
        .unwrap();

    // Execute the query.
    let contracts_iter = stmt.query_map(params![now], |row| {
        Ok(Contract {
            side: row.get(0)?,
            strike_price: row.get(1)?,
            quantity: row.get(2)?,
            expires: row.get(3)?,
            premium: 0.0, // Premium not needed for delta calculation.
        })
    }).unwrap();

    let contracts: Vec<Contract> = contracts_iter.map(|c| c.unwrap()).collect();
    
    if contracts.is_empty() {
        return HttpResponse::Ok().json(0.0);
    }

    let client = Client::new();

    // Get API URLs and risk-free rate from environment variables.
    let price_api_url = env::var("PRICE_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8081/price".to_string());
    let iv_api_url = env::var("IV_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8081/iv".to_string());
    
    let risk_free_rate: f64 = env::var("RISK_FREE_RATE")
        .unwrap_or_else(|_| "0.0".to_string())
        .parse()
        .unwrap_or(0.0);

    // Fetch the current asset price.
    let btc_price: f64 = client
        .get(&price_api_url)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let mut total_delta = 0.0;

    // Loop through each active contract to calculate and sum its delta.
    for contract in contracts.iter() {
        let t = (contract.expires - now) as f64 / (365.0 * 24.0 * 60.0 * 60.0);
        let iv: f64 = client
            .get(&format!(
                "{}?side={}&strike_price={}&expire={}",
                iv_api_url, contract.side, contract.strike_price, "1d" // Dummy "1d" expire.
            ))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        // Calculate delta using the Black-Scholes model.
        let delta = match contract.side {
            OptionSide::Call => {
                black_scholes::call_delta(btc_price, contract.strike_price, risk_free_rate, iv, t)
            }
            OptionSide::Put => {
                black_scholes::put_delta(btc_price, contract.strike_price, risk_free_rate, iv, t)
            }
        };
        // Add the weighted delta to the total.
        total_delta += delta * contract.quantity;
    }

    HttpResponse::Ok().json(total_delta)
}
