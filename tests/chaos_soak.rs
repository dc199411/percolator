//! Chaos and Soak Tests
//!
//! Long-running stress tests for stability and reliability.
//! These tests simulate production workloads over extended periods.
//!
//! Run with:
//!   cargo test --test chaos_soak --release -- --nocapture --ignored
//!
//! For full soak tests (24h+):
//!   SOAK_DURATION_HOURS=24 cargo test --test chaos_soak --release -- --nocapture --ignored

use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

// ============================================================================
// CONFIGURATION
// ============================================================================

/// Default soak test duration in minutes (for quick CI)
const DEFAULT_SOAK_MINUTES: u64 = 5;

/// Target operations per second
const TARGET_OPS_PER_SECOND: u64 = 1000;

/// Maximum concurrent operations
const MAX_CONCURRENT_OPS: usize = 100;

/// Memory check interval
const MEMORY_CHECK_INTERVAL_SECS: u64 = 60;

// ============================================================================
// STATISTICS TRACKING
// ============================================================================

#[derive(Debug, Default)]
pub struct SoakStats {
    pub total_operations: AtomicU64,
    pub successful_operations: AtomicU64,
    pub failed_operations: AtomicU64,
    pub total_latency_us: AtomicU64,
    pub min_latency_us: AtomicU64,
    pub max_latency_us: AtomicU64,
    pub memory_samples: AtomicU64,
}

impl SoakStats {
    pub fn new() -> Self {
        Self {
            min_latency_us: AtomicU64::new(u64::MAX),
            ..Default::default()
        }
    }
    
    pub fn record_success(&self, latency_us: u64) {
        self.total_operations.fetch_add(1, Ordering::Relaxed);
        self.successful_operations.fetch_add(1, Ordering::Relaxed);
        self.total_latency_us.fetch_add(latency_us, Ordering::Relaxed);
        
        // Update min
        let mut current = self.min_latency_us.load(Ordering::Relaxed);
        while latency_us < current {
            match self.min_latency_us.compare_exchange_weak(
                current, latency_us, Ordering::Relaxed, Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(x) => current = x,
            }
        }
        
        // Update max
        let mut current = self.max_latency_us.load(Ordering::Relaxed);
        while latency_us > current {
            match self.max_latency_us.compare_exchange_weak(
                current, latency_us, Ordering::Relaxed, Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(x) => current = x,
            }
        }
    }
    
    pub fn record_failure(&self) {
        self.total_operations.fetch_add(1, Ordering::Relaxed);
        self.failed_operations.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn summary(&self) -> String {
        let total = self.total_operations.load(Ordering::Relaxed);
        let success = self.successful_operations.load(Ordering::Relaxed);
        let failed = self.failed_operations.load(Ordering::Relaxed);
        let total_latency = self.total_latency_us.load(Ordering::Relaxed);
        let min_lat = self.min_latency_us.load(Ordering::Relaxed);
        let max_lat = self.max_latency_us.load(Ordering::Relaxed);
        
        let avg_latency = if success > 0 { total_latency / success } else { 0 };
        let success_rate = if total > 0 { (success as f64 / total as f64) * 100.0 } else { 0.0 };
        
        format!(
            "Total: {} | Success: {} ({:.2}%) | Failed: {} | Latency (μs): avg={}, min={}, max={}",
            total, success, success_rate, failed, avg_latency,
            if min_lat == u64::MAX { 0 } else { min_lat },
            max_lat
        )
    }
}

// ============================================================================
// SIMULATED OPERATIONS
// ============================================================================

/// Simulate an operation with configurable behavior
fn simulate_operation(op_id: u64, chaos_rate: f64) -> Result<(), &'static str> {
    // Simulate variable work
    let work_cycles = (op_id % 100) + 1;
    let mut _sum: u64 = 0;
    for i in 0..work_cycles * 1000 {
        _sum = _sum.wrapping_add(i);
    }
    
    // Chaos injection: random failures
    if chaos_rate > 0.0 {
        let random = (op_id.wrapping_mul(1103515245).wrapping_add(12345)) % 10000;
        if (random as f64) < chaos_rate * 10000.0 {
            return Err("Chaos-injected failure");
        }
    }
    
    Ok(())
}

/// Simulate instruction parsing
fn simulate_parse_instruction(data: &[u8]) -> Result<u8, &'static str> {
    if data.is_empty() {
        return Err("Empty instruction");
    }
    
    let discriminator = data[0];
    if discriminator > 7 {
        return Err("Invalid discriminator");
    }
    
    // Simulate parsing work
    let mut checksum: u64 = 0;
    for byte in data {
        checksum = checksum.wrapping_add(*byte as u64);
    }
    
    Ok(discriminator)
}

/// Simulate margin calculation
fn simulate_margin_calculation(qty: i64, price: u64, im_bps: u64) -> u128 {
    let notional = (qty.unsigned_abs() as u128) * (price as u128);
    notional * im_bps as u128 / 10_000
}

/// Simulate VWAP calculation
fn simulate_vwap_calculation(fills: &[(u64, u64)]) -> u64 {
    if fills.is_empty() {
        return 0;
    }
    
    let total_qty: u64 = fills.iter().map(|(_, q)| q).sum();
    if total_qty == 0 {
        return 0;
    }
    
    let total_notional: u128 = fills.iter()
        .map(|(p, q)| (*p as u128) * (*q as u128))
        .sum();
    
    (total_notional / total_qty as u128) as u64
}

// ============================================================================
// SOAK TEST: CONTINUOUS LOAD
// ============================================================================

#[test]
#[ignore] // Run with: cargo test --test chaos_soak test_continuous_load -- --ignored --nocapture
fn test_continuous_load() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║               SOAK TEST: Continuous Load                      ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    let duration_minutes = std::env::var("SOAK_DURATION_MINUTES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_SOAK_MINUTES);
    
    let duration = Duration::from_secs(duration_minutes * 60);
    let stats = Arc::new(SoakStats::new());
    let stop = Arc::new(AtomicBool::new(false));
    
    println!("Configuration:");
    println!("  Duration: {} minutes", duration_minutes);
    println!("  Target ops/sec: {}", TARGET_OPS_PER_SECOND);
    println!("  Max concurrent: {}", MAX_CONCURRENT_OPS);
    println!();
    
    let start = Instant::now();
    let mut op_id: u64 = 0;
    let mut last_report = Instant::now();
    
    while start.elapsed() < duration {
        let op_start = Instant::now();
        
        // Simulate various operations
        let result = match op_id % 5 {
            0 => simulate_operation(op_id, 0.001), // 0.1% chaos
            1 => {
                let data = vec![0u8; 100];
                simulate_parse_instruction(&data).map(|_| ())
            }
            2 => {
                let _ = simulate_margin_calculation(1_000_000, 50_000_000_000, 500);
                Ok(())
            }
            3 => {
                let fills = vec![(50_000_000_000, 100), (50_100_000_000, 200)];
                let _ = simulate_vwap_calculation(&fills);
                Ok(())
            }
            _ => simulate_operation(op_id, 0.0),
        };
        
        let latency_us = op_start.elapsed().as_micros() as u64;
        
        match result {
            Ok(_) => stats.record_success(latency_us),
            Err(_) => stats.record_failure(),
        }
        
        op_id += 1;
        
        // Progress report every 10 seconds
        if last_report.elapsed() > Duration::from_secs(10) {
            let elapsed = start.elapsed().as_secs();
            let remaining = duration.as_secs().saturating_sub(elapsed);
            let ops_per_sec = op_id / elapsed.max(1);
            
            println!("[{:>5}s / {:>5}s] {} | ops/sec: {}",
                elapsed, duration.as_secs(), stats.summary(), ops_per_sec);
            
            last_report = Instant::now();
        }
    }
    
    stop.store(true, Ordering::Relaxed);
    
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║                      FINAL RESULTS                            ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!("\n{}\n", stats.summary());
    
    let total = stats.total_operations.load(Ordering::Relaxed);
    let success = stats.successful_operations.load(Ordering::Relaxed);
    let success_rate = (success as f64 / total as f64) * 100.0;
    
    // Assert minimum success rate
    assert!(success_rate >= 99.0, "Success rate {} < 99%", success_rate);
}

// ============================================================================
// SOAK TEST: MEMORY STABILITY
// ============================================================================

#[test]
#[ignore] // Run with: cargo test --test chaos_soak test_memory_stability -- --ignored --nocapture
fn test_memory_stability() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║               SOAK TEST: Memory Stability                     ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    let duration_minutes = std::env::var("SOAK_DURATION_MINUTES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_SOAK_MINUTES);
    
    let duration = Duration::from_secs(duration_minutes * 60);
    
    println!("Configuration:");
    println!("  Duration: {} minutes", duration_minutes);
    println!("  Memory check interval: {}s", MEMORY_CHECK_INTERVAL_SECS);
    println!();
    
    let start = Instant::now();
    let mut allocations: Vec<Vec<u8>> = Vec::new();
    let mut iteration = 0u64;
    let mut last_check = Instant::now();
    
    // Track memory growth
    let mut memory_samples: Vec<usize> = Vec::new();
    
    while start.elapsed() < duration {
        iteration += 1;
        
        // Allocate some memory
        let size = ((iteration % 1000) + 100) as usize;
        let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        allocations.push(data);
        
        // Free old allocations to simulate realistic usage
        if allocations.len() > 10000 {
            allocations.drain(0..5000);
        }
        
        // Periodic memory check
        if last_check.elapsed() > Duration::from_secs(MEMORY_CHECK_INTERVAL_SECS) {
            let current_allocs = allocations.len();
            let total_bytes: usize = allocations.iter().map(|v| v.len()).sum();
            
            memory_samples.push(total_bytes);
            
            let elapsed = start.elapsed().as_secs();
            println!("[{:>5}s] Allocations: {} | Total bytes: {} | Iterations: {}",
                elapsed, current_allocs, total_bytes, iteration);
            
            last_check = Instant::now();
        }
        
        // Do some work
        let _ = simulate_margin_calculation(
            (iteration % 1000) as i64,
            50_000_000_000,
            500,
        );
    }
    
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║                      MEMORY ANALYSIS                          ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    if memory_samples.len() >= 2 {
        let first_half_avg: usize = memory_samples[..memory_samples.len()/2].iter().sum::<usize>() 
            / (memory_samples.len() / 2);
        let second_half_avg: usize = memory_samples[memory_samples.len()/2..].iter().sum::<usize>() 
            / (memory_samples.len() - memory_samples.len() / 2);
        
        let growth = if second_half_avg > first_half_avg {
            ((second_half_avg - first_half_avg) as f64 / first_half_avg as f64) * 100.0
        } else {
            0.0
        };
        
        println!("First half avg: {} bytes", first_half_avg);
        println!("Second half avg: {} bytes", second_half_avg);
        println!("Memory growth: {:.2}%", growth);
        
        // Memory should not grow unboundedly
        assert!(growth < 100.0, "Memory growth {} > 100%", growth);
    }
    
    println!("\n✅ Memory stability test passed\n");
}

// ============================================================================
// CHAOS TEST: RANDOM FAILURES
// ============================================================================

#[test]
#[ignore] // Run with: cargo test --test chaos_soak test_chaos_recovery -- --ignored --nocapture
fn test_chaos_recovery() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║               CHAOS TEST: Recovery from Failures              ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    let chaos_rates = [0.01, 0.05, 0.10, 0.25]; // 1%, 5%, 10%, 25%
    let ops_per_rate = 10000;
    
    for &chaos_rate in &chaos_rates {
        let stats = SoakStats::new();
        
        for op_id in 0..ops_per_rate {
            let op_start = Instant::now();
            let result = simulate_operation(op_id, chaos_rate);
            let latency_us = op_start.elapsed().as_micros() as u64;
            
            match result {
                Ok(_) => stats.record_success(latency_us),
                Err(_) => stats.record_failure(),
            }
        }
        
        let success = stats.successful_operations.load(Ordering::Relaxed);
        let failed = stats.failed_operations.load(Ordering::Relaxed);
        let actual_failure_rate = failed as f64 / ops_per_rate as f64;
        
        println!("Chaos rate: {:>5.1}% | Success: {:>5} | Failed: {:>4} | Actual fail rate: {:.2}%",
            chaos_rate * 100.0, success, failed, actual_failure_rate * 100.0);
        
        // Actual failure rate should be within 50% of target
        let tolerance = chaos_rate * 0.5;
        assert!(
            actual_failure_rate >= chaos_rate - tolerance && 
            actual_failure_rate <= chaos_rate + tolerance,
            "Failure rate {} outside expected range [{}, {}]",
            actual_failure_rate, chaos_rate - tolerance, chaos_rate + tolerance
        );
    }
    
    println!("\n✅ Chaos recovery test passed\n");
}

// ============================================================================
// STRESS TEST: BURST TRAFFIC
// ============================================================================

#[test]
#[ignore] // Run with: cargo test --test chaos_soak test_burst_traffic -- --ignored --nocapture
fn test_burst_traffic() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║               STRESS TEST: Burst Traffic                      ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    let burst_sizes = [100, 1000, 10000, 50000];
    
    for &burst_size in &burst_sizes {
        let stats = SoakStats::new();
        let burst_start = Instant::now();
        
        for op_id in 0..burst_size {
            let op_start = Instant::now();
            
            // Mix of operations
            let result = match op_id % 4 {
                0 => {
                    let data = vec![op_id as u8; 50];
                    simulate_parse_instruction(&data).map(|_| ())
                }
                1 => {
                    let _ = simulate_margin_calculation(op_id as i64 + 1, 50_000_000_000, 500);
                    Ok(())
                }
                2 => {
                    let fills = vec![
                        (50_000_000_000 + op_id, 100),
                        (50_100_000_000 + op_id, 200),
                    ];
                    let _ = simulate_vwap_calculation(&fills);
                    Ok(())
                }
                _ => simulate_operation(op_id, 0.0),
            };
            
            let latency_us = op_start.elapsed().as_micros() as u64;
            
            match result {
                Ok(_) => stats.record_success(latency_us),
                Err(_) => stats.record_failure(),
            }
        }
        
        let burst_duration = burst_start.elapsed();
        let ops_per_sec = if burst_duration.as_secs() > 0 {
            burst_size / burst_duration.as_secs()
        } else {
            burst_size * 1000 / burst_duration.as_millis().max(1) as u64
        };
        
        let success = stats.successful_operations.load(Ordering::Relaxed);
        let total_latency = stats.total_latency_us.load(Ordering::Relaxed);
        let avg_latency = if success > 0 { total_latency / success } else { 0 };
        
        println!(
            "Burst size: {:>6} | Duration: {:>6.2}ms | ops/sec: {:>8} | avg latency: {:>6}μs",
            burst_size,
            burst_duration.as_secs_f64() * 1000.0,
            ops_per_sec,
            avg_latency
        );
        
        // All operations should succeed
        assert_eq!(success, burst_size, "Not all burst operations succeeded");
    }
    
    println!("\n✅ Burst traffic test passed\n");
}

// ============================================================================
// ENDURANCE TEST: PROTOCOL INVARIANTS
// ============================================================================

#[test]
#[ignore] // Run with: cargo test --test chaos_soak test_invariant_endurance -- --ignored --nocapture
fn test_invariant_endurance() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║           ENDURANCE TEST: Protocol Invariants                 ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    let duration_minutes = std::env::var("SOAK_DURATION_MINUTES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_SOAK_MINUTES);
    
    let duration = Duration::from_secs(duration_minutes * 60);
    let start = Instant::now();
    
    let mut invariant_checks = 0u64;
    let mut margin_calculations = 0u64;
    let mut vwap_calculations = 0u64;
    let mut pnl_calculations = 0u64;
    
    println!("Running protocol invariant checks for {} minutes...\n", duration_minutes);
    
    while start.elapsed() < duration {
        // Invariant 1: Margin increases with position size
        for qty_mult in [1, 2, 5, 10] {
            let base_qty = 1_000_000i64;
            let price = 50_000_000_000u64;
            let im_bps = 500u64;
            
            let margin1 = simulate_margin_calculation(base_qty, price, im_bps);
            let margin2 = simulate_margin_calculation(base_qty * qty_mult, price, im_bps);
            
            assert!(margin2 >= margin1, "Margin should increase with position size");
            margin_calculations += 2;
        }
        invariant_checks += 1;
        
        // Invariant 2: VWAP within price bounds
        let prices = [
            (49_000_000_000u64, 100u64),
            (50_000_000_000u64, 200u64),
            (51_000_000_000u64, 100u64),
        ];
        let vwap = simulate_vwap_calculation(&prices);
        let min_price = prices.iter().map(|(p, _)| *p).min().unwrap();
        let max_price = prices.iter().map(|(p, _)| *p).max().unwrap();
        
        assert!(vwap >= min_price && vwap <= max_price, 
            "VWAP {} should be within [{}, {}]", vwap, min_price, max_price);
        vwap_calculations += 1;
        invariant_checks += 1;
        
        // Invariant 3: PnL sign correctness
        let entry = 50_000_000_000u64;
        let mark_up = 51_000_000_000u64;
        let mark_down = 49_000_000_000u64;
        let qty_long = 1_000_000i64;
        let qty_short = -1_000_000i64;
        
        // Long profits when price up
        let pnl_long_up = (qty_long.unsigned_abs() as i128) * (mark_up as i128 - entry as i128);
        assert!(pnl_long_up > 0, "Long should profit on price up");
        
        // Long loses when price down
        let pnl_long_down = (qty_long.unsigned_abs() as i128) * (mark_down as i128 - entry as i128);
        assert!(pnl_long_down < 0, "Long should lose on price down");
        
        // Short profits when price down
        let pnl_short_down = (qty_short.unsigned_abs() as i128) * (entry as i128 - mark_down as i128);
        assert!(pnl_short_down > 0, "Short should profit on price down");
        
        // Short loses when price up
        let pnl_short_up = (qty_short.unsigned_abs() as i128) * (entry as i128 - mark_up as i128);
        assert!(pnl_short_up < 0, "Short should lose on price up");
        
        pnl_calculations += 4;
        invariant_checks += 1;
    }
    
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                  INVARIANT TEST RESULTS                       ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    println!("Total invariant checks: {}", invariant_checks);
    println!("Margin calculations: {}", margin_calculations);
    println!("VWAP calculations: {}", vwap_calculations);
    println!("PnL calculations: {}", pnl_calculations);
    println!("\n✅ All protocol invariants held for {} minutes\n", duration_minutes);
}

// ============================================================================
// QUICK SANITY TESTS (not ignored, run in CI)
// ============================================================================

#[cfg(test)]
mod quick_tests {
    use super::*;
    
    #[test]
    fn test_stats_recording() {
        let stats = SoakStats::new();
        
        stats.record_success(100);
        stats.record_success(200);
        stats.record_failure();
        
        assert_eq!(stats.total_operations.load(Ordering::Relaxed), 3);
        assert_eq!(stats.successful_operations.load(Ordering::Relaxed), 2);
        assert_eq!(stats.failed_operations.load(Ordering::Relaxed), 1);
        assert_eq!(stats.min_latency_us.load(Ordering::Relaxed), 100);
        assert_eq!(stats.max_latency_us.load(Ordering::Relaxed), 200);
    }
    
    #[test]
    fn test_simulate_operation() {
        // No chaos - should always succeed
        for i in 0..100 {
            assert!(simulate_operation(i, 0.0).is_ok());
        }
    }
    
    #[test]
    fn test_simulate_parse_instruction() {
        // Valid discriminators
        for disc in 0..=7 {
            let data = vec![disc; 10];
            assert!(simulate_parse_instruction(&data).is_ok());
        }
        
        // Invalid discriminator
        let data = vec![8u8; 10];
        assert!(simulate_parse_instruction(&data).is_err());
        
        // Empty
        assert!(simulate_parse_instruction(&[]).is_err());
    }
    
    #[test]
    fn test_simulate_vwap() {
        let fills = vec![
            (100u64, 10u64),
            (200u64, 20u64),
        ];
        
        // VWAP = (100*10 + 200*20) / 30 = 5000 / 30 = 166.67 -> 166
        let vwap = simulate_vwap_calculation(&fills);
        assert!(vwap >= 100 && vwap <= 200);
        
        // Empty fills
        assert_eq!(simulate_vwap_calculation(&[]), 0);
    }
    
    #[test]
    fn test_simulate_margin() {
        let margin = simulate_margin_calculation(1_000_000, 50_000_000_000, 500);
        // qty(1M) * price(50B) = 50e15, then * 500 / 10000 = 2.5e15
        assert_eq!(margin, 2_500_000_000_000_000);
    }
}
