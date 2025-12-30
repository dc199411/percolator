pub mod initialize;
#[cfg(test)]
mod initialize_test;
pub mod reserve;
pub mod commit;
pub mod cancel;
pub mod batch_open;
pub mod liquidation;
pub mod funding;
pub mod add_instrument;
pub mod insurance;

pub use initialize::*;
pub use reserve::*;
pub use commit::*;
pub use cancel::*;
pub use batch_open::*;
pub use liquidation::*;
pub use funding::*;
pub use add_instrument::*;
pub use insurance::*;

/// Instruction discriminator
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlabInstruction {
    /// Reserve liquidity (phase 1 of two-phase execution)
    Reserve = 0,
    /// Commit reserved liquidity (phase 2 of two-phase execution)
    Commit = 1,
    /// Cancel a reservation
    Cancel = 2,
    /// Open a new batch (promote pending orders)
    BatchOpen = 3,
    /// Initialize slab state
    Initialize = 4,
    /// Add a new instrument
    AddInstrument = 5,
    /// Update funding rates
    UpdateFunding = 6,
    /// Execute liquidation
    Liquidation = 7,
    /// Initialize insurance pool (Phase 5)
    InitializeInsurance = 8,
    /// Contribute to insurance pool (Phase 5)
    ContributeInsurance = 9,
    /// Initiate insurance withdrawal (Phase 5)
    InitiateInsuranceWithdrawal = 10,
    /// Complete insurance withdrawal (Phase 5)
    CompleteInsuranceWithdrawal = 11,
    /// Cancel insurance withdrawal (Phase 5)
    CancelInsuranceWithdrawal = 12,
    /// Update insurance config (Phase 5)
    UpdateInsuranceConfig = 13,
}
