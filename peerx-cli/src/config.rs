use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::error::{CliError, Result};

/// Configuration for PeerX CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Network configuration
    pub network: NetworkConfig,
    
    /// Contract configuration
    pub contract: ContractConfig,
    
    /// Health check configuration
    pub health: HealthConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Network name (testnet, mainnet, local)
    pub name: String,
    
    /// RPC endpoint URL
    pub rpc_url: String,
    
    /// Horizon API URL
    pub horizon_url: String,
    
    /// Network passphrase
    pub passphrase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractConfig {
    /// Contract ID
    pub contract_id: String,
    
    /// Admin address
    pub admin_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    /// Timeout for health checks in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    
    /// Oracle freshness threshold in seconds
    #[serde(default = "default_oracle_freshness")]
    pub oracle_freshness_threshold_seconds: u64,
    
    /// Number of retries for network operations
    #[serde(default = "default_retries")]
    pub max_retries: u32,
}

fn default_timeout() -> u64 {
    30
}

fn default_oracle_freshness() -> u64 {
    300 // 5 minutes
}

fn default_retries() -> u32 {
    3
}

impl Default for Config {
    fn default() -> Self {
        Self {
            network: NetworkConfig {
                name: "testnet".to_string(),
                rpc_url: "https://soroban-testnet.stellar.org".to_string(),
                horizon_url: "https://horizon-testnet.stellar.org".to_string(),
                passphrase: "Test SDF Network ; September 2015".to_string(),
            },
            contract: ContractConfig {
                contract_id: String::new(),
                admin_address: None,
            },
            health: HealthConfig {
                timeout_seconds: default_timeout(),
                oracle_freshness_threshold_seconds: default_oracle_freshness(),
                max_retries: default_retries(),
            },
        }
    }
}

impl Config {
    /// Load configuration from environment variables and config file
    pub fn load() -> Result<Self> {
        let mut config = Self::default();
        
        // Override with environment variables
        if let Ok(network) = std::env::var("PEERX_NETWORK") {
            config.network.name = network;
        }
        
        if let Ok(rpc_url) = std::env::var("PEERX_RPC_URL") {
            config.network.rpc_url = rpc_url;
        }
        
        if let Ok(contract_id) = std::env::var("PEERX_CONTRACT_ID") {
            config.contract.contract_id = contract_id;
        }
        
        if let Ok(admin) = std::env::var("PEERX_ADMIN_ADDRESS") {
            config.contract.admin_address = Some(admin);
        }
        
        // Try to load from config file
        if let Some(config_path) = Self::find_config_file() {
            if let Ok(contents) = std::fs::read_to_string(&config_path) {
                if let Ok(file_config) = serde_json::from_str::<Config>(&contents) {
                    config = file_config;
                }
            }
        }
        
        Ok(config)
    }
    
    /// Find config file in standard locations
    fn find_config_file() -> Option<PathBuf> {
        let candidates = vec![
            PathBuf::from("peerx-cli.json"),
            PathBuf::from(".peerx-cli.json"),
            dirs::home_dir()?.join(".config/peerx/config.json"),
        ];
        
        candidates.into_iter().find(|p| p.exists())
    }
    
    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.contract.contract_id.is_empty() {
            return Err(CliError::Config(
                "Contract ID is required. Set PEERX_CONTRACT_ID environment variable or configure in config file.".to_string()
            ));
        }
        
        if self.network.rpc_url.is_empty() {
            return Err(CliError::Config("RPC URL is required".to_string()));
        }
        
        Ok(())
    }
}
