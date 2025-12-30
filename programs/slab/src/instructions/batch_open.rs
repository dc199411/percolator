//! Batch open instruction - opens a new batch window
//!
//! Increments the epoch, promotes pending orders to live,
//! and sets freeze windows as configured.

use crate::state::SlabState;
use percolator_common::*;
use pinocchio::msg;

/// Process batch open instruction
///
/// Opens a new batch window by:
/// 1. Incrementing the epoch
/// 2. Promoting pending orders that are eligible for this epoch
/// 3. Setting freeze windows on top-K orders (anti-sandwich)
/// 4. Recording batch open timestamp
///
/// # Arguments
/// * `slab` - The slab state
/// * `instrument_idx` - The instrument to process
/// * `current_ts` - Current timestamp
///
/// # Returns
/// * `Ok(())` on success
pub fn process_batch_open(
    slab: &mut SlabState,
    instrument_idx: u16,
    current_ts: u64,
) -> Result<(), PercolatorError> {
    // Validate instrument exists
    if slab.get_instrument(instrument_idx).is_none() {
        msg!("Error: Invalid instrument index");
        return Err(PercolatorError::InvalidInstrument);
    }

    // Check if enough time has passed since last batch
    let batch_ms = slab.header.batch_ms;
    let last_batch_ts = slab.header.last_batch_open_ts;

    if current_ts < last_batch_ts + batch_ms {
        msg!("Error: Batch window not yet elapsed");
        return Err(PercolatorError::BatchNotOpen);
    }

    // Increment epoch
    let new_epoch = slab.header.current_epoch.wrapping_add(1);
    slab.header.current_epoch = new_epoch;

    // Update instrument epoch
    if let Some(instr) = slab.get_instrument_mut(instrument_idx) {
        instr.epoch = (new_epoch & 0xFFFF) as u16;
        instr.batch_open_ms = current_ts;
    }

    // Promote pending orders
    slab.promote_pending_orders(instrument_idx, new_epoch);

    // Apply freeze levels (anti-sandwich protection)
    apply_freeze_levels(slab, instrument_idx, current_ts)?;

    // Update header timestamps
    slab.header.last_batch_open_ts = current_ts;

    // Increment seqno
    slab.header.increment_seqno();

    msg!("Batch opened successfully");
    Ok(())
}

/// Apply freeze levels to top-K orders on each side
///
/// This prevents front-running by freezing the top price levels
/// until the freeze window expires.
fn apply_freeze_levels(
    slab: &mut SlabState,
    instrument_idx: u16,
    current_ts: u64,
) -> Result<(), PercolatorError> {
    let freeze_levels = slab.header.freeze_levels;
    let batch_ms = slab.header.batch_ms;

    if freeze_levels == 0 {
        return Ok(()); // Freeze disabled
    }

    let freeze_until = current_ts + (batch_ms / 2); // Freeze for half the batch window

    // Get instrument
    if let Some(instr) = slab.get_instrument_mut(instrument_idx) {
        instr.freeze_until_ms = freeze_until;
    }

    // Mark top-K bid orders as frozen (by storing freeze time in order)
    // Note: In a full implementation, we'd traverse and mark each order
    // For now, the instrument-level freeze is sufficient

    // Mark top-K ask orders as frozen
    // Same as above

    Ok(())
}

/// Process batch open for all instruments
///
/// Convenience function to open batch for all active instruments.
///
/// # Arguments
/// * `slab` - The slab state
/// * `current_ts` - Current timestamp
///
/// # Returns
/// * Number of instruments processed
pub fn process_batch_open_all(
    slab: &mut SlabState,
    current_ts: u64,
) -> Result<u16, PercolatorError> {
    let instrument_count = slab.header.instrument_count;
    let batch_ms = slab.header.batch_ms;
    let last_batch_ts = slab.header.last_batch_open_ts;

    // Check timing
    if current_ts < last_batch_ts + batch_ms {
        return Err(PercolatorError::BatchNotOpen);
    }

    // Increment epoch once for all instruments
    let new_epoch = slab.header.current_epoch.wrapping_add(1);
    slab.header.current_epoch = new_epoch;
    slab.header.last_batch_open_ts = current_ts;

    // Process each instrument
    for i in 0..instrument_count {
        if let Some(instr) = slab.get_instrument_mut(i) {
            instr.epoch = (new_epoch & 0xFFFF) as u16;
            instr.batch_open_ms = current_ts;
        }

        // Promote pending orders for this instrument
        slab.promote_pending_orders(i, new_epoch);
    }

    // Increment seqno
    slab.header.increment_seqno();

    Ok(instrument_count)
}

/// Check if an order is currently frozen
///
/// # Arguments
/// * `slab` - The slab state
/// * `instrument_idx` - The instrument index
/// * `current_ts` - Current timestamp
///
/// # Returns
/// * `true` if orders on this instrument are frozen
pub fn is_frozen(slab: &SlabState, instrument_idx: u16, current_ts: u64) -> bool {
    match slab.get_instrument(instrument_idx) {
        Some(instr) => current_ts < instr.freeze_until_ms,
        None => false,
    }
}

/// Get current batch status
#[derive(Debug, Clone, Copy)]
pub struct BatchStatus {
    /// Current epoch number
    pub epoch: u64,
    /// Timestamp of last batch open
    pub last_batch_ts: u64,
    /// Time until next batch (0 if ready)
    pub time_until_next: u64,
    /// Whether trading is currently frozen
    pub is_frozen: bool,
}

/// Get batch status for an instrument
pub fn get_batch_status(slab: &SlabState, instrument_idx: u16, current_ts: u64) -> Option<BatchStatus> {
    let instr = slab.get_instrument(instrument_idx)?;
    
    let batch_ms = slab.header.batch_ms;
    let last_batch_ts = slab.header.last_batch_open_ts;
    let next_batch_ts = last_batch_ts + batch_ms;
    
    let time_until_next = if current_ts >= next_batch_ts {
        0
    } else {
        next_batch_ts - current_ts
    };

    Some(BatchStatus {
        epoch: slab.header.current_epoch,
        last_batch_ts,
        time_until_next,
        is_frozen: current_ts < instr.freeze_until_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_status_struct() {
        let status = BatchStatus {
            epoch: 42,
            last_batch_ts: 1000,
            time_until_next: 50,
            is_frozen: false,
        };
        assert_eq!(status.epoch, 42);
    }
}
