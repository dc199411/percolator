//! Output formatting utilities

use console::style;
use tabled::{Table, Tabled, settings::Style};
use solana_sdk::pubkey::Pubkey;

use percolator_sdk::{UserPortfolio, PortfolioMarginResult, usdc_from_raw};

// ============================================================================
// PORTFOLIO OUTPUT
// ============================================================================

pub fn print_portfolio_status(portfolio: &UserPortfolio) {
    println!("\n{}", style("═══ Portfolio Status ═══").bold().cyan());
    println!();
    
    let collateral = usdc_from_raw(portfolio.collateral_balance);
    let unrealized_pnl = portfolio.unrealized_pnl as f64 / 1_000_000.0;
    let realized_pnl = portfolio.realized_pnl as f64 / 1_000_000.0;
    let equity = collateral + unrealized_pnl;
    
    println!("{:<20} {:>15}", "Owner:", format_pubkey(&portfolio.owner));
    println!("{:<20} {:>15}", "Portfolio ID:", portfolio.portfolio_id);
    println!();
    println!("{}", style("─── Balances ───").dim());
    println!("{:<20} {:>15}", "Collateral:", format_usd(collateral));
    println!("{:<20} {:>15}", "Unrealized PnL:", format_pnl(unrealized_pnl));
    println!("{:<20} {:>15}", "Realized PnL:", format_pnl(realized_pnl));
    println!("{:<20} {:>15}", "Total Equity:", format_usd(equity));
    println!();
    println!("{}", style("─── Margin ───").dim());
    println!("{:<20} {:>15}", "IM Used:", format_usd(portfolio.initial_margin_used as f64 / 1_000_000.0));
    println!("{:<20} {:>15}", "MM Used:", format_usd(portfolio.maintenance_margin_used as f64 / 1_000_000.0));
    println!("{:<20} {:>15}", "Positions:", portfolio.position_count);
    println!();
}

pub fn print_positions(portfolio: &UserPortfolio) {
    println!("\n{}", style("═══ Positions ═══").bold().cyan());
    println!();
    
    if portfolio.position_count == 0 {
        println!("{}", style("No open positions").dim());
        return;
    }
    
    // Would print position table here
    println!("{}", style("(Position display not yet implemented)").dim());
}

pub fn print_margin_details(margin: &PortfolioMarginResult) {
    println!("\n{}", style("═══ Margin Details ═══").bold().cyan());
    println!();
    
    println!("{}", style("─── Initial Margin ───").dim());
    println!("{:<20} {:>15}", "Gross IM:", format_usd(margin.gross_im as f64 / 1_000_000.0));
    println!("{:<20} {:>15}", "Net IM:", format_usd(margin.net_im as f64 / 1_000_000.0));
    println!("{:<20} {:>15}", "Netting Benefit:", format_usd(margin.netting_benefit as f64 / 1_000_000.0));
    println!();
    println!("{}", style("─── Maintenance Margin ───").dim());
    println!("{:<20} {:>15}", "Gross MM:", format_usd(margin.gross_mm as f64 / 1_000_000.0));
    println!("{:<20} {:>15}", "Net MM:", format_usd(margin.net_mm as f64 / 1_000_000.0));
    println!();
    println!("{}", style("─── Available ───").dim());
    println!("{:<20} {:>15}", "Available Margin:", format_pnl(margin.available_margin as f64 / 1_000_000.0));
    println!("{:<20} {:>15}", "Margin Ratio:", format!("{}%", margin.margin_ratio_bps as f64 / 100.0));
    println!();
}

// ============================================================================
// INSURANCE OUTPUT
// ============================================================================

pub fn print_insurance_status_placeholder(slab: &Pubkey) {
    println!("\n{}", style("═══ Insurance Pool Status ═══").bold().cyan());
    println!();
    println!("{:<20} {}", "Slab:", format_pubkey(slab));
    println!();
    println!("{}", style("─── Balance ───").dim());
    println!("{:<20} {:>15}", "Current Balance:", format_usd(0.0));
    println!("{:<20} {:>15}", "Target Balance:", format_usd(0.0));
    println!("{:<20} {:>15}", "Funding Ratio:", "N/A");
    println!();
    println!("{}", style("─── Configuration ───").dim());
    println!("{:<20} {:>15}", "Contribution Rate:", "0.25%");
    println!("{:<20} {:>15}", "ADL Threshold:", "0.50%");
    println!("{:<20} {:>15}", "Withdrawal Lock:", "7 days");
    println!();
    println!("{}", style("─── Statistics ───").dim());
    println!("{:<20} {:>15}", "Total Contributions:", format_usd(0.0));
    println!("{:<20} {:>15}", "Total Payouts:", format_usd(0.0));
    println!("{:<20} {:>15}", "ADL Events:", "0");
    println!();
}

// ============================================================================
// INFO OUTPUT
// ============================================================================

pub fn print_protocol_stats() {
    println!("\n{}", style("═══ Protocol Statistics ═══").bold().cyan());
    println!();
    println!("{}", style("─── Overview ───").dim());
    println!("{:<25} {:>15}", "Total Value Locked:", format_usd(0.0));
    println!("{:<25} {:>15}", "24h Volume:", format_usd(0.0));
    println!("{:<25} {:>15}", "Active Portfolios:", "0");
    println!("{:<25} {:>15}", "Total Slabs:", "0");
    println!();
    println!("{}", style("─── Risk Metrics ───").dim());
    println!("{:<25} {:>15}", "Total Open Interest:", format_usd(0.0));
    println!("{:<25} {:>15}", "Insurance Fund:", format_usd(0.0));
    println!("{:<25} {:>15}", "Liquidations (24h):", "0");
    println!();
}

pub fn print_orderbook_placeholder(instrument: &str, depth: usize) {
    println!("\n{} {}", style("═══ Orderbook:").bold().cyan(), style(instrument).bold());
    println!();
    
    // Header
    println!("{:>15} {:>15} │ {:>15} {:>15}", 
        style("Bid Size").dim(),
        style("Bid").dim(),
        style("Ask").dim(),
        style("Ask Size").dim()
    );
    println!("{}", "─".repeat(64));
    
    // Empty orderbook
    for _ in 0..depth.min(5) {
        println!("{:>15} {:>15} │ {:>15} {:>15}", "-", "-", "-", "-");
    }
    println!();
    println!("{}", style("(No orders in book)").dim());
}

pub fn print_trades_placeholder(instrument: &str, count: usize) {
    println!("\n{} {}", style("═══ Recent Trades:").bold().cyan(), style(instrument).bold());
    println!();
    println!("{:>20} {:>10} {:>12} {:>12}",
        style("Time").dim(),
        style("Side").dim(),
        style("Price").dim(),
        style("Size").dim()
    );
    println!("{}", "─".repeat(56));
    println!();
    println!("{}", style("(No recent trades)").dim());
}

pub fn print_funding_placeholder(instrument: &str) {
    println!("\n{} {}", style("═══ Funding Rate:").bold().cyan(), style(instrument).bold());
    println!();
    println!("{:<20} {:>15}", "Current Rate:", "0.0000%");
    println!("{:<20} {:>15}", "Predicted Rate:", "0.0000%");
    println!("{:<20} {:>15}", "Next Funding:", "N/A");
    println!();
    println!("{}", style("─── 24h History ───").dim());
    println!("{:<20} {:>15}", "Average Rate:", "0.0000%");
    println!("{:<20} {:>15}", "Max Rate:", "0.0000%");
    println!("{:<20} {:>15}", "Min Rate:", "0.0000%");
    println!();
}

// ============================================================================
// FORMATTING HELPERS
// ============================================================================

fn format_usd(amount: f64) -> String {
    if amount >= 1_000_000.0 {
        format!("${:.2}M", amount / 1_000_000.0)
    } else if amount >= 1_000.0 {
        format!("${:.2}K", amount / 1_000.0)
    } else {
        format!("${:.2}", amount)
    }
}

fn format_pnl(amount: f64) -> String {
    let formatted = format_usd(amount.abs());
    if amount >= 0.0 {
        format!("{}", style(format!("+{}", formatted)).green())
    } else {
        format!("{}", style(format!("-{}", formatted)).red())
    }
}

fn format_pubkey(pubkey: &Pubkey) -> String {
    let s = pubkey.to_string();
    if s.len() > 12 {
        format!("{}...{}", &s[..6], &s[s.len()-4..])
    } else {
        s
    }
}

fn format_timestamp(ts: u64) -> String {
    use chrono::{DateTime, Utc};
    let dt = DateTime::<Utc>::from_timestamp(ts as i64, 0)
        .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap());
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}
