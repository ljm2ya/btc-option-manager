use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::time::{interval, Duration};

#[derive(Deserialize)]
struct DeribitResponse {
    result: Vec<OptionSummary>,
}

#[derive(Deserialize)]
struct OptionSummary {
    instrument_name: String,
    mark_iv: f64,
}

#[derive(Clone)]
pub struct IvOracle {
    client: Client,
    cache: Arc<RwLock<HashMap<String, HashMap<f64, HashMap<String, f64>>>>>,
    api_url: String,
}

impl IvOracle {
    pub fn new(api_url: String) -> Self {
        Self {
            client: Client::new(),
            cache: Arc::new(RwLock::new(HashMap::new())),
            api_url,
        }
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
        let url = format!("{}/public/get_book_summary_by_currency?currency=BTC&kind=option", self.api_url);
        let response: DeribitResponse = self.client
            .get(&url)
            .send()
            .await?
            .json()
            .await?;

        let mut new_cache = HashMap::new();

        for option in response.result {
            if let Some((expiry, strike, side)) = parse_instrument_name(&option.instrument_name) {
                new_cache
                    .entry(expiry)
                    .or_insert_with(HashMap::new)
                    .entry(strike)
                    .or_insert_with(HashMap::new)
                    .insert(side, option.mark_iv);
            }
        }

        let mut cache = self.cache.write().unwrap();
        *cache = new_cache;

        Ok(())
    }

    pub fn get_iv(&self, side: &str, strike_price: f64, expire: &str) -> Option<f64> {
        let cache = self.cache.read().unwrap();
        
        // Try to find matching expiry in cache
        for (cached_expiry, strikes) in cache.iter() {
            if let Some(sides) = strikes.get(&strike_price) {
                if let Some(iv) = sides.get(side) {
                    return Some(*iv);
                }
            }
        }
        
        None
    }
    
    pub fn get_iv_by_exact_expiry(&self, side: &str, strike_price: f64, expire: &str) -> Option<f64> {
        let cache = self.cache.read().unwrap();
        cache.get(expire)
            .and_then(|strikes| strikes.get(&strike_price))
            .and_then(|sides| sides.get(side))
            .copied()
    }
}

fn parse_instrument_name(name: &str) -> Option<(String, f64, String)> {
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