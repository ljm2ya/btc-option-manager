use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, SystemTime};
use tonic::transport::Channel;
use tonic::{Request, Status};

// Import the btc-option-manager's expected proto types
use crate::price_oracle::oracle::{
    GetPriceRequest, GetPriceResponse,
    HealthCheckRequest, HealthCheckResponse,
};

// We need to use a different name for oracle-node's proto to avoid conflicts
#[path = "../oracle.rs"]
pub mod oracle_node;

use oracle_aggregator::{
    oracle_service_client::OracleServiceClient,
    GetPriceRequest as AggregatorPriceRequest,
    HealthRequest as AggregatorHealthRequest,
};

/// Adapter that wraps oracle-node2's OracleService to provide the interface
/// that btc-option-manager expects
#[derive(Clone)]
pub struct OracleServiceAdapter {
    client: OracleServiceClient<Channel>,
}

impl OracleServiceAdapter {
    pub async fn new(aggregator_url: String) -> Result<Self, Box<dyn std::error::Error>> {
        let client = OracleServiceClient::connect(aggregator_url.clone()).await
            .map_err(|e| {
                format!(
                    "Failed to connect to Oracle Service at {}. \n\n\
                    Please ensure the Oracle Aggregator service is running.\n\n\
                    To start the oracle system:\n\
                    1. Start aggregator: cd /home/zeno/projects/oracle-node2/aggregator-server && nix-shell && cargo run\n\
                    2. Start oracle nodes: cd /home/zeno/projects/oracle-node2 && nix-shell && cargo run -- --node-id node1 --aggregator-url {}\n\n\
                    For detailed setup instructions, see: docs/ORACLE_SETUP.md\n\n\
                    Error: {}",
                    aggregator_url, aggregator_url, e
                )
            })?;
        
        Ok(Self { client })
    }
    
    /// Get price using oracle-node2's GetAggregatedPrice RPC
    pub async fn get_price(&mut self, asset: String) -> Result<GetPriceResponse, Box<dyn std::error::Error>> {
        // oracle-node2 doesn't use asset filter in the same way, but we can still call it
        let request = AggregatorPriceRequest {
            source_filter: None, // No specific source filter
        };
        
        let response = self.client.get_aggregated_price(request).await?;
        let inner = response.into_inner();
        
        if !inner.success {
            return Err("Failed to get aggregated price from oracle service".into());
        }
        
        // Convert oracle-node2 response to btc-option-manager format
        Ok(GetPriceResponse {
            asset,
            price: inner.aggregated_price,
            timestamp: inner.last_update as i64,
            num_sources: inner.data_points as i32,
        })
    }
    
    /// Health check using oracle-node2's HealthCheck RPC
    pub async fn health_check(&mut self) -> Result<HealthCheckResponse, Box<dyn std::error::Error>> {
        let request = AggregatorHealthRequest {
            node_id: "btc-option-manager".to_string(),
        };
        
        let response = self.client.health_check(request).await?;
        let inner = response.into_inner();
        
        // Convert oracle-node2 response to btc-option-manager format
        Ok(HealthCheckResponse {
            healthy: inner.healthy,
            version: inner.version,
            active_nodes: inner.active_nodes as i32,
        })
    }
}