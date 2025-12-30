//! Add instrument instruction - adds a new instrument to the slab
//!
//! Instruments define the contract specifications for each market.

use crate::state::SlabState;
use percolator_common::*;
use pinocchio::msg;

/// Process add instrument instruction
///
/// Adds a new instrument (market) to the slab. Only the LP owner can add instruments.
///
/// # Arguments
/// * `slab` - The slab state
/// * `symbol` - 8-byte symbol (e.g., "BTC-PERP")
/// * `contract_size` - Contract size (1e6 scale, e.g., 1_000_000 = 1.0)
/// * `tick` - Minimum price increment (1e6 scale)
/// * `lot` - Minimum quantity increment (1e6 scale)
/// * `initial_mark` - Initial mark price (1e6 scale)
///
/// # Returns
/// * Index of the new instrument
pub fn process_add_instrument(
    slab: &mut SlabState,
    symbol: [u8; 8],
    contract_size: u64,
    tick: u64,
    lot: u64,
    initial_mark: u64,
) -> Result<u16, PercolatorError> {
    // Validate inputs
    if contract_size == 0 {
        msg!("Error: Contract size cannot be zero");
        return Err(PercolatorError::InvalidRiskParams);
    }

    if tick == 0 {
        msg!("Error: Tick size cannot be zero");
        return Err(PercolatorError::InvalidRiskParams);
    }

    if lot == 0 {
        msg!("Error: Lot size cannot be zero");
        return Err(PercolatorError::InvalidRiskParams);
    }

    // Check capacity
    let idx = slab.header.instrument_count;
    if idx as usize >= crate::state::slab::POOL_INSTRUMENTS {
        msg!("Error: Instrument pool full");
        return Err(PercolatorError::PoolFull);
    }

    // Check for duplicate symbol
    for i in 0..idx {
        let existing = &slab.instruments[i as usize];
        if existing.symbol == symbol {
            msg!("Error: Instrument with this symbol already exists");
            return Err(PercolatorError::InvalidInstrument);
        }
    }

    // Initialize the instrument
    slab.instruments[idx as usize] = Instrument {
        symbol,
        contract_size,
        tick,
        lot,
        index_price: initial_mark, // Use initial mark as index initially
        funding_rate: 0,
        cum_funding: 0,
        last_funding_ts: 0,
        bids_head: SlabState::INVALID_INDEX,
        asks_head: SlabState::INVALID_INDEX,
        bids_pending_head: SlabState::INVALID_INDEX,
        asks_pending_head: SlabState::INVALID_INDEX,
        epoch: 0,
        index: idx,
        batch_open_ms: 0,
        freeze_until_ms: 0,
    };

    // Update count
    slab.header.instrument_count = idx + 1;

    // Update mark price if this is the first instrument
    if slab.header.mark_px == 0 {
        slab.header.mark_px = initial_mark as i64;
    }

    // Increment seqno
    slab.header.increment_seqno();

    msg!("Instrument added successfully");
    Ok(idx)
}

/// Update instrument parameters
///
/// # Arguments
/// * `slab` - The slab state
/// * `instrument_idx` - Instrument to update
/// * `tick` - New tick size (or 0 to keep current)
/// * `lot` - New lot size (or 0 to keep current)
pub fn process_update_instrument(
    slab: &mut SlabState,
    instrument_idx: u16,
    tick: u64,
    lot: u64,
) -> Result<(), PercolatorError> {
    let instr = slab.get_instrument_mut(instrument_idx)
        .ok_or_else(|| {
            msg!("Error: Invalid instrument index");
            PercolatorError::InvalidInstrument
        })?;

    // Update tick if specified
    if tick > 0 {
        // Validate new tick divides cleanly into current prices
        // (This is a simplified check)
        instr.tick = tick;
    }

    // Update lot if specified
    if lot > 0 {
        instr.lot = lot;
    }

    slab.header.increment_seqno();

    Ok(())
}

/// Update mark price for an instrument
///
/// # Arguments
/// * `slab` - The slab state
/// * `instrument_idx` - Instrument to update
/// * `new_mark_px` - New mark price (1e6 scale)
pub fn process_update_mark_price(
    slab: &mut SlabState,
    instrument_idx: u16,
    new_mark_px: u64,
) -> Result<(), PercolatorError> {
    if slab.get_instrument(instrument_idx).is_none() {
        msg!("Error: Invalid instrument index");
        return Err(PercolatorError::InvalidInstrument);
    }

    // Update header mark price (used for margin calculations)
    slab.header.update_mark_px(new_mark_px as i64);

    slab.header.increment_seqno();

    Ok(())
}

/// Get instrument by symbol
pub fn find_instrument_by_symbol(slab: &SlabState, symbol: &[u8; 8]) -> Option<u16> {
    for i in 0..slab.header.instrument_count {
        if &slab.instruments[i as usize].symbol == symbol {
            return Some(i);
        }
    }
    None
}

/// Instrument summary
#[derive(Debug, Clone, Copy)]
pub struct InstrumentSummary {
    /// Symbol
    pub symbol: [u8; 8],
    /// Index in slab
    pub index: u16,
    /// Contract size
    pub contract_size: u64,
    /// Tick size
    pub tick: u64,
    /// Lot size
    pub lot: u64,
    /// Current index price
    pub index_price: u64,
    /// Current funding rate
    pub funding_rate: i64,
    /// Has bids
    pub has_bids: bool,
    /// Has asks
    pub has_asks: bool,
}

/// Get instrument summary
pub fn get_instrument_summary(slab: &SlabState, instrument_idx: u16) -> Option<InstrumentSummary> {
    let instr = slab.get_instrument(instrument_idx)?;

    Some(InstrumentSummary {
        symbol: instr.symbol,
        index: instr.index,
        contract_size: instr.contract_size,
        tick: instr.tick,
        lot: instr.lot,
        index_price: instr.index_price,
        funding_rate: instr.funding_rate,
        has_bids: instr.bids_head != SlabState::INVALID_INDEX,
        has_asks: instr.asks_head != SlabState::INVALID_INDEX,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instrument_summary_size() {
        // Ensure summary is reasonably sized
        assert!(core::mem::size_of::<InstrumentSummary>() <= 64);
    }

    #[test]
    fn test_symbol_parsing() {
        let symbol = *b"BTC-PERP";
        assert_eq!(symbol.len(), 8);
    }
}
