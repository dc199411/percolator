//! Percolator Protocol Rust SDK
//!
//! This SDK provides a Rust client for interacting with the Percolator perpetual
//! futures protocol on Solana.
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use percolator_sdk::{PercolatorClient, Side, usdcToRaw};
//! use solana_sdk::pubkey::Pubkey;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = PercolatorClient::new("https://api.devnet.solana.com")?;
//!     
//!     // Check portfolio
//!     let portfolio = client.get_portfolio(&wallet_pubkey).await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod constants;
pub mod error;
pub mod instructions;
pub mod pda;
pub mod types;
pub mod client;

pub use constants::*;
pub use error::*;
pub use instructions::*;
pub use pda::*;
pub use types::*;
pub use client::*;
