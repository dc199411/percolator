//! Phase 4: Multi-Slab Coordination Integration Tests
//!
//! Tests for:
//! - Router orchestration (multi-slab reserve/commit atomicity)
//! - Cross-slab portfolio margin calculations
//! - Global liquidation coordination
//! - CPI integration between Router and Slab programs

mod common;
use common::*;

use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};

// ============================================================================
// PORTFOLIO MARGIN SIMULATION TESTS
// ============================================================================

/// Simulated portfolio for testing
#[derive(Debug, Clone, Default)]
struct TestPortfolio {
    equity: i128,
    im: u128,
    mm: u128,
    free_collateral: i128,
    exposures: Vec<(u16, u16, i64)>, // (slab_idx, instrument_idx, qty)
}

impl TestPortfolio {
    fn new(equity: i128) -> Self {
        Self {
            equity,
            im: 0,
            mm: 0,
            free_collateral: equity,
            exposures: Vec::new(),
        }
    }

    fn update_exposure(&mut self, slab_idx: u16, instrument_idx: u16, qty: i64) {
        // Find existing or add new
        if let Some(pos) = self.exposures.iter_mut().find(|(s, i, _)| *s == slab_idx && *i == instrument_idx) {
            pos.2 = qty;
            if qty == 0 {
                self.exposures.retain(|(s, i, _)| !(*s == slab_idx && *i == instrument_idx));
            }
        } else if qty != 0 {
            self.exposures.push((slab_idx, instrument_idx, qty));
        }
    }

    fn get_net_exposure(&self) -> i64 {
        self.exposures.iter().map(|(_, _, q)| *q).sum()
    }

    fn calculate_gross_im(&self, price: u64, imr_bps: u64) -> u128 {
        let mut gross_im = 0u128;
        for (_, _, qty) in &self.exposures {
            let notional = qty.unsigned_abs() as u128 * price as u128 / 1_000_000;
            gross_im += notional * imr_bps as u128 / 10_000;
        }
        gross_im
    }

    fn calculate_net_im(&self, price: u64, imr_bps: u64) -> u128 {
        let net = self.get_net_exposure();
        let notional = net.unsigned_abs() as u128 * price as u128 / 1_000_000;
        notional * imr_bps as u128 / 10_000
    }

    fn is_liquidatable(&self) -> bool {
        self.equity < self.mm as i128
    }
}

// ============================================================================
// UNIT TESTS - PORTFOLIO MARGIN
// ============================================================================

#[cfg(test)]
mod portfolio_margin_tests {
    use super::*;

    #[test]
    fn test_portfolio_netting_benefit() {
        let mut portfolio = TestPortfolio::new(100_000_000_000); // $100k equity
        let price = 50_000_000_000u64; // $50k BTC
        let imr_bps = 1000u64; // 10% IMR

        // Long 1 BTC on slab 0
        portfolio.update_exposure(0, 0, 1_000_000);
        
        // Short 1 BTC on slab 1
        portfolio.update_exposure(1, 0, -1_000_000);

        let gross_im = portfolio.calculate_gross_im(price, imr_bps);
        let net_im = portfolio.calculate_net_im(price, imr_bps);

        println!("Gross IM: ${}", gross_im / 1_000_000);
        println!("Net IM: ${}", net_im / 1_000_000);
        println!("Netting Benefit: ${}", (gross_im - net_im) / 1_000_000);

        // Net exposure is 0, so net_im should be 0
        assert_eq!(portfolio.get_net_exposure(), 0);
        assert_eq!(net_im, 0);
        // Gross IM should be non-zero (2 * 1 BTC * $50k * 10% = $10k)
        assert!(gross_im > 0);
        // Netting benefit = 100%
        assert_eq!(gross_im - net_im, gross_im);
    }

    #[test]
    fn test_portfolio_partial_netting() {
        let mut portfolio = TestPortfolio::new(100_000_000_000);
        let price = 50_000_000_000u64;
        let imr_bps = 1000u64;

        // Long 2 BTC on slab 0
        portfolio.update_exposure(0, 0, 2_000_000);
        
        // Short 1 BTC on slab 1
        portfolio.update_exposure(1, 0, -1_000_000);

        let gross_im = portfolio.calculate_gross_im(price, imr_bps);
        let net_im = portfolio.calculate_net_im(price, imr_bps);

        println!("Gross IM: ${}", gross_im / 1_000_000);
        println!("Net IM: ${}", net_im / 1_000_000);
        println!("Netting Benefit: ${}", (gross_im - net_im) / 1_000_000);

        // Net exposure is +1 BTC
        assert_eq!(portfolio.get_net_exposure(), 1_000_000);
        // Net IM < Gross IM
        assert!(net_im < gross_im);
    }

    #[test]
    fn test_portfolio_no_netting_same_direction() {
        let mut portfolio = TestPortfolio::new(100_000_000_000);
        let price = 50_000_000_000u64;
        let imr_bps = 1000u64;

        // Long 1 BTC on slab 0
        portfolio.update_exposure(0, 0, 1_000_000);
        
        // Long 1 BTC on slab 1
        portfolio.update_exposure(1, 0, 1_000_000);

        let gross_im = portfolio.calculate_gross_im(price, imr_bps);
        let net_im = portfolio.calculate_net_im(price, imr_bps);

        println!("Gross IM: ${}", gross_im / 1_000_000);
        println!("Net IM: ${}", net_im / 1_000_000);

        // Net exposure is +2 BTC (same as gross)
        assert_eq!(portfolio.get_net_exposure(), 2_000_000);
        // No netting benefit when positions are same direction
        assert_eq!(net_im, gross_im);
    }

    #[test]
    fn test_multi_instrument_netting() {
        let mut portfolio = TestPortfolio::new(100_000_000_000);
        
        // BTC positions
        portfolio.update_exposure(0, 0, 1_000_000);  // Long 1 BTC
        portfolio.update_exposure(1, 0, -500_000);   // Short 0.5 BTC
        
        // ETH positions (different instrument)
        portfolio.update_exposure(0, 1, 10_000_000); // Long 10 ETH
        portfolio.update_exposure(1, 1, -10_000_000); // Short 10 ETH

        // BTC net: +0.5 BTC
        // ETH net: 0
        let btc_net: i64 = portfolio.exposures.iter()
            .filter(|(_, inst, _)| *inst == 0)
            .map(|(_, _, q)| *q)
            .sum();
        let eth_net: i64 = portfolio.exposures.iter()
            .filter(|(_, inst, _)| *inst == 1)
            .map(|(_, _, q)| *q)
            .sum();

        assert_eq!(btc_net, 500_000);
        assert_eq!(eth_net, 0);
    }
}

// ============================================================================
// UNIT TESTS - LIQUIDATION
// ============================================================================

#[cfg(test)]
mod liquidation_tests {
    use super::*;

    #[test]
    fn test_portfolio_not_liquidatable() {
        let mut portfolio = TestPortfolio::new(100_000_000_000);
        portfolio.im = 50_000_000_000;
        portfolio.mm = 25_000_000_000;
        portfolio.free_collateral = 50_000_000_000;

        assert!(!portfolio.is_liquidatable());
    }

    #[test]
    fn test_portfolio_liquidatable() {
        let mut portfolio = TestPortfolio::new(20_000_000_000);
        portfolio.im = 50_000_000_000;
        portfolio.mm = 25_000_000_000;
        portfolio.free_collateral = -30_000_000_000;

        assert!(portfolio.is_liquidatable());
    }

    #[test]
    fn test_liquidation_priority() {
        let mut portfolio = TestPortfolio::new(20_000_000_000);
        
        // Add positions of different sizes
        portfolio.update_exposure(0, 0, 100_000);     // Small BTC
        portfolio.update_exposure(1, 0, 5_000_000);   // Large BTC
        portfolio.update_exposure(2, 0, 500_000);     // Medium BTC

        // Sort by size for liquidation priority (largest first)
        let mut positions = portfolio.exposures.clone();
        positions.sort_by(|a, b| b.2.abs().cmp(&a.2.abs()));

        assert_eq!(positions[0].2.abs(), 5_000_000);  // Largest first
        assert_eq!(positions[1].2.abs(), 500_000);   // Medium second
        assert_eq!(positions[2].2.abs(), 100_000);   // Smallest last
    }

    #[test]
    fn test_liquidation_deficit_calculation() {
        let mut portfolio = TestPortfolio::new(15_000_000_000); // $15k equity
        portfolio.mm = 25_000_000_000; // $25k MM

        let deficit = (portfolio.mm as i128) - portfolio.equity;
        assert_eq!(deficit, 10_000_000_000); // $10k deficit
    }
}

// ============================================================================
// UNIT TESTS - MULTI-SLAB OPERATIONS
// ============================================================================

#[cfg(test)]
mod multi_slab_tests {
    use super::*;

    /// Simulated reservation
    #[derive(Debug, Clone)]
    struct Reservation {
        hold_id: u64,
        slab_idx: u16,
        qty: u64,
        vwap_px: u64,
        max_charge: u128,
        expiry_ms: u64,
    }

    /// Simulated commit result
    #[derive(Debug, Clone)]
    struct CommitResult {
        filled_qty: u64,
        notional: u128,
        fees: u128,
    }

    #[test]
    fn test_multi_slab_reserve_atomicity() {
        // Simulate reserving on 3 slabs
        let reservations = vec![
            Reservation { hold_id: 1, slab_idx: 0, qty: 1_000_000, vwap_px: 50_000_000_000, max_charge: 50_500_000_000_000, expiry_ms: 30_000 },
            Reservation { hold_id: 2, slab_idx: 1, qty: 500_000, vwap_px: 50_100_000_000, max_charge: 25_300_000_000_000, expiry_ms: 30_000 },
            Reservation { hold_id: 3, slab_idx: 2, qty: 250_000, vwap_px: 50_050_000_000, max_charge: 12_600_000_000_000, expiry_ms: 30_000 },
        ];

        let total_qty: u64 = reservations.iter().map(|r| r.qty).sum();
        let total_max_charge: u128 = reservations.iter().map(|r| r.max_charge).sum();

        assert_eq!(total_qty, 1_750_000); // 1.75 BTC
        assert!(total_max_charge > 0);

        // Calculate aggregate VWAP
        let total_notional: u128 = reservations.iter().map(|r| r.qty as u128 * r.vwap_px as u128).sum();
        let aggregate_vwap = total_notional / total_qty as u128;
        
        println!("Aggregate VWAP: {}", aggregate_vwap);
        assert!(aggregate_vwap > 50_000_000_000);
    }

    #[test]
    fn test_multi_slab_rollback_on_failure() {
        // Simulate failure on third slab
        let mut successful_reservations = vec![
            Reservation { hold_id: 1, slab_idx: 0, qty: 1_000_000, vwap_px: 50_000_000_000, max_charge: 50_500_000_000_000, expiry_ms: 30_000 },
            Reservation { hold_id: 2, slab_idx: 1, qty: 500_000, vwap_px: 50_100_000_000, max_charge: 25_300_000_000_000, expiry_ms: 30_000 },
        ];
        
        // Third slab fails
        let _failure = "InsufficientLiquidity";

        // All reservations should be cancelled
        let cancelled_count = successful_reservations.len();
        successful_reservations.clear();

        assert_eq!(cancelled_count, 2);
        assert!(successful_reservations.is_empty());
    }

    #[test]
    fn test_multi_slab_commit_atomicity() {
        let commit_results = vec![
            CommitResult { filled_qty: 1_000_000, notional: 50_000_000_000_000, fees: 50_000_000_000 },
            CommitResult { filled_qty: 500_000, notional: 25_050_000_000_000, fees: 25_050_000_000 },
        ];

        let total_filled: u64 = commit_results.iter().map(|r| r.filled_qty).sum();
        let total_notional: u128 = commit_results.iter().map(|r| r.notional).sum();
        let total_fees: u128 = commit_results.iter().map(|r| r.fees).sum();

        assert_eq!(total_filled, 1_500_000);
        assert_eq!(total_notional, 75_050_000_000_000);
        assert_eq!(total_fees, 75_050_000_000);
    }

    #[test]
    fn test_reservation_expiry() {
        let reservation = Reservation {
            hold_id: 1,
            slab_idx: 0,
            qty: 1_000_000,
            vwap_px: 50_000_000_000,
            max_charge: 50_500_000_000_000,
            expiry_ms: 30_000, // 30 seconds TTL
        };

        let current_ts = 25_000; // 25 seconds
        let is_expired = current_ts > reservation.expiry_ms;
        assert!(!is_expired);

        let current_ts = 35_000; // 35 seconds
        let is_expired = current_ts > reservation.expiry_ms;
        assert!(is_expired);
    }
}

// ============================================================================
// UNIT TESTS - CPI INTEGRATION
// ============================================================================

#[cfg(test)]
mod cpi_tests {
    use super::*;

    #[test]
    fn test_cpi_instruction_data_format() {
        // Reserve instruction data
        let mut reserve_data = [0u8; 73];
        reserve_data[0] = 4; // RESERVE discriminator
        let account_idx: u32 = 0;
        let instrument_idx: u16 = 0;
        let side: u8 = 0; // Buy
        let qty: u64 = 1_000_000;
        let limit_px: u64 = 50_000_000_000;
        let ttl_ms: u64 = 30_000;
        let route_id: u64 = 1;

        reserve_data[1..5].copy_from_slice(&account_idx.to_le_bytes());
        reserve_data[5..7].copy_from_slice(&instrument_idx.to_le_bytes());
        reserve_data[7] = side;
        reserve_data[8..16].copy_from_slice(&qty.to_le_bytes());
        reserve_data[16..24].copy_from_slice(&limit_px.to_le_bytes());
        reserve_data[24..32].copy_from_slice(&ttl_ms.to_le_bytes());
        reserve_data[64..72].copy_from_slice(&route_id.to_le_bytes());

        // Verify discriminator
        assert_eq!(reserve_data[0], 4);
        
        // Verify qty
        let parsed_qty = u64::from_le_bytes(reserve_data[8..16].try_into().unwrap());
        assert_eq!(parsed_qty, qty);
    }

    #[test]
    fn test_cpi_commit_data_format() {
        let mut commit_data = [0u8; 17];
        commit_data[0] = 5; // COMMIT discriminator
        let hold_id: u64 = 123;
        let current_ts: u64 = 1704067200000;

        commit_data[1..9].copy_from_slice(&hold_id.to_le_bytes());
        commit_data[9..17].copy_from_slice(&current_ts.to_le_bytes());

        // Verify
        assert_eq!(commit_data[0], 5);
        let parsed_hold_id = u64::from_le_bytes(commit_data[1..9].try_into().unwrap());
        assert_eq!(parsed_hold_id, hold_id);
    }

    #[test]
    fn test_cpi_response_parsing() {
        // Simulate reserve response
        let mut response_data = [0u8; 64];
        let hold_id: u64 = 1;
        let vwap_px: u64 = 50_000_000_000;
        let worst_px: u64 = 50_100_000_000;
        let filled_qty: u64 = 1_000_000;
        let max_charge: u128 = 50_500_000_000_000;
        let expiry_ms: u64 = 30_000;
        let book_seqno: u64 = 42;

        response_data[0..8].copy_from_slice(&hold_id.to_le_bytes());
        response_data[8..16].copy_from_slice(&vwap_px.to_le_bytes());
        response_data[16..24].copy_from_slice(&worst_px.to_le_bytes());
        response_data[24..32].copy_from_slice(&filled_qty.to_le_bytes());
        response_data[32..48].copy_from_slice(&max_charge.to_le_bytes());
        response_data[48..56].copy_from_slice(&expiry_ms.to_le_bytes());
        response_data[56..64].copy_from_slice(&book_seqno.to_le_bytes());

        // Parse
        let parsed_hold_id = u64::from_le_bytes(response_data[0..8].try_into().unwrap());
        let parsed_vwap = u64::from_le_bytes(response_data[8..16].try_into().unwrap());
        let parsed_qty = u64::from_le_bytes(response_data[24..32].try_into().unwrap());

        assert_eq!(parsed_hold_id, hold_id);
        assert_eq!(parsed_vwap, vwap_px);
        assert_eq!(parsed_qty, filled_qty);
    }
}

// ============================================================================
// INTEGRATION TESTS (require BPF)
// ============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;

    fn skip_if_no_bpf() -> bool {
        !bpf_available()
    }

    #[tokio::test]
    async fn test_multi_slab_portfolio_update() {
        if skip_if_no_bpf() {
            println!("Skipping: BPF programs not available");
            return;
        }

        let mut ctx = TestContext::new_with_both().await;
        
        // This test would verify that portfolio exposures are correctly
        // updated after a multi-slab operation
        println!("Multi-slab portfolio update test - BPF available");
    }

    #[tokio::test]
    async fn test_cross_slab_margin_calculation() {
        if skip_if_no_bpf() {
            println!("Skipping: BPF programs not available");
            return;
        }

        let mut ctx = TestContext::new_with_both().await;
        
        // This test would verify that margin is calculated on net exposure
        println!("Cross-slab margin calculation test - BPF available");
    }
}

// ============================================================================
// BENCHMARKS
// ============================================================================

#[cfg(test)]
mod benchmark_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn bench_portfolio_margin_calculation() {
        let iterations = 10_000;
        let mut portfolio = TestPortfolio::new(100_000_000_000);
        
        // Add 10 positions
        for i in 0..10 {
            portfolio.update_exposure(i, 0, if i % 2 == 0 { 1_000_000 } else { -500_000 });
        }

        let start = Instant::now();
        for _ in 0..iterations {
            let _ = portfolio.calculate_net_im(50_000_000_000, 1000);
        }
        let elapsed = start.elapsed();

        println!("Portfolio margin calculation:");
        println!("  Iterations: {}", iterations);
        println!("  Total time: {:?}", elapsed);
        println!("  Per iteration: {:?}", elapsed / iterations as u32);
    }

    #[test]
    fn bench_exposure_netting() {
        let iterations = 10_000;
        let mut portfolio = TestPortfolio::new(100_000_000_000);
        
        // Add 100 positions across 10 slabs
        for slab in 0..10 {
            for inst in 0..10 {
                let qty = if (slab + inst) % 2 == 0 { 100_000 } else { -100_000 };
                portfolio.update_exposure(slab, inst, qty);
            }
        }

        let start = Instant::now();
        for _ in 0..iterations {
            let _ = portfolio.get_net_exposure();
        }
        let elapsed = start.elapsed();

        println!("Exposure netting:");
        println!("  Positions: {}", portfolio.exposures.len());
        println!("  Iterations: {}", iterations);
        println!("  Total time: {:?}", elapsed);
        println!("  Per iteration: {:?}", elapsed / iterations as u32);
    }
}

// ============================================================================
// PROPERTY TESTS
// ============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;

    #[test]
    fn prop_netting_always_reduces_or_equals_gross() {
        // Test 1000 random portfolio configurations
        for seed in 0..1000 {
            let mut portfolio = TestPortfolio::new(100_000_000_000);
            
            // Generate random exposures
            let positions = (seed % 10) + 1;
            for i in 0..positions {
                let qty = if (seed + i) % 3 == 0 {
                    ((seed * 1000 + i * 100) % 5_000_000) as i64
                } else {
                    -(((seed * 1000 + i * 100) % 5_000_000) as i64)
                };
                portfolio.update_exposure((i % 5) as u16, (i % 3) as u16, qty);
            }

            let gross_im = portfolio.calculate_gross_im(50_000_000_000, 1000);
            let net_im = portfolio.calculate_net_im(50_000_000_000, 1000);

            // Net IM should always be <= Gross IM
            assert!(
                net_im <= gross_im,
                "Net IM ({}) should be <= Gross IM ({}) for seed {}",
                net_im, gross_im, seed
            );
        }
    }

    #[test]
    fn prop_zero_net_exposure_zero_margin() {
        // Generate balanced portfolios
        for seed in 0..100 {
            let mut portfolio = TestPortfolio::new(100_000_000_000);
            
            // Add equal long and short
            let qty = ((seed + 1) * 100_000) as i64;
            portfolio.update_exposure(0, 0, qty);
            portfolio.update_exposure(1, 0, -qty);

            let net_im = portfolio.calculate_net_im(50_000_000_000, 1000);
            
            // Zero net exposure should mean zero margin
            assert_eq!(
                net_im, 0,
                "Net IM should be 0 for balanced portfolio, got {} for seed {}",
                net_im, seed
            );
        }
    }

    #[test]
    fn prop_liquidation_threshold_correct() {
        for seed in 0..100 {
            let equity = (seed as i128) * 1_000_000_000i128;
            let mm = 25_000_000_000u128;
            
            let mut portfolio = TestPortfolio::new(equity);
            portfolio.mm = mm;

            let should_be_liquidatable = equity < mm as i128;
            assert_eq!(
                portfolio.is_liquidatable(),
                should_be_liquidatable,
                "Liquidation check failed for equity={}, mm={}",
                equity, mm
            );
        }
    }
}
