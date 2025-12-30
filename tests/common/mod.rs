//! Common test utilities for integration tests
//!
//! This module provides shared test infrastructure for integration testing
//! using the solana-program-test crate.

use solana_program_test::*;
use solana_sdk::{
    account::Account,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
    instruction::{AccountMeta, Instruction},
    system_instruction,
    rent::Rent,
};

// Program IDs (from lib.rs declarations)
pub const SLAB_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0x0b, 0x3a, 0x5b, 0x7c, 0x8d, 0x9e, 0x0f, 0x1a,
    0x2b, 0x3c, 0x4d, 0x5e, 0x6f, 0x7a, 0x8b, 0x9c,
    0x0d, 0x1e, 0x2f, 0x3a, 0x4b, 0x5c, 0x6d, 0x7e,
    0x8f, 0x9a, 0x0b, 0x1c, 0x2d, 0x3e, 0x4f, 0x50,
]);

pub const ROUTER_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0x1a, 0x2b, 0x3c, 0x4d, 0x5e, 0x6f, 0x7a, 0x8b,
    0x9c, 0x0d, 0x1e, 0x2f, 0x3a, 0x4b, 0x5c, 0x6d,
    0x7e, 0x8f, 0x9a, 0x0b, 0x1c, 0x2d, 0x3e, 0x4f,
    0x50, 0x61, 0x72, 0x83, 0x94, 0xa5, 0xb6, 0xc7,
]);

/// Slab instruction discriminators
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

/// Router instruction discriminators
pub mod router_instruction {
    pub const INITIALIZE: u8 = 0;
    pub const INITIALIZE_PORTFOLIO: u8 = 1;
    pub const DEPOSIT: u8 = 2;
    pub const WITHDRAW: u8 = 3;
    pub const EXECUTE_CROSS_SLAB: u8 = 4;
}

/// Test context for integration tests
pub struct TestContext {
    pub banks_client: BanksClient,
    pub payer: Keypair,
    pub recent_blockhash: solana_sdk::hash::Hash,
}

impl TestContext {
    /// Create a new test context with programs loaded
    pub async fn new() -> Self {
        let mut program_test = ProgramTest::default();
        
        // Note: In a real test, we would load the actual BPF programs
        // For now, we'll use mock accounts for testing
        
        let (banks_client, payer, recent_blockhash) = program_test.start().await;
        
        Self {
            banks_client,
            payer,
            recent_blockhash,
        }
    }

    /// Refresh blockhash
    pub async fn refresh_blockhash(&mut self) {
        self.recent_blockhash = self.banks_client.get_latest_blockhash().await.unwrap();
    }

    /// Send a transaction
    pub async fn send_transaction(&mut self, instructions: &[Instruction], signers: &[&Keypair]) -> Result<(), BanksClientError> {
        self.refresh_blockhash().await;
        
        let mut all_signers = vec![&self.payer];
        all_signers.extend(signers);
        
        let transaction = Transaction::new_signed_with_payer(
            instructions,
            Some(&self.payer.pubkey()),
            &all_signers,
            self.recent_blockhash,
        );
        
        self.banks_client.process_transaction(transaction).await
    }

    /// Create an account with specified lamports
    pub async fn create_account(&mut self, keypair: &Keypair, lamports: u64, space: usize, owner: &Pubkey) -> Result<(), BanksClientError> {
        let rent = Rent::default();
        let min_lamports = rent.minimum_balance(space).max(lamports);
        
        let instruction = system_instruction::create_account(
            &self.payer.pubkey(),
            &keypair.pubkey(),
            min_lamports,
            space as u64,
            owner,
        );
        
        self.send_transaction(&[instruction], &[keypair]).await
    }

    /// Get account data
    pub async fn get_account(&mut self, pubkey: &Pubkey) -> Option<Account> {
        self.banks_client.get_account(*pubkey).await.unwrap()
    }

    /// Airdrop SOL to an account
    pub async fn airdrop(&mut self, to: &Pubkey, lamports: u64) -> Result<(), BanksClientError> {
        let instruction = system_instruction::transfer(
            &self.payer.pubkey(),
            to,
            lamports,
        );
        
        self.send_transaction(&[instruction], &[]).await
    }
}

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

/// Create reserve instruction
pub fn create_reserve_instruction(
    program_id: &Pubkey,
    slab_account: &Pubkey,
    account_idx: u32,
    instrument_idx: u16,
    side: u8,
    qty: u64,
    limit_px: u64,
    ttl_ms: u64,
    commitment_hash: [u8; 32],
    route_id: u64,
) -> Instruction {
    let mut data = vec![slab_instruction::RESERVE];
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
        accounts: vec![
            AccountMeta::new(*slab_account, false),
        ],
        data,
    }
}

/// Create commit instruction
pub fn create_commit_instruction(
    program_id: &Pubkey,
    slab_account: &Pubkey,
    hold_id: u64,
    current_ts: u64,
) -> Instruction {
    let mut data = vec![slab_instruction::COMMIT];
    data.extend_from_slice(&hold_id.to_le_bytes());
    data.extend_from_slice(&current_ts.to_le_bytes());
    
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*slab_account, false),
        ],
        data,
    }
}

/// Create cancel instruction
pub fn create_cancel_instruction(
    program_id: &Pubkey,
    slab_account: &Pubkey,
    hold_id: u64,
) -> Instruction {
    let mut data = vec![slab_instruction::CANCEL];
    data.extend_from_slice(&hold_id.to_le_bytes());
    
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

/// Create update funding instruction
pub fn create_update_funding_instruction(
    program_id: &Pubkey,
    slab_account: &Pubkey,
    instrument_idx: u16,
    index_price: u64,
    current_ts: u64,
) -> Instruction {
    let mut data = vec![slab_instruction::UPDATE_FUNDING];
    data.extend_from_slice(&instrument_idx.to_le_bytes());
    data.extend_from_slice(&index_price.to_le_bytes());
    data.extend_from_slice(&current_ts.to_le_bytes());
    
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*slab_account, false),
        ],
        data,
    }
}

/// Create liquidation instruction
pub fn create_liquidation_instruction(
    program_id: &Pubkey,
    slab_account: &Pubkey,
    account_idx: u32,
    deficit_target: i128,
    current_ts: u64,
) -> Instruction {
    let mut data = vec![slab_instruction::LIQUIDATION];
    data.extend_from_slice(&account_idx.to_le_bytes());
    data.extend_from_slice(&deficit_target.to_le_bytes());
    data.extend_from_slice(&current_ts.to_le_bytes());
    
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*slab_account, false),
        ],
        data,
    }
}
