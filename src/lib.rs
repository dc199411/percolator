//! Percolator - Sharded Perpetual Exchange Protocol
//!
//! This is the root package providing test infrastructure and common utilities.
//! The actual on-chain programs (slab, router) are no_std BPF programs that
//! are loaded as binaries during tests.

pub use percolator_common as common;

/// Program IDs for the deployed programs
pub mod program_ids {
    /// Slab program ID: SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk
    pub const SLAB: &str = "SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk";
    
    /// Router program ID: RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr
    pub const ROUTER: &str = "RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr";
}
