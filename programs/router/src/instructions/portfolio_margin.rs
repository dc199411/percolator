//! Cross-Slab Portfolio Margin Calculations
//!
//! Implements portfolio margin calculations across multiple slabs,
//! enabling capital efficiency through exposure netting.

use crate::state::Portfolio;
use percolator_common::*;

// ============================================================================
// CONSTANTS
// ============================================================================

/// Default Initial Margin Ratio (10%)
pub const DEFAULT_IMR_BPS: u64 = 1000;

/// Default Maintenance Margin Ratio (5%)
pub const DEFAULT_MMR_BPS: u64 = 500;

/// Maximum correlation benefit (50% reduction)
pub const MAX_CORRELATION_BENEFIT_BPS: u64 = 5000;

/// Minimum correlation between instruments to apply benefit
pub const MIN_CORRELATION_THRESHOLD: i32 = 500; // 0.5 in 1000 scale

// ============================================================================
// TYPES
// ============================================================================

/// Instrument risk parameters
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct InstrumentRiskParams {
    /// Instrument identifier (slab_idx, instrument_idx)
    pub slab_idx: u16,
    pub instrument_idx: u16,
    /// Initial margin ratio in basis points
    pub imr_bps: u64,
    /// Maintenance margin ratio in basis points
    pub mmr_bps: u64,
    /// Contract size (for notional calculation)
    pub contract_size: u64,
    /// Mark price
    pub mark_price: u64,
    /// Risk weight (for portfolio grouping)
    pub risk_weight: u64,
}

impl Default for InstrumentRiskParams {
    fn default() -> Self {
        Self {
            slab_idx: 0,
            instrument_idx: 0,
            imr_bps: DEFAULT_IMR_BPS,
            mmr_bps: DEFAULT_MMR_BPS,
            contract_size: 1_000_000, // 1e6 scale
            mark_price: 0,
            risk_weight: 100,
        }
    }
}

/// Correlation entry between two instruments
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CorrelationEntry {
    /// First instrument (slab_idx, instrument_idx)
    pub inst1_slab: u16,
    pub inst1_idx: u16,
    /// Second instrument (slab_idx, instrument_idx)
    pub inst2_slab: u16,
    pub inst2_idx: u16,
    /// Correlation coefficient (-1000 to 1000 representing -1.0 to 1.0)
    pub correlation: i32,
}

/// Portfolio margin calculation result
#[derive(Debug, Clone, Default)]
pub struct PortfolioMarginResult {
    /// Gross IM (sum of individual position IMs)
    pub gross_im: u128,
    /// Net IM (after netting and correlation benefits)
    pub net_im: u128,
    /// Gross MM
    pub gross_mm: u128,
    /// Net MM
    pub net_mm: u128,
    /// Total notional exposure
    pub total_notional: u128,
    /// Netting benefit (gross_im - net_im)
    pub netting_benefit: u128,
    /// Correlation benefit
    pub correlation_benefit: u128,
    /// Number of positions
    pub position_count: u8,
}

/// Position delta (exposure grouped by underlying)
#[derive(Debug, Clone, Copy, Default)]
struct PositionDelta {
    /// Underlying identifier (could be BTC, ETH, etc. in production)
    underlying_id: u8,
    /// Net delta (signed)
    net_delta: i128,
    /// Gross long notional
    long_notional: u128,
    /// Gross short notional
    short_notional: u128,
    /// Risk weight
    risk_weight: u64,
}

// ============================================================================
// PORTFOLIO MARGIN CALCULATION
// ============================================================================

/// Calculate portfolio margin across multiple slabs
///
/// This is the core capital efficiency calculation. It:
/// 1. Groups exposures by underlying
/// 2. Calculates net delta per underlying
/// 3. Applies margin on net exposure (not gross)
/// 4. Optionally applies correlation benefits
///
/// # Arguments
/// * `portfolio` - User's portfolio with exposures
/// * `risk_params` - Risk parameters per instrument
/// * `correlations` - Optional correlation matrix
///
/// # Returns
/// * `PortfolioMarginResult` with detailed breakdown
pub fn calculate_portfolio_margin(
    portfolio: &Portfolio,
    risk_params: &[InstrumentRiskParams],
    correlations: Option<&[CorrelationEntry]>,
) -> PortfolioMarginResult {
    let mut result = PortfolioMarginResult::default();
    
    // Step 1: Calculate gross margins (no netting)
    let mut position_margins: [(u128, u128, i64, u64); 64] = [(0, 0, 0, 0); 64]; // (im, mm, qty, price)
    let mut pos_count = 0usize;
    
    for i in 0..portfolio.exposure_count as usize {
        if pos_count >= 64 {
            break;
        }
        
        let (slab_idx, instrument_idx, qty) = portfolio.exposures[i];
        if qty == 0 {
            continue;
        }
        
        // Find risk params for this instrument
        let params = risk_params.iter()
            .find(|p| p.slab_idx == slab_idx && p.instrument_idx == instrument_idx)
            .cloned()
            .unwrap_or_else(|| {
                let mut default = InstrumentRiskParams::default();
                default.slab_idx = slab_idx;
                default.instrument_idx = instrument_idx;
                default
            });
        
        if params.mark_price == 0 {
            continue;
        }
        
        // Calculate notional
        let abs_qty = qty.unsigned_abs() as u128;
        let notional = (abs_qty * params.mark_price as u128 * params.contract_size as u128) / 1_000_000_000_000;
        
        result.total_notional += notional;
        
        // Calculate individual IM and MM
        let im = (notional * params.imr_bps as u128) / 10_000;
        let mm = (notional * params.mmr_bps as u128) / 10_000;
        
        result.gross_im += im;
        result.gross_mm += mm;
        
        position_margins[pos_count] = (im, mm, qty, params.mark_price);
        pos_count += 1;
    }
    
    result.position_count = pos_count as u8;
    
    // Step 2: Calculate net exposure by grouping same instruments across slabs
    let net_exposure = calculate_net_exposure_groups(portfolio, risk_params);
    
    // Step 3: Calculate margin on net exposure
    result.net_im = calculate_net_im(&net_exposure, risk_params);
    result.net_mm = result.net_im / 2; // MM = IM / 2 simplified
    
    // Step 4: Apply correlation benefits (if provided)
    if let Some(corrs) = correlations {
        let corr_benefit = calculate_correlation_benefit(&net_exposure, corrs, risk_params);
        result.correlation_benefit = corr_benefit;
        result.net_im = result.net_im.saturating_sub(corr_benefit);
        result.net_mm = result.net_mm.saturating_sub(corr_benefit / 2);
    }
    
    // Calculate netting benefit
    result.netting_benefit = result.gross_im.saturating_sub(result.net_im);
    
    result
}

/// Group exposures and calculate net delta per underlying
fn calculate_net_exposure_groups(
    portfolio: &Portfolio,
    risk_params: &[InstrumentRiskParams],
) -> [PositionDelta; 16] {
    let mut groups: [PositionDelta; 16] = [PositionDelta::default(); 16];
    
    // For v0, we group all instruments together (same underlying assumption)
    // In production, would use underlying_id from risk params
    let mut total_long: i128 = 0;
    let mut total_short: i128 = 0;
    let mut total_long_notional: u128 = 0;
    let mut total_short_notional: u128 = 0;
    
    for i in 0..portfolio.exposure_count as usize {
        let (slab_idx, instrument_idx, qty) = portfolio.exposures[i];
        if qty == 0 {
            continue;
        }
        
        let params = risk_params.iter()
            .find(|p| p.slab_idx == slab_idx && p.instrument_idx == instrument_idx)
            .cloned()
            .unwrap_or_default();
        
        let abs_qty = qty.unsigned_abs() as u128;
        let notional = abs_qty * params.mark_price as u128 / 1_000_000;
        
        if qty > 0 {
            total_long += qty as i128;
            total_long_notional += notional;
        } else {
            total_short += (-qty) as i128;
            total_short_notional += notional;
        }
    }
    
    groups[0] = PositionDelta {
        underlying_id: 0,
        net_delta: total_long - total_short,
        long_notional: total_long_notional,
        short_notional: total_short_notional,
        risk_weight: 100,
    };
    
    groups
}

/// Calculate IM on net exposure
fn calculate_net_im(
    groups: &[PositionDelta; 16],
    risk_params: &[InstrumentRiskParams],
) -> u128 {
    let mut total_im: u128 = 0;
    
    // Get average IMR from risk params
    let avg_imr = if risk_params.is_empty() {
        DEFAULT_IMR_BPS
    } else {
        risk_params.iter().map(|p| p.imr_bps).sum::<u64>() / risk_params.len() as u64
    };
    
    // Get average mark price
    let avg_price = if risk_params.is_empty() {
        1_000_000u64
    } else {
        let mut total_price = 0u64;
        let mut count = 0u64;
        for p in risk_params {
            if p.mark_price > 0 {
                total_price = total_price.saturating_add(p.mark_price);
                count += 1;
            }
        }
        if count == 0 { 1_000_000 } else { total_price / count }
    };
    
    for group in groups {
        if group.net_delta == 0 && group.long_notional == 0 && group.short_notional == 0 {
            continue;
        }
        
        // IM on NET exposure, not gross
        let abs_net = group.net_delta.unsigned_abs();
        let net_notional = abs_net * avg_price as u128 / 1_000_000;
        let im = (net_notional * avg_imr as u128) / 10_000;
        
        total_im += im;
    }
    
    total_im
}

/// Calculate correlation benefit
fn calculate_correlation_benefit(
    groups: &[PositionDelta; 16],
    correlations: &[CorrelationEntry],
    risk_params: &[InstrumentRiskParams],
) -> u128 {
    // Simplified correlation benefit calculation
    // In production, would use full variance-covariance matrix
    
    let mut benefit: u128 = 0;
    
    // Only apply benefit if there are offsetting positions
    for i in 0..groups.len() {
        for j in (i + 1)..groups.len() {
            let g1 = &groups[i];
            let g2 = &groups[j];
            
            if g1.net_delta == 0 || g2.net_delta == 0 {
                continue;
            }
            
            // Find correlation between these groups
            let corr = correlations.iter()
                .find(|c| {
                    (c.inst1_slab as u8 == g1.underlying_id && c.inst2_slab as u8 == g2.underlying_id) ||
                    (c.inst2_slab as u8 == g1.underlying_id && c.inst1_slab as u8 == g2.underlying_id)
                })
                .map(|c| c.correlation)
                .unwrap_or(0);
            
            // If positions are opposite and correlated, apply benefit
            if (g1.net_delta > 0) != (g2.net_delta > 0) && corr > MIN_CORRELATION_THRESHOLD {
                let smaller_notional = g1.long_notional.saturating_add(g1.short_notional)
                    .min(g2.long_notional.saturating_add(g2.short_notional));
                
                // Benefit = correlation * smaller_notional * max_benefit_factor
                let corr_factor = corr as u128;
                let max_benefit = (smaller_notional * MAX_CORRELATION_BENEFIT_BPS as u128) / 10_000;
                benefit += (max_benefit * corr_factor) / 1000;
            }
        }
    }
    
    // Also consider same-instrument netting across slabs
    // (This is already captured in net_delta calculation)
    let _ = risk_params;
    
    benefit
}

// ============================================================================
// MARK-TO-MARKET OPERATIONS
// ============================================================================

/// Update portfolio marks and recalculate margin
pub fn mark_to_market(
    portfolio: &mut Portfolio,
    mark_prices: &[(u16, u16, u64)], // (slab_idx, instrument_idx, mark_price)
    risk_params: &[InstrumentRiskParams],
) -> PortfolioMarginResult {
    // Calculate new margin based on current marks
    let result = calculate_portfolio_margin(portfolio, risk_params, None);
    
    // Update portfolio
    portfolio.update_margin(result.net_im, result.net_mm);
    portfolio.last_mark_ts = 0; // Would be current timestamp in production
    
    let _ = mark_prices; // Used for PnL calculation in production
    
    result
}

/// Calculate unrealized PnL for portfolio
pub fn calculate_unrealized_pnl(
    portfolio: &Portfolio,
    entry_prices: &[(u16, u16, u64)], // (slab_idx, instrument_idx, entry_price)
    mark_prices: &[(u16, u16, u64)], // (slab_idx, instrument_idx, mark_price)
) -> i128 {
    let mut total_pnl: i128 = 0;
    
    for i in 0..portfolio.exposure_count as usize {
        let (slab_idx, instrument_idx, qty) = portfolio.exposures[i];
        if qty == 0 {
            continue;
        }
        
        // Find entry price
        let entry = entry_prices.iter()
            .find(|(s, inst, _)| *s == slab_idx && *inst == instrument_idx)
            .map(|(_, _, p)| *p)
            .unwrap_or(0);
        
        // Find mark price
        let mark = mark_prices.iter()
            .find(|(s, inst, _)| *s == slab_idx && *inst == instrument_idx)
            .map(|(_, _, p)| *p)
            .unwrap_or(0);
        
        if entry == 0 || mark == 0 {
            continue;
        }
        
        // PnL = qty * (mark - entry) / 1e6
        let price_diff = mark as i128 - entry as i128;
        let pnl = (qty as i128 * price_diff) / 1_000_000;
        total_pnl += pnl;
    }
    
    total_pnl
}

// ============================================================================
// MARGIN CHECKING
// ============================================================================

/// Check if a portfolio meets initial margin requirements
pub fn check_im_requirement(
    portfolio: &Portfolio,
    risk_params: &[InstrumentRiskParams],
) -> bool {
    let margin = calculate_portfolio_margin(portfolio, risk_params, None);
    portfolio.equity >= margin.net_im as i128
}

/// Check if a portfolio meets maintenance margin requirements
pub fn check_mm_requirement(
    portfolio: &Portfolio,
    risk_params: &[InstrumentRiskParams],
) -> bool {
    let margin = calculate_portfolio_margin(portfolio, risk_params, None);
    portfolio.equity >= margin.net_mm as i128
}

/// Calculate maximum order size given current margin
pub fn calculate_max_order_size(
    portfolio: &Portfolio,
    instrument_params: &InstrumentRiskParams,
    side: Side,
) -> u64 {
    let free_margin = portfolio.free_collateral;
    if free_margin <= 0 {
        return 0;
    }
    
    // Max size = free_margin / (price * IMR / 1e6)
    let price = instrument_params.mark_price;
    let imr = instrument_params.imr_bps;
    
    if price == 0 || imr == 0 {
        return 0;
    }
    
    let margin_per_unit = (price as u128 * imr as u128) / (10_000 * 1_000_000);
    if margin_per_unit == 0 {
        return 0;
    }
    
    // Consider existing exposure for netting
    let existing_exposure = calculate_total_exposure(portfolio);
    let same_direction = (side == Side::Buy && existing_exposure >= 0) ||
                         (side == Side::Sell && existing_exposure <= 0);
    
    if same_direction {
        // Adding to position, no netting benefit
        (free_margin as u128 / margin_per_unit) as u64
    } else {
        // Opposite direction, potential netting benefit
        // Allow up to 2x the margin requirement due to netting
        (free_margin as u128 * 2 / margin_per_unit) as u64
    }
}

/// Calculate total net exposure from portfolio
fn calculate_total_exposure(portfolio: &Portfolio) -> i64 {
    let mut total = 0i64;
    for i in 0..portfolio.exposure_count as usize {
        total += portfolio.exposures[i].2;
    }
    total
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use pinocchio::pubkey::Pubkey;

    fn make_portfolio_with_exposure(exposures: &[(u16, u16, i64)], equity: i128) -> Portfolio {
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        portfolio.update_equity(equity);
        for (slab, inst, qty) in exposures {
            portfolio.update_exposure(*slab, *inst, *qty);
        }
        portfolio
    }

    fn make_risk_params(slab: u16, inst: u16, price: u64) -> InstrumentRiskParams {
        InstrumentRiskParams {
            slab_idx: slab,
            instrument_idx: inst,
            imr_bps: 1000, // 10%
            mmr_bps: 500,  // 5%
            contract_size: 1_000_000,
            mark_price: price,
            risk_weight: 100,
        }
    }

    #[test]
    fn test_portfolio_margin_empty() {
        let portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        let result = calculate_portfolio_margin(&portfolio, &[], None);
        
        assert_eq!(result.gross_im, 0);
        assert_eq!(result.net_im, 0);
        assert_eq!(result.position_count, 0);
    }

    #[test]
    fn test_portfolio_margin_single_position() {
        let portfolio = make_portfolio_with_exposure(&[(0, 0, 1_000_000)], 100_000_000_000);
        let risk_params = [make_risk_params(0, 0, 50_000_000_000)]; // $50k BTC
        
        let result = calculate_portfolio_margin(&portfolio, &risk_params, None);
        
        assert!(result.gross_im > 0);
        assert_eq!(result.gross_im, result.net_im); // Single position, no netting
        assert_eq!(result.position_count, 1);
    }

    #[test]
    fn test_portfolio_margin_netting() {
        // Long 1 BTC on slab 0, short 1 BTC on slab 1 = 0 net
        let portfolio = make_portfolio_with_exposure(&[
            (0, 0, 1_000_000),   // Long 1 BTC
            (1, 0, -1_000_000),  // Short 1 BTC
        ], 100_000_000_000);
        
        let risk_params = [
            make_risk_params(0, 0, 50_000_000_000),
            make_risk_params(1, 0, 50_000_000_000),
        ];
        
        let result = calculate_portfolio_margin(&portfolio, &risk_params, None);
        
        // Net exposure is 0, so net_im should be 0
        assert_eq!(result.net_im, 0);
        // But gross IM should be non-zero
        assert!(result.gross_im > 0);
        // Netting benefit = gross - net
        assert_eq!(result.netting_benefit, result.gross_im);
    }

    #[test]
    fn test_portfolio_margin_partial_netting() {
        // Long 2 BTC on slab 0, short 1 BTC on slab 1 = net long 1 BTC
        let portfolio = make_portfolio_with_exposure(&[
            (0, 0, 2_000_000),   // Long 2 BTC
            (1, 0, -1_000_000),  // Short 1 BTC
        ], 100_000_000_000);
        
        let risk_params = [
            make_risk_params(0, 0, 50_000_000_000),
            make_risk_params(1, 0, 50_000_000_000),
        ];
        
        let result = calculate_portfolio_margin(&portfolio, &risk_params, None);
        
        // Net IM should be less than gross IM
        assert!(result.net_im < result.gross_im);
        assert!(result.netting_benefit > 0);
    }

    #[test]
    fn test_check_im_requirement_pass() {
        let mut portfolio = make_portfolio_with_exposure(&[(0, 0, 1_000_000)], 100_000_000_000);
        portfolio.update_margin(50_000_000_000, 25_000_000_000);
        
        let risk_params = [make_risk_params(0, 0, 50_000_000_000)];
        assert!(check_im_requirement(&portfolio, &risk_params));
    }

    #[test]
    fn test_check_im_requirement_fail() {
        let mut portfolio = make_portfolio_with_exposure(&[(0, 0, 1_000_000)], 1_000_000_000);
        portfolio.update_margin(50_000_000_000, 25_000_000_000);
        
        let risk_params = [make_risk_params(0, 0, 50_000_000_000)];
        assert!(!check_im_requirement(&portfolio, &risk_params));
    }

    #[test]
    fn test_calculate_unrealized_pnl() {
        let portfolio = make_portfolio_with_exposure(&[(0, 0, 1_000_000)], 100_000_000_000);
        
        let entry_prices = [(0u16, 0u16, 45_000_000_000u64)]; // Entry at $45k
        let mark_prices = [(0u16, 0u16, 50_000_000_000u64)];  // Mark at $50k
        
        let pnl = calculate_unrealized_pnl(&portfolio, &entry_prices, &mark_prices);
        
        // PnL = 1 BTC * ($50k - $45k) = $5k = 5_000_000_000 (scaled)
        assert!(pnl > 0);
    }

    #[test]
    fn test_calculate_max_order_size() {
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        portfolio.update_equity(100_000_000_000); // $100k
        portfolio.update_margin(0, 0);
        portfolio.free_collateral = 100_000_000_000;
        
        let params = make_risk_params(0, 0, 50_000_000_000);
        
        let max_size = calculate_max_order_size(&portfolio, &params, Side::Buy);
        assert!(max_size > 0);
    }

    #[test]
    fn test_position_delta_struct_size() {
        assert!(core::mem::size_of::<PositionDelta>() <= 64);
    }

    #[test]
    fn test_instrument_risk_params_size() {
        assert!(core::mem::size_of::<InstrumentRiskParams>() <= 64);
    }
}
