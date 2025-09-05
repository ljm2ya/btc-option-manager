// This is a refactored version of main.rs with all architectural improvements
// After review, this can replace the original main.rs

use actix_web::{web, App, HttpResponse, HttpServer, Responder, middleware};
use serde::{Deserialize, Serialize};
use serde_json;
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
mod db_migration;
mod utils;
mod error;
mod risk_manager;

mod mutiny_wallet;

use crate::db::DbPool;
use crate::error::ApiError;
use crate::utils::{format_expires_timestamp, parse_duration, usd_to_cents, cents_to_usd, 
                   float_to_db_string, db_string_to_float, format_btc, round_btc, BTC_PRECISION};
use crate::mutiny_wallet::{MutinyWallet, Network};
use crate::risk_manager::{RiskManager};

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

// Contract structure for API input/output (uses floats for backward compatibility)
#[derive(Serialize, Deserialize, Clone)]
struct Contract {
    side: OptionSide,
    strike_price: f64,
    quantity: f64,
    expires: i64,
    premium: f64,
}

// Internal contract structure for database storage (uses strings for precision)
#[derive(Clone, Debug)]
struct ContractDb {
    side: OptionSide,
    strike_price_cents: i64,
    quantity_str: String,
    expires: i64,
    premium_str: String,
}

impl ContractDb {
    // Convert from API contract to DB contract
    fn from_contract(contract: &Contract) -> Self {
        Self {
            side: contract.side.clone(),
            strike_price_cents: usd_to_cents(contract.strike_price),
            quantity_str: float_to_db_string(round_btc(contract.quantity), BTC_PRECISION),
            expires: contract.expires,
            premium_str: float_to_db_string(round_btc(contract.premium), BTC_PRECISION),
        }
    }
    
    // Convert to API contract when needed for calculations
    fn to_contract(&self) -> Contract {
        Contract {
            side: self.side.clone(),
            strike_price: cents_to_usd(self.strike_price_cents),
            quantity: db_string_to_float(&self.quantity_str).unwrap_or(0.0),
            expires: self.expires,
            premium: db_string_to_float(&self.premium_str).unwrap_or(0.0),
        }
    }
}

// Request/Response structures
#[derive(Serialize)]
struct OptionsTableResponse {
    side: OptionSide,
    strike_price: f64,
    expire: String,
    premium: String,  // BTC amount as string for precision
    max_quantity: String,  // BTC amount as string for precision
    iv: f64,
    delta: f64,
}

// Contract response with string fields for precision
#[derive(Serialize)]
struct ContractResponse {
    side: OptionSide,
    strike_price: f64,
    quantity: String,  // BTC amount as string
    expires: i64,
    premium: String,   // BTC amount as string
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
    
    // Initialize IV oracle with data before starting server
    println!("🔄 Initializing IV Oracle with market data...");
    if let Err(e) = iv_oracle.initialize().await {
        eprintln!("WARNING: Failed to initialize IV Oracle: {}", e);
        eprintln!("The server will start but IV data may not be immediately available.");
    }
    
    // Start background updates after initial data is loaded
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
        pool_address: pool_address.clone(),
    });
    
    // Check pool wallet balance at initialization
    println!("🔍 Checking pool wallet balance at startup...");
    match app_state.get_pool_balance_btc().await {
        Ok(balance_btc) => {
            println!("✅ Pool wallet balance: {} BTC", balance_btc);
            
            // Get collateral rate from environment
            let collateral_rate: f64 = env::var("COLLATERAL_RATE")
                .unwrap_or_else(|_| "0.5".to_string())
                .parse()
                .unwrap_or(0.5);
            
            // Get current BTC price to show USD value
            match price_oracle.get_btc_price().await {
                Ok(btc_price) => {
                    let balance_usd = balance_btc * btc_price;
                    let tradeable_btc = balance_btc * collateral_rate;
                    let tradeable_usd = balance_usd * collateral_rate;
                    
                    println!("💰 Pool Balance Details:");
                    println!("   Total: {} BTC (${:.2} USD)", balance_btc, balance_usd);
                    println!("   Collateral Rate: {:.0}%", collateral_rate * 100.0);
                    println!("   Tradeable Amount: {} BTC (${:.2} USD)", tradeable_btc, tradeable_usd);
                    println!("   Network: {:?}", pool_network);
                    println!("   Address: {}", pool_address);
                },
                Err(e) => {
                    println!("⚠️  Could not fetch BTC price at startup: {}", e);
                    println!("   Pool Balance: {} BTC", balance_btc);
                    println!("   Collateral Rate: {:.0}%", collateral_rate * 100.0);
                }
            }
        },
        Err(e) => {
            eprintln!("⚠️  WARNING: Could not fetch pool balance at startup: {}", e);
            eprintln!("   The application will continue, but max_quantity calculations may fail.");
            eprintln!("   Please ensure:");
            eprintln!("   1. POOL_ADDRESS is set correctly in .env");
            eprintln!("   2. The address has a balance on the {} network", pool_network);
            eprintln!("   3. Network connectivity is available");
        }
    }
    
    println!("🚀 Starting BTC Options API server...");

    // Configure and start the main API server on port 8080
    let server1 = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .wrap(middleware::Logger::default())
            // Health check endpoints
            .route("/", web::get().to(health_check))
            .route("/health", web::get().to(health_check))
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
    .bind("0.0.0.0:8080")?
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

// Helper function to convert duration strings to seconds
fn duration_to_seconds(duration: &str) -> i64 {
    let d = duration.trim();
    let (num_str, unit) = d.split_at(d.len() - 1);
    let num: i64 = num_str.parse().unwrap_or(0);
    
    match unit {
        "m" => num * 60,           // minutes to seconds
        "h" => num * 60 * 60,      // hours to seconds  
        "d" => num * 24 * 60 * 60, // days to seconds
        _ => 0,
    }
}

// GET / - Health check endpoint
async fn health_check() -> Result<impl Responder, ApiError> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "BTC Options API",
        "version": "1.0.0"
    })))
}

// POST /contract - Create new contract
async fn post_contract(
    contract: web::Json<Contract>,
    state: web::Data<Arc<AppState>>,
) -> Result<impl Responder, ApiError> {
    // Log incoming contract request
    println!("📥 POST /contract request:");
    println!("   Side: {:?}", contract.side);
    println!("   Strike: ${:.2}", contract.strike_price);
    println!("   Quantity: {:.8} BTC", contract.quantity);
    println!("   Premium: {:.8} BTC", contract.premium);
    println!("   Expires: {}", contract.expires);
    
    // Validation
    let now = Utc::now().timestamp();
    if contract.expires <= now {
        eprintln!("❌ Contract validation failed: expiration date ({}) is not in the future (now: {})", contract.expires, now);
        return Err(ApiError::ValidationError(
            "Contract expiration date must be in the future.".to_string(),
        ));
    }

    // Get collateral parameters
    let collateral_rate: f64 = env::var("COLLATERAL_RATE")
        .unwrap_or_else(|_| "0.5".to_string())
        .parse()
        .unwrap_or(0.5);

    // Get real pool balance from Mutiny wallet (actual BTC balance from blockchain)
    let pool_qty: f64 = state.get_pool_balance_btc().await?;

    // Get BTC price from oracle
    let btc_price = state
        .price_oracle
        .get_btc_price()
        .await
        .map_err(|e| ApiError::PriceOracleError(e.to_string()))?;

    // Initialize risk manager
    let risk_margin = env::var("RISK_MARGIN")
        .unwrap_or_else(|_| "1.2".to_string())
        .parse()
        .unwrap_or(1.2);
    let risk_free_rate: f64 = env::var("RISK_FREE_RATE")
        .unwrap_or_else(|_| "0.0".to_string())
        .parse()
        .unwrap_or(0.0);
    
    let risk_manager = RiskManager::new(risk_margin);
    
    // Get existing contracts to calculate current risk exposure
    let conn = state.db_pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT side, strike_price_cents, quantity_str, expires, premium_str FROM contracts WHERE expires > ?1"
    )?;
    
    let contracts_iter = stmt.query_map(params![now], |row| {
        let quantity_str: String = row.get(2)?;
        let premium_str: String = row.get(4)?;
        
        Ok(Contract {
            side: row.get(0)?,
            strike_price: cents_to_usd(row.get(1)?),
            quantity: db_string_to_float(&quantity_str).unwrap_or(0.0),
            expires: row.get(3)?,
            premium: db_string_to_float(&premium_str).unwrap_or(0.0),
        })
    })?;
    
    let mut existing_contracts: Vec<Contract> = contracts_iter
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
    
    // Calculate current risk exposure WITHOUT the new contract
    let iv_oracle_closure = |side_str: &str, strike: f64, expire: &str| {
        state.iv_oracle.get_iv(side_str, strike, expire)
    };
    
    let total_existing_risk = risk_manager.calculate_portfolio_risk(
        &existing_contracts,
        btc_price,
        risk_free_rate,
        &iv_oracle_closure,
    );
    
    // Calculate available collateral
    let total_collateral_usd = pool_qty * btc_price * collateral_rate;
    let available_collateral_usd = total_collateral_usd - total_existing_risk;
    
    // Get IV for the new contract
    let time_to_expiry = (contract.expires - now) as f64 / (365.0 * 24.0 * 60.0 * 60.0);
    let side_str = match contract.side {
        OptionSide::Call => "C",
        OptionSide::Put => "P",
    };
    let expire_timestamp_ms = (contract.expires * 1000).to_string();
    let iv = state.iv_oracle.get_iv(side_str, contract.strike_price, &expire_timestamp_ms)
        .unwrap_or(0.4);
    
    // Calculate maximum allowed quantity for this specific contract
    let max_quantity = risk_manager.calculate_max_quantity(
        &contract.side,
        contract.strike_price,
        contract.premium,
        btc_price,
        iv,
        time_to_expiry,
        risk_free_rate,
        available_collateral_usd,
        total_existing_risk,
    );
    
    // Log risk calculation details
    println!("📊 Contract Risk Analysis:");
    println!("   Contract: {} expires {} @ ${} for {} qty", 
        contract.side, contract.expires, contract.strike_price, contract.quantity);
    println!("   Max allowed quantity: {:.2}", max_quantity);
    println!("   Available collateral: ${:.2}", available_collateral_usd);
    println!("   Existing portfolio risk: ${:.2}", total_existing_risk);
    
    // Check if requested quantity exceeds maximum allowed
    if contract.quantity > max_quantity {
        eprintln!("❌ Contract validation failed: requested quantity ({:.8}) exceeds maximum allowed ({:.8})", 
            contract.quantity, max_quantity);
        eprintln!("   Available collateral: ${:.2}", available_collateral_usd);
        eprintln!("   Existing risk exposure: ${:.2}", total_existing_risk);
        eprintln!("   Total collateral pool: ${:.2}", total_collateral_usd);
        return Err(ApiError::ValidationError(
            format!(
                "Requested quantity ({:.8}) exceeds maximum allowed quantity ({:.8}). \
                Available collateral: ${:.2}, \
                Existing risk exposure: ${:.2}, \
                Total collateral pool: ${:.2}",
                contract.quantity,
                max_quantity,
                available_collateral_usd,
                total_existing_risk,
                total_collateral_usd
            ),
        ));
    }
    
    // Now check total risk with the new contract
    existing_contracts.push((*contract).clone());
    let total_risk_with_new = risk_manager.calculate_portfolio_risk(
        &existing_contracts,
        btc_price,
        risk_free_rate,
        &iv_oracle_closure,
    );
    
    if total_risk_with_new > total_collateral_usd {
        // This should not happen if max_quantity check above is working correctly
        // But we keep it as a safety check
        let position_risk = risk_manager.calculate_position_risk(
            &contract.side,
            contract.strike_price,
            contract.premium,
            contract.quantity,
            btc_price,
            iv,
            time_to_expiry,
            risk_free_rate,
        );
        
        eprintln!("❌ Contract validation failed: risk exceeds available collateral");
        eprintln!("   New position margin required: ${:.2}", position_risk.margin_required);
        eprintln!("   Total portfolio margin would be: ${:.2}", total_risk_with_new);
        eprintln!("   Available collateral: ${:.2}", total_collateral_usd);
        
        return Err(ApiError::ValidationError(
            format!(
                "Contract risk exceeds available collateral. \
                New position margin required: ${:.2}, \
                Total portfolio margin would be: ${:.2}, \
                Available collateral: ${:.2}",
                position_risk.margin_required,
                total_risk_with_new,
                total_collateral_usd
            ),
        ));
    }

    // Save to database with proper conversions
    let rounded_quantity = round_btc(contract.quantity);
    let rounded_premium = round_btc(contract.premium);
    
    conn.execute(
        "INSERT INTO contracts (side, strike_price_cents, quantity_str, expires, premium_str) 
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            contract.side,
            usd_to_cents(contract.strike_price),
            float_to_db_string(rounded_quantity, BTC_PRECISION),
            contract.expires,
            float_to_db_string(rounded_premium, BTC_PRECISION)
        ],
    )?;

    // Save to premium history
    let product_key = format!("{}-{}-{}", contract.side, usd_to_cents(contract.strike_price), contract.expires);
    let _ = conn.execute(
        "INSERT OR REPLACE INTO premium_history (product_key, side, strike_price_cents, expires, premium_str) 
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            product_key,
            contract.side,
            usd_to_cents(contract.strike_price),
            contract.expires,
            float_to_db_string(rounded_premium, BTC_PRECISION)
        ],
    );

    Ok(HttpResponse::Ok().finish())
}

// GET /contracts - List all contracts
async fn get_contracts(state: web::Data<Arc<AppState>>) -> Result<impl Responder, ApiError> {
    let conn = state.db_pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT side, strike_price_cents, quantity_str, expires, premium_str FROM contracts"
    )?;

    let contracts_iter = stmt.query_map([], |row| {
        Ok(ContractResponse {
            side: row.get(0)?,
            strike_price: cents_to_usd(row.get(1)?),
            quantity: row.get(2)?,  // Keep as string
            expires: row.get(3)?,
            premium: row.get(4)?,   // Keep as string
        })
    })?;

    let contracts: Vec<ContractResponse> = contracts_iter
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(contracts))
}

// GET /optionsTable - Generate options table with automatic parameters
async fn get_options_table(
    state: web::Data<Arc<AppState>>,
) -> Result<impl Responder, ApiError> {
    // Get current BTC price from gRPC oracle
    let btc_price = state
        .price_oracle
        .get_btc_price()
        .await
        .map_err(|e| ApiError::PriceOracleError(e.to_string()))?;
    
    println!("📊 Generating options table for BTC price: ${:.2}", btc_price);
    
    // Check IV cache status
    let cache_size = state.iv_oracle.get_cache_size();
    if cache_size > 0 {
        println!("✅ IV cache populated with {} entries", cache_size);
    } else {
        println!("⚠️ IV cache is empty - fetching may be slower");
    }
    
    // Generate strike prices: round to nearest 1000, then ±5 strikes
    let center_strike = (btc_price / 1000.0).round() * 1000.0;
    let mut strike_prices: Vec<f64> = Vec::new();
    
    // Generate strikes from -5000 to +5000 relative to center
    for i in -5..=5 {
        let strike = center_strike + (i as f64 * 1000.0);
        if strike > 0.0 { // Ensure positive strikes only
            strike_prices.push(strike);
        }
    }
    
    // Sort strikes in ascending order
    strike_prices.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    println!("🎯 Generated {} strike prices: {:?}", strike_prices.len(), strike_prices);
    
    // Generate expiries: 1d, 2d, 3d, 5d, 7d
    let expires: Vec<String> = vec![
        "1d".to_string(),
        "2d".to_string(), 
        "3d".to_string(),
        "5d".to_string(),
        "7d".to_string(),
    ];
    
    println!("⏰ Generated expiries: {:?}", expires);

    // Get financial parameters
    let risk_free_rate: f64 = env::var("RISK_FREE_RATE")
        .unwrap_or_else(|_| "0.0".to_string())
        .parse()
        .unwrap_or(0.0);
    let collateral_rate: f64 = env::var("COLLATERAL_RATE")
        .unwrap_or_else(|_| "0.5".to_string())
        .parse()
        .unwrap_or(0.5);

    // Get real pool balance from Mutiny wallet (actual BTC balance from blockchain)
    let pool_qty: f64 = state.get_pool_balance_btc().await?;

    // Initialize client for IV fallback
    let client = Client::new();

    let btc_price = state
        .price_oracle
        .get_btc_price()
        .await
        .map_err(|e| ApiError::PriceOracleError(e.to_string()))?;

    // Initialize risk manager with 20% safety margin
    let risk_margin = env::var("RISK_MARGIN")
        .unwrap_or_else(|_| "1.2".to_string())
        .parse()
        .unwrap_or(1.2);
    let risk_manager = RiskManager::new(risk_margin);
    
    // Get existing contracts to calculate current risk exposure
    let conn = state.db_pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT side, strike_price_cents, quantity_str, expires, premium_str FROM contracts WHERE expires > ?1"
    )?;
    let now = Utc::now().timestamp();
    
    let contracts_iter = stmt.query_map(params![now], |row| {
        let quantity_str: String = row.get(2)?;
        let premium_str: String = row.get(4)?;
        
        Ok(Contract {
            side: row.get(0)?,
            strike_price: cents_to_usd(row.get(1)?),
            quantity: db_string_to_float(&quantity_str).unwrap_or(0.0),
            expires: row.get(3)?,
            premium: db_string_to_float(&premium_str).unwrap_or(0.0),
        })
    })?;
    
    let existing_contracts: Vec<Contract> = contracts_iter
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
    
    // Calculate total existing risk exposure
    let iv_oracle_closure = |side_str: &str, strike: f64, expire: &str| {
        state.iv_oracle.get_iv(side_str, strike, expire)
    };
    
    let total_existing_risk = risk_manager.calculate_portfolio_risk(
        &existing_contracts,
        btc_price,
        risk_free_rate,
        &iv_oracle_closure,
    );
    
    // Calculate available collateral
    let total_collateral_usd = pool_qty * btc_price * collateral_rate;
    let available_collateral_usd = total_collateral_usd - total_existing_risk;
    
    println!("💰 Risk Analysis:");
    println!("   Total Collateral: ${:.2}", total_collateral_usd);
    println!("   Existing Risk Exposure: ${:.2}", total_existing_risk);
    println!("   Available Collateral: ${:.2}", available_collateral_usd);
    println!("   Risk Margin: {:.0}%", (risk_margin - 1.0) * 100.0);

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

                // Convert expire string to timestamp for IV oracle
                let expire_for_iv = if expire.ends_with('d') || expire.ends_with('h') || expire.ends_with('m') {
                    // For durations, calculate future timestamp in milliseconds
                    let duration_seconds = duration_to_seconds(expire);
                    let future_timestamp_ms = (Utc::now().timestamp() + duration_seconds) * 1000;
                    future_timestamp_ms.to_string()
                } else {
                    // Assume it's already a timestamp or other format
                    expire.clone()
                };
                
                // Get IV from cache (should be pre-populated)
                let iv = state.iv_oracle.get_iv(side_str, *strike_price, &expire_for_iv)
                    .unwrap_or(0.3); // Default IV if not found in cache

                let t = parse_duration(expire);

                // Calculate premium using Black-Scholes (returns USD value)
                let premium_usd = match side {
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
                
                // Convert premium from USD to BTC
                let premium_btc = premium_usd / btc_price;

                // Calculate delta using Black-Scholes
                let delta = match side {
                    OptionSide::Call => black_scholes::call_delta(
                        btc_price,
                        *strike_price,
                        risk_free_rate,
                        iv,
                        t,
                    ),
                    OptionSide::Put => black_scholes::put_delta(
                        btc_price,
                        *strike_price,
                        risk_free_rate,
                        iv,
                        t,
                    ),
                };

                // Calculate risk-based max_quantity considering:
                // 1. Option-specific risk (max loss potential)
                // 2. Existing portfolio risk exposure
                // 3. Available collateral after risk margin
                let max_quantity = risk_manager.calculate_max_quantity(
                    side,
                    *strike_price,
                    premium_btc,
                    btc_price,
                    iv,
                    t,
                    risk_free_rate,
                    available_collateral_usd,
                    total_existing_risk,
                );

                table.push(OptionsTableResponse {
                    side: side.clone(),
                    strike_price: *strike_price,
                    expire: expire.clone(),
                    premium: format_btc(premium_btc),  // Format as string with 8 decimals
                    max_quantity: format_btc(max_quantity),  // Format as string with 8 decimals
                    iv,
                    delta,
                });
            }
        }
    }

    // Display formatted options table
    println!("\n📊 Generated Options Table Summary:");
    println!("   Total Options: {}", table.len());
    println!("   Strike Prices: {} (from ${} to ${})", 
        strike_prices.len(), 
        strike_prices.first().unwrap_or(&0.0),
        strike_prices.last().unwrap_or(&0.0)
    );
    println!("   Expiries: {}", expires.len());
    println!("   Current BTC Price: ${:.2}", btc_price);
    
    // Create formatted table display
    println!("\n🎯 Options Table:");
    println!("{:-<140}", "-");
    println!("{:<6} {:<10} {:<10} {:<12} {:<12} {:<10} {:<10} {:<12}", 
        "Type", "Strike", "Expiry", "Premium(BTC)", "Max Qty", "IV", "Delta", "Value(USD)");
    println!("{:-<140}", "-");
    
    // Group by expiry for better display
    for expire in &expires {
        println!("\n📅 Expiry: {}", expire);
        
        // Sort options for this expiry by strike price
        let mut expiry_options: Vec<&OptionsTableResponse> = table.iter()
            .filter(|opt| opt.expire == *expire)
            .collect();
        expiry_options.sort_by(|a, b| {
            a.strike_price.partial_cmp(&b.strike_price).unwrap()
                .then(a.side.to_string().cmp(&b.side.to_string()))
        });
        
        for opt in expiry_options {
            // Convert string premium to float only for calculation
            let premium_f64 = db_string_to_float(&opt.premium).unwrap_or(0.0);
            let option_value = premium_f64 * btc_price;
            println!("{:<6} ${:<9.0} {:<10} ₿{:<11} {:<11} {:<9.4} {:<9.4} ${:<11.2}", 
                format!("{}", opt.side),
                opt.strike_price,
                opt.expire,
                opt.premium,     // Already formatted string
                opt.max_quantity, // Already formatted string
                opt.iv,
                opt.delta,
                option_value
            );
        }
    }
    
    println!("{:-<140}", "-");
    println!("\n💰 Pool Information:");
    println!("   Pool Balance: {} BTC (${:.2} USD)", pool_qty, pool_qty * btc_price);
    println!("   Collateral Rate: {:.0}%", collateral_rate * 100.0);
    
    Ok(HttpResponse::Ok().json(table))
}

// GET /delta - Calculate portfolio delta
async fn get_delta(state: web::Data<Arc<AppState>>) -> Result<impl Responder, ApiError> {
    let now = Utc::now().timestamp();
    let conn = state.db_pool.get()?;

    let mut stmt = conn.prepare(
        "SELECT side, strike_price_cents, quantity_str, expires, premium_str FROM contracts WHERE expires > ?1"
    )?;

    let contracts_iter = stmt.query_map(params![now], |row| {
        let quantity_str: String = row.get(2)?;
        let premium_str: String = row.get(4)?;
        Ok(Contract {
            side: row.get(0)?,
            strike_price: cents_to_usd(row.get(1)?),
            quantity: db_string_to_float(&quantity_str).unwrap_or(0.0),
            expires: row.get(3)?,
            premium: db_string_to_float(&premium_str).unwrap_or(0.0),
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

    for contract in contracts.iter() {
        let t = (contract.expires - now) as f64 / (365.0 * 24.0 * 60.0 * 60.0);
        
        // Convert contract.expires (seconds) to milliseconds for IV oracle
        let expire_timestamp_ms = (contract.expires * 1000).to_string();
        
        // Try to get IV from oracle first
        let side_str = match contract.side {
            OptionSide::Call => "C",
            OptionSide::Put => "P",
        };
        
        let iv: f64 = state.iv_oracle.get_iv(side_str, contract.strike_price, &expire_timestamp_ms)
            .unwrap_or(0.3); // Default IV if not found in cache

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
            "SELECT COALESCE(SUM(CAST(quantity_str AS REAL)), 0.0) FROM contracts WHERE created_at >= ?1",
            params![twenty_four_hours_ago],
            |row| row.get(0),
        )?;

    // Get open interest
    let mut stmt = conn.prepare(
        "SELECT quantity_str, premium_str FROM contracts WHERE expires > ?1"
    )?;
    
    let contracts_iter = stmt.query_map(params![now], |row| {
        let quantity_str: String = row.get(0)?;
        let premium_str: String = row.get(1)?;
        Ok((
            db_string_to_float(&quantity_str).unwrap_or(0.0),
            db_string_to_float(&premium_str).unwrap_or(0.0)
        ))
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
        "SELECT side, strike_price_cents, expires, 
                SUM(CAST(quantity_str AS REAL)) as total_volume, 
                AVG(CAST(premium_str AS REAL)) as avg_premium
         FROM contracts 
         WHERE created_at >= ?1
         GROUP BY side, strike_price_cents, expires
         ORDER BY total_volume DESC
         LIMIT 6"
    )?;

    let products_iter = stmt.query_map(params![twenty_four_hours_ago], |row| {
        Ok((
            row.get::<_, OptionSide>(0)?,
            row.get::<_, i64>(1)?,  // strike_price_cents
            row.get::<_, i64>(2)?,
            row.get::<_, f64>(3)?,
            row.get::<_, f64>(4)?,
        ))
    })?;

    let mut highlights = Vec::new();

    for product in products_iter {
        if let Ok((side, strike_price_cents, expires, volume, current_premium)) = product {
            let product_key = format!("{}-{}-{}", side, strike_price_cents, expires);
            let strike_price = cents_to_usd(strike_price_cents);

            // Get premium from 24 hours ago
            let premium_24hr_ago_str: Option<String> = conn
                .query_row(
                    "SELECT premium_str FROM premium_history 
                     WHERE product_key = ?1 AND timestamp <= ?2 
                     ORDER BY timestamp DESC LIMIT 1",
                    params![&product_key, twenty_four_hours_ago],
                    |row| row.get(0),
                )
                .ok();

            let premium_24hr_ago = premium_24hr_ago_str
                .and_then(|s| db_string_to_float(&s).ok())
                .unwrap_or(0.0);

            let price_change_percent = if premium_24hr_ago > 0.0 {
                ((current_premium - premium_24hr_ago) / premium_24hr_ago) * 100.0
            } else {
                0.0
            };

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
        "SELECT DISTINCT side, strike_price_cents, expires FROM contracts WHERE expires > ?1"
    )?;

    let products_iter = stmt.query_map(params![now], |row| {
        Ok((
            row.get::<_, OptionSide>(0)?,
            row.get::<_, i64>(1)?,  // strike_price_cents
            row.get::<_, i64>(2)?,
        ))
    })?;

    let mut gainers = Vec::new();

    for product in products_iter {
        if let Ok((side, strike_price_cents, expires)) = product {
            // Get current premium
            let current_premium_str: Option<String> = conn
                .query_row(
                    "SELECT premium_str FROM contracts 
                     WHERE side = ?1 AND strike_price_cents = ?2 AND expires = ?3 
                     ORDER BY id DESC LIMIT 1",
                    params![&side, strike_price_cents, expires],
                    |row| row.get(0),
                )
                .ok();

            if let Some(current_str) = current_premium_str {
                let current = db_string_to_float(&current_str).unwrap_or(0.0);
                let product_key = format!("{}-{}-{}", side, strike_price_cents, expires);

                // For new contracts (< 24hr old), use creation premium as baseline
                // For older contracts, try to get premium from 24 hours ago
                let baseline_premium_str: Option<String> = conn
                    .query_row(
                        "SELECT premium_str FROM premium_history 
                         WHERE product_key = ?1 AND timestamp <= ?2 
                         ORDER BY timestamp DESC LIMIT 1",
                        params![&product_key, twenty_four_hours_ago],
                        |row| row.get(0),
                    )
                    .ok()
                    .or_else(|| {
                        // If no data from 24hr ago, get the earliest premium for this product from history
                        conn.query_row(
                            "SELECT premium_str FROM premium_history 
                             WHERE product_key = ?1 
                             ORDER BY timestamp ASC LIMIT 1",
                            params![&product_key],
                            |row| row.get(0),
                        )
                        .ok()
                    })
                    .or_else(|| {
                        // If no premium history at all, use the earliest contract premium as baseline
                        conn.query_row(
                            "SELECT premium_str FROM contracts 
                             WHERE side = ?1 AND strike_price_cents = ?2 AND expires = ?3 
                             ORDER BY id ASC LIMIT 1",
                            params![&side, strike_price_cents, expires],
                            |row| row.get(0),
                        )
                        .ok()
                    });

                if let Some(baseline_str) = baseline_premium_str {
                    let baseline_premium = db_string_to_float(&baseline_str).unwrap_or(0.0);
                    if baseline_premium > 0.0 {
                        let change_percent = if current != baseline_premium {
                            ((current - baseline_premium) / baseline_premium) * 100.0
                        } else {
                            // For new contracts with no price change, show 0% change
                            // This ensures they appear in the list
                            0.0
                        };
                        let expire_string = format_expires_timestamp(expires);
                        let strike_price = cents_to_usd(strike_price_cents);

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
        "SELECT side, strike_price_cents, expires, 
                SUM(CAST(quantity_str AS REAL) * CAST(premium_str AS REAL)) as total_volume_btc, 
                AVG(CAST(premium_str AS REAL)) as avg_premium
         FROM contracts 
         WHERE created_at >= ?1
         GROUP BY side, strike_price_cents, expires
         ORDER BY total_volume_btc DESC
         LIMIT 5"
    )?;

    let products_iter = stmt.query_map(params![twenty_four_hours_ago], |row| {
        Ok((
            row.get::<_, OptionSide>(0)?,
            row.get::<_, i64>(1)?,  // strike_price_cents
            row.get::<_, i64>(2)?,
            row.get::<_, f64>(3)?,
            row.get::<_, f64>(4)?,
        ))
    })?;

    let mut top_volume = Vec::new();

    for product in products_iter {
        if let Ok((side, strike_price_cents, expires, volume_btc, last_premium)) = product {
            let expire_string = format_expires_timestamp(expires);
            let strike_price = cents_to_usd(strike_price_cents);

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