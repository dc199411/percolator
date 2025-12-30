//! Reserve instruction - phase 1 of two-phase execution
//!
//! Walks the orderbook, locks slices at each price level, and calculates
//! VWAP and max_charge for the Router to use in escrow/cap creation.

use crate::state::SlabState;
use percolator_common::*;
use pinocchio::msg;

/// Result of a reserve operation
#[derive(Debug, Clone, Copy)]
pub struct ReserveResult {
    /// Unique hold ID for this reservation
    pub hold_id: u64,
    /// VWAP price of reserved slices
    pub vwap_px: u64,
    /// Worst price in reservation
    pub worst_px: u64,
    /// Total quantity reserved
    pub filled_qty: u64,
    /// Maximum charge (notional + fees)
    pub max_charge: u128,
    /// Expiry timestamp
    pub expiry_ms: u64,
    /// Book sequence number at reservation time
    pub book_seqno: u64,
}

/// Process reserve instruction
///
/// Walks the orderbook on the contra side, locking slices up to the requested
/// quantity and limit price. Returns reservation details for Router to create
/// escrow and capability tokens.
///
/// # Arguments
/// * `slab` - The slab state
/// * `account_idx` - Account index of the taker
/// * `instrument_idx` - Instrument index
/// * `side` - Side of the order (Buy/Sell)
/// * `qty` - Requested quantity (1e6 scale)
/// * `limit_px` - Worst acceptable price (1e6 scale)
/// * `ttl_ms` - Time-to-live for reservation in milliseconds
/// * `commitment_hash` - Hash for commit-reveal (optional)
/// * `route_id` - Route ID from router
///
/// # Returns
/// * `ReserveResult` with reservation details
pub fn process_reserve(
    slab: &mut SlabState,
    account_idx: u32,
    instrument_idx: u16,
    side: Side,
    qty: u64,
    limit_px: u64,
    ttl_ms: u64,
    commitment_hash: [u8; 32],
    route_id: u64,
) -> Result<ReserveResult, PercolatorError> {
    // Validate instrument
    if slab.get_instrument(instrument_idx).is_none() {
        msg!("Error: Invalid instrument index");
        return Err(PercolatorError::InvalidInstrument);
    }

    // Validate quantity
    if qty == 0 {
        msg!("Error: Quantity must be positive");
        return Err(PercolatorError::InvalidQuantity);
    }

    // Validate price
    if limit_px == 0 {
        msg!("Error: Limit price must be positive");
        return Err(PercolatorError::InvalidPrice);
    }

    // Allocate a new reservation
    let resv_idx = slab.alloc_reservation()
        .ok_or_else(|| {
            msg!("Error: Reservation pool full");
            PercolatorError::PoolFull
        })?;

    // Get the hold ID
    let hold_id = slab.header.next_hold_id();
    let book_seqno = slab.header.seqno as u64;

    // Calculate expiry
    // Note: In production, would use Clock sysvar for current time
    let expiry_ms = ttl_ms; // Relative TTL for now

    // Walk the book and reserve slices
    let (filled_qty, total_notional, worst_px, slice_head) = 
        walk_and_reserve(slab, instrument_idx, side, qty, limit_px, resv_idx)?;

    // If no liquidity was found, free the reservation and return error
    if filled_qty == 0 {
        slab.free_reservation(resv_idx);
        msg!("Error: Insufficient liquidity");
        return Err(PercolatorError::InsufficientLiquidity);
    }

    // Calculate VWAP
    let vwap_px = calculate_vwap(total_notional, filled_qty);

    // Calculate max charge (notional + taker fee)
    let taker_fee_bps = slab.header.taker_fee_bps;
    let fee = (total_notional * taker_fee_bps as u128) / 10_000;
    let max_charge = total_notional + fee;

    // Fill in the reservation
    if let Some(resv) = slab.get_reservation_mut(resv_idx) {
        resv.hold_id = hold_id;
        resv.route_id = route_id;
        resv.account_idx = account_idx;
        resv.instrument_idx = instrument_idx;
        resv.side = side;
        resv.qty = filled_qty;
        resv.vwap_px = vwap_px;
        resv.worst_px = worst_px;
        resv.max_charge = max_charge;
        resv.commitment_hash = commitment_hash;
        resv.salt = [0; 16]; // Will be filled by Router
        resv.book_seqno = book_seqno;
        resv.expiry_ms = expiry_ms;
        resv.slice_head = slice_head;
        resv.committed = false;
    }

    // Increment seqno
    slab.header.increment_seqno();

    Ok(ReserveResult {
        hold_id,
        vwap_px,
        worst_px,
        filled_qty,
        max_charge,
        expiry_ms,
        book_seqno,
    })
}

/// Walk the orderbook and reserve slices
///
/// Returns (filled_qty, total_notional, worst_px, slice_head)
fn walk_and_reserve(
    slab: &mut SlabState,
    instrument_idx: u16,
    side: Side,
    mut qty_remaining: u64,
    limit_px: u64,
    _resv_idx: u32,
) -> Result<(u64, u128, u64, u32), PercolatorError> {
    let mut total_qty = 0u64;
    let mut total_notional = 0u128;
    let mut worst_px = 0u64;
    let mut slice_head = SlabState::INVALID_INDEX;
    let mut prev_slice_idx = SlabState::INVALID_INDEX;

    // Get the head of the contra side
    let mut order_idx = match slab.get_best_contra(instrument_idx, side) {
        Some(idx) => idx,
        None => return Ok((0, 0, 0, SlabState::INVALID_INDEX)),
    };

    // Walk the book
    while qty_remaining > 0 && order_idx != SlabState::INVALID_INDEX {
        let order = match slab.get_order(order_idx) {
            Some(o) => o,
            None => break,
        };

        // Check if price is acceptable
        let price_ok = match side {
            Side::Buy => order.price <= limit_px,   // Buy: maker ask must be <= limit
            Side::Sell => order.price >= limit_px,  // Sell: maker bid must be >= limit
        };

        if !price_ok {
            break;
        }

        // Calculate available quantity (not already reserved)
        let available = order.qty.saturating_sub(order.reserved_qty);
        if available == 0 {
            order_idx = order.next;
            continue;
        }

        // Take what we can from this order
        let take = available.min(qty_remaining);
        
        // Allocate a slice
        let slice_idx = slab.alloc_slice()
            .ok_or_else(|| {
                msg!("Error: Slice pool full");
                PercolatorError::PoolFull
            })?;

        // Fill in the slice
        if let Some(slice) = slab.get_slice_mut(slice_idx) {
            slice.order_idx = order_idx;
            slice.qty = take;
            slice.next = SlabState::INVALID_INDEX;
        }

        // Link slices together
        if slice_head == SlabState::INVALID_INDEX {
            slice_head = slice_idx;
        } else if let Some(prev) = slab.get_slice_mut(prev_slice_idx) {
            prev.next = slice_idx;
        }
        prev_slice_idx = slice_idx;

        // Update order's reserved quantity
        if let Some(order) = slab.get_order_mut(order_idx) {
            order.reserved_qty += take;
        }

        // Update totals
        let order_price = slab.get_order(order_idx).map(|o| o.price).unwrap_or(0);
        total_qty += take;
        total_notional += mul_u64(take, order_price);
        worst_px = order_price;
        qty_remaining -= take;

        // Move to next order
        let next = slab.get_order(order_idx).map(|o| o.next).unwrap_or(SlabState::INVALID_INDEX);
        order_idx = next;
    }

    Ok((total_qty, total_notional, worst_px, slice_head))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full tests require setting up book state which needs more infrastructure
    // These are placeholder tests

    #[test]
    fn test_reserve_result_size() {
        assert!(core::mem::size_of::<ReserveResult>() <= 64);
    }
}
