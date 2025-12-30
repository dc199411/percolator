//! Cancel instruction - cancel a reservation
//!
//! Releases all slices locked during reserve, restoring the available
//! quantity on each order.

use crate::state::SlabState;
use percolator_common::*;
use pinocchio::msg;

/// Process cancel instruction
///
/// Cancels a reservation, releasing all locked slices and restoring
/// available quantity on the maker orders.
///
/// # Arguments
/// * `slab` - The slab state
/// * `hold_id` - The hold ID of the reservation to cancel
///
/// # Returns
/// * `Ok(())` on success
pub fn process_cancel(
    slab: &mut SlabState,
    hold_id: u64,
) -> Result<(), PercolatorError> {
    // Find the reservation
    let resv_idx = slab.find_reservation_by_hold_id(hold_id)
        .ok_or_else(|| {
            msg!("Error: Reservation not found");
            PercolatorError::ReservationNotFound
        })?;

    // Get reservation details
    let resv = slab.get_reservation(resv_idx)
        .ok_or(PercolatorError::ReservationNotFound)?;

    // Check if already committed (cannot cancel committed reservations)
    if resv.committed {
        msg!("Error: Cannot cancel committed reservation");
        return Err(PercolatorError::InvalidReservation);
    }

    let slice_head = resv.slice_head;

    // Release all slices
    release_slices(slab, slice_head);

    // Free the reservation
    slab.free_reservation(resv_idx);

    // Increment seqno
    slab.header.increment_seqno();

    msg!("Reservation cancelled successfully");
    Ok(())
}

/// Process cancel order instruction
///
/// Cancels a resting order from the book. Only the order owner can cancel.
///
/// # Arguments
/// * `slab` - The slab state
/// * `order_idx` - The order index to cancel
/// * `account_idx` - The account that owns the order (for authorization)
///
/// # Returns
/// * `Ok(())` on success
pub fn process_cancel_order(
    slab: &mut SlabState,
    order_idx: u32,
    account_idx: u32,
) -> Result<(), PercolatorError> {
    // Get the order
    let order = slab.get_order(order_idx)
        .ok_or_else(|| {
            msg!("Error: Order not found");
            PercolatorError::OrderNotFound
        })?;

    // Verify ownership
    if order.account_idx != account_idx {
        msg!("Error: Not authorized to cancel this order");
        return Err(PercolatorError::Unauthorized);
    }

    // Check if order has reserved quantity (partially locked)
    if order.reserved_qty > 0 {
        // Can only cancel unreserved portion
        let unreserved = order.qty.saturating_sub(order.reserved_qty);
        if unreserved == 0 {
            msg!("Error: Order is fully reserved");
            return Err(PercolatorError::ReservedQtyExceeded);
        }

        // Reduce order quantity to just the reserved amount
        if let Some(order) = slab.get_order_mut(order_idx) {
            order.qty = order.reserved_qty;
            order.qty_orig = order.reserved_qty; // Update original to match
        }

        msg!("Order partially cancelled (unreserved portion)");
    } else {
        // Fully cancel - remove from book and free
        slab.remove_order_from_book(order_idx);
        slab.free_order(order_idx);

        msg!("Order fully cancelled");
    }

    // Increment seqno
    slab.header.increment_seqno();

    Ok(())
}

/// Release all slices in a reservation chain
fn release_slices(slab: &mut SlabState, slice_head: u32) {
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

/// Cleanup expired reservations
///
/// Called periodically to clean up reservations that have expired without
/// being committed or explicitly cancelled.
///
/// # Arguments
/// * `slab` - The slab state
/// * `current_ts` - Current timestamp
/// * `max_cleanup` - Maximum number of reservations to clean up (gas limit)
///
/// # Returns
/// * Number of reservations cleaned up
pub fn cleanup_expired_reservations(
    slab: &mut SlabState,
    current_ts: u64,
    max_cleanup: u32,
) -> u32 {
    let mut cleaned = 0u32;

    // Iterate through reservations looking for expired ones
    for i in 0..crate::state::slab::POOL_RESERVATIONS {
        if cleaned >= max_cleanup {
            break;
        }

        let resv = match slab.get_reservation(i as u32) {
            Some(r) => r,
            None => continue,
        };

        // Skip committed reservations (shouldn't exist but just in case)
        if resv.committed {
            continue;
        }

        // Check if expired
        if resv.expiry_ms > 0 && current_ts > resv.expiry_ms {
            let slice_head = resv.slice_head;

            // Release slices
            release_slices(slab, slice_head);

            // Free reservation
            slab.free_reservation(i as u32);
            cleaned += 1;
        }
    }

    if cleaned > 0 {
        slab.header.increment_seqno();
    }

    cleaned
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_cancel_compiles() {
        // Basic compilation test
        assert!(true);
    }
}
