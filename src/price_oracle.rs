use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, SystemTime};
use tonic::transport::Channel;

// Include the generated proto code
pub mod oracle {
    tonic::include_proto!("oracle");
}

use oracle::oracle_service_client::OracleServiceClient;
use oracle::{GetPriceRequest, GetPriceResponse, HealthRequest};

#[derive(Clone)]
pub struct PriceOracle {
    cached_price: Arc<RwLock<Option<(f64, SystemTime)>>>,
    grpc_client: OracleServiceClient<Channel>,
    cache_duration: Duration,
}

impl PriceOracle {
    pub async fn new(aggregator_url: String) -> Result<Self, Box<dyn std::error::Error>> {
        // Connect to the gRPC server
        let client = OracleServiceClient::connect(aggregator_url.clone()).await
            .map_err(|e| {
                format!(
                    "Failed to connect to Oracle Aggregator at {}. \n\n\
                    Please ensure the Oracle Aggregator service is running.\n\n\
                    To start the oracle system:\n\
                    1. Start aggregator: cd /home/zeno/projects/oracle-node/aggregator-server && nix-shell && cargo run\n\
                    2. Start oracle nodes: cd /home/zeno/projects/oracle-node && nix-shell && cargo run -- --node-id node1 --aggregator-url {}\n\n\
                    For detailed setup instructions, see: docs/ORACLE_SETUP.md\n\n\
                    Error: {}",
                    aggregator_url, aggregator_url, e
                )
            })?;
        
        // Perform health check
        let mut client_clone = client.clone();
        let health_response = client_clone
            .health_check(HealthRequest { 
                node_id: "btc-option-manager".to_string() 
            })
            .await
            .map_err(|e| {
                format!(
                    "Oracle Aggregator health check failed. \n\n\
                    The service may not be fully initialized.\n\
                    Error: {}",
                    e
                )
            })?;
        
        let health = health_response.into_inner();
        if !health.healthy {
            return Err(format!(
                "Oracle Aggregator is not healthy. Active nodes: {}",
                health.active_nodes
            ).into());
        }
        
        println!(
            "Connected to Oracle Aggregator v{} with {} active nodes",
            health.version, health.active_nodes
        );
        
        Ok(Self {
            cached_price: Arc::new(RwLock::new(None)),
            grpc_client: client,
            cache_duration: Duration::from_secs(10), // Cache for 10 seconds
        })
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
        let response = self.get_detailed_price().await?;
        Ok(response.aggregated_price)
    }
    
    /// Get detailed price information including individual exchange prices
    pub async fn get_detailed_price(&self) -> Result<GetPriceResponse, Box<dyn std::error::Error>> {
        let mut client = self.grpc_client.clone();
        
        let request = tonic::Request::new(GetPriceRequest {
            source_filter: None, // No specific source filter
        });
        
        let response = client.get_aggregated_price(request).await?;
        let price_data = response.into_inner();
        
        if !price_data.success {
            return Err("Failed to get aggregated price from oracle service".into());
        }
        
        if price_data.data_points == 0 {
            return Err("No oracle sources available for price data".into());
        }
        
        Ok(price_data)
    }
}