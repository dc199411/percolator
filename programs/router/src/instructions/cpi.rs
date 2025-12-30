//! CPI Integration - Cross-Program Invocation between Router and Slab
//!
//! Production-ready CPI interface for the Router to call Slab program
//! instructions (reserve, commit, cancel, liquidation) with proper
//! lifetime handling and actual invoke calls.

use percolator_common::*;
use pinocchio::{
    account_info::AccountInfo,
    cpi::{invoke, get_return_data},
    instruction::{AccountMeta, Instruction},
    msg,
    pubkey::Pubkey,
};

// ============================================================================
// CONSTANTS
// ============================================================================

/// Slab program ID string for validation
pub const SLAB_PROGRAM_ID_STR: &str = "SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk";

/// CPI instruction discriminators for Slab program
pub mod slab_ix {
    pub const RESERVE: u8 = 4;
    pub const COMMIT: u8 = 5;
    pub const CANCEL: u8 = 6;
    pub const LIQUIDATION_CALL: u8 = 9;
}

/// Maximum accounts for CPI calls
pub const MAX_CPI_ACCOUNTS: usize = 16;

/// Reserve instruction data size
pub const RESERVE_IX_DATA_SIZE: usize = 73;

/// Commit instruction data size
pub const COMMIT_IX_DATA_SIZE: usize = 17;

/// Cancel instruction data size
pub const CANCEL_IX_DATA_SIZE: usize = 9;

/// Liquidation instruction data size
pub const LIQUIDATION_IX_DATA_SIZE: usize = 21;

// ============================================================================
// CPI RESPONSE TYPES
// ============================================================================

/// Response from reserve CPI
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct ReserveResponse {
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

impl ReserveResponse {
    /// Parse from CPI return data
    pub fn from_return_data(data: &[u8]) -> Result<Self, PercolatorError> {
        if data.len() < 56 {
            msg!("Error: Reserve return data too short");
            return Err(PercolatorError::CpiError);
        }
        
        Ok(Self {
            hold_id: u64::from_le_bytes(data[0..8].try_into().map_err(|_| PercolatorError::CpiError)?),
            vwap_px: u64::from_le_bytes(data[8..16].try_into().map_err(|_| PercolatorError::CpiError)?),
            worst_px: u64::from_le_bytes(data[16..24].try_into().map_err(|_| PercolatorError::CpiError)?),
            filled_qty: u64::from_le_bytes(data[24..32].try_into().map_err(|_| PercolatorError::CpiError)?),
            max_charge: u128::from_le_bytes(data[32..48].try_into().map_err(|_| PercolatorError::CpiError)?),
            expiry_ms: u64::from_le_bytes(data[48..56].try_into().map_err(|_| PercolatorError::CpiError)?),
            book_seqno: if data.len() >= 64 {
                u64::from_le_bytes(data[56..64].try_into().map_err(|_| PercolatorError::CpiError)?)
            } else {
                0
            },
        })
    }

    /// Serialize to bytes for return data
    pub fn to_bytes(&self) -> [u8; 64] {
        let mut bytes = [0u8; 64];
        bytes[0..8].copy_from_slice(&self.hold_id.to_le_bytes());
        bytes[8..16].copy_from_slice(&self.vwap_px.to_le_bytes());
        bytes[16..24].copy_from_slice(&self.worst_px.to_le_bytes());
        bytes[24..32].copy_from_slice(&self.filled_qty.to_le_bytes());
        bytes[32..48].copy_from_slice(&self.max_charge.to_le_bytes());
        bytes[48..56].copy_from_slice(&self.expiry_ms.to_le_bytes());
        bytes[56..64].copy_from_slice(&self.book_seqno.to_le_bytes());
        bytes
    }
}

/// Response from commit CPI
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct CommitResponse {
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

impl CommitResponse {
    /// Parse from CPI return data
    pub fn from_return_data(data: &[u8]) -> Result<Self, PercolatorError> {
        if data.len() < 64 {
            msg!("Error: Commit return data too short");
            return Err(PercolatorError::CpiError);
        }
        
        Ok(Self {
            filled_qty: u64::from_le_bytes(data[0..8].try_into().map_err(|_| PercolatorError::CpiError)?),
            vwap_px: u64::from_le_bytes(data[8..16].try_into().map_err(|_| PercolatorError::CpiError)?),
            notional: u128::from_le_bytes(data[16..32].try_into().map_err(|_| PercolatorError::CpiError)?),
            fees: u128::from_le_bytes(data[32..48].try_into().map_err(|_| PercolatorError::CpiError)?),
            realized_pnl: i128::from_le_bytes(data[48..64].try_into().map_err(|_| PercolatorError::CpiError)?),
        })
    }

    /// Serialize to bytes for return data
    pub fn to_bytes(&self) -> [u8; 64] {
        let mut bytes = [0u8; 64];
        bytes[0..8].copy_from_slice(&self.filled_qty.to_le_bytes());
        bytes[8..16].copy_from_slice(&self.vwap_px.to_le_bytes());
        bytes[16..32].copy_from_slice(&self.notional.to_le_bytes());
        bytes[32..48].copy_from_slice(&self.fees.to_le_bytes());
        bytes[48..64].copy_from_slice(&self.realized_pnl.to_le_bytes());
        bytes
    }
}

/// Response from liquidation CPI
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct LiquidationResponse {
    /// Quantity liquidated
    pub filled_qty: u64,
    /// Average price of liquidation
    pub avg_price: u64,
    /// Total notional value liquidated
    pub notional: u128,
    /// Remaining deficit after liquidation
    pub remaining_deficit: u128,
}

impl LiquidationResponse {
    /// Parse from CPI return data
    pub fn from_return_data(data: &[u8]) -> Result<Self, PercolatorError> {
        if data.len() < 48 {
            msg!("Error: Liquidation return data too short");
            return Err(PercolatorError::CpiError);
        }
        
        Ok(Self {
            filled_qty: u64::from_le_bytes(data[0..8].try_into().map_err(|_| PercolatorError::CpiError)?),
            avg_price: u64::from_le_bytes(data[8..16].try_into().map_err(|_| PercolatorError::CpiError)?),
            notional: u128::from_le_bytes(data[16..32].try_into().map_err(|_| PercolatorError::CpiError)?),
            remaining_deficit: u128::from_le_bytes(data[32..48].try_into().map_err(|_| PercolatorError::CpiError)?),
        })
    }
}

// ============================================================================
// INSTRUCTION DATA SERIALIZATION
// ============================================================================

/// Serialize reserve instruction data
#[inline]
pub fn serialize_reserve_data(
    account_idx: u32,
    instrument_idx: u16,
    side: Side,
    qty: u64,
    limit_px: u64,
    ttl_ms: u64,
    commitment_hash: &[u8; 32],
    route_id: u64,
) -> [u8; RESERVE_IX_DATA_SIZE] {
    let mut data = [0u8; RESERVE_IX_DATA_SIZE];
    data[0] = slab_ix::RESERVE;
    data[1..5].copy_from_slice(&account_idx.to_le_bytes());
    data[5..7].copy_from_slice(&instrument_idx.to_le_bytes());
    data[7] = side as u8;
    data[8..16].copy_from_slice(&qty.to_le_bytes());
    data[16..24].copy_from_slice(&limit_px.to_le_bytes());
    data[24..32].copy_from_slice(&ttl_ms.to_le_bytes());
    data[32..64].copy_from_slice(commitment_hash);
    data[64..72].copy_from_slice(&route_id.to_le_bytes());
    data
}

/// Serialize commit instruction data
#[inline]
pub fn serialize_commit_data(hold_id: u64, current_ts: u64) -> [u8; COMMIT_IX_DATA_SIZE] {
    let mut data = [0u8; COMMIT_IX_DATA_SIZE];
    data[0] = slab_ix::COMMIT;
    data[1..9].copy_from_slice(&hold_id.to_le_bytes());
    data[9..17].copy_from_slice(&current_ts.to_le_bytes());
    data
}

/// Serialize cancel instruction data
#[inline]
pub fn serialize_cancel_data(hold_id: u64) -> [u8; CANCEL_IX_DATA_SIZE] {
    let mut data = [0u8; CANCEL_IX_DATA_SIZE];
    data[0] = slab_ix::CANCEL;
    data[1..9].copy_from_slice(&hold_id.to_le_bytes());
    data
}

/// Serialize liquidation instruction data
#[inline]
pub fn serialize_liquidation_data(account_idx: u32, deficit: u128) -> [u8; LIQUIDATION_IX_DATA_SIZE] {
    let mut data = [0u8; LIQUIDATION_IX_DATA_SIZE];
    data[0] = slab_ix::LIQUIDATION_CALL;
    data[1..5].copy_from_slice(&account_idx.to_le_bytes());
    data[5..21].copy_from_slice(&deficit.to_le_bytes());
    data
}

// ============================================================================
// CPI EXECUTION - PRODUCTION IMPLEMENTATION
// ============================================================================

/// Execute reserve CPI to slab program
///
/// # Arguments
/// * `slab_program` - Slab program account info
/// * `slab_state` - Slab state account info (writable)
/// * `account_idx` - Account index on slab
/// * `instrument_idx` - Instrument index
/// * `side` - Order side
/// * `qty` - Quantity (1e6 scale)
/// * `limit_px` - Limit price (1e6 scale)
/// * `ttl_ms` - Reservation TTL in milliseconds
/// * `commitment_hash` - Hash for commit-reveal
/// * `route_id` - Route ID from router
///
/// # Returns
/// * `ReserveResponse` with reservation details
pub fn cpi_reserve<'a>(
    slab_program: &'a AccountInfo,
    slab_state: &'a AccountInfo,
    account_idx: u32,
    instrument_idx: u16,
    side: Side,
    qty: u64,
    limit_px: u64,
    ttl_ms: u64,
    commitment_hash: &[u8; 32],
    route_id: u64,
) -> Result<ReserveResponse, PercolatorError> {
    // Build instruction data
    let ix_data = serialize_reserve_data(
        account_idx,
        instrument_idx,
        side,
        qty,
        limit_px,
        ttl_ms,
        commitment_hash,
        route_id,
    );

    // Build account metas - slab_state must be writable
    let account_metas = [AccountMeta::writable(slab_state.key())];

    // Build instruction
    let instruction = Instruction {
        program_id: slab_program.key(),
        accounts: &account_metas,
        data: &ix_data,
    };

    // Execute CPI
    let account_infos = [slab_state];
    invoke::<1>(&instruction, &account_infos)
        .map_err(|_| {
            msg!("Error: Reserve CPI failed");
            PercolatorError::CpiError
        })?;

    // Parse return data
    let return_data = get_return_data()
        .ok_or_else(|| {
            msg!("Error: No return data from reserve");
            PercolatorError::CpiError
        })?;

    // Verify return data is from the slab program
    if return_data.program_id() != slab_program.key() {
        msg!("Error: Return data from wrong program");
        return Err(PercolatorError::CpiError);
    }

    ReserveResponse::from_return_data(return_data.as_slice())
}

/// Execute commit CPI to slab program
///
/// # Arguments
/// * `slab_program` - Slab program account info
/// * `slab_state` - Slab state account info (writable)
/// * `hold_id` - Hold ID from reserve
/// * `current_ts` - Current timestamp for expiry check
///
/// # Returns
/// * `CommitResponse` with execution details
pub fn cpi_commit<'a>(
    slab_program: &'a AccountInfo,
    slab_state: &'a AccountInfo,
    hold_id: u64,
    current_ts: u64,
) -> Result<CommitResponse, PercolatorError> {
    // Build instruction data
    let ix_data = serialize_commit_data(hold_id, current_ts);

    // Build account metas
    let account_metas = [AccountMeta::writable(slab_state.key())];

    // Build instruction
    let instruction = Instruction {
        program_id: slab_program.key(),
        accounts: &account_metas,
        data: &ix_data,
    };

    // Execute CPI
    let account_infos = [slab_state];
    invoke::<1>(&instruction, &account_infos)
        .map_err(|_| {
            msg!("Error: Commit CPI failed");
            PercolatorError::CpiError
        })?;

    // Parse return data
    let return_data = get_return_data()
        .ok_or_else(|| {
            msg!("Error: No return data from commit");
            PercolatorError::CpiError
        })?;

    if return_data.program_id() != slab_program.key() {
        msg!("Error: Return data from wrong program");
        return Err(PercolatorError::CpiError);
    }

    CommitResponse::from_return_data(return_data.as_slice())
}

/// Execute cancel CPI to slab program
///
/// # Arguments
/// * `slab_program` - Slab program account info
/// * `slab_state` - Slab state account info (writable)
/// * `hold_id` - Hold ID to cancel
pub fn cpi_cancel<'a>(
    slab_program: &'a AccountInfo,
    slab_state: &'a AccountInfo,
    hold_id: u64,
) -> Result<(), PercolatorError> {
    // Build instruction data
    let ix_data = serialize_cancel_data(hold_id);

    // Build account metas
    let account_metas = [AccountMeta::writable(slab_state.key())];

    // Build instruction
    let instruction = Instruction {
        program_id: slab_program.key(),
        accounts: &account_metas,
        data: &ix_data,
    };

    // Execute CPI
    let account_infos = [slab_state];
    invoke::<1>(&instruction, &account_infos)
        .map_err(|_| {
            msg!("Error: Cancel CPI failed");
            PercolatorError::CpiError
        })?;

    Ok(())
}

/// Execute liquidation CPI to slab program
///
/// # Arguments
/// * `slab_program` - Slab program account info
/// * `slab_state` - Slab state account info (writable)
/// * `account_idx` - Account index on slab
/// * `deficit` - Target deficit to liquidate
///
/// # Returns
/// * `LiquidationResponse` with liquidation details
pub fn cpi_liquidation<'a>(
    slab_program: &'a AccountInfo,
    slab_state: &'a AccountInfo,
    account_idx: u32,
    deficit: u128,
) -> Result<LiquidationResponse, PercolatorError> {
    // Build instruction data
    let ix_data = serialize_liquidation_data(account_idx, deficit);

    // Build account metas
    let account_metas = [AccountMeta::writable(slab_state.key())];

    // Build instruction
    let instruction = Instruction {
        program_id: slab_program.key(),
        accounts: &account_metas,
        data: &ix_data,
    };

    // Execute CPI
    let account_infos = [slab_state];
    invoke::<1>(&instruction, &account_infos)
        .map_err(|_| {
            msg!("Error: Liquidation CPI failed");
            PercolatorError::CpiError
        })?;

    // Parse return data
    let return_data = get_return_data()
        .ok_or_else(|| {
            msg!("Error: No return data from liquidation");
            PercolatorError::CpiError
        })?;

    if return_data.program_id() != slab_program.key() {
        msg!("Error: Return data from wrong program");
        return Err(PercolatorError::CpiError);
    }

    LiquidationResponse::from_return_data(return_data.as_slice())
}

// ============================================================================
// MULTI-SLAB CPI EXECUTION WITH ATOMICITY
// ============================================================================

/// Parameters for a single slab reservation
#[derive(Debug, Clone, Copy)]
pub struct ReserveParams {
    pub account_idx: u32,
    pub instrument_idx: u16,
    pub side: Side,
    pub qty: u64,
    pub limit_px: u64,
    pub ttl_ms: u64,
    pub commitment_hash: [u8; 32],
    pub route_id: u64,
}

/// Maximum slabs for multi-slab operations
pub const MAX_MULTI_SLAB_COUNT: usize = 8;

/// Result of multi-slab reserve operation
#[derive(Debug, Clone, Copy)]
pub struct MultiReserveResult {
    pub count: u8,
    pub responses: [ReserveResponse; MAX_MULTI_SLAB_COUNT],
}

impl Default for MultiReserveResult {
    fn default() -> Self {
        Self {
            count: 0,
            responses: [ReserveResponse::default(); MAX_MULTI_SLAB_COUNT],
        }
    }
}

/// Result of multi-slab commit operation
#[derive(Debug, Clone, Copy)]
pub struct MultiCommitResult {
    pub count: u8,
    pub responses: [CommitResponse; MAX_MULTI_SLAB_COUNT],
}

impl Default for MultiCommitResult {
    fn default() -> Self {
        Self {
            count: 0,
            responses: [CommitResponse::default(); MAX_MULTI_SLAB_COUNT],
        }
    }
}

/// Execute atomic multi-slab reserve with rollback on failure
///
/// This function executes reserve CPIs to multiple slabs atomically.
/// If any reservation fails, all previous reservations are cancelled.
///
/// # Arguments
/// * `slab_programs` - Slice of slab program account infos
/// * `slab_states` - Slice of slab state account infos
/// * `params` - Reserve parameters for each slab
///
/// # Returns
/// * `MultiReserveResult` on success
pub fn atomic_multi_reserve<'a>(
    slab_programs: &[&'a AccountInfo],
    slab_states: &[&'a AccountInfo],
    params: &[ReserveParams],
) -> Result<MultiReserveResult, PercolatorError> {
    // Validate input lengths
    if slab_programs.len() != slab_states.len() || slab_programs.len() != params.len() {
        msg!("Error: Mismatched input lengths");
        return Err(PercolatorError::InvalidInstruction);
    }

    if slab_programs.is_empty() {
        msg!("Error: Empty slab list");
        return Err(PercolatorError::InvalidInstruction);
    }

    if params.len() > MAX_MULTI_SLAB_COUNT {
        msg!("Error: Too many slabs");
        return Err(PercolatorError::InvalidSlabCount);
    }

    let mut result = MultiReserveResult::default();
    
    // Execute reservations
    for i in 0..params.len() {
        let reserve_result = cpi_reserve(
            slab_programs[i],
            slab_states[i],
            params[i].account_idx,
            params[i].instrument_idx,
            params[i].side,
            params[i].qty,
            params[i].limit_px,
            params[i].ttl_ms,
            &params[i].commitment_hash,
            params[i].route_id,
        );
        
        match reserve_result {
            Ok(r) => {
                result.responses[i] = r;
                result.count += 1;
            }
            Err(e) => {
                // Rollback: Cancel all previous reservations
                msg!("Error: Reserve failed, initiating rollback");
                for j in 0..result.count as usize {
                    let _ = cpi_cancel(slab_programs[j], slab_states[j], result.responses[j].hold_id);
                }
                return Err(e);
            }
        }
    }
    
    Ok(result)
}

/// Execute atomic multi-slab commit
///
/// This function executes commit CPIs to multiple slabs.
/// If any commit fails, remaining reservations are cancelled.
/// Note: Successful commits cannot be rolled back.
///
/// # Arguments
/// * `slab_programs` - Slice of slab program account infos
/// * `slab_states` - Slice of slab state account infos
/// * `reserve_result` - Reserve result from prior reservations
/// * `current_ts` - Current timestamp for expiry check
///
/// # Returns
/// * `MultiCommitResult` on success
pub fn atomic_multi_commit<'a>(
    slab_programs: &[&'a AccountInfo],
    slab_states: &[&'a AccountInfo],
    reserve_result: &MultiReserveResult,
    current_ts: u64,
) -> Result<MultiCommitResult, PercolatorError> {
    let reservation_count = reserve_result.count as usize;
    
    // Validate input lengths
    if slab_programs.len() < reservation_count || slab_states.len() < reservation_count {
        msg!("Error: Not enough accounts for reservations");
        return Err(PercolatorError::InvalidInstruction);
    }

    let mut result = MultiCommitResult::default();
    
    // Execute commits
    for i in 0..reservation_count {
        let commit_result = cpi_commit(
            slab_programs[i],
            slab_states[i],
            reserve_result.responses[i].hold_id,
            current_ts,
        );
        
        match commit_result {
            Ok(r) => {
                result.responses[i] = r;
                result.count += 1;
            }
            Err(e) => {
                // Cancel remaining reservations (commits can't be undone)
                msg!("Error: Commit failed, cancelling remaining");
                for j in (i + 1)..reservation_count {
                    let _ = cpi_cancel(slab_programs[j], slab_states[j], reserve_result.responses[j].hold_id);
                }
                return Err(e);
            }
        }
    }
    
    Ok(result)
}

/// Cancel multiple reservations
///
/// # Arguments
/// * `slab_programs` - Slice of slab program account infos
/// * `slab_states` - Slice of slab state account infos
/// * `hold_ids` - Hold IDs to cancel
pub fn multi_cancel<'a>(
    slab_programs: &[&'a AccountInfo],
    slab_states: &[&'a AccountInfo],
    hold_ids: &[u64],
) -> Result<(), PercolatorError> {
    if slab_programs.len() != slab_states.len() || slab_programs.len() != hold_ids.len() {
        msg!("Error: Mismatched input lengths");
        return Err(PercolatorError::InvalidInstruction);
    }

    for i in 0..hold_ids.len() {
        cpi_cancel(slab_programs[i], slab_states[i], hold_ids[i])?;
    }
    
    Ok(())
}

// ============================================================================
// CPI VALIDATION HELPERS
// ============================================================================

/// Validate slab program ID matches expected
#[inline]
pub fn validate_slab_program(program_id: &Pubkey, expected: &Pubkey) -> Result<(), PercolatorError> {
    if program_id != expected {
        msg!("Error: Invalid slab program ID");
        return Err(PercolatorError::InvalidProgram);
    }
    Ok(())
}

/// Validate account is writable
#[inline]
pub fn validate_writable(account: &AccountInfo) -> Result<(), PercolatorError> {
    if !account.is_writable() {
        msg!("Error: Account must be writable");
        return Err(PercolatorError::InvalidAccount);
    }
    Ok(())
}

/// Validate account owner
#[inline]
pub fn validate_owner(account: &AccountInfo, expected_owner: &Pubkey) -> Result<(), PercolatorError> {
    if account.owner() != expected_owner {
        msg!("Error: Invalid account owner");
        return Err(PercolatorError::InvalidAccount);
    }
    Ok(())
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_reserve_data() {
        let data = serialize_reserve_data(
            0, 0, Side::Buy, 1_000_000, 50_000_000_000, 30_000, &[0; 32], 1
        );
        
        assert_eq!(data[0], slab_ix::RESERVE);
        assert_eq!(data.len(), RESERVE_IX_DATA_SIZE);
        
        // Verify account_idx
        let parsed_account_idx = u32::from_le_bytes(data[1..5].try_into().unwrap());
        assert_eq!(parsed_account_idx, 0);
        
        // Verify qty
        let parsed_qty = u64::from_le_bytes(data[8..16].try_into().unwrap());
        assert_eq!(parsed_qty, 1_000_000);
    }

    #[test]
    fn test_serialize_commit_data() {
        let data = serialize_commit_data(123, 1704067200000);
        
        assert_eq!(data[0], slab_ix::COMMIT);
        assert_eq!(data.len(), COMMIT_IX_DATA_SIZE);
        
        let parsed_hold_id = u64::from_le_bytes(data[1..9].try_into().unwrap());
        assert_eq!(parsed_hold_id, 123);
    }

    #[test]
    fn test_serialize_cancel_data() {
        let data = serialize_cancel_data(456);
        
        assert_eq!(data[0], slab_ix::CANCEL);
        assert_eq!(data.len(), CANCEL_IX_DATA_SIZE);
        
        let parsed_hold_id = u64::from_le_bytes(data[1..9].try_into().unwrap());
        assert_eq!(parsed_hold_id, 456);
    }

    #[test]
    fn test_serialize_liquidation_data() {
        let data = serialize_liquidation_data(0, 1_000_000_000);
        
        assert_eq!(data[0], slab_ix::LIQUIDATION_CALL);
        assert_eq!(data.len(), LIQUIDATION_IX_DATA_SIZE);
    }

    #[test]
    fn test_reserve_response_parsing() {
        let mut data = [0u8; 64];
        data[0..8].copy_from_slice(&1u64.to_le_bytes());
        data[8..16].copy_from_slice(&50_000_000_000u64.to_le_bytes());
        data[16..24].copy_from_slice(&50_100_000_000u64.to_le_bytes());
        data[24..32].copy_from_slice(&1_000_000u64.to_le_bytes());
        data[32..48].copy_from_slice(&50_500_000_000_000u128.to_le_bytes());
        data[48..56].copy_from_slice(&30_000u64.to_le_bytes());
        data[56..64].copy_from_slice(&42u64.to_le_bytes());
        
        let response = ReserveResponse::from_return_data(&data).unwrap();
        
        assert_eq!(response.hold_id, 1);
        assert_eq!(response.vwap_px, 50_000_000_000);
        assert_eq!(response.filled_qty, 1_000_000);
        assert_eq!(response.book_seqno, 42);
    }

    #[test]
    fn test_reserve_response_roundtrip() {
        let original = ReserveResponse {
            hold_id: 123,
            vwap_px: 50_000_000_000,
            worst_px: 50_100_000_000,
            filled_qty: 1_000_000,
            max_charge: 50_500_000_000_000,
            expiry_ms: 30_000,
            book_seqno: 42,
        };
        
        let bytes = original.to_bytes();
        let parsed = ReserveResponse::from_return_data(&bytes).unwrap();
        
        assert_eq!(parsed.hold_id, original.hold_id);
        assert_eq!(parsed.vwap_px, original.vwap_px);
        assert_eq!(parsed.filled_qty, original.filled_qty);
    }

    #[test]
    fn test_commit_response_parsing() {
        let mut data = [0u8; 64];
        data[0..8].copy_from_slice(&1_000_000u64.to_le_bytes());
        data[8..16].copy_from_slice(&50_000_000_000u64.to_le_bytes());
        data[16..32].copy_from_slice(&50_000_000_000_000u128.to_le_bytes());
        data[32..48].copy_from_slice(&50_000_000_000u128.to_le_bytes());
        data[48..64].copy_from_slice(&1_000_000_000i128.to_le_bytes());
        
        let response = CommitResponse::from_return_data(&data).unwrap();
        
        assert_eq!(response.filled_qty, 1_000_000);
        assert_eq!(response.notional, 50_000_000_000_000);
    }

    #[test]
    fn test_response_struct_sizes() {
        assert_eq!(core::mem::size_of::<ReserveResponse>(), 64);
        assert_eq!(core::mem::size_of::<CommitResponse>(), 64);
        assert_eq!(core::mem::size_of::<LiquidationResponse>(), 48);
    }

    #[test]
    fn test_reserve_params_size() {
        assert!(core::mem::size_of::<ReserveParams>() <= 128);
    }
}
