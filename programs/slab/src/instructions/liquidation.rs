//! Liquidation instruction - executes liquidation of underwater positions
//!
//! Closes positions when account equity falls below maintenance margin.
//! Implements price-banded liquidation to prevent excessive slippage.

use crate::state::SlabState;
use percolator_common::*;
use pinocchio::msg;

/// Maximum price impact for liquidation (basis points)
pub const MAX_LIQUIDATION_IMPACT_BPS: u64 = 500; // 5% max impact

/// Liquidation fee (basis points)
pub const LIQUIDATION_FEE_BPS: u64 = 50; // 0.5% fee

/// Result of liquidation attempt
#[derive(Debug, Clone, Copy)]
pub struct LiquidationResult {
    /// Total positions liquidated
    pub positions_closed: u32,
    /// Total quantity liquidated
    pub total_qty_liquidated: u64,
    /// Total value liquidated
    pub total_value: u128,
    /// Total fees collected
    pub fees_collected: u128,
    /// Remaining deficit (if any)
    pub remaining_deficit: i128,
}

/// Process liquidation call from router
///
/// Attempts to liquidate positions for an account that is below maintenance margin.
/// Liquidation proceeds by market-selling positions within price bands.
///
/// # Arguments
/// * `slab` - The slab state
/// * `account_idx` - Account to liquidate
/// * `deficit_target` - Target deficit to cover
/// * `current_ts` - Current timestamp
///
/// # Returns
/// * `LiquidationResult` with details of the liquidation
pub fn process_liquidation(
    slab: &mut SlabState,
    account_idx: u32,
    deficit_target: i128,
    current_ts: u64,
) -> Result<LiquidationResult, PercolatorError> {
    // Verify account exists and is underwater
    let acc = slab.get_account(account_idx)
        .ok_or_else(|| {
            msg!("Error: Account not found");
            PercolatorError::InvalidAccount
        })?;

    // Calculate current equity
    let equity = calculate_account_equity(slab, account_idx);
    let mm = acc.mm as i128;

    if equity >= mm {
        msg!("Error: Account not below maintenance margin");
        return Err(PercolatorError::InvalidAccount);
    }

    let mut result = LiquidationResult {
        positions_closed: 0,
        total_qty_liquidated: 0,
        total_value: 0,
        fees_collected: 0,
        remaining_deficit: deficit_target,
    };

    // Get position list head
    let mut pos_idx = acc.position_head;
    let mark_px = slab.header.mark_px as u64;

    // Iterate through positions and liquidate
    while pos_idx != SlabState::INVALID_INDEX && result.remaining_deficit > 0 {
        let pos = match slab.get_position(pos_idx) {
            Some(p) => p,
            None => break,
        };

        let qty = pos.qty;
        let _instrument_idx = pos.instrument_idx;
        let entry_px = pos.entry_px;
        let next_pos = pos.next_in_account;

        if qty == 0 {
            pos_idx = next_pos;
            continue;
        }

        // Calculate liquidation price with price band
        let liq_price = calculate_liquidation_price(mark_px, qty > 0);

        // Calculate position value
        let abs_qty = qty.abs() as u64;
        let position_value = mul_u64(abs_qty, liq_price);

        // Calculate realized PnL
        let realized_pnl = calculate_pnl(qty, entry_px, liq_price);

        // Calculate liquidation fee
        let fee = (position_value * LIQUIDATION_FEE_BPS as u128) / 10_000;

        // Close the position
        close_position_for_liquidation(slab, account_idx, pos_idx, liq_price, current_ts)?;

        // Update result
        result.positions_closed += 1;
        result.total_qty_liquidated += abs_qty;
        result.total_value += position_value;
        result.fees_collected += fee;
        result.remaining_deficit -= realized_pnl;

        pos_idx = next_pos;
    }

    // Recalculate margin requirements first (needs immutable borrow)
    let (new_im, new_mm) = recalculate_margin_requirements(slab, account_idx);
    
    // Update account state (needs mutable borrow)
    if let Some(acc) = slab.get_account_mut(account_idx) {
        // Deduct fees from cash
        acc.cash = acc.cash.saturating_sub(result.fees_collected as i128);
        acc.im = new_im;
        acc.mm = new_mm;
    }

    // Increment seqno
    slab.header.increment_seqno();

    msg!("Liquidation complete");
    Ok(result)
}

/// Calculate account equity including unrealized PnL
fn calculate_account_equity(slab: &SlabState, account_idx: u32) -> i128 {
    let acc = match slab.get_account(account_idx) {
        Some(a) => a,
        None => return 0,
    };

    let mut equity = acc.cash;
    let mark_px = slab.header.mark_px as u64;

    // Add unrealized PnL from all positions
    let mut pos_idx = acc.position_head;
    while pos_idx != SlabState::INVALID_INDEX {
        let pos = match slab.get_position(pos_idx) {
            Some(p) => p,
            None => break,
        };

        let unrealized_pnl = calculate_pnl(pos.qty, pos.entry_px, mark_px);
        equity += unrealized_pnl;

        pos_idx = pos.next_in_account;
    }

    equity
}

/// Calculate liquidation price with price band protection
fn calculate_liquidation_price(mark_px: u64, is_long: bool) -> u64 {
    let max_impact = (mark_px * MAX_LIQUIDATION_IMPACT_BPS) / 10_000;

    if is_long {
        // Selling long position - price may slip down
        mark_px.saturating_sub(max_impact)
    } else {
        // Buying back short position - price may slip up
        mark_px.saturating_add(max_impact)
    }
}

/// Close a position during liquidation
fn close_position_for_liquidation(
    slab: &mut SlabState,
    account_idx: u32,
    pos_idx: u32,
    close_price: u64,
    current_ts: u64,
) -> Result<(), PercolatorError> {
    let pos = slab.get_position(pos_idx)
        .ok_or(PercolatorError::PositionNotFound)?;

    let qty = pos.qty;
    let instrument_idx = pos.instrument_idx;

    // Record liquidation trade
    let trade = Trade {
        ts: current_ts,
        order_id_maker: 0, // Market order
        order_id_taker: 0, // Liquidation
        instrument_idx,
        side: if qty > 0 { Side::Sell } else { Side::Buy },
        _padding: [0; 5],
        price: close_price,
        qty: qty.abs() as u64,
        hash: [0; 32],
        reveal_ms: 0,
    };
    slab.record_trade(trade);

    // Remove position from account's linked list
    unlink_position_from_account(slab, account_idx, pos_idx);

    // Free the position
    slab.free_position(pos_idx);

    Ok(())
}

/// Remove position from account's position linked list
fn unlink_position_from_account(slab: &mut SlabState, account_idx: u32, pos_idx: u32) {
    let acc = match slab.get_account(account_idx) {
        Some(a) => a,
        None => return,
    };

    let mut prev_idx = SlabState::INVALID_INDEX;
    let mut curr_idx = acc.position_head;

    // Find the position in the linked list
    while curr_idx != SlabState::INVALID_INDEX {
        if curr_idx == pos_idx {
            // Found it - unlink
            let next_idx = slab.get_position(curr_idx)
                .map(|p| p.next_in_account)
                .unwrap_or(SlabState::INVALID_INDEX);

            if prev_idx == SlabState::INVALID_INDEX {
                // Was head - update account head
                if let Some(acc) = slab.get_account_mut(account_idx) {
                    acc.position_head = next_idx;
                }
            } else {
                // Update previous position's next pointer
                if let Some(prev) = slab.get_position_mut(prev_idx) {
                    prev.next_in_account = next_idx;
                }
            }
            return;
        }

        prev_idx = curr_idx;
        curr_idx = slab.get_position(curr_idx)
            .map(|p| p.next_in_account)
            .unwrap_or(SlabState::INVALID_INDEX);
    }
}

/// Recalculate margin requirements for an account
fn recalculate_margin_requirements(slab: &SlabState, account_idx: u32) -> (u128, u128) {
    let acc = match slab.get_account(account_idx) {
        Some(a) => a,
        None => return (0, 0),
    };

    let mark_px = slab.header.mark_px as u64;
    let imr_bps = slab.header.imr_bps;
    let mmr_bps = slab.header.mmr_bps;

    let mut total_im = 0u128;
    let mut total_mm = 0u128;

    let mut pos_idx = acc.position_head;
    while pos_idx != SlabState::INVALID_INDEX {
        let pos = match slab.get_position(pos_idx) {
            Some(p) => p,
            None => break,
        };

        // Get instrument contract size (default to 1e6)
        let contract_size = slab.get_instrument(pos.instrument_idx)
            .map(|i| i.contract_size)
            .unwrap_or(1_000_000) as u64;

        let im = calculate_im(pos.qty, contract_size, mark_px, imr_bps);
        let mm = calculate_mm(pos.qty, contract_size, mark_px, mmr_bps);

        total_im += im;
        total_mm += mm;

        pos_idx = pos.next_in_account;
    }

    (total_im, total_mm)
}

/// Check if account is liquidatable
pub fn is_liquidatable(slab: &SlabState, account_idx: u32) -> bool {
    let acc = match slab.get_account(account_idx) {
        Some(a) => a,
        None => return false,
    };

    let equity = calculate_account_equity(slab, account_idx);
    equity < acc.mm as i128
}

/// Get liquidation preview for an account
#[derive(Debug, Clone, Copy)]
pub struct LiquidationPreview {
    /// Current equity
    pub equity: i128,
    /// Maintenance margin
    pub mm: u128,
    /// Deficit to cover
    pub deficit: i128,
    /// Number of positions
    pub position_count: u32,
    /// Total position value
    pub total_value: u128,
}

/// Get liquidation preview
pub fn get_liquidation_preview(slab: &SlabState, account_idx: u32) -> Option<LiquidationPreview> {
    let acc = slab.get_account(account_idx)?;
    let equity = calculate_account_equity(slab, account_idx);
    let mm = acc.mm;
    let deficit = (mm as i128) - equity;

    let mark_px = slab.header.mark_px as u64;
    let mut position_count = 0u32;
    let mut total_value = 0u128;

    let mut pos_idx = acc.position_head;
    while pos_idx != SlabState::INVALID_INDEX {
        let pos = match slab.get_position(pos_idx) {
            Some(p) => p,
            None => break,
        };

        position_count += 1;
        total_value += mul_u64(pos.qty.abs() as u64, mark_px);
        pos_idx = pos.next_in_account;
    }

    Some(LiquidationPreview {
        equity,
        mm,
        deficit,
        position_count,
        total_value,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_liquidation_price_long() {
        let mark = 50_000_000_000u64; // $50,000
        let liq_px = calculate_liquidation_price(mark, true);
        
        // Should be 5% below mark
        let expected = mark - (mark * MAX_LIQUIDATION_IMPACT_BPS) / 10_000;
        assert_eq!(liq_px, expected);
    }

    #[test]
    fn test_liquidation_price_short() {
        let mark = 50_000_000_000u64; // $50,000
        let liq_px = calculate_liquidation_price(mark, false);
        
        // Should be 5% above mark
        let expected = mark + (mark * MAX_LIQUIDATION_IMPACT_BPS) / 10_000;
        assert_eq!(liq_px, expected);
    }

    #[test]
    fn test_liquidation_fee() {
        // 0.5% fee
        assert_eq!(LIQUIDATION_FEE_BPS, 50);
    }
}
