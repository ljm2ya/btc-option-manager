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
// Standard library for sync primitives.
use std::sync::Arc;
// Dotenv for loading .env files.
use dotenv::dotenv;
// Rusqlite for SQLite database interaction.
use rusqlite::{Connection, Result, params};
use rusqlite::types::{ToSql, FromSql, ToSqlOutput, FromSqlError, ValueRef};

// Module for the mock API server.
mod mock_apis;
// Module for the IV oracle.
mod iv_oracle;
// Module for the price oracle.
mod price_oracle;

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

// Response structures for new endpoints
#[derive(Serialize)]
struct TopBannerResponse {
    volume_24hr: f64,
    open_interest_usd: f64,
    contract_count: i64,
}

#[derive(Serialize)]
struct MarketHighlightItem {
    product_symbol: String,
    side: OptionSide,
    strike_price: f64,
    expire: String,
    volume_24hr: f64,
    price_change_24hr_percent: f64,
}

#[derive(Serialize)]
struct TopGainerItem {
    product_symbol: String,
    side: OptionSide,
    strike_price: f64,
    expire: String,
    change_24hr_percent: f64,
    last_price: f64,
}

#[derive(Serialize)]
struct TopVolumeItem {
    product_symbol: String,
    side: OptionSide,
    strike_price: f64,
    expire: String,
    volume_usd: f64,
    last_price: f64,
}

// Initializes the SQLite database and creates the tables if they don't exist.
fn init_db() -> Result<Connection> {
    let conn = Connection::open("contracts.db")?;
    
    // Create contracts table with timestamp
    conn.execute(
        "CREATE TABLE IF NOT EXISTS contracts (
            id INTEGER PRIMARY KEY,
            side TEXT NOT NULL,
            strike_price REAL NOT NULL,
            quantity REAL NOT NULL,
            expires INTEGER NOT NULL,
            premium REAL NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
        )",
        [],
    )?;
    
    // Add created_at column to existing contracts table if it doesn't exist
    let _ = conn.execute("ALTER TABLE contracts ADD COLUMN created_at INTEGER DEFAULT (strftime('%s', 'now'))", []);
    
    // Create premium history table for tracking price movements
    conn.execute(
        "CREATE TABLE IF NOT EXISTS premium_history (
            id INTEGER PRIMARY KEY,
            product_key TEXT NOT NULL,
            side TEXT NOT NULL,
            strike_price REAL NOT NULL,
            expires INTEGER NOT NULL,
            premium REAL NOT NULL,
            timestamp INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
            UNIQUE(product_key, timestamp)
        )",
        [],
    )?;
    
    // Create index for efficient queries
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_contracts_created_at ON contracts(created_at)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_premium_history_product ON premium_history(product_key, timestamp)",
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

    // Initialize the IV Oracle with Deribit API.
    let deribit_url = env::var("DERIBIT_API_URL")
        .unwrap_or_else(|_| "https://www.deribit.com/api/v2".to_string());
    let iv_oracle = Arc::new(iv_oracle::IvOracle::new(deribit_url));
    
    // Start the IV oracle updates.
    iv_oracle.start_updates().await;
    
    // Do initial fetch to populate cache.
    let oracle_clone = iv_oracle.clone();
    tokio::spawn(async move {
        if let Err(e) = oracle_clone.fetch_and_update_iv().await {
            eprintln!("Error fetching initial IV data: {}", e);
        }
    });
    
    // Initialize the Price Oracle
    let price_api_url = env::var("PRICE_API_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8081/price".to_string());
    let price_oracle = Arc::new(price_oracle::PriceOracle::new(price_api_url));

    let iv_oracle_clone = iv_oracle.clone();
    let price_oracle_clone = price_oracle.clone();

    // Configure and start the main API server on port 8080.
    let server1 = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(iv_oracle_clone.clone()))
            .app_data(web::Data::new(price_oracle_clone.clone()))
            // Register API endpoints and their handlers.
            .service(web::resource("/contract").route(web::post().to(post_contract)))
            .service(web::resource("/contracts").route(web::get().to(get_contracts)))
            .service(web::resource("/optionsTable").route(web::get().to(get_options_table)))
            .service(web::resource("/delta").route(web::get().to(get_delta)))
            // New endpoints
            .service(web::resource("/topBanner").route(web::get().to(get_top_banner)))
            .service(web::resource("/marketHighlights").route(web::get().to(get_market_highlights)))
            .service(web::resource("/topGainers").route(web::get().to(get_top_gainers)))
            .service(web::resource("/topVolume").route(web::get().to(get_top_volume)))
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
async fn post_contract(
    contract: web::Json<Contract>,
    price_oracle: web::Data<Arc<price_oracle::PriceOracle>>
) -> impl Responder {
    // --- Validation Step 1: Check Expiration ---
    let now = Utc::now().timestamp();
    if contract.expires <= now {
        return HttpResponse::BadRequest().body("Contract expiration date must be in the future.");
    }

    // --- Validation Step 2: Check Collateral Risk ---
    let client = Client::new();

    // Get API URLs from environment variables, with mock fallbacks.
    let pool_api_url = env::var("POOL_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8081/pool".to_string());
    
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
    let btc_price: f64 = match price_oracle.get_btc_price().await {
        Ok(price) => price,
        Err(_) => return HttpResponse::InternalServerError().body("Error: Could not fetch BTC price from oracle."),
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
    
    // Also save to premium history for price tracking
    let product_key = format!("{}-{}-{}", contract.side, contract.strike_price, contract.expires);
    let _ = conn.execute(
        "INSERT OR REPLACE INTO premium_history (product_key, side, strike_price, expires, premium) 
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            product_key,
            contract.side,
            contract.strike_price,
            contract.expires,
            contract.premium
        ],
    );

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
async fn get_options_table(
    req: web::Query<OptionsTableRequest>, 
    iv_oracle: web::Data<Arc<iv_oracle::IvOracle>>,
    price_oracle: web::Data<Arc<price_oracle::PriceOracle>>
) -> impl Responder {
    let client = Client::new();

    // Get API URLs from environment variables, with mock fallbacks.
    let pool_api_url = env::var("POOL_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8081/pool".to_string());
    let price_api_url = env::var("PRICE_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8081/price".to_string());

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
    let btc_price: f64 = price_oracle.get_btc_price().await.unwrap_or(0.0);

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
                // Get Implied Volatility (IV) from the oracle.
                let side_str = match side {
                    OptionSide::Call => "C",
                    OptionSide::Put => "P",
                };
                
                let iv = iv_oracle.get_iv(side_str, *strike_price, expire).unwrap_or_else(|| {
                    // Fallback to mock API if not found in oracle
                    let iv_api_url = env::var("IV_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8081/iv".to_string());
                    futures::executor::block_on(async {
                        client
                            .get(&format!(
                                "{}?side={}&strike_price={}&expire={}",
                                iv_api_url, side, strike_price, expire
                            ))
                            .send()
                            .await
                            .unwrap()
                            .json()
                            .await
                            .unwrap()
                    })
                });

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
async fn get_delta(
    iv_oracle: web::Data<Arc<iv_oracle::IvOracle>>,
    price_oracle: web::Data<Arc<price_oracle::PriceOracle>>
) -> impl Responder {
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
    let iv_api_url = env::var("IV_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8081/iv".to_string());
    
    let risk_free_rate: f64 = env::var("RISK_FREE_RATE")
        .unwrap_or_else(|_| "0.0".to_string())
        .parse()
        .unwrap_or(0.0);

    // Fetch the current asset price.
    let btc_price: f64 = price_oracle.get_btc_price().await.unwrap_or(0.0);

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

// Handles GET requests for the top banner information.
async fn get_top_banner(price_oracle: web::Data<Arc<price_oracle::PriceOracle>>) -> impl Responder {
    let now = Utc::now().timestamp();
    let twenty_four_hours_ago = now - (24 * 60 * 60);
    
    let conn = Connection::open("contracts.db").unwrap();
    
    // Get 24hr trading volume (sum of quantities in last 24 hours)
    let volume_24hr: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(quantity), 0.0) FROM contracts WHERE created_at >= ?1",
            params![twenty_four_hours_ago],
            |row| row.get(0),
        )
        .unwrap_or(0.0);
    
    // Get open interest (all non-expired contracts) and calculate USD value
    let mut stmt = conn
        .prepare("SELECT quantity, premium FROM contracts WHERE expires > ?1")
        .unwrap();
    
    let contracts_iter = stmt.query_map(params![now], |row| {
        Ok((row.get::<_, f64>(0)?, row.get::<_, f64>(1)?))
    }).unwrap();
    
    let mut open_interest_btc = 0.0;
    for contract in contracts_iter {
        if let Ok((quantity, premium)) = contract {
            open_interest_btc += quantity * premium;
        }
    }
    
    // Get current BTC price to convert to USD
    let btc_price = price_oracle.get_btc_price().await.unwrap_or(0.0);
    let open_interest_usd = open_interest_btc * btc_price;
    
    // Get number of open contracts
    let contract_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM contracts WHERE expires > ?1",
            params![now],
            |row| row.get(0),
        )
        .unwrap_or(0);
    
    let response = TopBannerResponse {
        volume_24hr,
        open_interest_usd,
        contract_count,
    };
    
    HttpResponse::Ok().json(response)
}

// Handles GET requests for market highlights (top 6 volume products).
async fn get_market_highlights(price_oracle: web::Data<Arc<price_oracle::PriceOracle>>) -> impl Responder {
    let now = Utc::now().timestamp();
    let twenty_four_hours_ago = now - (24 * 60 * 60);
    
    let conn = Connection::open("contracts.db").unwrap();
    
    // Get top 6 products by 24hr volume
    let mut stmt = conn
        .prepare(
            "SELECT side, strike_price, expires, SUM(quantity) as total_volume, AVG(premium) as avg_premium
             FROM contracts 
             WHERE created_at >= ?1
             GROUP BY side, strike_price, expires
             ORDER BY total_volume DESC
             LIMIT 6"
        )
        .unwrap();
    
    let products_iter = stmt.query_map(params![twenty_four_hours_ago], |row| {
        Ok((
            row.get::<_, OptionSide>(0)?,
            row.get::<_, f64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, f64>(3)?,
            row.get::<_, f64>(4)?,
        ))
    }).unwrap();
    
    let mut highlights = Vec::new();
    
    for product in products_iter {
        if let Ok((side, strike_price, expires, volume, current_premium)) = product {
            // Generate product key
            let product_key = format!("{}-{}-{}", side, strike_price, expires);
            
            // Get premium from 24 hours ago from history table
            let premium_24hr_ago: Option<f64> = conn
                .query_row(
                    "SELECT premium FROM premium_history 
                     WHERE product_key = ?1 AND timestamp <= ?2 
                     ORDER BY timestamp DESC LIMIT 1",
                    params![&product_key, twenty_four_hours_ago],
                    |row| row.get(0),
                )
                .ok();
            
            let price_change_percent = if let Some(old_premium) = premium_24hr_ago {
                if old_premium > 0.0 {
                    ((current_premium - old_premium) / old_premium) * 100.0
                } else {
                    0.0
                }
            } else {
                0.0
            };
            
            // Convert expires timestamp to string representation
            let expire_string = format_expires_timestamp(expires);
            
            highlights.push(MarketHighlightItem {
                product_symbol: format!("BTC-{}-{}-{}", expire_string, strike_price, side),
                side: side.clone(),
                strike_price,
                expire: expire_string,
                volume_24hr: volume,
                price_change_24hr_percent: price_change_percent,
            });
        }
    }
    
    HttpResponse::Ok().json(highlights)
}

// Handles GET requests for top gainers (top 5 by 24hr % change).
async fn get_top_gainers(price_oracle: web::Data<Arc<price_oracle::PriceOracle>>) -> impl Responder {
    let now = Utc::now().timestamp();
    let twenty_four_hours_ago = now - (24 * 60 * 60);
    
    let conn = Connection::open("contracts.db").unwrap();
    
    // Get all active products
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT side, strike_price, expires 
             FROM contracts 
             WHERE expires > ?1"
        )
        .unwrap();
    
    let products_iter = stmt.query_map(params![now], |row| {
        Ok((
            row.get::<_, OptionSide>(0)?,
            row.get::<_, f64>(1)?,
            row.get::<_, i64>(2)?,
        ))
    }).unwrap();
    
    let mut gainers = Vec::new();
    
    for product in products_iter {
        if let Ok((side, strike_price, expires)) = product {
            // Get current premium
            let current_premium: Option<f64> = conn
                .query_row(
                    "SELECT premium FROM contracts 
                     WHERE side = ?1 AND strike_price = ?2 AND expires = ?3 
                     ORDER BY id DESC LIMIT 1",
                    params![&side, strike_price, expires],
                    |row| row.get(0),
                )
                .ok();
            
            if let Some(current) = current_premium {
                // Generate product key
                let product_key = format!("{}-{}-{}", side, strike_price, expires);
                
                // Get premium from 24 hours ago
                let premium_24hr_ago: Option<f64> = conn
                    .query_row(
                        "SELECT premium FROM premium_history 
                         WHERE product_key = ?1 AND timestamp <= ?2 
                         ORDER BY timestamp DESC LIMIT 1",
                        params![&product_key, twenty_four_hours_ago],
                        |row| row.get(0),
                    )
                    .ok();
                
                if let Some(old_premium) = premium_24hr_ago {
                    if old_premium > 0.0 {
                        let change_percent = ((current - old_premium) / old_premium) * 100.0;
                        let expire_string = format_expires_timestamp(expires);
                        
                        gainers.push(TopGainerItem {
                            product_symbol: format!("BTC-{}-{}-{}", expire_string, strike_price, side),
                            side: side.clone(),
                            strike_price,
                            expire: expire_string,
                            change_24hr_percent: change_percent,
                            last_price: current,
                        });
                    }
                }
            }
        }
    }
    
    // Sort by percentage change and take top 5
    gainers.sort_by(|a, b| b.change_24hr_percent.partial_cmp(&a.change_24hr_percent).unwrap());
    gainers.truncate(5);
    
    HttpResponse::Ok().json(gainers)
}

// Handles GET requests for top volume products (top 5 by 24hr volume).
async fn get_top_volume(price_oracle: web::Data<Arc<price_oracle::PriceOracle>>) -> impl Responder {
    let now = Utc::now().timestamp();
    let twenty_four_hours_ago = now - (24 * 60 * 60);
    
    let conn = Connection::open("contracts.db").unwrap();
    
    // Get BTC price for USD conversion
    let btc_price = price_oracle.get_btc_price().await.unwrap_or(0.0);
    
    // Get top 5 products by 24hr volume in USD
    let mut stmt = conn
        .prepare(
            "SELECT side, strike_price, expires, SUM(quantity * premium) as total_volume_btc, AVG(premium) as avg_premium
             FROM contracts 
             WHERE created_at >= ?1
             GROUP BY side, strike_price, expires
             ORDER BY total_volume_btc DESC
             LIMIT 5"
        )
        .unwrap();
    
    let products_iter = stmt.query_map(params![twenty_four_hours_ago], |row| {
        Ok((
            row.get::<_, OptionSide>(0)?,
            row.get::<_, f64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, f64>(3)?,
            row.get::<_, f64>(4)?,
        ))
    }).unwrap();
    
    let mut top_volume = Vec::new();
    
    for product in products_iter {
        if let Ok((side, strike_price, expires, volume_btc, last_premium)) = product {
            let expire_string = format_expires_timestamp(expires);
            
            top_volume.push(TopVolumeItem {
                product_symbol: format!("BTC-{}-{}-{}", expire_string, strike_price, side),
                side: side.clone(),
                strike_price,
                expire: expire_string,
                volume_usd: volume_btc * btc_price,
                last_price: last_premium,
            });
        }
    }
    
    HttpResponse::Ok().json(top_volume)
}

// Helper function to format expires timestamp to a readable string.
fn format_expires_timestamp(expires: i64) -> String {
    let now = Utc::now().timestamp();
    let duration_seconds = expires - now;
    
    if duration_seconds <= 0 {
        "EXPIRED".to_string()
    } else if duration_seconds < 3600 {
        let minutes = duration_seconds / 60;
        format!("{}m", minutes)
    } else if duration_seconds < 86400 {
        let hours = duration_seconds / 3600;
        format!("{}h", hours)
    } else {
        let days = duration_seconds / 86400;
        format!("{}d", days)
    }
}
