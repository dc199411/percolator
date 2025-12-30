//! Funding rate instruction - updates funding rates and accrues funding payments
//!
//! Implements time-weighted funding rate calculations and applies funding
//! payments to all open positions.

use crate::state::SlabState;
use percolator_common::*;
use pinocchio::msg;

/// Funding rate update interval (1 hour in milliseconds)
pub const FUNDING_INTERVAL_MS: u64 = 3_600_000;

/// Process update funding instruction
///
/// Updates the funding rate for an instrument and applies funding payments
/// to all positions. Funding rate is calculated based on the difference
/// between mark price and index price.
///
/// # Arguments
/// * `slab` - The slab state
/// * `instrument_idx` - The instrument to update
/// * `index_price` - Current index/spot price (1e6 scale)
/// * `current_ts` - Current timestamp
///
/// # Returns
/// * `Ok(())` on success
pub fn process_update_funding(
    slab: &mut SlabState,
    instrument_idx: u16,
    index_price: u64,
    current_ts: u64,
) -> Result<(), PercolatorError> {
    // Validate instrument exists
    let instr = slab.get_instrument(instrument_idx)
        .ok_or_else(|| {
            msg!("Error: Invalid instrument index");
            PercolatorError::InvalidInstrument
        })?;

    // Check if enough time has passed since last funding update
    let last_funding_ts = instr.last_funding_ts;
    let time_elapsed = current_ts.saturating_sub(last_funding_ts);

    if time_elapsed < FUNDING_INTERVAL_MS {
        msg!("Error: Funding interval not elapsed");
        return Err(PercolatorError::InvalidInstruction);
    }

    // Get mark price
    let mark_price = slab.header.mark_px as u64;
    if mark_price == 0 {
        msg!("Error: Mark price not set");
        return Err(PercolatorError::InvalidPrice);
    }

    // Calculate funding rate
    // Funding rate = (mark - index) / index * rate_factor
    // Positive rate means longs pay shorts
    let funding_rate = calculate_funding_rate(mark_price, index_price, time_elapsed);

    // Update cumulative funding
    let old_cum_funding = instr.cum_funding;
    let new_cum_funding = old_cum_funding + funding_rate;

    // Apply funding rate to instrument
    if let Some(instr) = slab.get_instrument_mut(instrument_idx) {
        instr.funding_rate = funding_rate as i64;
        instr.cum_funding = new_cum_funding;
        instr.index_price = index_price;
        instr.last_funding_ts = current_ts;
    }

    // Apply funding to all positions for this instrument
    apply_funding_to_positions(slab, instrument_idx, old_cum_funding, new_cum_funding)?;

    // Update header timestamp
    slab.header.last_funding_ts = current_ts;

    // Increment seqno
    slab.header.increment_seqno();

    msg!("Funding updated successfully");
    Ok(())
}

/// Calculate funding rate based on mark/index price difference
///
/// # Arguments
/// * `mark_price` - Current mark price (1e6 scale)
/// * `index_price` - Current index/spot price (1e6 scale)
/// * `time_elapsed_ms` - Time since last funding update
///
/// # Returns
/// * Funding rate adjustment (signed, 1e6 scale)
fn calculate_funding_rate(mark_price: u64, index_price: u64, time_elapsed_ms: u64) -> i128 {
    if index_price == 0 {
        return 0;
    }

    // Calculate premium/discount: (mark - index) / index
    let mark_i128 = mark_price as i128;
    let index_i128 = index_price as i128;

    // Premium in basis points * 1e4
    let premium_bps = ((mark_i128 - index_i128) * 10_000) / index_i128;

    // Time-weight: funding accrues over 8 hours typically
    // Hourly rate = premium / 8
    // Scale by time elapsed relative to 1 hour
    let hourly_rate = premium_bps / 8;

    // Scale by actual time elapsed
    let time_factor = (time_elapsed_ms as i128 * 1_000_000) / (FUNDING_INTERVAL_MS as i128);
    
    (hourly_rate * time_factor) / 1_000_000
}

/// Apply funding payments to all positions for an instrument
fn apply_funding_to_positions(
    slab: &mut SlabState,
    instrument_idx: u16,
    _old_cum_funding: i128,
    new_cum_funding: i128,
) -> Result<(), PercolatorError> {
    let funding_delta = new_cum_funding - _old_cum_funding;

    // Iterate through all accounts and their positions
    for acc_idx in 0..slab.header.account_count {
        // First pass: collect positions and calculate funding
        let mut positions_to_update: [(u32, i128); 32] = [(SlabState::INVALID_INDEX, 0); 32];
        let mut update_count = 0usize;
        let mut funding_payment: i128 = 0;
        
        // Get position head
        let pos_head = match slab.get_account(acc_idx as u32) {
            Some(a) => a.position_head,
            None => continue,
        };

        let mut pos_idx = pos_head;
        while pos_idx != SlabState::INVALID_INDEX && update_count < 32 {
            let (qty, instr_idx, next) = match slab.get_position(pos_idx) {
                Some(p) => (p.qty, p.instrument_idx, p.next_in_account),
                None => break,
            };

            if instr_idx == instrument_idx {
                // Calculate funding payment for this position
                let payment = calculate_funding_payment(qty, funding_delta, 0);
                funding_payment += payment;
                positions_to_update[update_count] = (pos_idx, new_cum_funding);
                update_count += 1;
            }

            pos_idx = next;
        }

        // Second pass: update positions
        for i in 0..update_count {
            let (idx, cum_funding) = positions_to_update[i];
            if let Some(pos) = slab.get_position_mut(idx) {
                pos.last_funding = cum_funding;
            }
        }

        // Apply funding payment to account cash balance
        if funding_payment != 0 {
            if let Some(acc) = slab.get_account_mut(acc_idx as u32) {
                acc.cash = acc.cash.saturating_sub(funding_payment);
            }
        }
    }

    Ok(())
}

/// Get pending funding payment for a position
///
/// # Arguments
/// * `slab` - The slab state
/// * `pos_idx` - Position index
///
/// # Returns
/// * Pending funding payment (positive = owed, negative = receivable)
pub fn get_pending_funding(slab: &SlabState, pos_idx: u32) -> Option<i128> {
    let pos = slab.get_position(pos_idx)?;
    let instr = slab.get_instrument(pos.instrument_idx)?;

    let cum_funding_now = instr.cum_funding;
    let cum_funding_entry = pos.last_funding;

    Some(calculate_funding_payment(pos.qty, cum_funding_now, cum_funding_entry))
}

/// Process batch funding update for all instruments
///
/// # Arguments
/// * `slab` - The slab state
/// * `index_prices` - Array of index prices for each instrument
/// * `current_ts` - Current timestamp
///
/// # Returns
/// * Number of instruments updated
pub fn process_batch_funding_update(
    slab: &mut SlabState,
    index_prices: &[u64],
    current_ts: u64,
) -> Result<u16, PercolatorError> {
    let instrument_count = slab.header.instrument_count;
    let mut updated = 0u16;

    for i in 0..instrument_count {
        let idx = i as usize;
        if idx >= index_prices.len() {
            break;
        }

        // Try to update this instrument (may fail if interval not elapsed)
        if process_update_funding(slab, i, index_prices[idx], current_ts).is_ok() {
            updated += 1;
        }
    }

    Ok(updated)
}

/// Funding summary for an account
#[derive(Debug, Clone, Copy, Default)]
pub struct FundingSummary {
    /// Total pending funding payments (positive = owed)
    pub total_pending: i128,
    /// Number of positions with pending funding
    pub positions_with_funding: u32,
}

/// Get funding summary for an account
pub fn get_account_funding_summary(slab: &SlabState, account_idx: u32) -> FundingSummary {
    let acc = match slab.get_account(account_idx) {
        Some(a) => a,
        None => return FundingSummary::default(),
    };

    let mut summary = FundingSummary::default();
    let mut pos_idx = acc.position_head;

    while pos_idx != SlabState::INVALID_INDEX {
        if let Some(pending) = get_pending_funding(slab, pos_idx) {
            if pending != 0 {
                summary.total_pending += pending;
                summary.positions_with_funding += 1;
            }
        }

        let pos = match slab.get_position(pos_idx) {
            Some(p) => p,
            None => break,
        };
        pos_idx = pos.next_in_account;
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_funding_rate_calculation() {
        // Mark above index = positive funding (longs pay)
        let rate = calculate_funding_rate(
            51_000_000_000, // $51,000 mark
            50_000_000_000, // $50,000 index
            FUNDING_INTERVAL_MS, // 1 hour
        );
        assert!(rate > 0, "Longs should pay when mark > index");

        // Mark below index = negative funding (shorts pay)
        let rate = calculate_funding_rate(
            49_000_000_000, // $49,000 mark
            50_000_000_000, // $50,000 index
            FUNDING_INTERVAL_MS,
        );
        assert!(rate < 0, "Shorts should pay when mark < index");

        // Equal prices = zero funding
        let rate = calculate_funding_rate(
            50_000_000_000,
            50_000_000_000,
            FUNDING_INTERVAL_MS,
        );
        assert_eq!(rate, 0);
    }

    #[test]
    fn test_funding_interval() {
        assert_eq!(FUNDING_INTERVAL_MS, 3_600_000); // 1 hour
    }
}
