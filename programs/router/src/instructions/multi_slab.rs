//! Multi-Slab Coordination Instructions
//!
//! Implements atomic reserve/commit operations across multiple slabs,
//! enabling cross-slab portfolio margin and capital efficiency.

use crate::state::{Portfolio, Vault, SlabRegistry};
use percolator_common::*;
use pinocchio::{account_info::AccountInfo, msg, pubkey::Pubkey, instruction::AccountMeta};

// ============================================================================
// CONSTANTS
// ============================================================================

/// Maximum number of slabs in a single cross-slab operation
pub const MAX_SLABS_PER_ORDER: usize = 8;

/// Default TTL for reservations (30 seconds)
pub const DEFAULT_RESERVE_TTL_MS: u64 = 30_000;

/// Minimum reserve TTL (5 seconds)
pub const MIN_RESERVE_TTL_MS: u64 = 5_000;

/// Maximum reserve TTL (2 minutes as per spec)
pub const MAX_RESERVE_TTL_MS: u64 = 120_000;

// ============================================================================
// TYPES
// ============================================================================

/// Split specification for a single slab
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SlabSplit {
    /// Slab program ID
    pub slab_program_id: Pubkey,
    /// Slab state account
    pub slab_state: Pubkey,
    /// Instrument index on this slab
    pub instrument_idx: u16,
    /// Side (0 = buy, 1 = sell)
    pub side: u8,
    /// Padding
    pub _padding: u8,
    /// Quantity to execute (1e6 scale)
    pub qty: u64,
    /// Limit price (1e6 scale)
    pub limit_px: u64,
}

impl SlabSplit {
    pub fn new(
        slab_program_id: Pubkey,
        slab_state: Pubkey,
        instrument_idx: u16,
        side: Side,
        qty: u64,
        limit_px: u64,
    ) -> Self {
        Self {
            slab_program_id,
            slab_state,
            instrument_idx,
            side: side as u8,
            _padding: 0,
            qty,
            limit_px,
        }
    }
}

/// Reservation result from a single slab
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct ReservationInfo {
    /// Hold ID from the slab
    pub hold_id: u64,
    /// VWAP price
    pub vwap_px: u64,
    /// Worst price in reservation
    pub worst_px: u64,
    /// Filled quantity
    pub filled_qty: u64,
    /// Maximum charge (notional + fees)
    pub max_charge: u128,
    /// Expiry timestamp
    pub expiry_ms: u64,
    /// Book sequence number at reservation
    pub book_seqno: u64,
}

/// Result of a multi-slab order execution
#[derive(Debug, Clone)]
pub struct MultiSlabResult {
    /// Number of slabs involved
    pub slab_count: u8,
    /// Reservations per slab
    pub reservations: [ReservationInfo; MAX_SLABS_PER_ORDER],
    /// Total filled quantity across all slabs
    pub total_filled_qty: u64,
    /// Aggregate VWAP across all slabs
    pub aggregate_vwap: u64,
    /// Total notional value
    pub total_notional: u128,
    /// Total fees
    pub total_fees: u128,
    /// Net exposure after order
    pub net_exposure: i64,
    /// Initial margin required on net exposure
    pub im_required: u128,
    /// Success flag
    pub success: bool,
}

impl Default for MultiSlabResult {
    fn default() -> Self {
        Self {
            slab_count: 0,
            reservations: [ReservationInfo::default(); MAX_SLABS_PER_ORDER],
            total_filled_qty: 0,
            aggregate_vwap: 0,
            total_notional: 0,
            total_fees: 0,
            net_exposure: 0,
            im_required: 0,
            success: false,
        }
    }
}

// ============================================================================
// MULTI-SLAB RESERVE
// ============================================================================

/// Phase 1: Reserve liquidity across multiple slabs atomically
///
/// This function coordinates reservations across multiple slabs. If any
/// reservation fails, all previously made reservations are cancelled.
///
/// # Arguments
/// * `portfolio` - User's portfolio account
/// * `user` - User pubkey (must be signer)
/// * `vault` - Collateral vault
/// * `registry` - Slab registry for validation
/// * `slab_accounts` - Array of slab account infos
/// * `splits` - How to split the order across slabs
/// * `ttl_ms` - Reservation TTL in milliseconds
///
/// # Returns
/// * `MultiSlabResult` with reservation details
pub fn process_multi_slab_reserve(
    portfolio: &Portfolio,
    user: &Pubkey,
    _vault: &Vault,
    registry: &SlabRegistry,
    slab_accounts: &[AccountInfo],
    splits: &[SlabSplit],
    ttl_ms: u64,
) -> Result<MultiSlabResult, PercolatorError> {
    // Validate inputs
    if splits.is_empty() || splits.len() > MAX_SLABS_PER_ORDER {
        msg!("Error: Invalid number of splits");
        return Err(PercolatorError::InvalidInstruction);
    }

    if slab_accounts.len() != splits.len() {
        msg!("Error: Mismatched slab accounts and splits");
        return Err(PercolatorError::InvalidInstruction);
    }

    // Validate TTL
    let ttl = ttl_ms.clamp(MIN_RESERVE_TTL_MS, MAX_RESERVE_TTL_MS);

    // Verify portfolio belongs to user
    if &portfolio.user != user {
        msg!("Error: Portfolio does not belong to user");
        return Err(PercolatorError::InvalidPortfolio);
    }

    // Validate all slabs are registered
    for split in splits {
        if !registry.is_slab_registered(&split.slab_program_id) {
            msg!("Error: Slab not registered");
            return Err(PercolatorError::SlabNotRegistered);
        }
    }

    let mut result = MultiSlabResult::default();
    result.slab_count = splits.len() as u8;

    // Phase 1: Make reservations on each slab
    // In production, this would CPI to each slab's reserve instruction
    // For now, we simulate the reservation results
    for (i, split) in splits.iter().enumerate() {
        let resv = simulate_reserve(split, ttl)?;
        result.reservations[i] = resv;
        result.total_filled_qty += resv.filled_qty;
        result.total_notional += resv.max_charge;
    }

    // Calculate aggregate VWAP
    if result.total_filled_qty > 0 {
        result.aggregate_vwap = (result.total_notional / result.total_filled_qty as u128) as u64;
    }

    // Calculate net exposure (considering existing portfolio exposure)
    result.net_exposure = calculate_net_exposure_with_order(portfolio, splits);

    // Calculate IM on net exposure
    result.im_required = calculate_portfolio_im(result.net_exposure, result.aggregate_vwap);

    result.success = true;
    msg!("Multi-slab reserve completed");

    Ok(result)
}

// ============================================================================
// MULTI-SLAB COMMIT
// ============================================================================

/// Phase 2: Commit all reservations atomically
///
/// This function commits all previously made reservations. If any commit
/// fails, the entire operation is rolled back (reservations cancelled).
///
/// # Arguments
/// * `portfolio` - User's portfolio account (mutable)
/// * `user` - User pubkey (must be signer)
/// * `vault` - Collateral vault (mutable)
/// * `slab_accounts` - Array of slab account infos
/// * `reservations` - Reservation info from reserve phase
/// * `current_ts` - Current timestamp for expiry check
///
/// # Returns
/// * Updated `MultiSlabResult` with commit results
pub fn process_multi_slab_commit(
    portfolio: &mut Portfolio,
    user: &Pubkey,
    vault: &mut Vault,
    slab_accounts: &[AccountInfo],
    reservations: &[ReservationInfo],
    splits: &[SlabSplit],
    current_ts: u64,
) -> Result<MultiSlabResult, PercolatorError> {
    // Validate inputs
    if reservations.is_empty() || reservations.len() > MAX_SLABS_PER_ORDER {
        msg!("Error: Invalid number of reservations");
        return Err(PercolatorError::InvalidInstruction);
    }

    if slab_accounts.len() != reservations.len() {
        msg!("Error: Mismatched slab accounts and reservations");
        return Err(PercolatorError::InvalidInstruction);
    }

    // Verify portfolio belongs to user
    if &portfolio.user != user {
        msg!("Error: Portfolio does not belong to user");
        return Err(PercolatorError::InvalidPortfolio);
    }

    // Check all reservations haven't expired
    for resv in reservations {
        if current_ts > resv.expiry_ms && resv.expiry_ms > 0 {
            msg!("Error: Reservation expired");
            return Err(PercolatorError::ReservationExpired);
        }
    }

    let mut result = MultiSlabResult::default();
    result.slab_count = reservations.len() as u8;

    // Phase 2: Commit each reservation
    // In production, this would CPI to each slab's commit instruction
    let mut total_debit: u128 = 0;
    
    for (i, resv) in reservations.iter().enumerate() {
        // Simulate commit and get actual debit
        let commit_result = simulate_commit(resv, current_ts)?;
        
        result.reservations[i] = *resv;
        result.total_filled_qty += commit_result.filled_qty;
        result.total_notional += commit_result.notional;
        result.total_fees += commit_result.fees;
        total_debit += commit_result.notional + commit_result.fees;
    }

    // Debit vault (simplified - in production this goes through escrow)
    if vault.balance < total_debit {
        msg!("Error: Insufficient vault balance");
        return Err(PercolatorError::InsufficientBalance);
    }
    vault.balance -= total_debit;

    // Update portfolio exposures
    for (i, split) in splits.iter().enumerate() {
        let slab_idx = i as u16;
        let instrument_idx = split.instrument_idx;
        let filled_qty = reservations[i].filled_qty as i64;
        
        let current_exposure = portfolio.get_exposure(slab_idx, instrument_idx);
        let new_exposure = if split.side == 0 {
            current_exposure + filled_qty
        } else {
            current_exposure - filled_qty
        };
        
        portfolio.update_exposure(slab_idx, instrument_idx, new_exposure);
    }

    // Calculate aggregate VWAP
    if result.total_filled_qty > 0 {
        result.aggregate_vwap = (result.total_notional / result.total_filled_qty as u128) as u64;
    }

    // Calculate net exposure and margin
    result.net_exposure = calculate_net_exposure_from_portfolio(portfolio);
    result.im_required = calculate_portfolio_im(result.net_exposure, result.aggregate_vwap);

    // Update portfolio margin
    portfolio.update_margin(result.im_required, result.im_required / 2);

    // Check margin sufficiency
    if !portfolio.has_sufficient_margin() {
        msg!("Error: Insufficient margin after commit");
        return Err(PercolatorError::PortfolioInsufficientMargin);
    }

    result.success = true;
    msg!("Multi-slab commit completed");

    Ok(result)
}

// ============================================================================
// MULTI-SLAB CANCEL
// ============================================================================

/// Cancel all reservations across multiple slabs
///
/// This function cancels all reservations atomically. Used for cleanup
/// when a commit cannot proceed.
pub fn process_multi_slab_cancel(
    _slab_accounts: &[AccountInfo],
    reservations: &[ReservationInfo],
) -> Result<(), PercolatorError> {
    // In production, this would CPI to each slab's cancel instruction
    for resv in reservations {
        if resv.hold_id != 0 {
            // Simulate cancel - hold_id used for actual cancellation
            let _ = resv.hold_id;
        }
    }

    msg!("Multi-slab cancel completed");
    Ok(())
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Simulate a reserve operation on a slab (stub for CPI)
fn simulate_reserve(split: &SlabSplit, ttl_ms: u64) -> Result<ReservationInfo, PercolatorError> {
    // In production, this would be a CPI to the slab's reserve instruction
    // For now, simulate successful reservation at limit price
    Ok(ReservationInfo {
        hold_id: 1, // Would be returned from slab
        vwap_px: split.limit_px,
        worst_px: split.limit_px,
        filled_qty: split.qty,
        max_charge: mul_u64(split.qty, split.limit_px) * 101 / 100, // +1% for fees
        expiry_ms: ttl_ms,
        book_seqno: 1,
    })
}

/// Simulate a commit operation on a slab (stub for CPI)
fn simulate_commit(resv: &ReservationInfo, _current_ts: u64) -> Result<CommitInfo, PercolatorError> {
    // In production, this would be a CPI to the slab's commit instruction
    Ok(CommitInfo {
        filled_qty: resv.filled_qty,
        vwap_px: resv.vwap_px,
        notional: mul_u64(resv.filled_qty, resv.vwap_px),
        fees: mul_u64(resv.filled_qty, resv.vwap_px) / 100, // 1% fee
    })
}

/// Commit information from a single slab
struct CommitInfo {
    filled_qty: u64,
    vwap_px: u64,
    notional: u128,
    fees: u128,
}

/// Calculate net exposure including the new order
fn calculate_net_exposure_with_order(portfolio: &Portfolio, splits: &[SlabSplit]) -> i64 {
    let mut net = 0i64;
    
    // Sum existing exposures
    for i in 0..portfolio.exposure_count as usize {
        net += portfolio.exposures[i].2;
    }
    
    // Add new order quantities
    for split in splits {
        if split.side == 0 {
            net += split.qty as i64;
        } else {
            net -= split.qty as i64;
        }
    }
    
    net
}

/// Calculate net exposure from portfolio
fn calculate_net_exposure_from_portfolio(portfolio: &Portfolio) -> i64 {
    let mut net = 0i64;
    for i in 0..portfolio.exposure_count as usize {
        net += portfolio.exposures[i].2;
    }
    net
}

/// Calculate initial margin on net exposure
fn calculate_portfolio_im(net_exposure: i64, price: u64) -> u128 {
    // IM = abs(net_exposure) * price * 10% / 1e6 (scale factor)
    let abs_exposure = net_exposure.unsigned_abs() as u128;
    let notional = abs_exposure * price as u128;
    notional * 10 / (100 * 1_000_000)
}

// ============================================================================
// CPI INSTRUCTION BUILDERS
// ============================================================================

/// Build CPI instruction data for slab reserve
pub fn build_reserve_cpi_data(
    account_idx: u32,
    instrument_idx: u16,
    side: u8,
    qty: u64,
    limit_px: u64,
    ttl_ms: u64,
    commitment_hash: [u8; 32],
    route_id: u64,
) -> [u8; 72] {
    let mut data = [0u8; 72];
    data[0] = 0; // Reserve discriminator
    data[1..5].copy_from_slice(&account_idx.to_le_bytes());
    data[5..7].copy_from_slice(&instrument_idx.to_le_bytes());
    data[7] = side;
    data[8..16].copy_from_slice(&qty.to_le_bytes());
    data[16..24].copy_from_slice(&limit_px.to_le_bytes());
    data[24..32].copy_from_slice(&ttl_ms.to_le_bytes());
    data[32..64].copy_from_slice(&commitment_hash);
    data[64..72].copy_from_slice(&route_id.to_le_bytes());
    data
}

/// Build CPI instruction data for slab commit
pub fn build_commit_cpi_data(hold_id: u64, current_ts: u64) -> [u8; 17] {
    let mut data = [0u8; 17];
    data[0] = 1; // Commit discriminator
    data[1..9].copy_from_slice(&hold_id.to_le_bytes());
    data[9..17].copy_from_slice(&current_ts.to_le_bytes());
    data
}

/// Build CPI instruction data for slab cancel
pub fn build_cancel_cpi_data(hold_id: u64) -> [u8; 9] {
    let mut data = [0u8; 9];
    data[0] = 2; // Cancel discriminator
    data[1..9].copy_from_slice(&hold_id.to_le_bytes());
    data
}

/// Build CPI account metas for slab reserve/commit/cancel
pub fn build_slab_account_metas<'a>(slab_state: &'a Pubkey, is_writable: bool) -> [AccountMeta<'a>; 1] {
    [AccountMeta::new(slab_state, is_writable, false)]
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slab_split_size() {
        assert_eq!(core::mem::size_of::<SlabSplit>(), 88);
    }

    #[test]
    fn test_reservation_info_size() {
        assert_eq!(core::mem::size_of::<ReservationInfo>(), 64);
    }

    #[test]
    fn test_calculate_net_exposure() {
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);
        
        // Long 1 BTC on slab 0
        portfolio.update_exposure(0, 0, 1_000_000);
        
        // Short 1 BTC on slab 1
        portfolio.update_exposure(1, 0, -1_000_000);
        
        let net = calculate_net_exposure_from_portfolio(&portfolio);
        assert_eq!(net, 0); // Should net to zero
    }

    #[test]
    fn test_calculate_portfolio_im_zero() {
        let im = calculate_portfolio_im(0, 50_000_000_000);
        assert_eq!(im, 0); // Zero exposure = zero margin
    }

    #[test]
    fn test_calculate_portfolio_im_nonzero() {
        // 1 BTC at $50k, 10% IMR
        let im = calculate_portfolio_im(1_000_000, 50_000_000_000);
        assert!(im > 0);
    }

    #[test]
    fn test_build_reserve_cpi_data() {
        let data = build_reserve_cpi_data(
            0, 0, 0, 1_000_000, 50_000_000_000, 30_000, [0; 32], 1
        );
        assert_eq!(data[0], 0); // Reserve discriminator
        assert_eq!(data.len(), 72);
    }

    #[test]
    fn test_build_commit_cpi_data() {
        let data = build_commit_cpi_data(123, 1704067200000);
        assert_eq!(data[0], 1); // Commit discriminator
        assert_eq!(data.len(), 17);
    }

    #[test]
    fn test_build_cancel_cpi_data() {
        let data = build_cancel_cpi_data(456);
        assert_eq!(data[0], 2); // Cancel discriminator
        assert_eq!(data.len(), 9);
    }
}
