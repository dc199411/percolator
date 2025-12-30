//! Common test utilities for integration tests
//!
//! Provides shared infrastructure for testing Percolator programs
//! using the solana-program-test crate.

use solana_program_test::*;
use solana_sdk::{
    account::Account,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
    instruction::{AccountMeta, Instruction},
    rent::Rent,
    compute_budget::ComputeBudgetInstruction,
};
use std::str::FromStr;

// ============================================================================
// PROGRAM IDS
// ============================================================================

pub const SLAB_PROGRAM_ID_STR: &str = "SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk";
pub const ROUTER_PROGRAM_ID_STR: &str = "RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr";

pub fn slab_program_id() -> Pubkey {
    Pubkey::from_str(SLAB_PROGRAM_ID_STR).unwrap()
}

pub fn router_program_id() -> Pubkey {
    Pubkey::from_str(ROUTER_PROGRAM_ID_STR).unwrap()
}

/// Slab state size (10 MB)
pub const SLAB_STATE_SIZE: usize = 10 * 1024 * 1024;

// ============================================================================
// INSTRUCTION DISCRIMINATORS
// ============================================================================

pub mod slab_ix {
    pub const RESERVE: u8 = 0;
    pub const COMMIT: u8 = 1;
    pub const CANCEL: u8 = 2;
    pub const BATCH_OPEN: u8 = 3;
    pub const INITIALIZE: u8 = 4;
    pub const ADD_INSTRUMENT: u8 = 5;
    pub const UPDATE_FUNDING: u8 = 6;
    pub const LIQUIDATION: u8 = 7;
}

pub mod router_ix {
    pub const INITIALIZE: u8 = 0;
    pub const INITIALIZE_PORTFOLIO: u8 = 1;
    pub const DEPOSIT: u8 = 2;
    pub const WITHDRAW: u8 = 3;
    pub const EXECUTE_CROSS_SLAB: u8 = 4;
}

// ============================================================================
// TEST CONTEXT
// ============================================================================

/// Enhanced test context for integration tests
pub struct TestContext {
    pub ctx: ProgramTestContext,
    pub slab_program_id: Pubkey,
    pub router_program_id: Pubkey,
}

impl TestContext {
    /// Create test context with slab program loaded
    pub async fn new_with_slab() -> Self {
        let slab_id = slab_program_id();
        let router_id = router_program_id();
        
        let mut program_test = ProgramTest::default();
        program_test.add_program("percolator_slab", slab_id, None);
        program_test.set_compute_max_units(1_400_000);
        
        let ctx = program_test.start_with_context().await;
        
        Self {
            ctx,
            slab_program_id: slab_id,
            router_program_id: router_id,
        }
    }
    
    /// Create test context with both programs
    pub async fn new_with_both() -> Self {
        let slab_id = slab_program_id();
        let router_id = router_program_id();
        
        let mut program_test = ProgramTest::default();
        program_test.add_program("percolator_slab", slab_id, None);
        program_test.add_program("percolator_router", router_id, None);
        program_test.set_compute_max_units(1_400_000);
        
        let ctx = program_test.start_with_context().await;
        
        Self {
            ctx,
            slab_program_id: slab_id,
            router_program_id: router_id,
        }
    }
    
    /// Get latest blockhash
    pub async fn get_blockhash(&mut self) -> solana_sdk::hash::Hash {
        self.ctx.banks_client.get_latest_blockhash().await.unwrap()
    }
    
    /// Process a transaction
    pub async fn process_tx(&mut self, tx: Transaction) -> Result<(), BanksClientError> {
        self.ctx.banks_client.process_transaction(tx).await
    }
    
    /// Create a slab account with 10MB
    pub async fn create_slab_account(&mut self) -> Keypair {
        let slab = Keypair::new();
        let rent = Rent::default();
        let lamports = rent.minimum_balance(SLAB_STATE_SIZE);
        
        let create_ix = solana_sdk::system_instruction::create_account(
            &self.ctx.payer.pubkey(),
            &slab.pubkey(),
            lamports,
            SLAB_STATE_SIZE as u64,
            &self.slab_program_id,
        );
        
        let blockhash = self.get_blockhash().await;
        let tx = Transaction::new_signed_with_payer(
            &[create_ix],
            Some(&self.ctx.payer.pubkey()),
            &[&self.ctx.payer, &slab],
            blockhash,
        );
        
        self.process_tx(tx).await.unwrap();
        slab
    }
    
    /// Get account data
    pub async fn get_account(&mut self, pubkey: &Pubkey) -> Option<Account> {
        self.ctx.banks_client.get_account(*pubkey).await.unwrap()
    }
    
    /// Send instruction with compute budget
    pub async fn send_ix_with_budget(
        &mut self, 
        ix: Instruction, 
        budget: u32,
        signers: &[&Keypair]
    ) -> Result<(), BanksClientError> {
        let compute_ix = ComputeBudgetInstruction::set_compute_unit_limit(budget);
        let blockhash = self.get_blockhash().await;
        
        let mut all_signers: Vec<&Keypair> = vec![&self.ctx.payer];
        all_signers.extend(signers);
        
        let tx = Transaction::new_signed_with_payer(
            &[compute_ix, ix],
            Some(&self.ctx.payer.pubkey()),
            &all_signers,
            blockhash,
        );
        
        self.process_tx(tx).await
    }
}

// ============================================================================
// INSTRUCTION BUILDERS
// ============================================================================

/// Create initialize slab instruction
pub fn ix_initialize_slab(
    program_id: &Pubkey,
    slab: &Pubkey,
    market_id: [u8; 32],
    lp_owner: &Pubkey,
    router_id: &Pubkey,
    imr_bps: u64,
    mmr_bps: u64,
    maker_fee_bps: i64,
    taker_fee_bps: u64,
    batch_ms: u64,
) -> Instruction {
    let mut data = vec![slab_ix::INITIALIZE];
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
        accounts: vec![AccountMeta::new(*slab, false)],
        data,
    }
}

/// Create add instrument instruction
pub fn ix_add_instrument(
    program_id: &Pubkey,
    slab: &Pubkey,
    symbol: [u8; 8],
    contract_size: u64,
    tick: u64,
    lot: u64,
    initial_mark: u64,
) -> Instruction {
    let mut data = vec![slab_ix::ADD_INSTRUMENT];
    data.extend_from_slice(&symbol);
    data.extend_from_slice(&contract_size.to_le_bytes());
    data.extend_from_slice(&tick.to_le_bytes());
    data.extend_from_slice(&lot.to_le_bytes());
    data.extend_from_slice(&initial_mark.to_le_bytes());
    
    Instruction {
        program_id: *program_id,
        accounts: vec![AccountMeta::new(*slab, false)],
        data,
    }
}

/// Create batch open instruction
pub fn ix_batch_open(
    program_id: &Pubkey,
    slab: &Pubkey,
    instrument_idx: u16,
    current_ts: u64,
) -> Instruction {
    let mut data = vec![slab_ix::BATCH_OPEN];
    data.extend_from_slice(&instrument_idx.to_le_bytes());
    data.extend_from_slice(&current_ts.to_le_bytes());
    
    Instruction {
        program_id: *program_id,
        accounts: vec![AccountMeta::new(*slab, false)],
        data,
    }
}

/// Create reserve instruction
pub fn ix_reserve(
    program_id: &Pubkey,
    slab: &Pubkey,
    account_idx: u32,
    instrument_idx: u16,
    side: u8,
    qty: u64,
    limit_px: u64,
    ttl_ms: u64,
    commitment_hash: [u8; 32],
    route_id: u64,
) -> Instruction {
    let mut data = vec![slab_ix::RESERVE];
    data.extend_from_slice(&account_idx.to_le_bytes());
    data.extend_from_slice(&instrument_idx.to_le_bytes());
    data.push(side);
    data.extend_from_slice(&qty.to_le_bytes());
    data.extend_from_slice(&limit_px.to_le_bytes());
    data.extend_from_slice(&ttl_ms.to_le_bytes());
    data.extend_from_slice(&commitment_hash);
    data.extend_from_slice(&route_id.to_le_bytes());
    
    Instruction {
        program_id: *program_id,
        accounts: vec![AccountMeta::new(*slab, false)],
        data,
    }
}

/// Create commit instruction
pub fn ix_commit(
    program_id: &Pubkey,
    slab: &Pubkey,
    hold_id: u64,
    current_ts: u64,
) -> Instruction {
    let mut data = vec![slab_ix::COMMIT];
    data.extend_from_slice(&hold_id.to_le_bytes());
    data.extend_from_slice(&current_ts.to_le_bytes());
    
    Instruction {
        program_id: *program_id,
        accounts: vec![AccountMeta::new(*slab, false)],
        data,
    }
}

/// Create cancel instruction
pub fn ix_cancel(program_id: &Pubkey, slab: &Pubkey, hold_id: u64) -> Instruction {
    let mut data = vec![slab_ix::CANCEL];
    data.extend_from_slice(&hold_id.to_le_bytes());
    
    Instruction {
        program_id: *program_id,
        accounts: vec![AccountMeta::new(*slab, false)],
        data,
    }
}

/// Create update funding instruction
pub fn ix_update_funding(
    program_id: &Pubkey,
    slab: &Pubkey,
    instrument_idx: u16,
    index_price: u64,
    current_ts: u64,
) -> Instruction {
    let mut data = vec![slab_ix::UPDATE_FUNDING];
    data.extend_from_slice(&instrument_idx.to_le_bytes());
    data.extend_from_slice(&index_price.to_le_bytes());
    data.extend_from_slice(&current_ts.to_le_bytes());
    
    Instruction {
        program_id: *program_id,
        accounts: vec![AccountMeta::new(*slab, false)],
        data,
    }
}

/// Create liquidation instruction
pub fn ix_liquidation(
    program_id: &Pubkey,
    slab: &Pubkey,
    account_idx: u32,
    deficit_target: i128,
    current_ts: u64,
) -> Instruction {
    let mut data = vec![slab_ix::LIQUIDATION];
    data.extend_from_slice(&account_idx.to_le_bytes());
    data.extend_from_slice(&deficit_target.to_le_bytes());
    data.extend_from_slice(&current_ts.to_le_bytes());
    
    Instruction {
        program_id: *program_id,
        accounts: vec![AccountMeta::new(*slab, false)],
        data,
    }
}

// ============================================================================
// TEST HELPERS
// ============================================================================

/// Check if BPF programs are available
pub fn bpf_available() -> bool {
    std::path::Path::new("target/deploy/percolator_slab.so").exists() &&
    (std::env::var("SBF_OUT_DIR").is_ok() || std::env::var("BPF_OUT_DIR").is_ok())
}

/// Create a market ID from string
pub fn market_id(name: &str) -> [u8; 32] {
    let mut id = [0u8; 32];
    let bytes = name.as_bytes();
    let len = bytes.len().min(32);
    id[..len].copy_from_slice(&bytes[..len]);
    id
}

/// Create a symbol from string
pub fn symbol(name: &str) -> [u8; 8] {
    let mut sym = [0u8; 8];
    let bytes = name.as_bytes();
    let len = bytes.len().min(8);
    sym[..len].copy_from_slice(&bytes[..len]);
    sym
}
