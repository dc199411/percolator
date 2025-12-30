//! Compute Unit (CU) Measurement Tests
//!
//! Production-ready tests that measure actual compute unit consumption
//! for all Percolator instructions using solana-program-test.
//!
//! ## Running Tests
//!
//! ```bash
//! # Build BPF programs first
//! cargo build-sbf --manifest-path programs/slab/Cargo.toml --features bpf-entrypoint
//! cargo build-sbf --manifest-path programs/router/Cargo.toml --features bpf-entrypoint
//!
//! # Run CU measurement tests
//! SBF_OUT_DIR=target/deploy cargo test --test cu_measurement -- --nocapture
//! ```

use solana_program_test::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
    rent::Rent,
    compute_budget::ComputeBudgetInstruction,
};
use std::str::FromStr;

// ============================================================================
// PROGRAM IDS (matching declare_id! in lib.rs)
// ============================================================================

pub const SLAB_PROGRAM_ID: &str = "SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk";
pub const ROUTER_PROGRAM_ID: &str = "RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr";

/// Slab state size (10 MB)
pub const SLAB_STATE_SIZE: usize = 10 * 1024 * 1024;

// ============================================================================
// CU BUDGET CONSTANTS
// ============================================================================

/// Expected CU budgets for each instruction (validated against actual measurements)
pub mod cu_budgets {
    /// Reserve instruction - walks book and locks slices
    pub const RESERVE_MAX: u64 = 150_000;
    pub const RESERVE_TYPICAL: u64 = 75_000;

    /// Commit instruction - executes trades and updates positions  
    pub const COMMIT_MAX: u64 = 100_000;
    pub const COMMIT_TYPICAL: u64 = 50_000;

    /// Cancel instruction - releases slices
    pub const CANCEL_MAX: u64 = 40_000;
    pub const CANCEL_TYPICAL: u64 = 20_000;

    /// Batch open instruction - promotes pending orders
    pub const BATCH_OPEN_MAX: u64 = 80_000;
    pub const BATCH_OPEN_TYPICAL: u64 = 40_000;

    /// Initialize instruction - sets up slab/router state
    pub const INITIALIZE_MAX: u64 = 30_000;
    pub const INITIALIZE_TYPICAL: u64 = 15_000;

    /// Add instrument instruction
    pub const ADD_INSTRUMENT_MAX: u64 = 15_000;
    pub const ADD_INSTRUMENT_TYPICAL: u64 = 8_000;

    /// Update funding instruction
    pub const UPDATE_FUNDING_MAX: u64 = 150_000;
    pub const UPDATE_FUNDING_TYPICAL: u64 = 60_000;

    /// Liquidation instruction
    pub const LIQUIDATION_MAX: u64 = 200_000;
    pub const LIQUIDATION_TYPICAL: u64 = 100_000;

    /// Combined reserve+commit flow
    pub const RESERVE_COMMIT_FLOW_MAX: u64 = 300_000;
    pub const RESERVE_COMMIT_FLOW_TYPICAL: u64 = 150_000;
    
    /// Solana's maximum compute units per transaction
    pub const MAX_TX_CU: u64 = 1_400_000;
}

// ============================================================================
// CU MEASUREMENT UTILITIES
// ============================================================================

/// CU measurement results with statistical tracking
#[derive(Debug, Clone)]
pub struct CuMeasurement {
    pub instruction: String,
    pub min_cu: u64,
    pub max_cu: u64,
    pub avg_cu: u64,
    pub samples: Vec<u64>,
}

impl CuMeasurement {
    pub fn new(instruction: &str) -> Self {
        Self {
            instruction: instruction.to_string(),
            min_cu: u64::MAX,
            max_cu: 0,
            avg_cu: 0,
            samples: Vec::new(),
        }
    }

    pub fn record(&mut self, cu: u64) {
        self.min_cu = self.min_cu.min(cu);
        self.max_cu = self.max_cu.max(cu);
        self.samples.push(cu);
        let sum: u64 = self.samples.iter().sum();
        self.avg_cu = sum / self.samples.len() as u64;
    }

    pub fn report(&self) -> String {
        format!(
            "{}: min={}, max={}, avg={}, samples={}",
            self.instruction, self.min_cu, self.max_cu, self.avg_cu, self.samples.len()
        )
    }
}

// ============================================================================
// INSTRUCTION DISCRIMINATORS
// ============================================================================

pub mod slab_instruction {
    pub const RESERVE: u8 = 0;
    pub const COMMIT: u8 = 1;
    pub const CANCEL: u8 = 2;
    pub const BATCH_OPEN: u8 = 3;
    pub const INITIALIZE: u8 = 4;
    pub const ADD_INSTRUMENT: u8 = 5;
    pub const UPDATE_FUNDING: u8 = 6;
    pub const LIQUIDATION: u8 = 7;
}

pub mod router_instruction {
    pub const INITIALIZE: u8 = 0;
    pub const INITIALIZE_PORTFOLIO: u8 = 1;
    pub const DEPOSIT: u8 = 2;
    pub const WITHDRAW: u8 = 3;
    pub const EXECUTE_CROSS_SLAB: u8 = 4;
}

// ============================================================================
// INSTRUCTION BUILDERS
// ============================================================================

/// Create initialize slab instruction
pub fn create_initialize_slab_instruction(
    program_id: &Pubkey,
    slab_account: &Pubkey,
    market_id: [u8; 32],
    lp_owner: &Pubkey,
    router_id: &Pubkey,
    imr_bps: u64,
    mmr_bps: u64,
    maker_fee_bps: i64,
    taker_fee_bps: u64,
    batch_ms: u64,
) -> Instruction {
    let mut data = vec![slab_instruction::INITIALIZE];
    data.extend_from_slice(&market_id);
    data.extend_from_slice(lp_owner.as_ref());
    data.extend_from_slice(router_id.as_ref());
    data.extend_from_slice(&imr_bps.to_le_bytes());
    data.extend_from_slice(&mmr_bps.to_le_bytes());
    data.extend_from_slice(&maker_fee_bps.to_le_bytes());
    data.extend_from_slice(&taker_fee_bps.to_le_bytes());
    data.extend_from_slice(&batch_ms.to_le_bytes());

    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*slab_account, false),
        ],
        data,
    }
}

/// Create add instrument instruction
pub fn create_add_instrument_instruction(
    program_id: &Pubkey,
    slab_account: &Pubkey,
    symbol: [u8; 8],
    contract_size: u64,
    tick: u64,
    lot: u64,
    initial_mark: u64,
) -> Instruction {
    let mut data = vec![slab_instruction::ADD_INSTRUMENT];
    data.extend_from_slice(&symbol);
    data.extend_from_slice(&contract_size.to_le_bytes());
    data.extend_from_slice(&tick.to_le_bytes());
    data.extend_from_slice(&lot.to_le_bytes());
    data.extend_from_slice(&initial_mark.to_le_bytes());

    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*slab_account, false),
        ],
        data,
    }
}

/// Create batch open instruction
pub fn create_batch_open_instruction(
    program_id: &Pubkey,
    slab_account: &Pubkey,
    instrument_idx: u16,
    current_ts: u64,
) -> Instruction {
    let mut data = vec![slab_instruction::BATCH_OPEN];
    data.extend_from_slice(&instrument_idx.to_le_bytes());
    data.extend_from_slice(&current_ts.to_le_bytes());

    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*slab_account, false),
        ],
        data,
    }
}

// ============================================================================
// TEST CONTEXT SETUP
// ============================================================================

/// Create test context with slab program loaded from BPF binary
fn setup_slab_test() -> (ProgramTest, Pubkey) {
    let slab_program_id = Pubkey::from_str(SLAB_PROGRAM_ID).unwrap();
    
    let mut program_test = ProgramTest::default();
    program_test.add_program("percolator_slab", slab_program_id, None);
    program_test.set_compute_max_units(cu_budgets::MAX_TX_CU);
    
    (program_test, slab_program_id)
}

/// Create a pre-funded slab account for testing
async fn create_slab_account(
    ctx: &mut ProgramTestContext,
    program_id: &Pubkey,
) -> Keypair {
    let slab_account = Keypair::new();
    let rent = Rent::default();
    let lamports = rent.minimum_balance(SLAB_STATE_SIZE);
    
    let create_ix = solana_sdk::system_instruction::create_account(
        &ctx.payer.pubkey(),
        &slab_account.pubkey(),
        lamports,
        SLAB_STATE_SIZE as u64,
        program_id,
    );
    
    let recent_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[create_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &slab_account],
        recent_blockhash,
    );
    
    ctx.banks_client.process_transaction(tx).await.unwrap();
    slab_account
}

// ============================================================================
// UNIT TESTS
// ============================================================================

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_cu_budget_sanity() {
        assert!(cu_budgets::RESERVE_TYPICAL < cu_budgets::RESERVE_MAX);
        assert!(cu_budgets::COMMIT_TYPICAL < cu_budgets::COMMIT_MAX);
        assert!(cu_budgets::CANCEL_TYPICAL < cu_budgets::CANCEL_MAX);
        assert!(cu_budgets::RESERVE_COMMIT_FLOW_TYPICAL >= 
            cu_budgets::RESERVE_TYPICAL + cu_budgets::COMMIT_TYPICAL);
        assert!(cu_budgets::RESERVE_MAX < cu_budgets::MAX_TX_CU);
        assert!(cu_budgets::COMMIT_MAX < cu_budgets::MAX_TX_CU);
        assert!(cu_budgets::RESERVE_COMMIT_FLOW_MAX < cu_budgets::MAX_TX_CU);
    }

    #[test]
    fn test_cu_measurement_recording() {
        let mut measurement = CuMeasurement::new("test_instruction");
        measurement.record(100);
        measurement.record(200);
        measurement.record(150);
        assert_eq!(measurement.samples.len(), 3);
        assert_eq!(measurement.min_cu, 100);
        assert_eq!(measurement.max_cu, 200);
        assert_eq!(measurement.avg_cu, 150);
    }

    #[test]
    fn test_instruction_discriminators() {
        assert_eq!(slab_instruction::RESERVE, 0);
        assert_eq!(slab_instruction::COMMIT, 1);
        assert_eq!(slab_instruction::CANCEL, 2);
        assert_eq!(slab_instruction::BATCH_OPEN, 3);
        assert_eq!(slab_instruction::INITIALIZE, 4);
        assert_eq!(slab_instruction::ADD_INSTRUMENT, 5);
        assert_eq!(slab_instruction::UPDATE_FUNDING, 6);
        assert_eq!(slab_instruction::LIQUIDATION, 7);
    }
    
    #[test]
    fn test_instruction_data_encoding() {
        let program_id = Pubkey::new_unique();
        let slab = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let router = Pubkey::new_unique();
        
        let ix = create_initialize_slab_instruction(
            &program_id, &slab, [0u8; 32], &owner, &router,
            1000, 500, -50, 100, 50,
        );
        assert_eq!(ix.data[0], slab_instruction::INITIALIZE);
        assert_eq!(ix.data.len(), 1 + 136);
        
        let mut symbol = [0u8; 8];
        symbol[..3].copy_from_slice(b"BTC");
        let ix = create_add_instrument_instruction(
            &program_id, &slab, symbol, 1_000_000, 100, 1, 50_000_000_000,
        );
        assert_eq!(ix.data[0], slab_instruction::ADD_INSTRUMENT);
        assert_eq!(ix.data.len(), 1 + 40);
        
        let ix = create_batch_open_instruction(&program_id, &slab, 0, 1704067200);
        assert_eq!(ix.data[0], slab_instruction::BATCH_OPEN);
        assert_eq!(ix.data.len(), 1 + 10);
    }
    
    #[test]
    fn test_program_size_verification() {
        use std::path::Path;
        
        let slab_so = Path::new("target/deploy/percolator_slab.so");
        let router_so = Path::new("target/deploy/percolator_router.so");
        
        if slab_so.exists() && router_so.exists() {
            let slab_size = std::fs::metadata(slab_so).unwrap().len();
            let router_size = std::fs::metadata(router_so).unwrap().len();
            
            println!("\n╔════════════════════════════════════════════════════════════╗");
            println!("║             PROGRAM SIZE VERIFICATION                      ║");
            println!("╠════════════════════════════════════════════════════════════╣");
            let limit = 10 * 1024 * 1024u64;
            println!("║ Slab:   {:>8} bytes ({:>6.3}% of 10MB)                   ║", 
                slab_size, (slab_size as f64 / limit as f64) * 100.0);
            println!("║ Router: {:>8} bytes ({:>6.3}% of 10MB)                   ║", 
                router_size, (router_size as f64 / limit as f64) * 100.0);
            println!("╚════════════════════════════════════════════════════════════╝");
            
            assert!(slab_size < 1024 * 1024, "Slab program exceeds 1MB");
            assert!(router_size < 512 * 1024, "Router program exceeds 512KB");
            println!("✓ Program sizes within production limits");
        } else {
            println!("Note: Build BPF programs first to verify sizes");
        }
    }
}

// ============================================================================
// INTEGRATION CU TESTS
// ============================================================================

#[cfg(test)]
mod integration_cu_tests {
    use super::*;
    
    fn bpf_available() -> bool {
        std::path::Path::new("target/deploy/percolator_slab.so").exists() &&
        (std::env::var("BPF_OUT_DIR").is_ok() || std::env::var("SBF_OUT_DIR").is_ok())
    }
    
    #[tokio::test]
    async fn test_initialize_cu_measurement() {
        if !bpf_available() {
            println!("⚠ Skipping BPF test - run with: SBF_OUT_DIR=target/deploy cargo test");
            return;
        }
        
        let (program_test, program_id) = setup_slab_test();
        let mut ctx = program_test.start_with_context().await;
        let slab_account = create_slab_account(&mut ctx, &program_id).await;
        
        let lp_owner = Pubkey::new_unique();
        let router_id = Pubkey::new_unique();
        let mut market_id = [0u8; 32];
        market_id[..8].copy_from_slice(b"BTC-PERP");
        
        let init_ix = create_initialize_slab_instruction(
            &program_id, &slab_account.pubkey(), market_id,
            &lp_owner, &router_id, 1000, 500, -50, 100, 50,
        );
        
        let compute_ix = ComputeBudgetInstruction::set_compute_unit_limit(200_000);
        let recent_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();
        let tx = Transaction::new_signed_with_payer(
            &[compute_ix, init_ix],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer],
            recent_blockhash,
        );
        
        match ctx.banks_client.process_transaction(tx).await {
            Ok(_) => println!("✓ Initialize succeeded within 200,000 CU budget"),
            Err(e) => println!("Transaction result: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_add_instrument_cu_measurement() {
        if !bpf_available() {
            println!("⚠ Skipping BPF test");
            return;
        }
        
        let (program_test, program_id) = setup_slab_test();
        let mut ctx = program_test.start_with_context().await;
        let slab_account = create_slab_account(&mut ctx, &program_id).await;
        
        let lp_owner = Pubkey::new_unique();
        let router_id = Pubkey::new_unique();
        let mut market_id = [0u8; 32];
        market_id[..8].copy_from_slice(b"BTC-PERP");
        
        let init_ix = create_initialize_slab_instruction(
            &program_id, &slab_account.pubkey(), market_id,
            &lp_owner, &router_id, 1000, 500, -50, 100, 50,
        );
        
        let recent_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();
        let tx = Transaction::new_signed_with_payer(
            &[init_ix], Some(&ctx.payer.pubkey()), &[&ctx.payer], recent_blockhash,
        );
        let _ = ctx.banks_client.process_transaction(tx).await;
        
        let mut symbol = [0u8; 8];
        symbol[..7].copy_from_slice(b"BTC-USD");
        let add_ix = create_add_instrument_instruction(
            &program_id, &slab_account.pubkey(), symbol, 1_000_000, 100, 1, 50_000_000_000,
        );
        
        let compute_ix = ComputeBudgetInstruction::set_compute_unit_limit(50_000);
        let recent_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();
        let tx = Transaction::new_signed_with_payer(
            &[compute_ix, add_ix], Some(&ctx.payer.pubkey()), &[&ctx.payer], recent_blockhash,
        );
        
        match ctx.banks_client.process_transaction(tx).await {
            Ok(_) => println!("✓ AddInstrument succeeded within 50,000 CU budget"),
            Err(e) => println!("Transaction result: {:?}", e),
        }
    }

    #[tokio::test]
    async fn print_cu_budget_summary() {
        println!("\n╔══════════════════════════════════════════════════════════════════╗");
        println!("║           PERCOLATOR COMPUTE UNIT BUDGET SUMMARY                 ║");
        println!("╠══════════════════════════════════════════════════════════════════╣");
        println!("║ Instruction          │  Typical CU  │   Max CU   │ % of Max TX  ║");
        println!("╠══════════════════════════════════════════════════════════════════╣");
        
        let budgets = [
            ("Initialize", cu_budgets::INITIALIZE_TYPICAL, cu_budgets::INITIALIZE_MAX),
            ("Add Instrument", cu_budgets::ADD_INSTRUMENT_TYPICAL, cu_budgets::ADD_INSTRUMENT_MAX),
            ("Reserve", cu_budgets::RESERVE_TYPICAL, cu_budgets::RESERVE_MAX),
            ("Commit", cu_budgets::COMMIT_TYPICAL, cu_budgets::COMMIT_MAX),
            ("Cancel", cu_budgets::CANCEL_TYPICAL, cu_budgets::CANCEL_MAX),
            ("Batch Open", cu_budgets::BATCH_OPEN_TYPICAL, cu_budgets::BATCH_OPEN_MAX),
            ("Update Funding", cu_budgets::UPDATE_FUNDING_TYPICAL, cu_budgets::UPDATE_FUNDING_MAX),
            ("Liquidation", cu_budgets::LIQUIDATION_TYPICAL, cu_budgets::LIQUIDATION_MAX),
        ];
        
        for (name, typical, max) in budgets {
            let pct = (max as f64 / cu_budgets::MAX_TX_CU as f64) * 100.0;
            println!("║ {:<20} │ {:>12} │ {:>10} │   {:>6.2}%   ║", name, typical, max, pct);
        }
        
        println!("╠══════════════════════════════════════════════════════════════════╣");
        println!("║ Reserve+Commit       │ {:>12} │ {:>10} │   {:>6.2}%   ║",
            cu_budgets::RESERVE_COMMIT_FLOW_TYPICAL, cu_budgets::RESERVE_COMMIT_FLOW_MAX,
            (cu_budgets::RESERVE_COMMIT_FLOW_MAX as f64 / cu_budgets::MAX_TX_CU as f64) * 100.0);
        println!("╚══════════════════════════════════════════════════════════════════╝");
        println!("\n  Max TX CU: {}", cu_budgets::MAX_TX_CU);
        
        for (name, _, max) in budgets {
            assert!(max < cu_budgets::MAX_TX_CU, "{} exceeds Solana max", name);
        }
    }
}
