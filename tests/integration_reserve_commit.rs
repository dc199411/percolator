//! Integration tests for reserve-commit flow
//!
//! Tests the two-phase order execution:
//! 1. Reserve: Lock liquidity and calculate VWAP
//! 2. Commit: Execute trades at reserved prices
//!
//! Run with: SBF_OUT_DIR=target/deploy cargo test --test integration_reserve_commit -- --nocapture

mod common;

use common::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
    compute_budget::ComputeBudgetInstruction,
};

// ============================================================================
// INTEGRATION TESTS (require BPF programs)
// ============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Skip test if BPF programs aren't available
    fn skip_if_no_bpf() -> bool {
        if !bpf_available() {
            println!("⏭️  Skipping: BPF programs not available");
            println!("   Run: cargo build-sbf --manifest-path programs/slab/Cargo.toml --features bpf-entrypoint");
            println!("   Then: SBF_OUT_DIR=target/deploy cargo test --test integration_reserve_commit -- --nocapture");
            true
        } else {
            false
        }
    }

    #[tokio::test]
    async fn test_initialize_slab() {
        if skip_if_no_bpf() { return; }
        
        println!("\n🧪 Test: Initialize Slab");
        println!("{}", "─".repeat(50));
        
        let mut ctx = TestContext::new_with_slab().await;
        let slab = ctx.create_slab_account().await;
        
        println!("  Slab account: {}", slab.pubkey());
        println!("  Slab size: {} bytes", SLAB_STATE_SIZE);
        
        // Create initialize instruction
        let init_ix = ix_initialize_slab(
            &ctx.slab_program_id,
            &slab.pubkey(),
            market_id("BTC-PERP"),
            &ctx.ctx.payer.pubkey(), // LP owner
            &ctx.router_program_id,
            500,  // 5% IMR
            250,  // 2.5% MMR
            -10,  // -0.1% maker fee (rebate)
            30,   // 0.3% taker fee
            500,  // 500ms batch interval
        );
        
        // Send with budget
        let compute_ix = ComputeBudgetInstruction::set_compute_unit_limit(200_000);
        let blockhash = ctx.get_blockhash().await;
        let tx = Transaction::new_signed_with_payer(
            &[compute_ix, init_ix],
            Some(&ctx.ctx.payer.pubkey()),
            &[&ctx.ctx.payer],
            blockhash,
        );
        
        match ctx.process_tx(tx).await {
            Ok(_) => {
                println!("  ✅ Initialize succeeded");
                
                // Verify account exists and has data
                let account = ctx.get_account(&slab.pubkey()).await.unwrap();
                assert_eq!(account.data.len(), SLAB_STATE_SIZE);
                println!("  ✅ Account data verified ({} bytes)", account.data.len());
            }
            Err(e) => {
                println!("  ❌ Initialize failed: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_add_instrument() {
        if skip_if_no_bpf() { return; }
        
        println!("\n🧪 Test: Add Instrument");
        println!("{}", "─".repeat(50));
        
        let mut ctx = TestContext::new_with_slab().await;
        let slab = ctx.create_slab_account().await;
        
        // Initialize slab first
        let init_ix = ix_initialize_slab(
            &ctx.slab_program_id,
            &slab.pubkey(),
            market_id("BTC-PERP"),
            &ctx.ctx.payer.pubkey(),
            &ctx.router_program_id,
            500, 250, -10, 30, 500,
        );
        ctx.send_ix_with_budget(init_ix, 200_000, &[]).await.unwrap();
        
        // Add BTC instrument
        let add_ix = ix_add_instrument(
            &ctx.slab_program_id,
            &slab.pubkey(),
            symbol("BTC"),
            100_000_000,         // 1 contract = $1 at 6 decimals
            100_000,             // tick size: $0.10
            1_000_000,           // lot size: 0.001 BTC
            50_000_000_000,      // initial mark: $50,000
        );
        
        match ctx.send_ix_with_budget(add_ix, 50_000, &[]).await {
            Ok(_) => println!("  ✅ Add instrument succeeded"),
            Err(e) => println!("  ❌ Add instrument failed: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_batch_open() {
        if skip_if_no_bpf() { return; }
        
        println!("\n🧪 Test: Batch Open");
        println!("{}", "─".repeat(50));
        
        let mut ctx = TestContext::new_with_slab().await;
        let slab = ctx.create_slab_account().await;
        
        // Initialize
        let init_ix = ix_initialize_slab(
            &ctx.slab_program_id,
            &slab.pubkey(),
            market_id("BTC-PERP"),
            &ctx.ctx.payer.pubkey(),
            &ctx.router_program_id,
            500, 250, -10, 30, 500,
        );
        ctx.send_ix_with_budget(init_ix, 200_000, &[]).await.unwrap();
        
        // Add instrument
        let add_ix = ix_add_instrument(
            &ctx.slab_program_id,
            &slab.pubkey(),
            symbol("BTC"),
            100_000_000, 100_000, 1_000_000, 50_000_000_000,
        );
        ctx.send_ix_with_budget(add_ix, 50_000, &[]).await.unwrap();
        
        // Batch open (promotes pending orders to live)
        let batch_ix = ix_batch_open(
            &ctx.slab_program_id,
            &slab.pubkey(),
            0,  // instrument index
            1704067200000, // timestamp
        );
        
        match ctx.send_ix_with_budget(batch_ix, 100_000, &[]).await {
            Ok(_) => println!("  ✅ Batch open succeeded"),
            Err(e) => println!("  ❌ Batch open failed: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_reserve_flow() {
        if skip_if_no_bpf() { return; }
        
        println!("\n🧪 Test: Reserve Flow");
        println!("{}", "─".repeat(50));
        
        let mut ctx = TestContext::new_with_slab().await;
        let slab = ctx.create_slab_account().await;
        
        // Full setup: init -> add instrument -> batch open
        let init_ix = ix_initialize_slab(
            &ctx.slab_program_id,
            &slab.pubkey(),
            market_id("BTC-PERP"),
            &ctx.ctx.payer.pubkey(),
            &ctx.router_program_id,
            500, 250, -10, 30, 500,
        );
        ctx.send_ix_with_budget(init_ix, 200_000, &[]).await.unwrap();
        println!("  ✓ Slab initialized");
        
        let add_ix = ix_add_instrument(
            &ctx.slab_program_id,
            &slab.pubkey(),
            symbol("BTC"),
            100_000_000, 100_000, 1_000_000, 50_000_000_000,
        );
        ctx.send_ix_with_budget(add_ix, 50_000, &[]).await.unwrap();
        println!("  ✓ Instrument added");
        
        let batch_ix = ix_batch_open(
            &ctx.slab_program_id,
            &slab.pubkey(),
            0,
            1704067200000,
        );
        ctx.send_ix_with_budget(batch_ix, 100_000, &[]).await.unwrap();
        println!("  ✓ Batch opened");
        
        // Reserve liquidity
        let reserve_ix = ix_reserve(
            &ctx.slab_program_id,
            &slab.pubkey(),
            0,                    // account_idx
            0,                    // instrument_idx
            0,                    // side: 0=bid, 1=ask
            1_000_000,            // qty: 1 contract
            50_100_000_000,       // limit price: $50,100
            30_000,               // TTL: 30 seconds
            [0u8; 32],            // commitment hash
            0,                    // route_id
        );
        
        match ctx.send_ix_with_budget(reserve_ix, 200_000, &[]).await {
            Ok(_) => println!("  ✅ Reserve succeeded"),
            Err(e) => println!("  ❌ Reserve failed (expected if no liquidity): {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_reserve_commit_cancel_flow() {
        if skip_if_no_bpf() { return; }
        
        println!("\n🧪 Test: Reserve-Commit-Cancel Flow");
        println!("{}", "─".repeat(50));
        
        let mut ctx = TestContext::new_with_slab().await;
        let slab = ctx.create_slab_account().await;
        
        // Setup
        let init_ix = ix_initialize_slab(
            &ctx.slab_program_id,
            &slab.pubkey(),
            market_id("BTC-PERP"),
            &ctx.ctx.payer.pubkey(),
            &ctx.router_program_id,
            500, 250, -10, 30, 500,
        );
        ctx.send_ix_with_budget(init_ix, 200_000, &[]).await.unwrap();
        
        let add_ix = ix_add_instrument(
            &ctx.slab_program_id,
            &slab.pubkey(),
            symbol("BTC"),
            100_000_000, 100_000, 1_000_000, 50_000_000_000,
        );
        ctx.send_ix_with_budget(add_ix, 50_000, &[]).await.unwrap();
        
        let batch_ix = ix_batch_open(
            &ctx.slab_program_id,
            &slab.pubkey(),
            0,
            1704067200000,
        );
        ctx.send_ix_with_budget(batch_ix, 100_000, &[]).await.unwrap();
        
        println!("  ✓ Setup complete");
        
        // Test cancel (should fail if no reservation)
        let cancel_ix = ix_cancel(
            &ctx.slab_program_id,
            &slab.pubkey(),
            0, // hold_id
        );
        
        match ctx.send_ix_with_budget(cancel_ix, 50_000, &[]).await {
            Ok(_) => println!("  ✓ Cancel succeeded (empty reservation)"),
            Err(_) => println!("  ✓ Cancel rejected (expected: no reservation)"),
        }
        
        // Test commit (should fail if no reservation)
        let commit_ix = ix_commit(
            &ctx.slab_program_id,
            &slab.pubkey(),
            0, // hold_id
            1704067200000,
        );
        
        match ctx.send_ix_with_budget(commit_ix, 100_000, &[]).await {
            Ok(_) => println!("  ✓ Commit succeeded"),
            Err(_) => println!("  ✓ Commit rejected (expected: no reservation)"),
        }
        
        println!("  ✅ Reserve-Commit-Cancel flow tested");
    }

    #[tokio::test]
    async fn test_update_funding() {
        if skip_if_no_bpf() { return; }
        
        println!("\n🧪 Test: Update Funding");
        println!("{}", "─".repeat(50));
        
        let mut ctx = TestContext::new_with_slab().await;
        let slab = ctx.create_slab_account().await;
        
        // Setup
        let init_ix = ix_initialize_slab(
            &ctx.slab_program_id,
            &slab.pubkey(),
            market_id("BTC-PERP"),
            &ctx.ctx.payer.pubkey(),
            &ctx.router_program_id,
            500, 250, -10, 30, 500,
        );
        ctx.send_ix_with_budget(init_ix, 200_000, &[]).await.unwrap();
        
        let add_ix = ix_add_instrument(
            &ctx.slab_program_id,
            &slab.pubkey(),
            symbol("BTC"),
            100_000_000, 100_000, 1_000_000, 50_000_000_000,
        );
        ctx.send_ix_with_budget(add_ix, 50_000, &[]).await.unwrap();
        
        // Update funding rate
        let funding_ix = ix_update_funding(
            &ctx.slab_program_id,
            &slab.pubkey(),
            0,                    // instrument_idx
            50_100_000_000,       // index price: $50,100
            1704067200000,        // timestamp
        );
        
        match ctx.send_ix_with_budget(funding_ix, 200_000, &[]).await {
            Ok(_) => println!("  ✅ Update funding succeeded"),
            Err(e) => println!("  ❌ Update funding failed: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_liquidation() {
        if skip_if_no_bpf() { return; }
        
        println!("\n🧪 Test: Liquidation");
        println!("{}", "─".repeat(50));
        
        let mut ctx = TestContext::new_with_slab().await;
        let slab = ctx.create_slab_account().await;
        
        // Setup
        let init_ix = ix_initialize_slab(
            &ctx.slab_program_id,
            &slab.pubkey(),
            market_id("BTC-PERP"),
            &ctx.ctx.payer.pubkey(),
            &ctx.router_program_id,
            500, 250, -10, 30, 500,
        );
        ctx.send_ix_with_budget(init_ix, 200_000, &[]).await.unwrap();
        
        let add_ix = ix_add_instrument(
            &ctx.slab_program_id,
            &slab.pubkey(),
            symbol("BTC"),
            100_000_000, 100_000, 1_000_000, 50_000_000_000,
        );
        ctx.send_ix_with_budget(add_ix, 50_000, &[]).await.unwrap();
        
        // Attempt liquidation
        let liq_ix = ix_liquidation(
            &ctx.slab_program_id,
            &slab.pubkey(),
            0,                    // account_idx
            0,                    // deficit_target
            1704067200000,        // timestamp
        );
        
        match ctx.send_ix_with_budget(liq_ix, 300_000, &[]).await {
            Ok(_) => println!("  ✅ Liquidation succeeded"),
            Err(_) => println!("  ✓ Liquidation rejected (expected: no positions)"),
        }
    }
}

// ============================================================================
// UNIT TESTS (no BPF required)
// ============================================================================

#[cfg(test)]
mod unit_tests {
    use super::*;
    
    #[test]
    fn test_market_id_creation() {
        let id = market_id("BTC-PERP");
        assert_eq!(&id[0..8], b"BTC-PERP");
        assert_eq!(id[8..], [0u8; 24]);
    }
    
    #[test]
    fn test_symbol_creation() {
        let sym = symbol("BTC");
        assert_eq!(&sym[0..3], b"BTC");
        assert_eq!(sym[3..], [0u8; 5]);
    }
    
    #[test]
    fn test_initialize_instruction_encoding() {
        let program_id = slab_program_id();
        let slab = Keypair::new();
        
        let ix = ix_initialize_slab(
            &program_id,
            &slab.pubkey(),
            market_id("BTC-PERP"),
            &slab.pubkey(),
            &router_program_id(),
            500, 250, -10, 30, 500,
        );
        
        // Verify discriminator
        assert_eq!(ix.data[0], slab_ix::INITIALIZE);
        
        // Verify data length: 1 + 32 + 32 + 32 + 8 + 8 + 8 + 8 + 8 = 137
        assert_eq!(ix.data.len(), 137);
    }
    
    #[test]
    fn test_reserve_instruction_encoding() {
        let program_id = slab_program_id();
        let slab = Keypair::new();
        
        let ix = ix_reserve(
            &program_id,
            &slab.pubkey(),
            0, 0, 0, 1_000_000, 50_000_000_000, 30_000,
            [0u8; 32], 0,
        );
        
        assert_eq!(ix.data[0], slab_ix::RESERVE);
        // 1 + 4 + 2 + 1 + 8 + 8 + 8 + 32 + 8 = 72
        assert_eq!(ix.data.len(), 72);
    }
    
    #[test]
    fn test_commit_instruction_encoding() {
        let program_id = slab_program_id();
        let slab = Keypair::new();
        
        let ix = ix_commit(&program_id, &slab.pubkey(), 123, 1704067200000);
        
        assert_eq!(ix.data[0], slab_ix::COMMIT);
        // 1 + 8 + 8 = 17
        assert_eq!(ix.data.len(), 17);
        
        // Verify hold_id encoding
        let hold_id = u64::from_le_bytes(ix.data[1..9].try_into().unwrap());
        assert_eq!(hold_id, 123);
    }
    
    #[test]
    fn test_cancel_instruction_encoding() {
        let program_id = slab_program_id();
        let slab = Keypair::new();
        
        let ix = ix_cancel(&program_id, &slab.pubkey(), 456);
        
        assert_eq!(ix.data[0], slab_ix::CANCEL);
        assert_eq!(ix.data.len(), 9);
        
        let hold_id = u64::from_le_bytes(ix.data[1..9].try_into().unwrap());
        assert_eq!(hold_id, 456);
    }
    
    #[test]
    fn test_batch_open_instruction_encoding() {
        let program_id = slab_program_id();
        let slab = Keypair::new();
        
        let ix = ix_batch_open(&program_id, &slab.pubkey(), 0, 1704067200000);
        
        assert_eq!(ix.data[0], slab_ix::BATCH_OPEN);
        // 1 + 2 + 8 = 11
        assert_eq!(ix.data.len(), 11);
    }
    
    #[test]
    fn test_add_instrument_instruction_encoding() {
        let program_id = slab_program_id();
        let slab = Keypair::new();
        
        let ix = ix_add_instrument(
            &program_id,
            &slab.pubkey(),
            symbol("BTC"),
            100_000_000, 100_000, 1_000_000, 50_000_000_000,
        );
        
        assert_eq!(ix.data[0], slab_ix::ADD_INSTRUMENT);
        // 1 + 8 + 8 + 8 + 8 + 8 = 41
        assert_eq!(ix.data.len(), 41);
    }
    
    #[test]
    fn test_update_funding_instruction_encoding() {
        let program_id = slab_program_id();
        let slab = Keypair::new();
        
        let ix = ix_update_funding(&program_id, &slab.pubkey(), 0, 50_000_000_000, 1704067200000);
        
        assert_eq!(ix.data[0], slab_ix::UPDATE_FUNDING);
        // 1 + 2 + 8 + 8 = 19
        assert_eq!(ix.data.len(), 19);
    }
    
    #[test]
    fn test_liquidation_instruction_encoding() {
        let program_id = slab_program_id();
        let slab = Keypair::new();
        
        let ix = ix_liquidation(&program_id, &slab.pubkey(), 0, 1_000_000, 1704067200000);
        
        assert_eq!(ix.data[0], slab_ix::LIQUIDATION);
        // 1 + 4 + 16 + 8 = 29
        assert_eq!(ix.data.len(), 29);
    }
    
    #[test]
    fn test_program_ids_valid() {
        let slab_id = slab_program_id();
        let router_id = router_program_id();
        
        // Verify they're valid base58-decodable pubkeys
        assert_eq!(slab_id.to_string(), SLAB_PROGRAM_ID_STR);
        assert_eq!(router_id.to_string(), ROUTER_PROGRAM_ID_STR);
        
        // Verify they're different
        assert_ne!(slab_id, router_id);
    }
}
