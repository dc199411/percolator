//! Global Liquidation Coordination
//!
//! Router-coordinated liquidation across multiple slabs.
//! Ensures positions are closed in the correct order to minimize
//! system risk and maximize recovery.

use crate::state::{Portfolio, Vault, SlabRegistry};
use percolator_common::*;
use pinocchio::{account_info::AccountInfo, msg, pubkey::Pubkey};

// ============================================================================
// CONSTANTS
// ============================================================================

/// Maximum positions to liquidate in a single transaction
pub const MAX_LIQUIDATION_POSITIONS: usize = 16;

/// Liquidation fee basis points (0.5%)
pub const LIQUIDATION_FEE_BPS: u64 = 50;

/// Insurance fund contribution basis points (0.25%)
pub const INSURANCE_FUND_BPS: u64 = 25;

/// Maximum slippage for liquidation orders (2%)
pub const MAX_LIQUIDATION_SLIPPAGE_BPS: u64 = 200;

// ============================================================================
// TYPES
// ============================================================================

/// Position to be liquidated
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct LiquidatablePosition {
    /// Slab index
    pub slab_idx: u16,
    /// Instrument index
    pub instrument_idx: u16,
    /// Position quantity (signed)
    pub qty: i64,
    /// Mark price at liquidation
    pub mark_price: u64,
    /// Notional value
    pub notional: u128,
    /// Unrealized PnL
    pub unrealized_pnl: i128,
}

/// Liquidation order
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct LiquidationOrder {
    /// Slab program ID
    pub slab_program_id: Pubkey,
    /// Slab state account
    pub slab_state: Pubkey,
    /// Account index on slab
    pub account_idx: u32,
    /// Instrument index
    pub instrument_idx: u16,
    /// Side to close (opposite of position)
    pub close_side: u8,
    /// Quantity to close
    pub close_qty: u64,
    /// Maximum acceptable price (with slippage)
    pub max_price: u64,
}

/// Result of liquidation health check
#[derive(Debug, Clone)]
pub struct LiquidationHealthCheck {
    /// Is the portfolio below maintenance margin?
    pub is_liquidatable: bool,
    /// Current equity
    pub equity: i128,
    /// Maintenance margin requirement
    pub mm: u128,
    /// Deficit (how much below MM)
    pub deficit: i128,
    /// Positions to liquidate (ordered by priority)
    pub positions: [Option<LiquidatablePosition>; MAX_LIQUIDATION_POSITIONS],
    /// Number of positions
    pub position_count: u8,
    /// Total notional at risk
    pub total_notional_at_risk: u128,
}

impl Default for LiquidationHealthCheck {
    fn default() -> Self {
        Self {
            is_liquidatable: false,
            equity: 0,
            mm: 0,
            deficit: 0,
            positions: [None; MAX_LIQUIDATION_POSITIONS],
            position_count: 0,
            total_notional_at_risk: 0,
        }
    }
}

/// Result of liquidation execution
#[derive(Debug, Clone)]
pub struct LiquidationResult {
    /// Number of positions closed
    pub positions_closed: u8,
    /// Total notional liquidated
    pub total_notional: u128,
    /// Total fees collected
    pub total_fees: u128,
    /// Insurance fund contribution
    pub insurance_contribution: u128,
    /// Realized PnL (usually negative for liquidations)
    pub realized_pnl: i128,
    /// Remaining deficit (if any)
    pub remaining_deficit: i128,
    /// Success flag
    pub success: bool,
}

impl Default for LiquidationResult {
    fn default() -> Self {
        Self {
            positions_closed: 0,
            total_notional: 0,
            total_fees: 0,
            insurance_contribution: 0,
            realized_pnl: 0,
            remaining_deficit: 0,
            success: false,
        }
    }
}

// ============================================================================
// LIQUIDATION HEALTH CHECK
// ============================================================================

/// Check if a portfolio is liquidatable
///
/// Calculates equity, margin requirements, and identifies positions
/// to liquidate in priority order (largest positions first).
pub fn check_liquidation_health(
    portfolio: &Portfolio,
    mark_prices: &[(u16, u16, u64)], // (slab_idx, instrument_idx, mark_price)
) -> LiquidationHealthCheck {
    let mut result = LiquidationHealthCheck::default();
    
    // Calculate equity based on current mark prices
    let mut total_unrealized_pnl: i128 = 0;
    let mut positions: [(i128, LiquidatablePosition); MAX_LIQUIDATION_POSITIONS] = 
        [(0, LiquidatablePosition {
            slab_idx: 0,
            instrument_idx: 0,
            qty: 0,
            mark_price: 0,
            notional: 0,
            unrealized_pnl: 0,
        }); MAX_LIQUIDATION_POSITIONS];
    let mut pos_count = 0usize;
    
    // Iterate through portfolio exposures
    for i in 0..portfolio.exposure_count as usize {
        if pos_count >= MAX_LIQUIDATION_POSITIONS {
            break;
        }
        
        let (slab_idx, instrument_idx, qty) = portfolio.exposures[i];
        if qty == 0 {
            continue;
        }
        
        // Find mark price for this position
        let mark_price = mark_prices.iter()
            .find(|(s, inst, _)| *s == slab_idx && *inst == instrument_idx)
            .map(|(_, _, p)| *p)
            .unwrap_or(0);
        
        if mark_price == 0 {
            continue;
        }
        
        // Calculate notional and unrealized PnL
        let notional = mul_u64(qty.unsigned_abs(), mark_price);
        let unrealized_pnl = calculate_position_pnl(qty, mark_price);
        
        total_unrealized_pnl += unrealized_pnl;
        
        positions[pos_count] = (notional as i128, LiquidatablePosition {
            slab_idx,
            instrument_idx,
            qty,
            mark_price,
            notional,
            unrealized_pnl,
        });
        pos_count += 1;
        
        result.total_notional_at_risk += notional;
    }
    
    // Sort positions by notional (largest first for priority liquidation)
    // Simple bubble sort for small arrays (no_std compatible)
    for i in 0..pos_count {
        for j in 0..(pos_count - i - 1) {
            if positions[j].0 < positions[j + 1].0 {
                positions.swap(j, j + 1);
            }
        }
    }
    
    // Copy to result
    for i in 0..pos_count {
        result.positions[i] = Some(positions[i].1);
    }
    result.position_count = pos_count as u8;
    
    // Calculate equity (simplified - assumes cash is in equity field)
    result.equity = portfolio.equity + total_unrealized_pnl;
    result.mm = portfolio.mm;
    
    // Check if liquidatable
    result.is_liquidatable = result.equity < result.mm as i128;
    
    if result.is_liquidatable {
        result.deficit = result.mm as i128 - result.equity;
    }
    
    result
}

/// Calculate unrealized PnL for a position (simplified)
fn calculate_position_pnl(qty: i64, mark_price: u64) -> i128 {
    // Simplified: assume entry price is stored elsewhere
    // For now, return 0 (actual implementation would use entry price)
    let _ = (qty, mark_price);
    0
}

// ============================================================================
// LIQUIDATION EXECUTION
// ============================================================================

/// Execute global liquidation across slabs
///
/// Closes positions across multiple slabs until the portfolio is back
/// above maintenance margin or all positions are closed.
///
/// # Arguments
/// * `portfolio` - User's portfolio (mutable)
/// * `user` - User pubkey
/// * `vault` - Collateral vault (mutable)
/// * `registry` - Slab registry
/// * `slab_accounts` - Slab account infos
/// * `health_check` - Result of liquidation health check
/// * `current_ts` - Current timestamp
///
/// # Returns
/// * `LiquidationResult` with execution details
pub fn process_global_liquidation(
    portfolio: &mut Portfolio,
    user: &Pubkey,
    vault: &mut Vault,
    registry: &SlabRegistry,
    _slab_accounts: &[AccountInfo],
    health_check: &LiquidationHealthCheck,
    current_ts: u64,
) -> Result<LiquidationResult, PercolatorError> {
    // Validate
    if &portfolio.user != user {
        msg!("Error: Portfolio does not belong to user");
        return Err(PercolatorError::InvalidPortfolio);
    }
    
    if !health_check.is_liquidatable {
        msg!("Error: Portfolio is not liquidatable");
        return Err(PercolatorError::PortfolioNotLiquidatable);
    }
    
    let mut result = LiquidationResult::default();
    let mut remaining_deficit = health_check.deficit;
    
    // Process positions in priority order (largest first)
    for i in 0..health_check.position_count as usize {
        let pos = match &health_check.positions[i] {
            Some(p) => p,
            None => continue,
        };
        
        // Skip if we've covered the deficit
        if remaining_deficit <= 0 {
            break;
        }
        
        // Validate slab is registered
        // In production, would look up slab program ID from registry
        let _ = registry;
        
        // Build liquidation order
        let close_side = if pos.qty > 0 { Side::Sell } else { Side::Buy };
        let slippage_mult = 10_000 + MAX_LIQUIDATION_SLIPPAGE_BPS;
        let max_price = if close_side == Side::Sell {
            (pos.mark_price as u128 * (10_000 - MAX_LIQUIDATION_SLIPPAGE_BPS) as u128 / 10_000) as u64
        } else {
            (pos.mark_price as u128 * slippage_mult as u128 / 10_000) as u64
        };
        
        let liq_order = LiquidationOrder {
            slab_program_id: Pubkey::default(), // Would be from registry
            slab_state: Pubkey::default(), // Would be from accounts
            account_idx: 0,
            instrument_idx: pos.instrument_idx,
            close_side: close_side as u8,
            close_qty: pos.qty.unsigned_abs(),
            max_price,
        };
        
        // Execute liquidation on slab (CPI in production)
        let liq_result = execute_slab_liquidation(&liq_order, current_ts)?;
        
        // Calculate fees
        let liq_fee = (liq_result.notional * LIQUIDATION_FEE_BPS as u128) / 10_000;
        let insurance = (liq_result.notional * INSURANCE_FUND_BPS as u128) / 10_000;
        
        // Update results
        result.positions_closed += 1;
        result.total_notional += liq_result.notional;
        result.total_fees += liq_fee;
        result.insurance_contribution += insurance;
        result.realized_pnl += liq_result.realized_pnl;
        
        // Update portfolio exposure
        portfolio.update_exposure(pos.slab_idx, pos.instrument_idx, 0);
        
        // Update remaining deficit
        // Simplified: assume liquidation reduces deficit by notional * MM%
        let total_notional_at_risk = health_check.total_notional_at_risk;
        let mm_freed = if total_notional_at_risk > 0 {
            liq_result.notional * portfolio.mm / total_notional_at_risk
        } else {
            0
        };
        remaining_deficit -= mm_freed as i128;
    }
    
    result.remaining_deficit = remaining_deficit.max(0);
    
    // Update vault (credit from position closure, debit fees)
    if result.realized_pnl > 0 {
        vault.balance += result.realized_pnl as u128;
    } else {
        let loss = (-result.realized_pnl) as u128;
        vault.balance = vault.balance.saturating_sub(loss);
    }
    vault.balance = vault.balance.saturating_sub(result.total_fees + result.insurance_contribution);
    
    // Recalculate portfolio margin
    let net_exposure = calculate_remaining_exposure(portfolio);
    let new_im = calculate_portfolio_im(net_exposure, health_check.positions[0]
        .map(|p| p.mark_price).unwrap_or(0));
    portfolio.update_margin(new_im, new_im / 2);
    
    result.success = result.remaining_deficit <= 0;
    
    msg!("Global liquidation completed");
    
    Ok(result)
}

/// Execute liquidation on a single slab (stub for CPI)
fn execute_slab_liquidation(order: &LiquidationOrder, _current_ts: u64) -> Result<SlabLiquidationResult, PercolatorError> {
    // In production, this would CPI to the slab's liquidation_call instruction
    Ok(SlabLiquidationResult {
        filled_qty: order.close_qty,
        avg_price: order.max_price,
        notional: mul_u64(order.close_qty, order.max_price),
        realized_pnl: 0, // Would be calculated by slab
    })
}

struct SlabLiquidationResult {
    filled_qty: u64,
    avg_price: u64,
    notional: u128,
    realized_pnl: i128,
}

/// Calculate remaining net exposure from portfolio
fn calculate_remaining_exposure(portfolio: &Portfolio) -> i64 {
    let mut net = 0i64;
    for i in 0..portfolio.exposure_count as usize {
        net += portfolio.exposures[i].2;
    }
    net
}

/// Calculate initial margin (same as in multi_slab module)
fn calculate_portfolio_im(net_exposure: i64, price: u64) -> u128 {
    let abs_exposure = net_exposure.unsigned_abs() as u128;
    let notional = abs_exposure * price as u128;
    notional * 10 / (100 * 1_000_000)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_liquidation_health_check_not_liquidatable() {
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        portfolio.update_equity(100_000_000_000); // $100k equity
        portfolio.update_margin(50_000_000_000, 25_000_000_000); // $50k IM, $25k MM
        
        let health = check_liquidation_health(&portfolio, &[]);
        assert!(!health.is_liquidatable);
    }

    #[test]
    fn test_liquidation_health_check_liquidatable() {
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        portfolio.update_equity(20_000_000_000); // $20k equity
        portfolio.update_margin(50_000_000_000, 25_000_000_000); // $50k IM, $25k MM
        portfolio.update_exposure(0, 0, 1_000_000); // 1 BTC position
        
        let mark_prices = [(0u16, 0u16, 50_000_000_000u64)];
        let health = check_liquidation_health(&portfolio, &mark_prices);
        
        assert!(health.is_liquidatable);
        assert!(health.deficit > 0);
    }

    #[test]
    fn test_liquidation_order_size() {
        assert!(core::mem::size_of::<LiquidationOrder>() <= 128);
    }

    #[test]
    fn test_liquidatable_position_size() {
        assert!(core::mem::size_of::<LiquidatablePosition>() <= 64);
    }

    #[test]
    fn test_calculate_portfolio_im() {
        // 1 BTC at $50k, 10% IMR = $5k IM
        let im = calculate_portfolio_im(1_000_000, 50_000_000_000);
        assert!(im > 0);
    }
}
