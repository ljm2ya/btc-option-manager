use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum MutinyWalletError {
    NetworkError(String),
    ParseError(String),
    ApiError(String),
}

impl fmt::Display for MutinyWalletError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MutinyWalletError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            MutinyWalletError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            MutinyWalletError::ApiError(msg) => write!(f, "API error: {}", msg),
        }
    }
}

impl Error for MutinyWalletError {}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AddressInfo {
    pub address: String,
    pub chain_stats: ChainStats,
    pub mempool_stats: MempoolStats,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChainStats {
    pub funded_txo_count: u64,
    pub funded_txo_sum: u64,
    pub spent_txo_count: u64,
    pub spent_txo_sum: u64,
    pub tx_count: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MempoolStats {
    pub funded_txo_count: u64,
    pub funded_txo_sum: u64,
    pub spent_txo_count: u64,
    pub spent_txo_sum: u64,
    pub tx_count: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Utxo {
    pub txid: String,
    pub vout: u32,
    pub status: UtxoStatus,
    pub value: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UtxoStatus {
    pub confirmed: bool,
    pub block_height: Option<u64>,
    pub block_hash: Option<String>,
    pub block_time: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    pub txid: String,
    pub version: u32,
    pub locktime: u32,
    pub vin: Vec<Vin>,
    pub vout: Vec<Vout>,
    pub size: u32,
    pub weight: u32,
    pub fee: u64,
    pub status: TxStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Vin {
    pub txid: String,
    pub vout: u32,
    pub prevout: Option<Prevout>,
    pub scriptsig: String,
    pub scriptsig_asm: String,
    pub witness: Option<Vec<String>>,
    pub is_coinbase: bool,
    pub sequence: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Prevout {
    pub scriptpubkey: String,
    pub scriptpubkey_asm: String,
    pub scriptpubkey_type: String,
    pub scriptpubkey_address: Option<String>,
    pub value: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Vout {
    pub scriptpubkey: String,
    pub scriptpubkey_asm: String,
    pub scriptpubkey_type: String,
    pub scriptpubkey_address: Option<String>,
    pub value: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TxStatus {
    pub confirmed: bool,
    pub block_height: Option<u64>,
    pub block_hash: Option<String>,
    pub block_time: Option<u64>,
}

#[derive(Debug, Serialize, Clone)]
pub struct WalletBalance {
    pub address: String,
    pub confirmed_balance: u64,
    pub unconfirmed_balance: u64,
    pub total_balance: u64,
    pub confirmed_utxo_count: u64,
    pub unconfirmed_utxo_count: u64,
    pub total_utxo_count: u64,
}

pub struct MutinyWallet {
    client: Client,
    base_url: String,
    #[allow(dead_code)]
    network: Network,
}

#[derive(Debug, Clone, Copy)]
pub enum Network {
    Mainnet,
    Testnet,
    Signet,
}

impl std::fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Network::Mainnet => write!(f, "mainnet"),
            Network::Testnet => write!(f, "testnet"),
            Network::Signet => write!(f, "signet"),
        }
    }
}

impl MutinyWallet {
    pub fn new(network: Network) -> Self {
        let base_url = match network {
            Network::Mainnet => "https://mutiny.mempool.space/api".to_string(),
            Network::Testnet | Network::Signet => "https://mutinynet.com/api".to_string(),
        };

        Self {
            client: Client::new(),
            base_url,
            network,
        }
    }

    pub fn with_custom_url(url: String, network: Network) -> Self {
        Self {
            client: Client::new(),
            base_url: url,
            network,
        }
    }

    pub async fn get_address_info(&self, address: &str) -> Result<AddressInfo, MutinyWalletError> {
        let url = format!("{}/address/{}", self.base_url, address);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| MutinyWalletError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(MutinyWalletError::ApiError(
                format!("API returned status: {}", response.status())
            ));
        }

        let address_info = response
            .json::<AddressInfo>()
            .await
            .map_err(|e| MutinyWalletError::ParseError(e.to_string()))?;

        Ok(address_info)
    }

    pub async fn get_address_utxos(&self, address: &str) -> Result<Vec<Utxo>, MutinyWalletError> {
        let url = format!("{}/address/{}/utxo", self.base_url, address);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| MutinyWalletError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(MutinyWalletError::ApiError(
                format!("API returned status: {}", response.status())
            ));
        }

        let utxos = response
            .json::<Vec<Utxo>>()
            .await
            .map_err(|e| MutinyWalletError::ParseError(e.to_string()))?;

        Ok(utxos)
    }

    pub async fn get_address_transactions(&self, address: &str) -> Result<Vec<Transaction>, MutinyWalletError> {
        let url = format!("{}/address/{}/txs", self.base_url, address);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| MutinyWalletError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(MutinyWalletError::ApiError(
                format!("API returned status: {}", response.status())
            ));
        }

        let transactions = response
            .json::<Vec<Transaction>>()
            .await
            .map_err(|e| MutinyWalletError::ParseError(e.to_string()))?;

        Ok(transactions)
    }

    pub async fn get_wallet_balance(&self, address: &str) -> Result<WalletBalance, MutinyWalletError> {
        let address_info = self.get_address_info(address).await?;
        let utxos = self.get_address_utxos(address).await?;

        let confirmed_balance = address_info.chain_stats.funded_txo_sum - address_info.chain_stats.spent_txo_sum;
        let unconfirmed_balance = address_info.mempool_stats.funded_txo_sum - address_info.mempool_stats.spent_txo_sum;
        let total_balance = confirmed_balance + unconfirmed_balance;

        let confirmed_utxo_count = utxos.iter().filter(|u| u.status.confirmed).count() as u64;
        let unconfirmed_utxo_count = utxos.iter().filter(|u| !u.status.confirmed).count() as u64;
        let total_utxo_count = utxos.len() as u64;

        Ok(WalletBalance {
            address: address.to_string(),
            confirmed_balance,
            unconfirmed_balance,
            total_balance,
            confirmed_utxo_count,
            unconfirmed_utxo_count,
            total_utxo_count,
        })
    }

    pub async fn get_transaction(&self, txid: &str) -> Result<Transaction, MutinyWalletError> {
        let url = format!("{}/tx/{}", self.base_url, txid);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| MutinyWalletError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(MutinyWalletError::ApiError(
                format!("API returned status: {}", response.status())
            ));
        }

        let transaction = response
            .json::<Transaction>()
            .await
            .map_err(|e| MutinyWalletError::ParseError(e.to_string()))?;

        Ok(transaction)
    }

    pub fn satoshis_to_btc(satoshis: u64) -> f64 {
        satoshis as f64 / 100_000_000.0
    }

    pub fn btc_to_satoshis(btc: f64) -> u64 {
        (btc * 100_000_000.0) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mutiny_testnet_balance_query() {
        let wallet = MutinyWallet::new(Network::Signet);
        
        // Test with a known testnet address
        let test_address = "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx";
        
        println!("Testing Mutiny testnet balance query for address: {}", test_address);
        
        match wallet.get_wallet_balance(test_address).await {
            Ok(balance) => {
                println!("\n=== Wallet Balance Information ===");
                println!("Address: {}", balance.address);
                println!("Confirmed Balance: {} sats ({} BTC)", 
                    balance.confirmed_balance, 
                    MutinyWallet::satoshis_to_btc(balance.confirmed_balance));
                println!("Unconfirmed Balance: {} sats ({} BTC)", 
                    balance.unconfirmed_balance,
                    MutinyWallet::satoshis_to_btc(balance.unconfirmed_balance));
                println!("Total Balance: {} sats ({} BTC)", 
                    balance.total_balance,
                    MutinyWallet::satoshis_to_btc(balance.total_balance));
                println!("\n=== UTXO Information ===");
                println!("Confirmed UTXOs: {}", balance.confirmed_utxo_count);
                println!("Unconfirmed UTXOs: {}", balance.unconfirmed_utxo_count);
                println!("Total UTXOs: {}", balance.total_utxo_count);
            }
            Err(e) => {
                println!("Error querying balance: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_get_utxos() {
        let wallet = MutinyWallet::new(Network::Signet);
        let test_address = "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx";
        
        match wallet.get_address_utxos(test_address).await {
            Ok(utxos) => {
                println!("\n=== UTXOs for {} ===", test_address);
                for (i, utxo) in utxos.iter().enumerate() {
                    println!("\nUTXO #{}:", i + 1);
                    println!("  TXID: {}", utxo.txid);
                    println!("  Vout: {}", utxo.vout);
                    println!("  Value: {} sats", utxo.value);
                    println!("  Confirmed: {}", utxo.status.confirmed);
                    if let Some(height) = utxo.status.block_height {
                        println!("  Block Height: {}", height);
                    }
                }
            }
            Err(e) => {
                println!("Error getting UTXOs: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_real_mutinynet_address() {
        let wallet = MutinyWallet::new(Network::Signet);
        
        // Try with a real Mutinynet faucet address or known address
        let test_addresses = vec![
            "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx",
            "tb1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gdcccefvpysxf3q0sl5k7",
        ];
        
        for address in test_addresses {
            println!("\n=== Testing address: {} ===", address);
            
            match wallet.get_address_info(address).await {
                Ok(info) => {
                    let confirmed_balance = info.chain_stats.funded_txo_sum - info.chain_stats.spent_txo_sum;
                    println!("Success! Balance: {} sats", confirmed_balance);
                }
                Err(e) => {
                    println!("Failed: {}", e);
                }
            }
        }
    }
}