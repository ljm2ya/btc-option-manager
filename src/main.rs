// This line brings in the necessary tools from the 'actix-web' library
// to build our web server and handle web requests.
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
// 'serde' is used for converting our Rust data structures into JSON format and back.
// JSON is a standard way for web services to talk to each other.
use serde::{Deserialize, Serialize};
// 'chrono' helps us work with dates and times, which we need for contract expiration.
use chrono::Utc;
// 'reqwest' is a library for making HTTP requests to other web services.
// We use it to get data from our mock financial APIs.
use reqwest::Client;
// 'fmt' is used to format how our custom data types are displayed as text.
use std::fmt;
// 'env' allows us to read environment variables, for configuration.
use std::env;
// 'dotenv' loads configuration variables from a .env file.
use dotenv::dotenv;
// 'rusqlite' is the library we use to interact with our SQLite database.
use rusqlite::{Connection, Result, params};
use rusqlite::types::{ToSql, FromSql, ToSqlOutput, FromSqlError, ValueRef};

// This includes the code from our 'mock_apis.rs' file,
// which runs a separate server to simulate real financial data services.
mod mock_apis;

// This defines the two types of options we support: Call and Put.
// A 'Call' option is a bet that the price will go up.
// A 'Put' option is a bet that the price will go down.
#[derive(Serialize, Deserialize, Clone, Debug)]
enum OptionSide {
    Call,
    Put,
}

// This block of code teaches our program how to save the 'OptionSide' (Call/Put)
// into the database as plain text.
impl ToSql for OptionSide {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        Ok(self.to_string().into())
    }
}

// This block of code teaches our program how to read the 'OptionSide' (Call/Put)
// from the database and convert it back into our special 'OptionSide' type.
impl FromSql for OptionSide {
    fn column_result(value: ValueRef<'_>) -> std::result::Result<Self, FromSqlError> {
        value.as_str()?.parse()
    }
}

// This allows us to convert a simple string like "Call" or "Put"
// into our special 'OptionSide' type.
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


// This allows us to print the 'OptionSide' as a string, for example, in log messages
// or when building URLs for API requests.
impl fmt::Display for OptionSide {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            OptionSide::Call => write!(f, "Call"),
            OptionSide::Put => write!(f, "Put"),
        }
    }
}

// This defines the structure for a financial contract.
// It holds all the key information about a trade.
#[derive(Serialize, Deserialize, Clone)]
struct Contract {
    side: OptionSide, // Is it a Call or a Put?
    strike_price: f64, // The price at which the option can be exercised.
    quantity: f64,    // How many units of the asset are in the contract.
    expires: i64,     // The exact time the contract expires (as a Unix timestamp).
    premium: f64,     // The price paid for the contract itself.
}

// This defines the structure for a request to our '/optionsTable' endpoint.
// It expects comma-separated strings for strike prices and expiration dates.
#[derive(Deserialize)]
struct OptionsTableRequest {
    strike_prices: String, // e.g., "100000,110000"
    expires: String,       // e.g., "1d,7d"
}

// This defines the structure for the response from our '/optionsTable' endpoint.
// It provides a detailed breakdown of available options.
#[derive(Serialize)]
struct OptionsTableResponse {
    side: OptionSide,   // Call or Put.
    strike_price: f64,  // The target price.
    expire: String,     // The expiration period (e.g., "7d").
    premium: f64,       // The calculated cost of the option.
    max_quantity: f64,  // The maximum amount that can be traded.
    iv: f64,            // Implied Volatility: a measure of market risk expectation.
}

// This function sets up our SQLite database.
// It creates a file named 'contracts.db' if it doesn't exist
// and sets up the 'contracts' table inside it.
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

// This is the main entry point of our application.
// The `#[tokio::main]` attribute sets up the asynchronous environment
// that allows our server to handle many requests at once.
#[tokio::main]
async fn main() -> std::io::Result<()> {
    // This loads configuration variables from a '.env' file in the project directory.
    // It's a good practice to keep configuration separate from code.
    dotenv().ok();

    // Initialize the database when the server starts.
    init_db().unwrap();

    // Here, we configure and start our main API server.
    // It listens for incoming requests on address 127.0.0.1 and port 8080.
    let server1 = HttpServer::new(move || {
        App::new()
            // The '.service()' calls define the different API endpoints our server has.
            // Each endpoint is associated with a function that handles requests to it.
            .service(web::resource("/contract").route(web::post().to(post_contract)))
            .service(web::resource("/contracts").route(web::get().to(get_contracts)))
            .service(web::resource("/optionsTable").route(web::get().to(get_options_table)))
            .service(web::resource("/delta").route(web::get().to(get_delta)))
    })
    .bind("127.0.0.1:8080")?
    .run();

    // We also start our mock API server, which simulates real financial data services.
    let server2 = mock_apis::mock_server();

    // This runs both servers at the same time. Our program will keep running
    // until both servers have stopped.
    let _ = tokio::join!(server1, server2);

    // If everything starts up correctly, we return 'Ok'.
    Ok(())
}

// This function handles POST requests to the '/contract' endpoint.
// It receives new contract data in JSON format and saves it to the database.
async fn post_contract(contract: web::Json<Contract>) -> impl Responder {
    // Open a connection to our SQLite database.
    let conn = Connection::open("contracts.db").unwrap();
    // Execute a SQL command to insert the new contract data into the 'contracts' table.
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

    // Return a simple 'OK' response to indicate success.
    HttpResponse::Ok().finish()
}

// This function handles GET requests to the '/contracts' endpoint.
// It's a debugging tool to see all contracts currently in the database.
async fn get_contracts() -> impl Responder {
    // Open a connection to our SQLite database.
    let conn = Connection::open("contracts.db").unwrap();
    // Prepare a SQL query to select all data from the 'contracts' table.
    let mut stmt = conn
        .prepare("SELECT side, strike_price, quantity, expires, premium FROM contracts")
        .unwrap();

    // Execute the query and map the results into a list of 'Contract' objects.
    let contracts_iter = stmt.query_map([], |row| {
        Ok(Contract {
            side: row.get(0)?,
            strike_price: row.get(1)?,
            quantity: row.get(2)?,
            expires: row.get(3)?,
            premium: row.get(4)?,
        })
    }).unwrap();

    // Collect the results into a vector.
    let contracts: Vec<Contract> = contracts_iter.map(|c| c.unwrap()).collect();
    // Return the list of contracts as a JSON response.
    HttpResponse::Ok().json(contracts)
}


// This function handles GET requests to the '/optionsTable' endpoint.
// It generates and returns a table of available Call and Put options.
async fn get_options_table(req: web::Query<OptionsTableRequest>) -> impl Responder {
    // Create a new HTTP client to make requests to our mock APIs.
    let client = Client::new();

    // Read the API URLs from environment variables, with fallbacks to the mock server.
    let pool_api_url = env::var("POOL_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8081/pool".to_string());
    let price_api_url = env::var("PRICE_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8081/price".to_string());
    let iv_api_url = env::var("IV_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8081/iv".to_string());

    // Read financial rates from environment variables, with sensible defaults.
    let risk_free_rate: f64 = env::var("RISK_FREE_RATE")
        .unwrap_or_else(|_| "0.0".to_string())
        .parse()
        .unwrap_or(0.0);

    let collateral_rate: f64 = env::var("COLLATERAL_RATE")
        .unwrap_or_else(|_| "0.5".to_string())
        .parse()
        .unwrap_or(0.5);

    // Fetch the total quantity available in the liquidity pool from the mock API.
    let pool_qty: f64 = client
        .get(&pool_api_url)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // Fetch the current price of Bitcoin from the mock API.
    let btc_price: f64 = client
        .get(&price_api_url)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // This will hold the final list of option data to be returned.
    let mut table = Vec::new();

    // Parse the comma-separated input strings into lists of numbers and strings.
    let strike_prices: Vec<f64> = req
        .strike_prices
        .split(',')
        .filter_map(|s| s.parse().ok())
        .collect();
    let expires: Vec<String> = req.expires.split(',').map(|s| s.to_string()).collect();

    // We will generate options for both Call and Put sides.
    let sides = [OptionSide::Call, OptionSide::Put];

    // Loop through every combination of strike price, expiration, and side.
    for strike_price in &strike_prices {
        for expire in &expires {
            for side in &sides {
                // Fetch the Implied Volatility (IV) for the current option from the mock API.
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

                // Convert the expiration duration (e.g., "7d") into a fraction of a year.
                let t = parse_duration(expire);

                // Calculate the premium (price) of the option using the Black-Scholes formula.
                let premium = match side {
                    OptionSide::Call => black_scholes::call(
                        btc_price,
                        *strike_price,
                        risk_free_rate, // r: risk-free interest rate
                        iv,             // v: implied volatility
                        t,              // t: time to expiration
                    ),
                    OptionSide::Put => black_scholes::put(
                        btc_price,
                        *strike_price,
                        risk_free_rate, // r
                        iv,             // v
                        t,              // t
                    ),
                };
                // Calculate the maximum quantity that can be traded based on the pool size and collateral rate.
                let max_quantity = pool_qty * collateral_rate / (premium * btc_price);

                // Add the fully calculated option data to our response table.
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

    // Return the completed table as a JSON response.
    HttpResponse::Ok().json(table)
}

// A helper function to parse a duration string (e.g., "30m", "1d")
// into a floating-point number representing a fraction of a year.
fn parse_duration(duration: &str) -> f64 {
    let d = duration.trim();
    // Splits the string into the number part and the unit part (e.g., "30" and "m").
    let (num_str, unit) = d.split_at(d.len() - 1);
    let num: f64 = num_str.parse().unwrap();
    // Convert the duration into a fraction of a year based on the unit.
    match unit {
        "m" => num / (365.0 * 24.0 * 60.0), // minutes
        "h" => num / (365.0 * 24.0),       // hours
        "d" => num / 365.0,                // days
        _ => 0.0,                          // default to 0 if unit is unknown
    }
}

// This function handles GET requests to the '/delta' endpoint.
// It calculates the total delta of all non-expired contracts in the database.
// Delta is a measure of how much an option's price is expected to change
// for a one-dollar change in the underlying asset's price.
async fn get_delta() -> impl Responder {
    let now = Utc::now().timestamp();
    // Open a connection to our SQLite database.
    let conn = Connection::open("contracts.db").unwrap();

    // Prepare a SQL query to select all contracts that have not yet expired.
    let mut stmt = conn
        .prepare("SELECT side, strike_price, quantity, expires FROM contracts WHERE expires > ?1")
        .unwrap();

    // Execute the query, with 'now' as the parameter for '?1'.
    let contracts_iter = stmt.query_map(params![now], |row| {
        Ok(Contract {
            side: row.get(0)?,
            strike_price: row.get(1)?,
            quantity: row.get(2)?,
            expires: row.get(3)?,
            premium: 0.0, // Premium is not needed for delta calculation.
        })
    }).unwrap();

    // Collect the results into a list of contracts.
    let contracts: Vec<Contract> = contracts_iter.map(|c| c.unwrap()).collect();
    
    // If there are no active contracts, the total delta is 0.
    if contracts.is_empty() {
        return HttpResponse::Ok().json(0.0);
    }

    // Create a new HTTP client to make requests to our mock APIs.
    let client = Client::new();

    // Read API URLs and the risk-free rate from environment variables.
    let price_api_url = env::var("PRICE_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8081/price".to_string());
    let iv_api_url = env::var("IV_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8081/iv".to_string());
    
    let risk_free_rate: f64 = env::var("RISK_FREE_RATE")
        .unwrap_or_else(|_| "0.0".to_string())
        .parse()
        .unwrap_or(0.0);

    // Fetch the current price of Bitcoin from the mock API.
    let btc_price: f64 = client
        .get(&price_api_url)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // This will accumulate the delta from all contracts.
    let mut total_delta = 0.0;

    // Loop through each active contract to calculate its delta.
    for contract in contracts.iter() {
        // Calculate the time to expiration as a fraction of a year.
        let t = (contract.expires - now) as f64 / (365.0 * 24.0 * 60.0 * 60.0);
        // Fetch the Implied Volatility for the current contract.
        let iv: f64 = client
            .get(&format!(
                "{}?side={}&strike_price={}&expire={}",
                iv_api_url, contract.side, contract.strike_price, "1d" // Using a dummy "1d" expire for now.
            ))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        // Calculate the delta for the contract using the appropriate Black-Scholes function.
        let delta = match contract.side {
            OptionSide::Call => {
                black_scholes::call_delta(btc_price, contract.strike_price, risk_free_rate, iv, t)
            }
            OptionSide::Put => {
                black_scholes::put_delta(btc_price, contract.strike_price, risk_free_rate, iv, t)
            }
        };
        // Add the contract's individual delta (multiplied by its quantity) to the total.
        total_delta += delta * contract.quantity;
    }

    // Return the final calculated total delta as a JSON response.
    HttpResponse::Ok().json(total_delta)
}
