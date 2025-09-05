use btc_options_api::iv_oracle::IvOracle;
use tokio;

#[tokio::main]
async fn main() {
    println!("Testing IV Oracle with Deribit API");
    println!("==================================\n");

    // Create IvOracle with real Deribit API URL
    let oracle = IvOracle::new("https://www.deribit.com/api/v2".to_string());
    
    // Check initial state
    println!("Initial state:");
    println!("- Cache empty: {}", oracle.is_cache_empty());
    println!("- Cache size: {}\n", oracle.get_cache_size());

    // Fetch real data from Deribit
    println!("Fetching data from Deribit API...");
    match oracle.fetch_and_update_iv().await {
        Ok(_) => {
            println!("✅ Successfully fetched data!\n");
            
            // Show cache state after fetch
            println!("After fetch:");
            println!("- Cache empty: {}", oracle.is_cache_empty());
            println!("- Cache size: {} entries", oracle.get_cache_size());
            
            // Show available expiries
            let expiries = oracle.get_cached_expiries();
            println!("- Available expiries: {} different dates", expiries.len());
            if expiries.len() > 5 {
                println!("  First 5: {:?}", &expiries[..5]);
            } else {
                println!("  All: {:?}", expiries);
            }
            println!();
            
            // Test with common strike prices
            println!("Testing get_iv() with various strike prices:");
            println!("============================================");
            
            let test_strikes = vec![
                20000.0, 30000.0, 40000.0, 45000.0, 50000.0, 
                55000.0, 60000.0, 70000.0, 80000.0, 90000.0, 100000.0
            ];
            
            for side in &["C", "P"] {
                println!("\n{} Options:", if *side == "C" { "Call" } else { "Put" });
                println!("Strike Price | IV Value | Found");
                println!("-------------|----------|-------");
                
                for strike in &test_strikes {
                    let iv = oracle.get_iv(side, *strike, "1d");
                    match iv {
                        Some(value) => {
                            println!("{:>12.0} | {:>7.4}  | ✓", strike, value);
                        }
                        None => {
                            println!("{:>12.0} | -        | ✗ (not in cache)", strike);
                        }
                    }
                }
            }
            
            println!("\nNote: get_iv() ignores the expire parameter and searches all cached expiries");
            println!("To test specific expiry, use get_iv_by_exact_expiry()");
            
            // Show some actual data structure insights
            println!("\nDebugging info:");
            println!("- The cache uses exact strike prices from Deribit");
            println!("- Common BTC option strikes are usually at round numbers");
            println!("- Strikes like 45000 or 50000 might not exist if Deribit uses different intervals");
            
        }
        Err(e) => {
            println!("❌ Failed to fetch data from Deribit: {}", e);
            println!("\nPossible reasons:");
            println!("- Network connectivity issues");
            println!("- Deribit API is down or rate limiting");
            println!("- Invalid API endpoint");
        }
    }
    
    // Test the parsing function
    println!("\n\nTesting parse_instrument_name function:");
    println!("=====================================");
    test_parse_instrument_name();
}

fn test_parse_instrument_name() {
    use btc_options_api::iv_oracle::parse_instrument_name;
    
    let test_names = vec![
        "BTC-29DEC23-50000-C",
        "BTC-29DEC23-50000-P",
        "BTC-1JAN24-45000-C",
        "BTC-PERPETUAL",
        "ETH-29DEC23-2000-C",
        "INVALID-NAME",
    ];
    
    for name in test_names {
        match parse_instrument_name(name) {
            Some((expiry, strike, side)) => {
                println!("✓ {} -> expiry: {}, strike: {}, side: {}", 
                    name, expiry, strike, side);
            }
            None => {
                println!("✗ {} -> Could not parse", name);
            }
        }
    }
}