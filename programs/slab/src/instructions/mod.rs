pub mod initialize;
pub mod reserve;
pub mod commit;
pub mod cancel;
pub mod batch_open;
pub mod liquidation;
pub mod funding;
pub mod add_instrument;

pub use initialize::*;
pub use reserve::*;
pub use commit::*;
pub use cancel::*;
pub use batch_open::*;
pub use liquidation::*;
pub use funding::*;
pub use add_instrument::*;

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
}
