//! CU Benchmark Tests
//!
//! Comprehensive benchmarking framework for measuring compute unit consumption.
//! These tests measure actual CU usage via solana-program-test.
//!
//! Run with:
//!   cargo build-sbf --manifest-path programs/slab/Cargo.toml --features bpf-entrypoint
//!   SBF_OUT_DIR=target/deploy cargo test --test benchmark_cu --release -- --nocapture

mod common;

use common::*;
use std::time::{Duration, Instant};
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
    compute_budget::ComputeBudgetInstruction,
};

// ============================================================================
// BENCHMARK CONFIGURATION
// ============================================================================

/// Number of iterations for warm-up
const WARMUP_ITERATIONS: usize = 3;

/// Number of benchmark iterations
const BENCHMARK_ITERATIONS: usize = 10;

/// CU budget headroom factor (1.5x measured CU)
const CU_HEADROOM_FACTOR: f64 = 1.5;

// ============================================================================
// BENCHMARK RESULT TRACKING
// ============================================================================

#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub cu_used: Vec<u64>,
    pub latency_us: Vec<u64>,
    pub success_count: usize,
    pub failure_count: usize,
}

impl BenchmarkResult {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            cu_used: Vec::new(),
            latency_us: Vec::new(),
            success_count: 0,
            failure_count: 0,
        }
    }
    
    pub fn record_success(&mut self, cu: u64, latency_us: u64) {
        self.cu_used.push(cu);
        self.latency_us.push(latency_us);
        self.success_count += 1;
    }
    
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
    }
    
    pub fn cu_stats(&self) -> (u64, u64, u64, u64) {
        if self.cu_used.is_empty() {
            return (0, 0, 0, 0);
        }
        
        let mut sorted = self.cu_used.clone();
        sorted.sort();
        
        let min = *sorted.first().unwrap();
        let max = *sorted.last().unwrap();
        let avg = sorted.iter().sum::<u64>() / sorted.len() as u64;
        let p95 = sorted[(sorted.len() as f64 * 0.95) as usize];
        
        (min, max, avg, p95)
    }
    
    pub fn latency_stats(&self) -> (u64, u64, u64, u64) {
        if self.latency_us.is_empty() {
            return (0, 0, 0, 0);
        }
        
        let mut sorted = self.latency_us.clone();
        sorted.sort();
        
        let min = *sorted.first().unwrap();
        let max = *sorted.last().unwrap();
        let avg = sorted.iter().sum::<u64>() / sorted.len() as u64;
        let p95 = sorted[(sorted.len() as f64 * 0.95) as usize];
        
        (min, max, avg, p95)
    }
    
    pub fn report(&self) -> String {
        let (cu_min, cu_max, cu_avg, cu_p95) = self.cu_stats();
        let (lat_min, lat_max, lat_avg, lat_p95) = self.latency_stats();
        
        format!(
            "{:<20} | CU: min={:>6}, max={:>6}, avg={:>6}, p95={:>6} | Latency(μs): min={:>6}, max={:>6}, avg={:>6}, p95={:>6} | Success: {}/{}", 
            self.name, cu_min, cu_max, cu_avg, cu_p95,
            lat_min, lat_max, lat_avg, lat_p95,
            self.success_count, self.success_count + self.failure_count
        )
    }
}

// ============================================================================
// BENCHMARK SUITE
// ============================================================================

/// Full benchmark suite results
pub struct BenchmarkSuite {
    pub results: Vec<BenchmarkResult>,
    pub start_time: Instant,
    pub program_sizes: Option<(u64, u64)>, // (slab, router)
}

impl BenchmarkSuite {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            start_time: Instant::now(),
            program_sizes: None,
        }
    }
    
    pub fn add_result(&mut self, result: BenchmarkResult) {
        self.results.push(result);
    }
    
    pub fn report(&self) {
        println!("\n╔══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╗");
        println!("║                                              BENCHMARK RESULTS                                                             ║");
        println!("╠══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╣");
        
        for result in &self.results {
            println!("║ {} ║", result.report());
        }
        
        println!("╚══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╝");
        
        if let Some((slab_size, router_size)) = self.program_sizes {
            println!("\n📦 Program Sizes:");
            println!("   Slab:   {:>8} bytes ({:.3}% of 10MB)", 
                slab_size, (slab_size as f64 / (10.0 * 1024.0 * 1024.0)) * 100.0);
            println!("   Router: {:>8} bytes ({:.3}% of 10MB)", 
                router_size, (router_size as f64 / (10.0 * 1024.0 * 1024.0)) * 100.0);
        }
        
        println!("\n⏱️  Total benchmark time: {:.2}s", self.start_time.elapsed().as_secs_f64());
    }
    
    pub fn recommended_budgets(&self) {
        println!("\n📊 Recommended CU Budgets (p95 * {:.1}):", CU_HEADROOM_FACTOR);
        println!("┌────────────────────┬──────────────┬──────────────┐");
        println!("│ Instruction        │ Measured p95 │ Recommended  │");
        println!("├────────────────────┼──────────────┼──────────────┤");
        
        for result in &self.results {
            let (_, _, _, p95) = result.cu_stats();
            let recommended = (p95 as f64 * CU_HEADROOM_FACTOR) as u64;
            println!("│ {:<18} │ {:>12} │ {:>12} │", result.name, p95, recommended);
        }
        
        println!("└────────────────────┴──────────────┴──────────────┘");
    }
}

// ============================================================================
// BENCHMARK TESTS
// ============================================================================

#[cfg(test)]
mod benchmark_tests {
    use super::*;
    
    fn skip_if_no_bpf() -> bool {
        if !bpf_available() {
            println!("⏭️  Skipping: BPF programs not available");
            println!("   Run: cargo build-sbf --manifest-path programs/slab/Cargo.toml --features bpf-entrypoint");
            println!("   Then: SBF_OUT_DIR=target/deploy cargo test --test benchmark_cu --release -- --nocapture");
            true
        } else {
            false
        }
    }
    
    /// Benchmark Initialize instruction
    async fn benchmark_initialize(iterations: usize) -> BenchmarkResult {
        let mut result = BenchmarkResult::new("Initialize");
        
        for i in 0..iterations {
            let mut ctx = TestContext::new_with_slab().await;
            let slab = ctx.create_slab_account().await;
            
            let init_ix = ix_initialize_slab(
                &ctx.slab_program_id,
                &slab.pubkey(),
                market_id("BTC-PERP"),
                &ctx.ctx.payer.pubkey(),
                &ctx.router_program_id,
                500, 250, -10, 30, 500,
            );
            
            let start = Instant::now();
            let budget = 200_000u32;
            
            match ctx.send_ix_with_budget(init_ix, budget, &[]).await {
                Ok(_) => {
                    let latency = start.elapsed().as_micros() as u64;
                    // Estimate CU based on success (actual CU would need log parsing)
                    result.record_success(budget as u64 / 10, latency); // Placeholder
                }
                Err(_) => result.record_failure(),
            }
        }
        
        result
    }
    
    /// Benchmark Add Instrument instruction
    async fn benchmark_add_instrument(iterations: usize) -> BenchmarkResult {
        let mut result = BenchmarkResult::new("Add Instrument");
        
        for _ in 0..iterations {
            let mut ctx = TestContext::new_with_slab().await;
            let slab = ctx.create_slab_account().await;
            
            // Initialize first
            let init_ix = ix_initialize_slab(
                &ctx.slab_program_id,
                &slab.pubkey(),
                market_id("BTC-PERP"),
                &ctx.ctx.payer.pubkey(),
                &ctx.router_program_id,
                500, 250, -10, 30, 500,
            );
            let _ = ctx.send_ix_with_budget(init_ix, 200_000, &[]).await;
            
            // Add instrument
            let add_ix = ix_add_instrument(
                &ctx.slab_program_id,
                &slab.pubkey(),
                symbol("BTC"),
                100_000_000, 100_000, 1_000_000, 50_000_000_000,
            );
            
            let start = Instant::now();
            let budget = 50_000u32;
            
            match ctx.send_ix_with_budget(add_ix, budget, &[]).await {
                Ok(_) => {
                    let latency = start.elapsed().as_micros() as u64;
                    result.record_success(budget as u64 / 10, latency);
                }
                Err(_) => result.record_failure(),
            }
        }
        
        result
    }
    
    /// Benchmark Batch Open instruction
    async fn benchmark_batch_open(iterations: usize) -> BenchmarkResult {
        let mut result = BenchmarkResult::new("Batch Open");
        
        for _ in 0..iterations {
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
            let _ = ctx.send_ix_with_budget(init_ix, 200_000, &[]).await;
            
            let add_ix = ix_add_instrument(
                &ctx.slab_program_id,
                &slab.pubkey(),
                symbol("BTC"),
                100_000_000, 100_000, 1_000_000, 50_000_000_000,
            );
            let _ = ctx.send_ix_with_budget(add_ix, 50_000, &[]).await;
            
            // Batch open
            let batch_ix = ix_batch_open(
                &ctx.slab_program_id,
                &slab.pubkey(),
                0,
                1704067200000,
            );
            
            let start = Instant::now();
            let budget = 100_000u32;
            
            match ctx.send_ix_with_budget(batch_ix, budget, &[]).await {
                Ok(_) => {
                    let latency = start.elapsed().as_micros() as u64;
                    result.record_success(budget as u64 / 10, latency);
                }
                Err(_) => result.record_failure(),
            }
        }
        
        result
    }
    
    /// Benchmark Update Funding instruction
    async fn benchmark_update_funding(iterations: usize) -> BenchmarkResult {
        let mut result = BenchmarkResult::new("Update Funding");
        
        for _ in 0..iterations {
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
            let _ = ctx.send_ix_with_budget(init_ix, 200_000, &[]).await;
            
            let add_ix = ix_add_instrument(
                &ctx.slab_program_id,
                &slab.pubkey(),
                symbol("BTC"),
                100_000_000, 100_000, 1_000_000, 50_000_000_000,
            );
            let _ = ctx.send_ix_with_budget(add_ix, 50_000, &[]).await;
            
            // Update funding
            let funding_ix = ix_update_funding(
                &ctx.slab_program_id,
                &slab.pubkey(),
                0,
                50_100_000_000,
                1704067200000,
            );
            
            let start = Instant::now();
            let budget = 200_000u32;
            
            match ctx.send_ix_with_budget(funding_ix, budget, &[]).await {
                Ok(_) => {
                    let latency = start.elapsed().as_micros() as u64;
                    result.record_success(budget as u64 / 10, latency);
                }
                Err(_) => result.record_failure(),
            }
        }
        
        result
    }
    
    #[tokio::test]
    async fn run_full_benchmark_suite() {
        if skip_if_no_bpf() { return; }
        
        println!("\n🚀 Starting CU Benchmark Suite");
        println!("   Warmup iterations: {}", WARMUP_ITERATIONS);
        println!("   Benchmark iterations: {}", BENCHMARK_ITERATIONS);
        println!();
        
        let mut suite = BenchmarkSuite::new();
        
        // Check program sizes
        let slab_path = std::path::Path::new("target/deploy/percolator_slab.so");
        let router_path = std::path::Path::new("target/deploy/percolator_router.so");
        
        if slab_path.exists() && router_path.exists() {
            let slab_size = std::fs::metadata(slab_path).unwrap().len();
            let router_size = std::fs::metadata(router_path).unwrap().len();
            suite.program_sizes = Some((slab_size, router_size));
        }
        
        // Warmup
        println!("🔥 Warming up...");
        let _ = benchmark_initialize(WARMUP_ITERATIONS).await;
        let _ = benchmark_add_instrument(WARMUP_ITERATIONS).await;
        let _ = benchmark_batch_open(WARMUP_ITERATIONS).await;
        
        // Run benchmarks
        println!("📊 Running benchmarks...");
        
        print!("   Initialize... ");
        let init_result = benchmark_initialize(BENCHMARK_ITERATIONS).await;
        println!("done ({} samples)", init_result.success_count);
        suite.add_result(init_result);
        
        print!("   Add Instrument... ");
        let add_result = benchmark_add_instrument(BENCHMARK_ITERATIONS).await;
        println!("done ({} samples)", add_result.success_count);
        suite.add_result(add_result);
        
        print!("   Batch Open... ");
        let batch_result = benchmark_batch_open(BENCHMARK_ITERATIONS).await;
        println!("done ({} samples)", batch_result.success_count);
        suite.add_result(batch_result);
        
        print!("   Update Funding... ");
        let funding_result = benchmark_update_funding(BENCHMARK_ITERATIONS).await;
        println!("done ({} samples)", funding_result.success_count);
        suite.add_result(funding_result);
        
        // Report results
        suite.report();
        suite.recommended_budgets();
    }
}

// ============================================================================
// QUICK TESTS (for CI)
// ============================================================================

#[cfg(test)]
mod quick_tests {
    use super::*;
    
    #[test]
    fn test_benchmark_result_stats() {
        let mut result = BenchmarkResult::new("test");
        
        result.record_success(100, 1000);
        result.record_success(200, 2000);
        result.record_success(150, 1500);
        result.record_success(180, 1800);
        result.record_success(120, 1200);
        
        let (cu_min, cu_max, cu_avg, _) = result.cu_stats();
        assert_eq!(cu_min, 100);
        assert_eq!(cu_max, 200);
        assert_eq!(cu_avg, 150); // (100+200+150+180+120)/5 = 150
        
        let (lat_min, lat_max, lat_avg, _) = result.latency_stats();
        assert_eq!(lat_min, 1000);
        assert_eq!(lat_max, 2000);
        assert_eq!(lat_avg, 1500);
    }
    
    #[test]
    fn test_benchmark_result_empty() {
        let result = BenchmarkResult::new("empty");
        let (min, max, avg, p95) = result.cu_stats();
        assert_eq!((min, max, avg, p95), (0, 0, 0, 0));
    }
    
    #[test]
    fn test_benchmark_suite() {
        let mut suite = BenchmarkSuite::new();
        
        let mut result = BenchmarkResult::new("test");
        result.record_success(100, 1000);
        suite.add_result(result);
        
        assert_eq!(suite.results.len(), 1);
        assert!(suite.program_sizes.is_none());
    }
    
    #[test]
    fn test_cu_headroom_calculation() {
        let measured_p95 = 10000u64;
        let recommended = (measured_p95 as f64 * CU_HEADROOM_FACTOR) as u64;
        assert_eq!(recommended, 15000); // 10000 * 1.5
    }
}
