//! Commit instruction - phase 2 of two-phase execution
//!
//! Executes trades at the maker prices captured during reserve.
//! Applies fees, updates positions, and records trades.

use crate::state::SlabState;
use percolator_common::*;
use pinocchio::msg;

/// Result of a commit operation
#[derive(Debug, Clone, Copy)]
pub struct CommitResult {
    /// Total filled quantity
    pub filled_qty: u64,
    /// VWAP price of fills
    pub vwap_px: u64,
    /// Total notional value
    pub notional: u128,
    /// Total fees charged
    pub fees: u128,
    /// Realized PnL (if closing position)
    pub realized_pnl: i128,
}

/// Process commit instruction
///
/// Executes a previously reserved fill. Validates the reservation hasn't expired,
/// applies anti-toxicity checks, executes trades at captured maker prices,
/// updates positions, and records trades.
///
/// # Arguments
/// * `slab` - The slab state
/// * `hold_id` - The hold ID from reserve
/// * `current_ts` - Current timestamp (for expiry check)
///
/// # Returns
/// * `CommitResult` with execution details
pub fn process_commit(
    slab: &mut SlabState,
    hold_id: u64,
    current_ts: u64,
) -> Result<CommitResult, PercolatorError> {
    // Find the reservation
    let resv_idx = slab.find_reservation_by_hold_id(hold_id)
        .ok_or_else(|| {
            msg!("Error: Reservation not found");
            PercolatorError::ReservationNotFound
        })?;

    // Get reservation details
    let resv = slab.get_reservation(resv_idx)
        .ok_or(PercolatorError::ReservationNotFound)?;

    // Check if already committed
    if resv.committed {
        msg!("Error: Reservation already committed");
        return Err(PercolatorError::InvalidReservation);
    }

    // Check expiry
    if current_ts > resv.expiry_ms && resv.expiry_ms > 0 {
        // Release the reservation and return error
        release_reservation_slices(slab, resv_idx);
        slab.free_reservation(resv_idx);
        msg!("Error: Reservation expired");
        return Err(PercolatorError::ReservationExpired);
    }

    // Anti-toxicity: Kill band check
    // The mark price should not have moved beyond the kill band since reservation
    let mark_at_reserve = slab.header.prev_mark_px;
    let mark_now = slab.header.mark_px;
    
    if mark_at_reserve != 0 && mark_now != 0 {
        let kill_band_bps = slab.header.kill_band_bps;
        let diff = (mark_now - mark_at_reserve).abs();
        let threshold = (mark_at_reserve.abs() as u64 * kill_band_bps) / 10_000;
        
        if diff > threshold as i64 {
            release_reservation_slices(slab, resv_idx);
            slab.free_reservation(resv_idx);
            msg!("Error: Kill band exceeded");
            return Err(PercolatorError::KillBandExceeded);
        }
    }

    // Extract reservation data before mutable operations
    let account_idx = resv.account_idx;
    let instrument_idx = resv.instrument_idx;
    let side = resv.side;
    let slice_head = resv.slice_head;

    // Execute fills
    let (filled_qty, total_notional, fees) = execute_fills(slab, slice_head, current_ts)?;

    // Calculate VWAP
    let vwap_px = if filled_qty > 0 {
        (total_notional / filled_qty as u128) as u64
    } else {
        0
    };

    // Update position
    let realized_pnl = update_position(slab, account_idx, instrument_idx, side, filled_qty as i64, vwap_px)?;

    // Mark reservation as committed and free it
    if let Some(resv) = slab.get_reservation_mut(resv_idx) {
        resv.committed = true;
    }
    slab.free_reservation(resv_idx);

    // Increment seqno
    slab.header.increment_seqno();

    Ok(CommitResult {
        filled_qty,
        vwap_px,
        notional: total_notional,
        fees,
        realized_pnl,
    })
}

/// Execute fills for all slices in a reservation
///
/// Returns (filled_qty, total_notional, fees)
fn execute_fills(
    slab: &mut SlabState,
    slice_head: u32,
    current_ts: u64,
) -> Result<(u64, u128, u128), PercolatorError> {
    let mut total_qty = 0u64;
    let mut total_notional = 0u128;
    let mut total_fees = 0u128;

    let taker_fee_bps = slab.header.taker_fee_bps;
    let maker_fee_bps = slab.header.maker_fee_bps;
    let maker_rebate_min_ms = slab.header.maker_rebate_min_ms;

    let mut slice_idx = slice_head;
    while slice_idx != SlabState::INVALID_INDEX {
        let slice = match slab.get_slice(slice_idx) {
            Some(s) => s,
            None => break,
        };

        let order_idx = slice.order_idx;
        let fill_qty = slice.qty;
        let next_slice = slice.next;

        // Get order details
        let order = match slab.get_order(order_idx) {
            Some(o) => o,
            None => {
                // Order was canceled, skip this slice
                slab.free_slice(slice_idx);
                slice_idx = next_slice;
                continue;
            }
        };

        let fill_price = order.price;
        let order_created_ms = order.created_ms;
        let instrument_idx = order.instrument_idx;

        // Calculate notional for this fill
        let fill_notional = mul_u64(fill_qty, fill_price);

        // Calculate taker fee
        let taker_fee = (fill_notional * taker_fee_bps as u128) / 10_000;

        // Calculate maker fee/rebate (with JIT penalty check)
        let _maker_fee = if current_ts.saturating_sub(order_created_ms) < maker_rebate_min_ms {
            // JIT penalty: no rebate for orders posted too recently
            0i128
        } else {
            // Normal maker fee (can be negative for rebate)
            (fill_notional as i128 * maker_fee_bps as i128) / 10_000
        };
        // Note: maker_fee is tracked but settlement happens via router

        // Update order quantity
        if let Some(order) = slab.get_order_mut(order_idx) {
            order.qty = order.qty.saturating_sub(fill_qty);
            order.reserved_qty = order.reserved_qty.saturating_sub(fill_qty);

            // If order is fully filled, remove from book
            if order.qty == 0 {
                let order_idx_copy = order_idx;
                // End mutable borrow before calling other methods
                let _ = order;
                slab.remove_order_from_book(order_idx_copy);
                slab.free_order(order_idx_copy);
            }
        }

        // Record trade
        let trade = Trade {
            ts: current_ts,
            order_id_maker: slab.get_order(order_idx).map(|o| o.order_id).unwrap_or(0),
            order_id_taker: 0, // Filled by caller if needed
            instrument_idx,
            side: Side::Buy, // From taker perspective
            _padding: [0; 5],
            price: fill_price,
            qty: fill_qty,
            hash: [0; 32],
            reveal_ms: 0,
        };
        slab.record_trade(trade);

        // Update totals
        total_qty += fill_qty;
        total_notional += fill_notional;
        total_fees += taker_fee;

        // Free the slice
        slab.free_slice(slice_idx);
        slice_idx = next_slice;
    }

    Ok((total_qty, total_notional, total_fees))
}

/// Release all slices in a reservation (restore order available qty)
fn release_reservation_slices(slab: &mut SlabState, resv_idx: u32) {
    let slice_head = match slab.get_reservation(resv_idx) {
        Some(r) => r.slice_head,
        None => return,
    };

    let mut slice_idx = slice_head;
    while slice_idx != SlabState::INVALID_INDEX {
        let slice = match slab.get_slice(slice_idx) {
            Some(s) => s,
            None => break,
        };

        let order_idx = slice.order_idx;
        let qty = slice.qty;
        let next = slice.next;

        // Restore order's reserved_qty
        if let Some(order) = slab.get_order_mut(order_idx) {
            order.reserved_qty = order.reserved_qty.saturating_sub(qty);
        }

        // Free the slice
        slab.free_slice(slice_idx);
        slice_idx = next;
    }
}

/// Update position after a fill
///
/// Returns realized PnL if closing/reducing position
fn update_position(
    slab: &mut SlabState,
    account_idx: u32,
    instrument_idx: u16,
    side: Side,
    fill_qty: i64,
    fill_price: u64,
) -> Result<i128, PercolatorError> {
    // Calculate signed quantity change
    let qty_change = match side {
        Side::Buy => fill_qty,
        Side::Sell => -fill_qty,
    };

    // Find existing position for this account/instrument
    let pos_idx = find_position(slab, account_idx, instrument_idx);

    match pos_idx {
        Some(idx) => {
            // Update existing position
            let pos = slab.get_position_mut(idx)
                .ok_or(PercolatorError::PositionNotFound)?;

            let old_qty = pos.qty;
            let old_entry = pos.entry_px;
            let new_qty = old_qty + qty_change;

            // Calculate realized PnL for any reduction
            let mut realized_pnl = 0i128;

            if (old_qty > 0 && qty_change < 0) || (old_qty < 0 && qty_change > 0) {
                // Position is being reduced or flipped
                let reduced_qty = old_qty.abs().min(qty_change.abs());
                realized_pnl = calculate_pnl(
                    if old_qty > 0 { reduced_qty } else { -reduced_qty },
                    old_entry,
                    fill_price,
                );
            }

            if new_qty == 0 {
                // Position fully closed
                slab.free_position(idx);
            } else {
                // Update position
                pos.qty = new_qty;

                // Update entry price (VWAP for adds, keep for reductions)
                if (old_qty > 0 && qty_change > 0) || (old_qty < 0 && qty_change < 0) {
                    // Adding to position - update VWAP
                    let old_notional = (old_qty.abs() as u128) * (old_entry as u128);
                    let add_notional = (qty_change.abs() as u128) * (fill_price as u128);
                    let new_notional = old_notional + add_notional;
                    let new_abs_qty = new_qty.abs() as u128;
                    if new_abs_qty > 0 {
                        pos.entry_px = (new_notional / new_abs_qty) as u64;
                    }
                }
                // For reductions, entry price stays the same (remaining position at original entry)
            }

            Ok(realized_pnl)
        }
        None => {
            // Create new position
            let new_idx = slab.alloc_position()
                .ok_or_else(|| {
                    msg!("Error: Position pool full");
                    PercolatorError::PoolFull
                })?;

            if let Some(pos) = slab.get_position_mut(new_idx) {
                pos.account_idx = account_idx;
                pos.instrument_idx = instrument_idx;
                pos.qty = qty_change;
                pos.entry_px = fill_price;
                pos.last_funding = 0;
                pos.next_in_account = SlabState::INVALID_INDEX;
            }

            // Link to account's position list
            if let Some(acc) = slab.get_account_mut(account_idx) {
                let old_head = acc.position_head;
                if let Some(pos) = slab.get_position_mut(new_idx) {
                    pos.next_in_account = old_head;
                }
                if let Some(acc) = slab.get_account_mut(account_idx) {
                    acc.position_head = new_idx;
                }
            }

            Ok(0) // No realized PnL for new position
        }
    }
}

/// Find position for account/instrument
fn find_position(slab: &SlabState, account_idx: u32, instrument_idx: u16) -> Option<u32> {
    let acc = slab.get_account(account_idx)?;
    let mut pos_idx = acc.position_head;

    while pos_idx != SlabState::INVALID_INDEX {
        let pos = slab.get_position(pos_idx)?;
        if pos.instrument_idx == instrument_idx {
            return Some(pos_idx);
        }
        pos_idx = pos.next_in_account;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_result_size() {
        assert!(core::mem::size_of::<CommitResult>() <= 64);
    }
}
