use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, SystemTime};

// For now we'll use HTTP as a fallback since we don't have the exact proto definitions
// This can be replaced with proper gRPC client when proto files are available
#[derive(Clone)]
pub struct PriceOracle {
    cached_price: Arc<RwLock<Option<(f64, SystemTime)>>>,
    oracle_url: String,
    cache_duration: Duration,
}

impl PriceOracle {
    pub fn new(oracle_url: String) -> Self {
        Self {
            cached_price: Arc::new(RwLock::new(None)),
            oracle_url,
            cache_duration: Duration::from_secs(10), // Cache for 10 seconds
        }
    }

    pub async fn get_btc_price(&self) -> Result<f64, Box<dyn std::error::Error>> {
        // Check cache first
        {
            let cache = self.cached_price.read().await;
            if let Some((price, timestamp)) = *cache {
                if timestamp.elapsed().unwrap_or(Duration::from_secs(u64::MAX)) < self.cache_duration {
                    return Ok(price);
                }
            }
        }

        // Fetch new price
        let price = self.fetch_price_from_oracle().await?;
        
        // Update cache
        {
            let mut cache = self.cached_price.write().await;
            *cache = Some((price, SystemTime::now()));
        }

        Ok(price)
    }

    async fn fetch_price_from_oracle(&self) -> Result<f64, Box<dyn std::error::Error>> {
        // TODO: Replace with actual gRPC call when proto definitions are available
        // For now, use HTTP fallback
        let client = reqwest::Client::new();
        let price: f64 = client
            .get(&self.oracle_url)
            .send()
            .await?
            .json()
            .await?;
        
        Ok(price)
    }

    // This method will be implemented once we have the gRPC proto definitions
    #[allow(dead_code)]
    async fn fetch_price_grpc(&self) -> Result<f64, Box<dyn std::error::Error>> {
        // Placeholder for gRPC implementation
        // Will connect to oracle node at AGGREGATOR_URL (default: localhost:50051)
        todo!("Implement gRPC client when proto definitions are available")
    }
}