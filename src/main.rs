// This is a refactored version of main.rs with all architectural improvements
// After review, this can replace the original main.rs

use actix_web::{web, App, HttpResponse, HttpServer, Responder, middleware};
use serde::{Deserialize, Serialize};
use chrono::Utc;
use reqwest::Client;
use std::fmt;
use std::env;
use std::sync::Arc;
use dotenv::dotenv;
use rusqlite::{params, types::{ToSql, FromSql, ToSqlOutput, FromSqlError, ValueRef}};

// Import our modules
mod mock_apis;
mod iv_oracle;
mod price_oracle;
mod db;
mod utils;
mod error;
mod mutiny_wallet;

use crate::db::DbPool;
use crate::error::ApiError;
use crate::utils::{format_expires_timestamp, parse_duration};
use crate::mutiny_wallet::{MutinyWallet, Network};

// Represents the side of an option: Call or Put.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum OptionSide {
    Call,
    Put,
}

impl ToSql for OptionSide {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(self.to_string().into())
    }
}

impl FromSql for OptionSide {
    fn column_result(value: ValueRef<'_>) -> std::result::Result<Self, FromSqlError> {
        value.as_str()?.parse()
    }
}

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

impl fmt::Display for OptionSide {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            OptionSide::Call => write!(f, "Call"),
            OptionSide::Put => write!(f, "Put"),
        }
    }
}

// Contract structure
#[derive(Serialize, Deserialize, Clone)]
struct Contract {
    side: OptionSide,
    strike_price: f64,
    quantity: f64,
    expires: i64,
    premium: f64,
}

// Request/Response structures
#[derive(Deserialize)]
struct OptionsTableRequest {
    strike_prices: String,
    expires: String,
}

#[derive(Serialize)]
struct OptionsTableResponse {
    side: OptionSide,
    strike_price: f64,
    expire: String,
    premium: f64,
    max_quantity: f64,
    iv: f64,
}

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

// Application state
pub struct AppState {
    db_pool: DbPool,
    iv_oracle: Arc<iv_oracle::IvOracle>,
    price_oracle: Arc<price_oracle::PriceOracle>,
    mutiny_wallet: Arc<MutinyWallet>,
    pool_address: String,
}

// Main application entry point
#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    // Initialize database pool
    let db_pool = db::create_pool()
        .expect("Failed to create database pool");

    // Initialize the IV Oracle
    let deribit_url = env::var("DERIBIT_API_URL")
        .unwrap_or_else(|_| "https://www.deribit.com/api/v2".to_string());
    let iv_oracle = Arc::new(iv_oracle::IvOracle::new(deribit_url));
    iv_oracle.start_updates().await;

    // Initialize the Price Oracle with gRPC
    let aggregator_url = env::var("AGGREGATOR_URL")
        .unwrap_or_else(|_| "http://localhost:50051".to_string());
    let price_oracle = Arc::new(
        price_oracle::PriceOracle::new(aggregator_url)
            .await
            .unwrap_or_else(|e| {
                eprintln!("ERROR: {}", e);
                std::process::exit(1);
            })
    );

    // Initialize Mutiny Wallet
    let pool_network = match env::var("POOL_NETWORK").unwrap_or_else(|_| "signet".to_string()).as_str() {
        "mainnet" => Network::Mainnet,
        "testnet" => Network::Testnet,
        _ => Network::Signet,
    };
    let mutiny_wallet = Arc::new(MutinyWallet::new(pool_network));
    
    // Get pool address from environment
    let pool_address = env::var("POOL_ADDRESS")
        .expect("POOL_ADDRESS must be set in environment");

    // Create app state
    let app_state = Arc::new(AppState {
        db_pool: db_pool.clone(),
        iv_oracle: iv_oracle.clone(),
        price_oracle: price_oracle.clone(),
        mutiny_wallet: mutiny_wallet.clone(),
        pool_address,
    });

    // Configure and start the main API server on port 8080
    let server1 = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .wrap(middleware::Logger::default())
            // Register API endpoints
            .service(web::resource("/contract").route(web::post().to(post_contract)))
            .service(web::resource("/contracts").route(web::get().to(get_contracts)))
            .service(web::resource("/optionsTable").route(web::get().to(get_options_table)))
            .service(web::resource("/delta").route(web::get().to(get_delta)))
            // Analytics endpoints
            .service(web::resource("/topBanner").route(web::get().to(get_top_banner)))
            .service(web::resource("/marketHighlights").route(web::get().to(get_market_highlights)))
            .service(web::resource("/topGainers").route(web::get().to(get_top_gainers)))
            .service(web::resource("/topVolume").route(web::get().to(get_top_volume)))
    })
    .bind("127.0.0.1:8080")?
    .run();

    // Start the mock API server
    let server2 = mock_apis::mock_server();

    // Run both servers concurrently
    let _ = tokio::join!(server1, server2);

    Ok(())
}

impl AppState {
    // Helper method to get pool balance in BTC
    async fn get_pool_balance_btc(&self) -> Result<f64, ApiError> {
        let wallet_balance = self.mutiny_wallet
            .get_wallet_balance(&self.pool_address)
            .await
            .map_err(|e| ApiError::ExternalApiError(format!("Failed to get pool balance: {}", e)))?;
        
        // Convert satoshis to BTC
        Ok(MutinyWallet::satoshis_to_btc(wallet_balance.total_balance))
    }
}

// POST /contract - Create new contract
async fn post_contract(
    contract: web::Json<Contract>,
    state: web::Data<Arc<AppState>>,
) -> Result<impl Responder, ApiError> {
    // Validation
    let now = Utc::now().timestamp();
    if contract.expires <= now {
        return Err(ApiError::ValidationError(
            "Contract expiration date must be in the future.".to_string(),
        ));
    }

    // Get collateral parameters
    let collateral_rate: f64 = env::var("COLLATERAL_RATE")
        .unwrap_or_else(|_| "0.5".to_string())
        .parse()
        .unwrap_or(0.5);

    // Get pool balance from Mutiny wallet
    let pool_qty: f64 = state.get_pool_balance_btc().await?;

    // Get BTC price from oracle
    let btc_price = state
        .price_oracle
        .get_btc_price()
        .await
        .map_err(|e| ApiError::PriceOracleError(e.to_string()))?;

    // Calculate collateral requirements
    let max_collateral_usd = pool_qty * btc_price * collateral_rate;

    // Check existing risk
    let conn = state.db_pool.get()?;
    let mut stmt = conn.prepare("SELECT premium, quantity FROM contracts")?;
    let contracts_iter = stmt.query_map([], |row| {
        let premium: f64 = row.get(0)?;
        let quantity: f64 = row.get(1)?;
        Ok(premium * quantity)
    })?;

    let current_total_risk: f64 = contracts_iter.map(|r| r.unwrap_or(0.0)).sum();
    let new_contract_risk = contract.premium * contract.quantity;

    if current_total_risk + new_contract_risk > max_collateral_usd {
        return Err(ApiError::ValidationError(
            "Contract risk exceeds available collateral.".to_string(),
        ));
    }

    // Save to database
    conn.execute(
        "INSERT INTO contracts (side, strike_price, quantity, expires, premium) 
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            contract.side,
            contract.strike_price,
            contract.quantity,
            contract.expires,
            contract.premium
        ],
    )?;

    // Save to premium history
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

    Ok(HttpResponse::Ok().finish())
}

// GET /contracts - List all contracts
async fn get_contracts(state: web::Data<Arc<AppState>>) -> Result<impl Responder, ApiError> {
    let conn = state.db_pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT side, strike_price, quantity, expires, premium FROM contracts"
    )?;

    let contracts_iter = stmt.query_map([], |row| {
        Ok(Contract {
            side: row.get(0)?,
            strike_price: row.get(1)?,
            quantity: row.get(2)?,
            expires: row.get(3)?,
            premium: row.get(4)?,
        })
    })?;

    let contracts: Vec<Contract> = contracts_iter
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(contracts))
}

// GET /optionsTable - Generate options table
async fn get_options_table(
    req: web::Query<OptionsTableRequest>,
    state: web::Data<Arc<AppState>>,
) -> Result<impl Responder, ApiError> {
    // Parse request parameters
    let strike_prices: Vec<f64> = req
        .strike_prices
        .split(',')
        .filter_map(|s| s.parse().ok())
        .collect();
    
    if strike_prices.is_empty() {
        return Err(ApiError::ValidationError(
            "Invalid strike prices format".to_string(),
        ));
    }

    let expires: Vec<String> = req.expires.split(',').map(|s| s.to_string()).collect();
    
    if expires.is_empty() {
        return Err(ApiError::ValidationError(
            "Invalid expires format".to_string(),
        ));
    }

    // Get financial parameters
    let risk_free_rate: f64 = env::var("RISK_FREE_RATE")
        .unwrap_or_else(|_| "0.0".to_string())
        .parse()
        .unwrap_or(0.0);
    let collateral_rate: f64 = env::var("COLLATERAL_RATE")
        .unwrap_or_else(|_| "0.5".to_string())
        .parse()
        .unwrap_or(0.5);

    // Get pool balance from Mutiny wallet
    let pool_qty: f64 = state.get_pool_balance_btc().await?;

    // Initialize client for IV fallback
    let client = Client::new();

    let btc_price = state
        .price_oracle
        .get_btc_price()
        .await
        .map_err(|e| ApiError::PriceOracleError(e.to_string()))?;

    let mut table = Vec::new();
    let sides = [OptionSide::Call, OptionSide::Put];

    // Generate options table
    for strike_price in &strike_prices {
        for expire in &expires {
            for side in &sides {
                // Get IV from oracle
                let side_str = match side {
                    OptionSide::Call => "C",
                    OptionSide::Put => "P",
                };

                let iv = state.iv_oracle.get_iv(side_str, *strike_price, expire).unwrap_or_else(|| {
                    // Fallback to mock API
                    let iv_api_url = env::var("IV_API_URL")
                        .unwrap_or_else(|_| "http://127.0.0.1:8081/iv".to_string());
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
                            .unwrap_or(0.3) // Default IV if all else fails
                    })
                });

                let t = parse_duration(expire);

                // Calculate premium using Black-Scholes
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

                let max_quantity = pool_qty * collateral_rate / (premium * btc_price);

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

    Ok(HttpResponse::Ok().json(table))
}

// GET /delta - Calculate portfolio delta
async fn get_delta(state: web::Data<Arc<AppState>>) -> Result<impl Responder, ApiError> {
    let now = Utc::now().timestamp();
    let conn = state.db_pool.get()?;

    let mut stmt = conn.prepare(
        "SELECT side, strike_price, quantity, expires FROM contracts WHERE expires > ?1"
    )?;

    let contracts_iter = stmt.query_map(params![now], |row| {
        Ok(Contract {
            side: row.get(0)?,
            strike_price: row.get(1)?,
            quantity: row.get(2)?,
            expires: row.get(3)?,
            premium: 0.0,
        })
    })?;

    let contracts: Vec<Contract> = contracts_iter
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    if contracts.is_empty() {
        return Ok(HttpResponse::Ok().json(0.0));
    }

    let btc_price = state
        .price_oracle
        .get_btc_price()
        .await
        .map_err(|e| ApiError::PriceOracleError(e.to_string()))?;

    let risk_free_rate: f64 = env::var("RISK_FREE_RATE")
        .unwrap_or_else(|_| "0.0".to_string())
        .parse()
        .unwrap_or(0.0);

    let mut total_delta = 0.0;
    let client = Client::new();
    let iv_api_url = env::var("IV_API_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8081/iv".to_string());

    for contract in contracts.iter() {
        let t = (contract.expires - now) as f64 / (365.0 * 24.0 * 60.0 * 60.0);
        
        let iv: f64 = client
            .get(&format!(
                "{}?side={}&strike_price={}&expire={}",
                iv_api_url, contract.side, contract.strike_price, "1d"
            ))
            .send()
            .await
            .map_err(|e| ApiError::ExternalApiError(e.to_string()))?
            .json()
            .await
            .unwrap_or(0.3);

        let delta = match contract.side {
            OptionSide::Call => {
                black_scholes::call_delta(btc_price, contract.strike_price, risk_free_rate, iv, t)
            }
            OptionSide::Put => {
                black_scholes::put_delta(btc_price, contract.strike_price, risk_free_rate, iv, t)
            }
        };

        total_delta += delta * contract.quantity;
    }

    Ok(HttpResponse::Ok().json(total_delta))
}

// GET /topBanner - Market statistics
async fn get_top_banner(state: web::Data<Arc<AppState>>) -> Result<impl Responder, ApiError> {
    let now = Utc::now().timestamp();
    let twenty_four_hours_ago = now - (24 * 60 * 60);
    
    // Validate time range
    if twenty_four_hours_ago >= now {
        return Err(ApiError::ValidationError(
            "Invalid time range".to_string(),
        ));
    }

    let conn = state.db_pool.get()?;

    // Get 24hr volume
    let volume_24hr: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(quantity), 0.0) FROM contracts WHERE created_at >= ?1",
            params![twenty_four_hours_ago],
            |row| row.get(0),
        )?;

    // Get open interest
    let mut stmt = conn.prepare(
        "SELECT quantity, premium FROM contracts WHERE expires > ?1"
    )?;
    
    let contracts_iter = stmt.query_map(params![now], |row| {
        Ok((row.get::<_, f64>(0)?, row.get::<_, f64>(1)?))
    })?;

    let mut open_interest_btc = 0.0;
    for contract in contracts_iter {
        if let Ok((quantity, premium)) = contract {
            open_interest_btc += quantity * premium;
        }
    }

    let btc_price = state
        .price_oracle
        .get_btc_price()
        .await
        .map_err(|e| ApiError::PriceOracleError(e.to_string()))?;
    
    let open_interest_usd = open_interest_btc * btc_price;

    // Get contract count
    let contract_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM contracts WHERE expires > ?1",
            params![now],
            |row| row.get(0),
        )?;

    Ok(HttpResponse::Ok().json(TopBannerResponse {
        volume_24hr,
        open_interest_usd,
        contract_count,
    }))
}

// GET /marketHighlights - Top products by volume
async fn get_market_highlights(state: web::Data<Arc<AppState>>) -> Result<impl Responder, ApiError> {
    let now = Utc::now().timestamp();
    let twenty_four_hours_ago = now - (24 * 60 * 60);

    let conn = state.db_pool.get()?;

    let mut stmt = conn.prepare(
        "SELECT side, strike_price, expires, SUM(quantity) as total_volume, AVG(premium) as avg_premium
         FROM contracts 
         WHERE created_at >= ?1
         GROUP BY side, strike_price, expires
         ORDER BY total_volume DESC
         LIMIT 6"
    )?;

    let products_iter = stmt.query_map(params![twenty_four_hours_ago], |row| {
        Ok((
            row.get::<_, OptionSide>(0)?,
            row.get::<_, f64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, f64>(3)?,
            row.get::<_, f64>(4)?,
        ))
    })?;

    let mut highlights = Vec::new();

    for product in products_iter {
        if let Ok((side, strike_price, expires, volume, current_premium)) = product {
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

            let price_change_percent = premium_24hr_ago
                .filter(|&old| old > 0.0)
                .map(|old| ((current_premium - old) / old) * 100.0)
                .unwrap_or(0.0);

            let expire_string = format_expires_timestamp(expires);

            highlights.push(MarketHighlightItem {
                product_symbol: format!("BTC-{}-{}-{}", expire_string, strike_price, side),
                side,
                strike_price,
                expire: expire_string,
                volume_24hr: volume,
                price_change_24hr_percent: price_change_percent,
            });
        }
    }

    Ok(HttpResponse::Ok().json(highlights))
}

// GET /topGainers - Top gainers by percentage
async fn get_top_gainers(state: web::Data<Arc<AppState>>) -> Result<impl Responder, ApiError> {
    let now = Utc::now().timestamp();
    let twenty_four_hours_ago = now - (24 * 60 * 60);

    let conn = state.db_pool.get()?;

    let mut stmt = conn.prepare(
        "SELECT DISTINCT side, strike_price, expires FROM contracts WHERE expires > ?1"
    )?;

    let products_iter = stmt.query_map(params![now], |row| {
        Ok((
            row.get::<_, OptionSide>(0)?,
            row.get::<_, f64>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;

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

                if let Some(old_premium) = premium_24hr_ago.filter(|&p| p > 0.0) {
                    let change_percent = ((current - old_premium) / old_premium) * 100.0;
                    let expire_string = format_expires_timestamp(expires);

                    gainers.push(TopGainerItem {
                        product_symbol: format!("BTC-{}-{}-{}", expire_string, strike_price, side),
                        side,
                        strike_price,
                        expire: expire_string,
                        change_24hr_percent: change_percent,
                        last_price: current,
                    });
                }
            }
        }
    }

    // Sort and take top 5
    gainers.sort_by(|a, b| b.change_24hr_percent.partial_cmp(&a.change_24hr_percent).unwrap());
    gainers.truncate(5);

    Ok(HttpResponse::Ok().json(gainers))
}

// GET /topVolume - Top products by volume
async fn get_top_volume(state: web::Data<Arc<AppState>>) -> Result<impl Responder, ApiError> {
    let now = Utc::now().timestamp();
    let twenty_four_hours_ago = now - (24 * 60 * 60);

    let conn = state.db_pool.get()?;

    let btc_price = state
        .price_oracle
        .get_btc_price()
        .await
        .map_err(|e| ApiError::PriceOracleError(e.to_string()))?;

    let mut stmt = conn.prepare(
        "SELECT side, strike_price, expires, SUM(quantity * premium) as total_volume_btc, AVG(premium) as avg_premium
         FROM contracts 
         WHERE created_at >= ?1
         GROUP BY side, strike_price, expires
         ORDER BY total_volume_btc DESC
         LIMIT 5"
    )?;

    let products_iter = stmt.query_map(params![twenty_four_hours_ago], |row| {
        Ok((
            row.get::<_, OptionSide>(0)?,
            row.get::<_, f64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, f64>(3)?,
            row.get::<_, f64>(4)?,
        ))
    })?;

    let mut top_volume = Vec::new();

    for product in products_iter {
        if let Ok((side, strike_price, expires, volume_btc, last_premium)) = product {
            let expire_string = format_expires_timestamp(expires);

            top_volume.push(TopVolumeItem {
                product_symbol: format!("BTC-{}-{}-{}", expire_string, strike_price, side),
                side,
                strike_price,
                expire: expire_string,
                volume_usd: volume_btc * btc_price,
                last_price: last_premium,
            });
        }
    }

    Ok(HttpResponse::Ok().json(top_volume))
}