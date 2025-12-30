/// Router instruction handlers
///
/// Phase 4: Multi-Slab Coordination
/// - Multi-slab atomic reserve/commit
/// - Cross-slab portfolio margin
/// - Global liquidation coordination
/// - CPI integration with Slab programs

pub mod initialize;
pub mod initialize_portfolio;
pub mod deposit;
pub mod withdraw;
pub mod execute_cross_slab;
pub mod multi_slab;
pub mod liquidation;
pub mod portfolio_margin;
pub mod cpi;

pub use initialize::*;
pub use initialize_portfolio::*;
pub use deposit::*;
pub use withdraw::*;
pub use execute_cross_slab::*;
pub use multi_slab::*;
pub use liquidation::*;
pub use portfolio_margin::*;
pub use cpi::*;

/// Instruction discriminator
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouterInstruction {
    /// Initialize router registry
    Initialize = 0,
    /// Initialize user portfolio
    InitializePortfolio = 1,
    /// Deposit collateral to vault
    Deposit = 2,
    /// Withdraw collateral from vault
    Withdraw = 3,
    /// Execute cross-slab order (v0 main instruction)
    ExecuteCrossSlab = 4,
    /// Multi-slab reserve (Phase 4)
    MultiSlabReserve = 5,
    /// Multi-slab commit (Phase 4)
    MultiSlabCommit = 6,
    /// Multi-slab cancel (Phase 4)
    MultiSlabCancel = 7,
    /// Global liquidation (Phase 4)
    GlobalLiquidation = 8,
    /// Mark-to-market update (Phase 4)
    MarkToMarket = 9,
}

// Note: Instruction dispatching is handled in entrypoint.rs
// The functions in this module are called from the entrypoint after
// account deserialization and validation.
