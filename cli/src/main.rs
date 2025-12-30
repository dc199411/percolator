//! Percolator Protocol CLI
//!
//! Command-line interface for LP operations on Percolator Protocol.

mod commands;
mod config;
mod output;

use anyhow::Result;
use clap::{Parser, Subcommand};
use console::style;

use commands::*;

/// Percolator Protocol CLI
#[derive(Parser)]
#[command(name = "percolator")]
#[command(author = "Percolator Protocol")]
#[command(version = "0.1.0")]
#[command(about = "CLI tools for Percolator Protocol LP operations", long_about = None)]
struct Cli {
    /// RPC endpoint URL
    #[arg(short, long, env = "SOLANA_RPC_URL", default_value = "https://api.devnet.solana.com")]
    rpc_url: String,

    /// Path to keypair file
    #[arg(short, long, env = "KEYPAIR_PATH")]
    keypair: Option<String>,

    /// Output format (text, json)
    #[arg(short, long, default_value = "text")]
    output: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Portfolio management commands
    Portfolio {
        #[command(subcommand)]
        command: PortfolioCommands,
    },

    /// Slab management commands (LP only)
    Slab {
        #[command(subcommand)]
        command: SlabCommands,
    },

    /// Insurance pool commands (LP only)
    Insurance {
        #[command(subcommand)]
        command: InsuranceCommands,
    },

    /// Trading commands
    Trade {
        #[command(subcommand)]
        command: TradeCommands,
    },

    /// Information and status commands
    Info {
        #[command(subcommand)]
        command: InfoCommands,
    },

    /// Configuration commands
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
}

#[derive(Subcommand)]
pub enum PortfolioCommands {
    /// Initialize a new portfolio
    Init,
    /// Show portfolio status
    Status,
    /// Deposit USDC collateral
    Deposit {
        /// Amount in USDC
        amount: f64,
    },
    /// Withdraw USDC collateral
    Withdraw {
        /// Amount in USDC
        amount: f64,
    },
    /// Show all positions
    Positions,
    /// Show margin details
    Margin,
}

#[derive(Subcommand)]
pub enum SlabCommands {
    /// Initialize a new slab
    Init {
        /// Initial margin requirement (bps)
        #[arg(long, default_value = "500")]
        imr_bps: u64,
        /// Maintenance margin requirement (bps)
        #[arg(long, default_value = "250")]
        mmr_bps: u64,
        /// Maker fee (bps, negative for rebate)
        #[arg(long, default_value = "-5")]
        maker_fee_bps: i64,
        /// Taker fee (bps)
        #[arg(long, default_value = "20")]
        taker_fee_bps: u64,
        /// Batch window (ms)
        #[arg(long, default_value = "100")]
        batch_ms: u64,
    },
    /// Show slab status
    Status {
        /// Slab address
        address: String,
    },
    /// List all slabs
    List,
    /// Add instrument to slab
    AddInstrument {
        /// Slab address
        slab: String,
        /// Instrument symbol
        symbol: String,
        /// Tick size (price increment)
        #[arg(long)]
        tick_size: f64,
        /// Lot size (quantity increment)
        #[arg(long)]
        lot_size: f64,
    },
    /// Update slab parameters
    Update {
        /// Slab address
        slab: String,
        /// New IMR (bps)
        #[arg(long)]
        imr_bps: Option<u64>,
        /// New MMR (bps)
        #[arg(long)]
        mmr_bps: Option<u64>,
    },
}

#[derive(Subcommand)]
pub enum InsuranceCommands {
    /// Initialize insurance pool
    Init {
        /// Slab address
        slab: String,
        /// Contribution rate (bps)
        #[arg(long, default_value = "25")]
        contribution_rate: u64,
        /// ADL threshold (bps)
        #[arg(long, default_value = "50")]
        adl_threshold: u64,
        /// Withdrawal timelock (days)
        #[arg(long, default_value = "7")]
        timelock_days: u64,
    },
    /// Show insurance pool status
    Status {
        /// Slab address
        slab: String,
    },
    /// Contribute to insurance pool
    Contribute {
        /// Slab address
        slab: String,
        /// Amount in USDC
        amount: f64,
    },
    /// Initiate withdrawal (starts timelock)
    InitiateWithdraw {
        /// Slab address
        slab: String,
        /// Amount in USDC
        amount: f64,
    },
    /// Complete withdrawal (after timelock)
    CompleteWithdraw {
        /// Slab address
        slab: String,
    },
    /// Cancel pending withdrawal
    CancelWithdraw {
        /// Slab address
        slab: String,
    },
    /// Show insurance event history
    History {
        /// Slab address
        slab: String,
        /// Number of events to show
        #[arg(short, long, default_value = "10")]
        count: usize,
    },
}

#[derive(Subcommand)]
pub enum TradeCommands {
    /// Place a market order
    Market {
        /// Instrument (e.g., BTC-PERP)
        instrument: String,
        /// Side (buy/sell)
        side: String,
        /// Quantity
        qty: f64,
    },
    /// Place a limit order
    Limit {
        /// Instrument (e.g., BTC-PERP)
        instrument: String,
        /// Side (buy/sell)
        side: String,
        /// Quantity
        qty: f64,
        /// Price
        price: f64,
        /// Post-only flag
        #[arg(long)]
        post_only: bool,
    },
    /// Cancel an order
    Cancel {
        /// Order ID
        order_id: u64,
    },
    /// Cancel all orders
    CancelAll {
        /// Instrument (optional)
        instrument: Option<String>,
    },
    /// Show open orders
    Orders,
}

#[derive(Subcommand)]
pub enum InfoCommands {
    /// Show protocol statistics
    Stats,
    /// Show orderbook for instrument
    Orderbook {
        /// Instrument (e.g., BTC-PERP)
        instrument: String,
        /// Depth
        #[arg(short, long, default_value = "10")]
        depth: usize,
    },
    /// Show recent trades
    Trades {
        /// Instrument (e.g., BTC-PERP)
        instrument: String,
        /// Number of trades
        #[arg(short, long, default_value = "20")]
        count: usize,
    },
    /// Show funding rate
    Funding {
        /// Instrument (e.g., BTC-PERP)
        instrument: String,
    },
    /// Show liquidatable portfolios
    Liquidatable,
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Show current configuration
    Show,
    /// Set RPC endpoint
    SetRpc {
        /// RPC URL
        url: String,
    },
    /// Set keypair path
    SetKeypair {
        /// Keypair file path
        path: String,
    },
    /// Initialize configuration file
    Init,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    println!(
        "{} {}",
        style("Percolator Protocol CLI").bold().cyan(),
        style("v0.1.0").dim()
    );

    let result = match cli.command {
        Commands::Portfolio { command } => {
            handle_portfolio_command(&cli.rpc_url, cli.keypair.as_deref(), command).await
        }
        Commands::Slab { command } => {
            handle_slab_command(&cli.rpc_url, cli.keypair.as_deref(), command).await
        }
        Commands::Insurance { command } => {
            handle_insurance_command(&cli.rpc_url, cli.keypair.as_deref(), command).await
        }
        Commands::Trade { command } => {
            handle_trade_command(&cli.rpc_url, cli.keypair.as_deref(), command).await
        }
        Commands::Info { command } => handle_info_command(&cli.rpc_url, command).await,
        Commands::Config { command } => handle_config_command(command).await,
    };

    if let Err(e) = result {
        eprintln!("{} {}", style("Error:").red().bold(), e);
        std::process::exit(1);
    }

    Ok(())
}
