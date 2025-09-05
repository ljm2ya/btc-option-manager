use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::time::{interval, Duration};
use std::hash::{Hash, Hasher};
use chrono::{DateTime, NaiveDate, Utc};

// Wrapper for f64 to use as HashMap key
#[derive(Clone, Copy, Debug)]
struct StrikePrice(f64);

impl Hash for StrikePrice {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialEq for StrikePrice {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for StrikePrice {}

#[derive(Deserialize)]
struct DeribitResponse {
    result: Vec<OptionSummary>,
}

#[derive(Deserialize)]
struct OptionSummary {
    instrument_name: String,
    mark_iv: f64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct InstrumentData {
    instrument_name: String,
    is_active: bool,
    expiration_timestamp: i64,
    strike: f64,
    option_type: String,
}

#[derive(Debug, Deserialize)]
struct InstrumentsResponse {
    result: Vec<InstrumentData>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ExpiryInfo {
    date_str: String,      // e.g., "19SEP25"
    timestamp: i64,        // milliseconds
}

#[derive(Clone)]
pub struct IvOracle {
    client: Client,
    cache: Arc<RwLock<HashMap<String, HashMap<StrikePrice, HashMap<String, f64>>>>>,
    expiry_map: Arc<RwLock<HashMap<String, i64>>>,  // Maps date strings to timestamps
    api_url: String,
}

impl IvOracle {
    pub fn new(api_url: String) -> Self {
        Self {
            client: Client::new(),
            cache: Arc::new(RwLock::new(HashMap::new())),
            expiry_map: Arc::new(RwLock::new(HashMap::new())),
            api_url,
        }
    }

    pub async fn initialize(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("📊 Initializing IV Oracle - fetching initial data...");
        self.fetch_and_update_iv().await?;
        println!("✅ IV Oracle initialized with data");
        Ok(())
    }

    pub async fn start_updates(&self) {
        let oracle = self.clone();
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(15));
            loop {
                ticker.tick().await;
                if let Err(e) = oracle.fetch_and_update_iv().await {
                    eprintln!("Error updating IV data: {}", e);
                }
            }
        });
    }

    pub async fn fetch_and_update_iv(&self) -> Result<(), Box<dyn std::error::Error>> {
        // First, get all available BTC option instruments to see what's available
        let instruments_url = format!("{}/public/get_instruments?currency=BTC&kind=option&expired=false", self.api_url);
        match self.client.get(&instruments_url).send().await {
            Ok(resp) => {
                if let Ok(instruments_response) = resp.json::<InstrumentsResponse>().await {
                    // Log the number of instruments found
                    //println!("Found {} active BTC option instruments", instruments_response.result.len());
                    
                    // Collect unique expiries for logging
                    let mut unique_expiries = std::collections::HashSet::new();
                    for instrument in &instruments_response.result {
                        if let Some((expiry, _, _)) = parse_instrument_name(&instrument.instrument_name) {
                            unique_expiries.insert(expiry);
                        }
                    }
                    let mut expiries_vec: Vec<String> = unique_expiries.into_iter().collect();
                    expiries_vec.sort();
                    //println!("Unique expiries found ({}): {:?}", expiries_vec.len(), expiries_vec);
                }
            }
            Err(e) => {
                eprintln!("Failed to fetch instruments: {}", e);
            }
        }
        
        // Now fetch the book summary for all options (this includes IV data)
        let url = format!("{}/public/get_book_summary_by_currency?currency=BTC&kind=option", self.api_url);
        let response: DeribitResponse = self.client
            .get(&url)
            .send()
            .await?
            .json()
            .await?;

        let mut new_cache = HashMap::new();
        let mut new_expiry_map = HashMap::new();

        for option in response.result {
            if let Some((expiry, strike, side)) = parse_instrument_name(&option.instrument_name) {
                // Convert IV from percentage to decimal (e.g., 35.16 -> 0.3516)
                let iv_decimal = option.mark_iv / 100.0;
                
                // Store IV in cache
                new_cache
                    .entry(expiry.clone())
                    .or_insert_with(HashMap::new)
                    .entry(StrikePrice(strike))
                    .or_insert_with(HashMap::new)
                    .insert(side, iv_decimal);
                
                // Parse and store expiry timestamp if not already present
                if !new_expiry_map.contains_key(&expiry) {
                    if let Some(timestamp) = Self::parse_expiry_to_timestamp(&expiry) {
                        new_expiry_map.insert(expiry, timestamp);
                    }
                }
            }
        }

        // Update both caches atomically
        let mut cache = self.cache.write().unwrap();
        *cache = new_cache;
        
        let mut expiry_map = self.expiry_map.write().unwrap();
        *expiry_map = new_expiry_map;

        Ok(())
    }

    /// Get implied volatility for a given option.
    /// 
    /// The expire parameter should be a timestamp in milliseconds.
    /// This method will find the nearest matching expiry in the cache.
    pub fn get_iv(&self, side: &str, strike_price: f64, expire: &str) -> Option<f64> {
        // Try to parse expire as millisecond timestamp
        if let Ok(timestamp_ms) = expire.parse::<i64>() {
            return self.get_iv_by_timestamp(side, strike_price, timestamp_ms);
        }
        
        // Fallback: search all cached expiries (backward compatibility)
        let cache = self.cache.read().unwrap();
        
        for (_cached_expiry, strikes) in cache.iter() {
            if let Some(sides) = strikes.get(&StrikePrice(strike_price)) {
                if let Some(iv) = sides.get(side) {
                    return Some(*iv);
                }
            }
        }
        
        None
    }
    
    pub fn get_cache_size(&self) -> usize {
        let cache = self.cache.read().unwrap();
        cache.values()
            .map(|strikes| strikes.values()
                .map(|sides| sides.len())
                .sum::<usize>())
            .sum()
    }

    pub fn get_cached_expiries(&self) -> Vec<String> {
        let cache = self.cache.read().unwrap();
        cache.keys().cloned().collect()
    }
    
    pub fn get_expiry_timestamps(&self) -> Vec<(String, i64)> {
        let expiry_map = self.expiry_map.read().unwrap();
        expiry_map.iter().map(|(k, v)| (k.clone(), *v)).collect()
    }
    
    pub fn get_expiry_timestamp(&self, expiry_str: &str) -> Option<i64> {
        let expiry_map = self.expiry_map.read().unwrap();
        expiry_map.get(expiry_str).copied()
    }

    pub fn is_cache_empty(&self) -> bool {
        let cache = self.cache.read().unwrap();
        cache.is_empty()
    }
    
    /// Get all cached expiries sorted by date
    pub fn get_sorted_expiries(&self) -> Vec<(String, i64)> {
        let expiry_map = self.expiry_map.read().unwrap();
        let mut expiries: Vec<(String, i64)> = expiry_map.iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        expiries.sort_by_key(|(_, timestamp)| *timestamp);
        expiries
    }

    pub fn get_iv_by_exact_expiry(&self, side: &str, strike_price: f64, expire: &str) -> Option<f64> {
        let cache = self.cache.read().unwrap();
        cache.get(expire)
            .and_then(|strikes| strikes.get(&StrikePrice(strike_price)))
            .and_then(|sides| sides.get(side))
            .map(|iv| *iv)
    }
    /// Parse Deribit date format (e.g., "19SEP25" or "6SEP25") to timestamp
    fn parse_expiry_to_timestamp(expiry: &str) -> Option<i64> {
        // Format: DDMMMYY or DMMMYY (e.g., "19SEP25" or "6SEP25")
        if expiry.len() < 6 || expiry.len() > 7 {
            return None;
        }
        
        // Determine if we have 1 or 2 digit day
        let (day_str, month_start) = if expiry.len() == 6 {
            (&expiry[0..1], 1)  // Single digit day
        } else {
            (&expiry[0..2], 2)  // Two digit day
        };
        
        let day = day_str.parse::<u32>().ok()?;
        let month_str = &expiry[month_start..month_start+3];
        let year = expiry[month_start+3..month_start+5].parse::<i32>().ok()?;
        
        let month = match month_str {
            "JAN" => 1, "FEB" => 2, "MAR" => 3, "APR" => 4,
            "MAY" => 5, "JUN" => 6, "JUL" => 7, "AUG" => 8,
            "SEP" => 9, "OCT" => 10, "NOV" => 11, "DEC" => 12,
            _ => return None,
        };
        
        // Convert YY to full year (25 -> 2025)
        let full_year = if year < 50 { 2000 + year } else { 1900 + year };
        
        // Create date at 08:00 UTC (Deribit standard expiry time)
        let date = NaiveDate::from_ymd_opt(full_year, month, day)?;
        let datetime = date.and_hms_opt(8, 0, 0)?;
        let utc_datetime = DateTime::<Utc>::from_naive_utc_and_offset(datetime, Utc);
        
        Some(utc_datetime.timestamp_millis())
    }
    
    /// Find the expiry closest to the given timestamp
    fn find_nearest_expiry(&self, target_timestamp: i64) -> Option<String> {
        let expiry_map = self.expiry_map.read().unwrap();
        
        if expiry_map.is_empty() {
            return None;
        }
        
        let mut closest_expiry = None;
        let mut min_diff = i64::MAX;
        
        for (expiry_str, &timestamp) in expiry_map.iter() {
            let diff = (timestamp - target_timestamp).abs();
            if diff < min_diff {
                min_diff = diff;
                closest_expiry = Some(expiry_str.clone());
            }
        }
        
        closest_expiry
    }
    
    /// Get IV for a specific option with timestamp-based expiry matching
    pub fn get_iv_by_timestamp(&self, side: &str, strike_price: f64, expire_timestamp_ms: i64) -> Option<f64> {
        // Find the nearest expiry
        let nearest_expiry = self.find_nearest_expiry(expire_timestamp_ms)?;
        
        // Get IV for that specific expiry
        self.get_iv_by_exact_expiry(side, strike_price, &nearest_expiry)
    }
}

pub fn parse_instrument_name(name: &str) -> Option<(String, f64, String)> {
    let parts: Vec<&str> = name.split('-').collect();
    if parts.len() >= 4 && parts[0] == "BTC" {
        let expiry = parts[1].to_string();
        if let Ok(strike) = parts[2].parse::<f64>() {
            let side = parts[3].to_string();
            return Some((expiry, strike, side));
        }
    }
    None
}
