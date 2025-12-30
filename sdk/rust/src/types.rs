//! Protocol types and account structures

use borsh::{BorshDeserialize, BorshSerialize};
use solana_sdk::pubkey::Pubkey;
use crate::constants::*;

// ============================================================================
// ACCOUNT TYPES
// ============================================================================

/// User portfolio account
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct UserPortfolio {
    /// Owner pubkey
    pub owner: Pubkey,
    /// Portfolio ID
    pub portfolio_id: u64,
    /// USDC collateral balance
    pub collateral_balance: u64,
    /// Initial margin used
    pub initial_margin_used: u64,
    /// Maintenance margin used
    pub maintenance_margin_used: u64,
    /// Unrealized PnL
    pub unrealized_pnl: i64,
    /// Realized PnL
    pub realized_pnl: i64,
    /// Active positions (simplified)
    pub position_count: u8,
    /// Bump seed
    pub bump: u8,
}

/// Position info
#[derive(Debug, Clone, Copy, Default, BorshSerialize, BorshDeserialize)]
pub struct PositionInfo {
    /// Slab index
    pub slab_index: u8,
    /// Instrument index
    pub instrument_index: u8,
    /// Signed quantity (positive = long, negative = short)
    pub qty: i64,
    /// Entry price (scaled)
    pub entry_price: u64,
    /// Entry value
    pub entry_value: u64,
    /// Last mark price
    pub last_mark_price: u64,
    /// Unrealized PnL
    pub unrealized_pnl: i64,
}

/// Slab header (simplified for SDK)
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct SlabHeader {
    /// Magic bytes
    pub magic: [u8; 8],
    /// Version
    pub version: u32,
    /// Sequence number
    pub seqno: u32,
    /// Slab program ID
    pub program_id: Pubkey,
    /// LP owner
    pub lp_owner: Pubkey,
    /// Router ID
    pub router_id: Pubkey,
    /// Initial margin ratio (bps)
    pub imr_bps: u64,
    /// Maintenance margin ratio (bps)
    pub mmr_bps: u64,
    /// Maker fee (bps, signed)
    pub maker_fee_bps: i64,
    /// Taker fee (bps)
    pub taker_fee_bps: u64,
    /// Batch window (ms)
    pub batch_ms: u64,
    /// Kill band (bps)
    pub kill_band_bps: u64,
    /// Freeze levels
    pub freeze_levels: u16,
    /// JIT penalty enabled
    pub jit_penalty_on: bool,
    /// Mark price
    pub mark_px: i64,
    /// Instrument count
    pub instrument_count: u16,
    /// Order count
    pub order_count: u32,
}

/// Insurance pool state
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct InsurancePool {
    /// Current balance
    pub balance: u128,
    /// Target balance
    pub target_balance: u128,
    /// Contribution rate (bps)
    pub contribution_rate_bps: u64,
    /// ADL threshold (bps)
    pub adl_threshold_bps: u64,
    /// Withdrawal timelock (seconds)
    pub withdrawal_timelock_secs: u64,
    /// Pending withdrawal amount
    pub pending_withdrawal: u128,
    /// Pending withdrawal unlock timestamp
    pub pending_withdrawal_unlock_ts: u64,
    /// LP owner
    pub lp_owner: Pubkey,
    /// Total open interest
    pub total_open_interest: u128,
}

/// Insurance statistics
#[derive(Debug, Clone, Default, BorshSerialize, BorshDeserialize)]
pub struct InsuranceStats {
    /// Total contributions
    pub total_contributions: u128,
    /// Total payouts
    pub total_payouts: u128,
    /// ADL events count
    pub adl_events: u64,
    /// Shortfall events count
    pub shortfall_events: u64,
    /// Maximum single payout
    pub max_single_payout: u128,
    /// Last contribution timestamp
    pub last_contribution_ts: u64,
    /// Last payout timestamp
    pub last_payout_ts: u64,
}

// ============================================================================
// ORDER TYPES
// ============================================================================

/// Order parameters for placing orders
#[derive(Debug, Clone, Default)]
pub struct OrderParams {
    /// Instrument index
    pub instrument_index: u8,
    /// Order side
    pub side: Side,
    /// Limit price (scaled by PRICE_SCALE)
    pub price: u64,
    /// Quantity (scaled by QTY_SCALE)
    pub qty: u64,
    /// Time in force
    pub time_in_force: TimeInForce,
    /// Client order ID (optional)
    pub client_order_id: Option<u64>,
    /// Reduce only flag
    pub reduce_only: bool,
}

/// Slab split for multi-slab operations
#[derive(Debug, Clone, Default)]
pub struct SlabSplit {
    /// Slab index
    pub slab_index: u8,
    /// Instrument index
    pub instrument_index: u8,
    /// Quantity for this slab
    pub qty: u64,
    /// Limit price
    pub limit_price: u64,
}

/// Multi-slab reserve parameters
#[derive(Debug, Clone, Default)]
pub struct MultiSlabReserveParams {
    /// Splits per slab
    pub splits: Vec<SlabSplit>,
    /// Total quantity
    pub total_qty: u64,
    /// Request ID
    pub request_id: u64,
    /// Expiry timestamp
    pub expiry_ts: u64,
}

/// Deposit parameters
#[derive(Debug, Clone)]
pub struct DepositParams {
    /// Amount to deposit (in USDC, 6 decimals)
    pub amount: u64,
}

/// Withdraw parameters
#[derive(Debug, Clone)]
pub struct WithdrawParams {
    /// Amount to withdraw (in USDC, 6 decimals)
    pub amount: u64,
}

/// Insurance initialization parameters
#[derive(Debug, Clone)]
pub struct InitializeInsuranceParams {
    /// Contribution rate (bps)
    pub contribution_rate_bps: u64,
    /// ADL threshold (bps)
    pub adl_threshold_bps: u64,
    /// Withdrawal timelock (seconds)
    pub withdrawal_timelock_secs: u64,
}

/// Insurance contribution parameters
#[derive(Debug, Clone)]
pub struct ContributeInsuranceParams {
    /// Amount to contribute
    pub amount: u64,
}

/// Insurance withdrawal parameters
#[derive(Debug, Clone)]
pub struct InitiateWithdrawalParams {
    /// Amount to withdraw
    pub amount: u64,
}

// ============================================================================
// RESPONSE TYPES
// ============================================================================

/// Reserve response
#[derive(Debug, Clone)]
pub struct ReserveResponse {
    /// Hold ID
    pub hold_id: u64,
    /// Reserved quantity
    pub reserved_qty: u64,
    /// Reserved price
    pub reserved_price: u64,
    /// Expiry timestamp
    pub expiry_ts: u64,
}

/// Commit response
#[derive(Debug, Clone)]
pub struct CommitResponse {
    /// Filled quantity
    pub filled_qty: u64,
    /// Average fill price
    pub avg_price: u64,
    /// Fees paid
    pub fees: u64,
    /// New position size
    pub new_position_size: i64,
}

/// Portfolio margin result
#[derive(Debug, Clone, Default)]
pub struct PortfolioMarginResult {
    /// Gross initial margin
    pub gross_im: u64,
    /// Net initial margin (after netting)
    pub net_im: u64,
    /// Gross maintenance margin
    pub gross_mm: u64,
    /// Net maintenance margin
    pub net_mm: u64,
    /// Netting benefit
    pub netting_benefit: u64,
    /// Available margin
    pub available_margin: i64,
    /// Margin ratio (bps)
    pub margin_ratio_bps: u64,
}
