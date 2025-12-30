//! Initialize instruction - initialize slab state with full pools

use crate::pda::derive_slab_pda;
use crate::state::{SlabHeader, SlabState};
use percolator_common::*;
use pinocchio::{account_info::AccountInfo, msg, pubkey::Pubkey};

/// Process initialize instruction for slab
///
/// Initializes the slab state account with header, pools, and quote cache.
/// This is called once during slab deployment for each market.
///
/// # Arguments
/// * `program_id` - The slab program ID
/// * `slab_account` - The slab account to initialize (must be PDA)
/// * `market_id` - Unique market identifier (32 bytes)
/// * `lp_owner` - LP owner pubkey
/// * `router_id` - Router program ID
/// * `imr_bps` - Initial margin ratio (basis points)
/// * `mmr_bps` - Maintenance margin ratio (basis points)
/// * `maker_fee_bps` - Maker fee (signed, can be negative for rebates)
/// * `taker_fee_bps` - Taker fee (basis points)
/// * `batch_ms` - Batch window in milliseconds
pub fn process_initialize_slab(
    program_id: &Pubkey,
    slab_account: &AccountInfo,
    market_id: [u8; 32],
    lp_owner: Pubkey,
    router_id: Pubkey,
    imr_bps: u64,
    mmr_bps: u64,
    maker_fee_bps: i64,
    taker_fee_bps: u64,
    batch_ms: u64,
) -> Result<(), PercolatorError> {
    // Derive and verify slab PDA
    let (expected_pda, bump) = derive_slab_pda(&market_id, program_id);

    if slab_account.key() != &expected_pda {
        msg!("Error: Slab account is not the correct PDA");
        return Err(PercolatorError::InvalidAccount);
    }

    // Verify account has enough space
    let data = slab_account.try_borrow_data()
        .map_err(|_| PercolatorError::InvalidAccount)?;

    if data.len() < SlabState::LEN {
        msg!("Error: Slab account has insufficient size");
        return Err(PercolatorError::InvalidAccount);
    }

    // Check if already initialized (magic bytes should not match)
    if data.len() >= 8 && &data[0..8] == SlabHeader::MAGIC {
        msg!("Error: Slab account already initialized");
        return Err(PercolatorError::InvalidAccount);
    }

    drop(data);

    // Initialize the slab state
    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // Initialize header with parameters
    slab.header = SlabHeader::new(
        *program_id,
        lp_owner,
        router_id,
        imr_bps,
        mmr_bps,
        maker_fee_bps,
        taker_fee_bps,
        batch_ms,
        bump,
    );

    // Initialize all pools with freelists
    slab.initialize_pools();

    msg!("Slab initialized successfully");
    Ok(())
}

#[cfg(test)]
#[path = "initialize_test.rs"]
mod initialize_test;
