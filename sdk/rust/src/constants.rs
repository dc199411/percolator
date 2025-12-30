//! Protocol constants and program IDs

use solana_sdk::pubkey::Pubkey;

// ============================================================================
// PROGRAM IDS
// ============================================================================

/// Slab Program ID
pub const SLAB_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk");

/// Router Program ID
pub const ROUTER_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr");

// ============================================================================
// PDA SEEDS
// ============================================================================

/// Router registry PDA seed
pub const REGISTRY_SEED: &[u8] = b"registry";

/// Router vault PDA seed
pub const VAULT_SEED: &[u8] = b"vault";

/// User portfolio PDA seed
pub const PORTFOLIO_SEED: &[u8] = b"portfolio";

/// Slab state PDA seed
pub const SLAB_SEED: &[u8] = b"slab";

/// Insurance pool PDA seed
pub const INSURANCE_SEED: &[u8] = b"insurance";

// ============================================================================
// SCALING FACTORS
// ============================================================================

/// Price scale (1e6)
pub const PRICE_SCALE: u64 = 1_000_000;

/// Quantity scale (1e6)
pub const QTY_SCALE: u64 = 1_000_000;

/// USDC scale (6 decimals)
pub const USDC_SCALE: u64 = 1_000_000;

/// Basis points scale (10000 = 100%)
pub const BPS_SCALE: u64 = 10_000;

// ============================================================================
// INSTRUCTION DISCRIMINATORS
// ============================================================================

/// Router instruction discriminators
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouterInstruction {
    Initialize = 0,
    InitializePortfolio = 1,
    Deposit = 2,
    Withdraw = 3,
    ExecuteCrossSlab = 4,
    MultiSlabReserve = 5,
    MultiSlabCommit = 6,
    MultiSlabCancel = 7,
    GlobalLiquidation = 8,
    MarkToMarket = 9,
}

/// Slab instruction discriminators
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlabInstruction {
    Reserve = 0,
    Commit = 1,
    Cancel = 2,
    BatchOpen = 3,
    Initialize = 4,
    AddInstrument = 5,
    UpdateFunding = 6,
    Liquidation = 7,
    InitializeInsurance = 8,
    ContributeInsurance = 9,
    InitiateInsuranceWithdrawal = 10,
    CompleteInsuranceWithdrawal = 11,
    CancelInsuranceWithdrawal = 12,
    UpdateInsuranceConfig = 13,
}

// ============================================================================
// LIMITS AND DEFAULTS
// ============================================================================

/// Maximum slabs in router registry
pub const MAX_SLABS: usize = 8;

/// Maximum instruments per slab
pub const MAX_INSTRUMENTS: usize = 8;

/// Maximum positions per user
pub const MAX_POSITIONS: usize = 16;

/// Default batch window (milliseconds)
pub const DEFAULT_BATCH_MS: u64 = 100;

/// Default IMR (basis points)
pub const DEFAULT_IMR_BPS: u64 = 500; // 5%

/// Default MMR (basis points)
pub const DEFAULT_MMR_BPS: u64 = 250; // 2.5%

/// Default maker fee (basis points, negative = rebate)
pub const DEFAULT_MAKER_FEE_BPS: i64 = -5; // -0.05%

/// Default taker fee (basis points)
pub const DEFAULT_TAKER_FEE_BPS: u64 = 20; // 0.2%

/// Default insurance contribution rate (basis points)
pub const DEFAULT_INSURANCE_RATE_BPS: u64 = 25; // 0.25%

/// Insurance pool withdrawal timelock (seconds)
pub const INSURANCE_WITHDRAWAL_TIMELOCK_SECS: u64 = 7 * 24 * 60 * 60; // 7 days

// ============================================================================
// ORDER TYPES
// ============================================================================

/// Order side
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Side {
    #[default]
    Buy = 0,
    Sell = 1,
}

/// Time in force
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeInForce {
    #[default]
    GTC = 0, // Good til cancelled
    IOC = 1, // Immediate or cancel
    FOK = 2, // Fill or kill
    POST = 3, // Post only (maker only)
}

/// Maker class for anti-toxicity
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MakerClass {
    #[default]
    Retail = 0,
    Informed = 1,
    MM = 2,
}

/// Order status
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OrderStatus {
    #[default]
    Pending = 0,
    Open = 1,
    PartiallyFilled = 2,
    Filled = 3,
    Cancelled = 4,
    Expired = 5,
}

/// Reservation status
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReservationStatus {
    #[default]
    Active = 0,
    Committed = 1,
    Cancelled = 2,
    Expired = 3,
}
