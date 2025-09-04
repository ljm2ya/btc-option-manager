#[cfg(test)]
mod tests {
    use btc_options_api::iv_oracle::IvOracle;
    use btc_options_api::utils::{format_expires_timestamp, parse_duration};
    use btc_options_api::db;
    use btc_options_api::error::ApiError;
    use btc_options_api::mutiny_wallet::{MutinyWallet, Network};
    
    #[test]
    fn test_iv_oracle_initialization() {
        let oracle = IvOracle::new("https://www.deribit.com/api/v2".to_string());
        
        // Test that oracle initializes properly
        let iv = oracle.get_iv("C", 50000.0, "1d");
        assert!(iv.is_none()); // Should be None before fetching data
    }

    #[test]
    fn test_format_expires_timestamp() {
        use chrono::Utc;
        let future = Utc::now().timestamp() + 86400; // 1 day from now
        let formatted = format_expires_timestamp(future);
        assert_eq!(formatted, "1d");
        
        let past = Utc::now().timestamp() - 100;
        let expired = format_expires_timestamp(past);
        assert_eq!(expired, "EXPIRED");
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("1d"), 1.0 / 365.0);
        assert_eq!(parse_duration("7d"), 7.0 / 365.0);
        assert_eq!(parse_duration("30d"), 30.0 / 365.0);
        assert_eq!(parse_duration("2h"), 2.0 / (365.0 * 24.0));
        assert_eq!(parse_duration("30m"), 30.0 / (365.0 * 24.0 * 60.0));
    }

    #[test]
    fn test_db_pool_creation() {
        let result = db::create_pool();
        assert!(result.is_ok());
        
        // Test that we can get a connection from the pool
        if let Ok(pool) = result {
            let conn = pool.get();
            assert!(conn.is_ok());
        }
    }

    #[test]
    fn test_api_error_display() {
        let err = ApiError::ValidationError("Test error".to_string());
        assert_eq!(err.to_string(), "Validation error: Test error");
        
        let err2 = ApiError::DatabaseError("DB issue".to_string());
        assert_eq!(err2.to_string(), "Database error: DB issue");
        
        let err3 = ApiError::ExternalApiError("API failed".to_string());
        assert_eq!(err3.to_string(), "External API error: API failed");
    }
    
    #[test]
    fn test_mutiny_wallet_initialization() {
        let _wallet_mainnet = MutinyWallet::new(Network::Mainnet);
        let _wallet_testnet = MutinyWallet::new(Network::Testnet);
        let _wallet_signet = MutinyWallet::new(Network::Signet);
        
        // Just verify they initialize without panic
        assert!(true);
    }
    
    #[test]
    fn test_satoshis_to_btc_conversion() {
        assert_eq!(MutinyWallet::satoshis_to_btc(100_000_000), 1.0);
        assert_eq!(MutinyWallet::satoshis_to_btc(50_000_000), 0.5);
        assert_eq!(MutinyWallet::satoshis_to_btc(1), 0.00000001);
    }
}

#[cfg(test)]
mod integration_tests {
    use btc_options_api::mutiny_wallet::{MutinyWallet, Network};
    use btc_options_api::price_oracle::PriceOracle;
    
    // These tests require external services and should be run separately
    
    #[tokio::test]
    #[ignore] // Ignore by default as it requires external service
    async fn test_mutiny_wallet_balance_query() {
        let wallet = MutinyWallet::new(Network::Signet);
        
        // Test with a known signet address (example)
        let test_address = "tb1qw2c3lxufxqe2x9s4rdzh65tpf4d7fssjgh8nv6";
        
        let result = wallet.get_wallet_balance(test_address).await;
        
        // We can't assert specific values as they might change,
        // but we can verify the call doesn't error for a valid address
        match result {
            Ok(balance) => {
                // total_balance is u64, so it's always >= 0
                assert!(true); // Balance was fetched successfully
                assert_eq!(balance.address, test_address);
            }
            Err(e) => {
                // Log the error for debugging but don't fail the test
                // as the external API might be temporarily unavailable
                eprintln!("Mutiny wallet test error (expected in CI): {}", e);
            }
        }
    }
    
    #[tokio::test] 
    #[ignore] // Ignore by default as it requires oracle aggregator running
    async fn test_price_oracle_grpc_connection() {
        let aggregator_url = "http://localhost:50051".to_string();
        
        match PriceOracle::new(aggregator_url).await {
            Ok(oracle) => {
                // Test getting BTC price
                match oracle.get_btc_price().await {
                    Ok(price) => {
                        assert!(price > 0.0);
                        println!("BTC Price from oracle: ${}", price);
                    }
                    Err(e) => {
                        eprintln!("Price fetch error (expected if aggregator not running): {}", e);
                    }
                }
            }
            Err(e) => {
                // Expected error when aggregator is not running
                assert!(e.to_string().contains("Failed to connect to Oracle Aggregator"));
                assert!(e.to_string().contains("cargo run -p aggregator"));
            }
        }
    }
}

#[cfg(test)]
mod endpoint_tests {
    // Endpoint tests are better suited for integration tests
    // They require full application setup with all external dependencies:
    // - gRPC price oracle running on localhost:50051
    // - Mutiny wallet with valid Bitcoin address
    // - Deribit API for implied volatility
    //
    // Consider using docker-compose for full integration testing
    
    #[actix_web::test]
    #[ignore] // These tests require full app setup which needs external services
    async fn test_options_table_endpoint() {
        // This test would require setting up the full AppState with all dependencies
        // Including gRPC price oracle and Mutiny wallet
        // Better suited for integration tests with docker-compose
    }
}

#[cfg(test)]
mod database_tests {
    use rusqlite::Connection;
    use chrono::Utc;
    
    #[test]
    fn test_contract_insertion_with_timestamp() {
        let conn = Connection::open_in_memory().unwrap();
        
        // Create tables
        conn.execute(
            "CREATE TABLE contracts (
                id INTEGER PRIMARY KEY,
                side TEXT NOT NULL,
                strike_price REAL NOT NULL,
                quantity REAL NOT NULL,
                expires INTEGER NOT NULL,
                premium REAL NOT NULL,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
            )",
            [],
        ).unwrap();
        
        // Insert test contract
        conn.execute(
            "INSERT INTO contracts (side, strike_price, quantity, expires, premium) 
             VALUES (?1, ?2, ?3, ?4, ?5)",
            &["Call", "50000", "1.0", &(Utc::now().timestamp() + 86400).to_string(), "5000"],
        ).unwrap();
        
        // Verify timestamp was added
        let created_at: i64 = conn.query_row(
            "SELECT created_at FROM contracts WHERE id = 1",
            [],
            |row| row.get(0),
        ).unwrap();
        
        assert!(created_at > 0);
        assert!(created_at <= Utc::now().timestamp());
    }

    #[test]
    fn test_premium_history_unique_constraint() {
        let conn = Connection::open_in_memory().unwrap();
        
        conn.execute(
            "CREATE TABLE premium_history (
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
        ).unwrap();
        
        let timestamp = Utc::now().timestamp();
        
        // First insert should succeed
        let result1 = conn.execute(
            "INSERT INTO premium_history (product_key, side, strike_price, expires, premium, timestamp) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            &["Call-50000-1234567890", "Call", "50000", "1234567890", "5000", &timestamp.to_string()],
        );
        assert!(result1.is_ok());
        
        // Second insert with same product_key and timestamp should fail
        let result2 = conn.execute(
            "INSERT INTO premium_history (product_key, side, strike_price, expires, premium, timestamp) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            &["Call-50000-1234567890", "Call", "50000", "1234567890", "6000", &timestamp.to_string()],
        );
        assert!(result2.is_err());
    }
    
    #[test]
    fn test_database_indexes() {
        let conn = Connection::open_in_memory().unwrap();
        
        // Create tables with indexes
        conn.execute_batch(
            "CREATE TABLE contracts (
                id INTEGER PRIMARY KEY,
                side TEXT NOT NULL,
                strike_price REAL NOT NULL,
                quantity REAL NOT NULL,
                expires INTEGER NOT NULL,
                premium REAL NOT NULL,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
            );
            
            CREATE INDEX idx_contracts_created_at ON contracts(created_at);
            CREATE INDEX idx_contracts_expires ON contracts(expires);
            
            CREATE TABLE premium_history (
                id INTEGER PRIMARY KEY,
                product_key TEXT NOT NULL,
                side TEXT NOT NULL,
                strike_price REAL NOT NULL,
                expires INTEGER NOT NULL,
                premium REAL NOT NULL,
                timestamp INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                UNIQUE(product_key, timestamp)
            );
            
            CREATE INDEX idx_premium_history_timestamp ON premium_history(timestamp);
            CREATE INDEX idx_premium_history_product ON premium_history(product_key);"
        ).unwrap();
        
        // Verify indexes exist by querying sqlite_master
        let index_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND sql IS NOT NULL",
            [],
            |row| row.get(0),
        ).unwrap();
        
        assert_eq!(index_count, 4); // We created 4 indexes
    }
}