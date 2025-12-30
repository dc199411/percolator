//! CLI Configuration

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// CLI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// RPC endpoint URL
    pub rpc_url: String,
    /// Path to keypair file
    pub keypair_path: Option<String>,
    /// Default output format
    pub output_format: String,
    /// USDC mint address
    pub usdc_mint: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rpc_url: "https://api.devnet.solana.com".to_string(),
            keypair_path: None,
            output_format: "text".to_string(),
            usdc_mint: None,
        }
    }
}

impl Config {
    /// Get config directory path
    fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("percolator")
    }

    /// Get config file path
    fn config_path() -> PathBuf {
        Self::config_dir().join("config.json")
    }

    /// Load configuration from file
    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&contents)?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let dir = Self::config_dir();
        fs::create_dir_all(&dir)?;
        let path = Self::config_path();
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }
}

/// Get home directory for config
mod dirs {
    use std::path::PathBuf;

    pub fn config_dir() -> Option<PathBuf> {
        std::env::var("HOME")
            .ok()
            .map(|home| PathBuf::from(home).join(".config"))
    }
}
