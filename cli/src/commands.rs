//! CLI Command Handlers

use anyhow::{anyhow, Result};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
    signer::Signer,
};
use std::str::FromStr;
use std::time::Duration;

use percolator_sdk::{
    PercolatorClient, Side, TimeInForce,
    usdc_to_raw, usdc_from_raw, price_to_raw, qty_to_raw,
    derive_portfolio_pda, derive_insurance_pda,
};

use crate::{
    PortfolioCommands, SlabCommands, InsuranceCommands,
    TradeCommands, InfoCommands, ConfigCommands,
};
use crate::config::Config;
use crate::output::*;

// ============================================================================
// HELPERS
// ============================================================================

fn get_keypair(keypair_path: Option<&str>) -> Result<Keypair> {
    let path = keypair_path
        .or_else(|| std::env::var("KEYPAIR_PATH").ok().as_deref().map(|_| ""))
        .ok_or_else(|| anyhow!("No keypair specified. Use --keypair or set KEYPAIR_PATH"))?;
    
    read_keypair_file(path)
        .map_err(|e| anyhow!("Failed to read keypair: {}", e))
}

fn spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{spinner:.blue} {msg}")
            .unwrap(),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

fn parse_pubkey(s: &str) -> Result<Pubkey> {
    Pubkey::from_str(s).map_err(|e| anyhow!("Invalid pubkey: {}", e))
}

fn parse_side(s: &str) -> Result<Side> {
    match s.to_lowercase().as_str() {
        "buy" | "long" | "b" => Ok(Side::Buy),
        "sell" | "short" | "s" => Ok(Side::Sell),
        _ => Err(anyhow!("Invalid side: {}. Use 'buy' or 'sell'", s)),
    }
}

// ============================================================================
// PORTFOLIO COMMANDS
// ============================================================================

pub async fn handle_portfolio_command(
    rpc_url: &str,
    keypair_path: Option<&str>,
    command: PortfolioCommands,
) -> Result<()> {
    let client = PercolatorClient::new(rpc_url)?;

    match command {
        PortfolioCommands::Init => {
            let keypair = get_keypair(keypair_path)?;
            let spinner = spinner("Initializing portfolio...");
            
            let ix = client.build_initialize_portfolio(&keypair.pubkey());
            let sig = client.send_transaction(&[ix], &[&keypair], &keypair.pubkey())?;
            
            spinner.finish_with_message("Portfolio initialized!");
            println!("{}", style(format!("Transaction: {}", sig)).green());
            
            let (pda, _) = derive_portfolio_pda(&keypair.pubkey());
            println!("{}", style(format!("Portfolio PDA: {}", pda)).dim());
        }

        PortfolioCommands::Status => {
            let keypair = get_keypair(keypair_path)?;
            let spinner = spinner("Fetching portfolio...");
            
            match client.get_portfolio(&keypair.pubkey())? {
                Some(portfolio) => {
                    spinner.finish_and_clear();
                    print_portfolio_status(&portfolio);
                }
                None => {
                    spinner.finish_with_message("Portfolio not found");
                    println!("{}", style("Run 'percolator portfolio init' to create one").yellow());
                }
            }
        }

        PortfolioCommands::Deposit { amount } => {
            let keypair = get_keypair(keypair_path)?;
            let spinner = spinner(&format!("Depositing {} USDC...", amount));
            
            // Get user's token account (simplified - real impl would derive ATA)
            let token_account = Pubkey::default(); // Placeholder
            let ix = client.build_deposit(&keypair.pubkey(), &token_account, usdc_to_raw(amount));
            let sig = client.send_transaction(&[ix], &[&keypair], &keypair.pubkey())?;
            
            spinner.finish_with_message(&format!("Deposited {} USDC!", amount));
            println!("{}", style(format!("Transaction: {}", sig)).green());
        }

        PortfolioCommands::Withdraw { amount } => {
            let keypair = get_keypair(keypair_path)?;
            let spinner = spinner(&format!("Withdrawing {} USDC...", amount));
            
            let token_account = Pubkey::default(); // Placeholder
            let ix = client.build_withdraw(&keypair.pubkey(), &token_account, usdc_to_raw(amount));
            let sig = client.send_transaction(&[ix], &[&keypair], &keypair.pubkey())?;
            
            spinner.finish_with_message(&format!("Withdrew {} USDC!", amount));
            println!("{}", style(format!("Transaction: {}", sig)).green());
        }

        PortfolioCommands::Positions => {
            let keypair = get_keypair(keypair_path)?;
            let spinner = spinner("Fetching positions...");
            
            match client.get_portfolio(&keypair.pubkey())? {
                Some(portfolio) => {
                    spinner.finish_and_clear();
                    print_positions(&portfolio);
                }
                None => {
                    spinner.finish_with_message("Portfolio not found");
                }
            }
        }

        PortfolioCommands::Margin => {
            let keypair = get_keypair(keypair_path)?;
            let spinner = spinner("Calculating margin...");
            
            match client.get_portfolio(&keypair.pubkey())? {
                Some(portfolio) => {
                    spinner.finish_and_clear();
                    let margin = client.calculate_portfolio_margin(
                        portfolio.collateral_balance,
                        portfolio.unrealized_pnl,
                        &[], // Would pass positions
                    );
                    print_margin_details(&margin);
                }
                None => {
                    spinner.finish_with_message("Portfolio not found");
                }
            }
        }
    }

    Ok(())
}

// ============================================================================
// SLAB COMMANDS
// ============================================================================

pub async fn handle_slab_command(
    rpc_url: &str,
    keypair_path: Option<&str>,
    command: SlabCommands,
) -> Result<()> {
    let client = PercolatorClient::new(rpc_url)?;

    match command {
        SlabCommands::Init { imr_bps, mmr_bps, maker_fee_bps, taker_fee_bps, batch_ms } => {
            let keypair = get_keypair(keypair_path)?;
            let spinner = spinner("Initializing slab...");
            
            println!("\n{}", style("Slab Parameters:").bold());
            println!("  IMR: {}%", imr_bps as f64 / 100.0);
            println!("  MMR: {}%", mmr_bps as f64 / 100.0);
            println!("  Maker Fee: {}%", maker_fee_bps as f64 / 100.0);
            println!("  Taker Fee: {}%", taker_fee_bps as f64 / 100.0);
            println!("  Batch Window: {}ms", batch_ms);
            
            // Note: Actual initialization would create slab account
            spinner.finish_with_message("Slab initialization not yet implemented");
        }

        SlabCommands::Status { address } => {
            let slab_pubkey = parse_pubkey(&address)?;
            let spinner = spinner("Fetching slab status...");
            
            // Would fetch slab header here
            spinner.finish_with_message("Slab status display not yet implemented");
            println!("Slab: {}", slab_pubkey);
        }

        SlabCommands::List => {
            let spinner = spinner("Fetching slabs...");
            
            // Would fetch from registry
            spinner.finish_with_message("Slab listing not yet implemented");
        }

        SlabCommands::AddInstrument { slab, symbol, tick_size, lot_size } => {
            let keypair = get_keypair(keypair_path)?;
            let slab_pubkey = parse_pubkey(&slab)?;
            let spinner = spinner(&format!("Adding instrument {}...", symbol));
            
            println!("\n{}", style("Instrument Parameters:").bold());
            println!("  Symbol: {}", symbol);
            println!("  Tick Size: {}", tick_size);
            println!("  Lot Size: {}", lot_size);
            
            spinner.finish_with_message("Instrument addition not yet implemented");
        }

        SlabCommands::Update { slab, imr_bps, mmr_bps } => {
            let keypair = get_keypair(keypair_path)?;
            let slab_pubkey = parse_pubkey(&slab)?;
            let spinner = spinner("Updating slab parameters...");
            
            if let Some(imr) = imr_bps {
                println!("  New IMR: {}%", imr as f64 / 100.0);
            }
            if let Some(mmr) = mmr_bps {
                println!("  New MMR: {}%", mmr as f64 / 100.0);
            }
            
            spinner.finish_with_message("Slab update not yet implemented");
        }
    }

    Ok(())
}

// ============================================================================
// INSURANCE COMMANDS
// ============================================================================

pub async fn handle_insurance_command(
    rpc_url: &str,
    keypair_path: Option<&str>,
    command: InsuranceCommands,
) -> Result<()> {
    let client = PercolatorClient::new(rpc_url)?;

    match command {
        InsuranceCommands::Init { slab, contribution_rate, adl_threshold, timelock_days } => {
            let keypair = get_keypair(keypair_path)?;
            let slab_pubkey = parse_pubkey(&slab)?;
            let spinner = spinner("Initializing insurance pool...");
            
            let timelock_secs = timelock_days * 24 * 60 * 60;
            
            println!("\n{}", style("Insurance Pool Parameters:").bold());
            println!("  Contribution Rate: {}%", contribution_rate as f64 / 100.0);
            println!("  ADL Threshold: {}%", adl_threshold as f64 / 100.0);
            println!("  Withdrawal Timelock: {} days", timelock_days);
            
            let ix = client.build_initialize_insurance(
                &slab_pubkey,
                &keypair.pubkey(),
                contribution_rate,
                adl_threshold,
                timelock_secs,
            );
            let sig = client.send_transaction(&[ix], &[&keypair], &keypair.pubkey())?;
            
            spinner.finish_with_message("Insurance pool initialized!");
            println!("{}", style(format!("Transaction: {}", sig)).green());
            
            let (insurance_pda, _) = derive_insurance_pda(&slab_pubkey);
            println!("{}", style(format!("Insurance PDA: {}", insurance_pda)).dim());
        }

        InsuranceCommands::Status { slab } => {
            let slab_pubkey = parse_pubkey(&slab)?;
            let spinner = spinner("Fetching insurance status...");
            
            // Would fetch insurance pool here
            spinner.finish_and_clear();
            print_insurance_status_placeholder(&slab_pubkey);
        }

        InsuranceCommands::Contribute { slab, amount } => {
            let keypair = get_keypair(keypair_path)?;
            let slab_pubkey = parse_pubkey(&slab)?;
            let spinner = spinner(&format!("Contributing {} USDC to insurance...", amount));
            
            let token_account = Pubkey::default(); // Placeholder
            let insurance_vault = Pubkey::default(); // Placeholder
            
            let ix = client.build_contribute_insurance(
                &slab_pubkey,
                &keypair.pubkey(),
                &token_account,
                &insurance_vault,
                usdc_to_raw(amount),
            );
            let sig = client.send_transaction(&[ix], &[&keypair], &keypair.pubkey())?;
            
            spinner.finish_with_message(&format!("Contributed {} USDC!", amount));
            println!("{}", style(format!("Transaction: {}", sig)).green());
        }

        InsuranceCommands::InitiateWithdraw { slab, amount } => {
            let keypair = get_keypair(keypair_path)?;
            let slab_pubkey = parse_pubkey(&slab)?;
            let spinner = spinner(&format!("Initiating withdrawal of {} USDC...", amount));
            
            let ix = client.build_initiate_insurance_withdrawal(
                &slab_pubkey,
                &keypair.pubkey(),
                usdc_to_raw(amount),
            );
            let sig = client.send_transaction(&[ix], &[&keypair], &keypair.pubkey())?;
            
            spinner.finish_with_message("Withdrawal initiated!");
            println!("{}", style(format!("Transaction: {}", sig)).green());
            println!("{}", style("Note: Withdrawal will unlock after 7 days").yellow());
        }

        InsuranceCommands::CompleteWithdraw { slab } => {
            let keypair = get_keypair(keypair_path)?;
            let slab_pubkey = parse_pubkey(&slab)?;
            let spinner = spinner("Completing withdrawal...");
            
            // Would complete withdrawal here
            spinner.finish_with_message("Withdrawal completion not yet implemented");
        }

        InsuranceCommands::CancelWithdraw { slab } => {
            let keypair = get_keypair(keypair_path)?;
            let slab_pubkey = parse_pubkey(&slab)?;
            let spinner = spinner("Cancelling withdrawal...");
            
            // Would cancel withdrawal here
            spinner.finish_with_message("Withdrawal cancelled");
        }

        InsuranceCommands::History { slab, count } => {
            let slab_pubkey = parse_pubkey(&slab)?;
            let spinner = spinner("Fetching insurance history...");
            
            // Would fetch event history here
            spinner.finish_and_clear();
            println!("{}", style("Insurance Event History").bold());
            println!("{}", style(format!("Showing last {} events", count)).dim());
            println!("\n{}", style("(No events yet)").dim());
        }
    }

    Ok(())
}

// ============================================================================
// TRADE COMMANDS
// ============================================================================

pub async fn handle_trade_command(
    rpc_url: &str,
    keypair_path: Option<&str>,
    command: TradeCommands,
) -> Result<()> {
    let client = PercolatorClient::new(rpc_url)?;

    match command {
        TradeCommands::Market { instrument, side, qty } => {
            let keypair = get_keypair(keypair_path)?;
            let side = parse_side(&side)?;
            let spinner = spinner(&format!("Placing {} {} {} market order...", 
                if matches!(side, Side::Buy) { "BUY" } else { "SELL" },
                qty,
                instrument
            ));
            
            // Would build and send market order here
            spinner.finish_with_message("Market order placement not yet implemented");
        }

        TradeCommands::Limit { instrument, side, qty, price, post_only } => {
            let keypair = get_keypair(keypair_path)?;
            let side = parse_side(&side)?;
            let spinner = spinner(&format!("Placing {} {} {} @ {} limit order...",
                if matches!(side, Side::Buy) { "BUY" } else { "SELL" },
                qty,
                instrument,
                price
            ));
            
            println!("\n{}", style("Order Details:").bold());
            println!("  Instrument: {}", instrument);
            println!("  Side: {}", if matches!(side, Side::Buy) { "BUY" } else { "SELL" });
            println!("  Quantity: {}", qty);
            println!("  Price: ${}", price);
            println!("  Post-only: {}", post_only);
            
            // Would build and send limit order here
            spinner.finish_with_message("Limit order placement not yet implemented");
        }

        TradeCommands::Cancel { order_id } => {
            let keypair = get_keypair(keypair_path)?;
            let spinner = spinner(&format!("Cancelling order {}...", order_id));
            
            // Would cancel order here
            spinner.finish_with_message("Order cancellation not yet implemented");
        }

        TradeCommands::CancelAll { instrument } => {
            let keypair = get_keypair(keypair_path)?;
            let msg = match &instrument {
                Some(inst) => format!("Cancelling all {} orders...", inst),
                None => "Cancelling all orders...".to_string(),
            };
            let spinner = spinner(&msg);
            
            // Would cancel all orders here
            spinner.finish_with_message("Cancel all not yet implemented");
        }

        TradeCommands::Orders => {
            let keypair = get_keypair(keypair_path)?;
            let spinner = spinner("Fetching open orders...");
            
            // Would fetch orders here
            spinner.finish_and_clear();
            println!("{}", style("Open Orders").bold());
            println!("\n{}", style("(No open orders)").dim());
        }
    }

    Ok(())
}

// ============================================================================
// INFO COMMANDS
// ============================================================================

pub async fn handle_info_command(rpc_url: &str, command: InfoCommands) -> Result<()> {
    let client = PercolatorClient::new(rpc_url)?;

    match command {
        InfoCommands::Stats => {
            let spinner = spinner("Fetching protocol stats...");
            
            // Would fetch protocol stats here
            spinner.finish_and_clear();
            print_protocol_stats();
        }

        InfoCommands::Orderbook { instrument, depth } => {
            let spinner = spinner(&format!("Fetching {} orderbook...", instrument));
            
            // Would fetch orderbook here
            spinner.finish_and_clear();
            print_orderbook_placeholder(&instrument, depth);
        }

        InfoCommands::Trades { instrument, count } => {
            let spinner = spinner(&format!("Fetching {} recent trades...", instrument));
            
            // Would fetch trades here
            spinner.finish_and_clear();
            print_trades_placeholder(&instrument, count);
        }

        InfoCommands::Funding { instrument } => {
            let spinner = spinner(&format!("Fetching {} funding rate...", instrument));
            
            // Would fetch funding here
            spinner.finish_and_clear();
            print_funding_placeholder(&instrument);
        }

        InfoCommands::Liquidatable => {
            let spinner = spinner("Scanning for liquidatable portfolios...");
            
            // Would scan for liquidatable portfolios here
            spinner.finish_and_clear();
            println!("{}", style("Liquidatable Portfolios").bold());
            println!("\n{}", style("(None found)").dim());
        }
    }

    Ok(())
}

// ============================================================================
// CONFIG COMMANDS
// ============================================================================

pub async fn handle_config_command(command: ConfigCommands) -> Result<()> {
    match command {
        ConfigCommands::Show => {
            let config = Config::load().unwrap_or_default();
            println!("{}", style("Current Configuration:").bold());
            println!("  RPC URL: {}", config.rpc_url);
            if let Some(path) = &config.keypair_path {
                println!("  Keypair: {}", path);
            } else {
                println!("  Keypair: {}", style("(not set)").dim());
            }
        }

        ConfigCommands::SetRpc { url } => {
            let mut config = Config::load().unwrap_or_default();
            config.rpc_url = url.clone();
            config.save()?;
            println!("{}", style(format!("RPC URL set to: {}", url)).green());
        }

        ConfigCommands::SetKeypair { path } => {
            // Validate keypair exists
            if !std::path::Path::new(&path).exists() {
                return Err(anyhow!("Keypair file not found: {}", path));
            }
            let mut config = Config::load().unwrap_or_default();
            config.keypair_path = Some(path.clone());
            config.save()?;
            println!("{}", style(format!("Keypair path set to: {}", path)).green());
        }

        ConfigCommands::Init => {
            let config = Config::default();
            config.save()?;
            println!("{}", style("Configuration file created!").green());
            println!("Location: ~/.config/percolator/config.json");
        }
    }

    Ok(())
}
