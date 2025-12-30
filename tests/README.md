# Percolator Test Suite

Production-ready test suite for the Percolator protocol, including integration tests,
property-based tests, fuzz tests, chaos/soak tests, and CU benchmarks.

## Quick Start

```bash
# Build BPF programs (required for integration tests)
cargo build-sbf --manifest-path programs/slab/Cargo.toml --features bpf-entrypoint
cargo build-sbf --manifest-path programs/router/Cargo.toml --features bpf-entrypoint

# Run all unit tests (no BPF required)
cargo test --workspace --lib

# Run integration tests
SBF_OUT_DIR=target/deploy cargo test --test integration_reserve_commit -- --nocapture

# Run property tests
cargo test --test property_invariants -- --nocapture

# Run fuzz tests (10,000 iterations)
cargo test --test fuzz_instructions -- --nocapture

# Run CU benchmarks
SBF_OUT_DIR=target/deploy cargo test --test benchmark_cu --release -- --nocapture
```

## Test Files

| File | Type | Description |
|------|------|-------------|
| `common/mod.rs` | Utilities | Shared test infrastructure and instruction builders |
| `integration_reserve_commit.rs` | Integration | Reserve-commit flow tests using solana-program-test |
| `property_invariants.rs` | Property | Protocol invariants using proptest (1000+ cases) |
| `fuzz_instructions.rs` | Fuzz | Instruction parsing fuzzing (10,000+ cases) |
| `chaos_soak.rs` | Soak | Long-running stability and load tests |
| `benchmark_cu.rs` | Benchmark | CU consumption measurements |
| `cu_measurement.rs` | CU | CU budget verification |
| `integration_portfolio.rs` | Integration | Portfolio management tests (template) |
| `integration_anti_toxicity.rs` | Integration | Anti-toxicity mechanism tests (template) |
| `v0_*.rs` | Integration | V0 protocol tests (templates) |

## Test Categories

### 1. Integration Tests (`integration_*.rs`)

Full end-to-end tests using `solana-program-test`:

```bash
# Run with BPF programs
SBF_OUT_DIR=target/deploy cargo test --test integration_reserve_commit -- --nocapture
```

Tests include:
- Initialize slab and verify state
- Add instruments and configure parameters  
- Batch open to promote pending orders
- Reserve liquidity (Phase 1)
- Commit trades (Phase 2)
- Cancel reservations
- Update funding rates
- Liquidation flows

### 2. Property-Based Tests (`property_invariants.rs`)

Tests protocol invariants using proptest:

```bash
cargo test --test property_invariants -- --nocapture
```

Invariants tested:
- **Safety**: Capability constraints, escrow isolation
- **Matching**: Price-time priority, reservation bounds, VWAP bounds
- **Risk**: Margin monotonicity, liquidation thresholds, cross-margin convexity
- **Anti-toxicity**: Kill bands, JIT detection, slippage bounds

Each property runs 1000+ test cases with random inputs.

### 3. Fuzz Tests (`fuzz_instructions.rs`)

Instruction parsing fuzzing with edge cases:

```bash
cargo test --test fuzz_instructions -- --nocapture
```

Tests include:
- Empty input handling
- Invalid discriminator rejection
- Truncated payload handling  
- Boundary value testing (u64, i64, i128)
- All valid discriminators
- Extra bytes handling
- Deterministic parsing
- Arithmetic overflow protection

Each fuzz test runs 10,000+ iterations.

### 4. Chaos/Soak Tests (`chaos_soak.rs`)

Long-running stability tests:

```bash
# Quick 5-minute test
cargo test --test chaos_soak --release -- --nocapture --ignored

# Extended 24-hour test
SOAK_DURATION_MINUTES=1440 cargo test --test chaos_soak test_continuous_load --release -- --nocapture --ignored
```

Test scenarios:
- **Continuous Load**: Sustained operations for hours/days
- **Memory Stability**: Track memory usage over time
- **Chaos Recovery**: Random failure injection (1-25% rates)
- **Burst Traffic**: Handle sudden traffic spikes
- **Invariant Endurance**: Protocol invariants under load

### 5. CU Benchmarks (`benchmark_cu.rs`)

Measure actual compute unit consumption:

```bash
SBF_OUT_DIR=target/deploy cargo test --test benchmark_cu --release -- --nocapture
```

Benchmark includes:
- Initialize instruction
- Add instrument instruction
- Batch open instruction
- Update funding instruction
- Statistical analysis (min, max, avg, p95)
- Recommended CU budgets with headroom

## CU Budget Reference

| Instruction | Typical CU | Max CU | % of Max TX |
|-------------|------------|--------|-------------|
| Reserve | 75,000 | 150,000 | 10.71% |
| Commit | 50,000 | 100,000 | 7.14% |
| Cancel | 20,000 | 40,000 | 2.86% |
| Batch Open | 40,000 | 80,000 | 5.71% |
| Initialize | 15,000 | 30,000 | 2.14% |
| Add Instrument | 8,000 | 15,000 | 1.07% |
| Update Funding | 60,000 | 150,000 | 10.71% |
| Liquidation | 100,000 | 200,000 | 14.29% |
| **Reserve+Commit** | **150,000** | **300,000** | **21.43%** |

Max transaction CU: 1,400,000

## Running Specific Tests

```bash
# Single test file
cargo test --test property_invariants

# Single test function
cargo test --test property_invariants prop_vwap_bounds

# With output
cargo test --test fuzz_instructions -- --nocapture

# Release mode (faster)
cargo test --release --test chaos_soak

# Ignored tests (soak tests)
cargo test --test chaos_soak -- --ignored
```

## Test Coverage

### Unit Tests (71 passing)
- `percolator-common`: 35 tests
- `percolator-slab`: 25 tests
- `percolator-router`: 11 tests

### Integration Tests
- Reserve-commit flow: 6 integration tests
- Property tests: 14 properties × 1000+ cases each
- Fuzz tests: 12 fuzz properties × 10,000 cases each
- Chaos tests: 5 scenarios (long-running)

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `SBF_OUT_DIR` | BPF program output directory | - |
| `BPF_OUT_DIR` | Alternative to SBF_OUT_DIR | - |
| `SOAK_DURATION_MINUTES` | Soak test duration | 5 |
| `SOAK_DURATION_HOURS` | Alternative duration format | - |

### Proptest Configuration

Property tests use these defaults:
- Cases per property: 1000
- Max shrink iterations: 1000
- Failure persistence: enabled

## Debugging Failed Tests

```bash
# Verbose output
RUST_LOG=debug cargo test --test integration_reserve_commit -- --nocapture

# Specific test with backtrace
RUST_BACKTRACE=1 cargo test --test fuzz_instructions test_boundary_values -- --nocapture

# Proptest seed reproduction
PROPTEST_SEED="0x1234..." cargo test --test property_invariants
```

## CI Integration

The test suite is designed for CI:

```yaml
# Fast tests (< 2 minutes)
- cargo test --workspace --lib
- cargo test --test property_invariants
- cargo test --test fuzz_instructions

# Integration tests (requires BPF build)
- cargo build-sbf --manifest-path programs/slab/Cargo.toml --features bpf-entrypoint
- SBF_OUT_DIR=target/deploy cargo test --test integration_reserve_commit

# Nightly soak tests
- SOAK_DURATION_MINUTES=60 cargo test --test chaos_soak --release -- --ignored
```

## Writing New Tests

### Adding Integration Tests

```rust
#[tokio::test]
async fn test_new_feature() {
    if !bpf_available() { return; }
    
    let mut ctx = TestContext::new_with_slab().await;
    let slab = ctx.create_slab_account().await;
    
    // Setup
    let init_ix = ix_initialize_slab(/* ... */);
    ctx.send_ix_with_budget(init_ix, 200_000, &[]).await.unwrap();
    
    // Test
    let result = /* ... */;
    assert!(result.is_ok());
}
```

### Adding Property Tests

```rust
proptest! {
    #[test]
    fn prop_new_invariant(
        value in 0u64..1_000_000u64,
    ) {
        // Property assertion
        prop_assert!(/* invariant holds */);
    }
}
```

### Adding Fuzz Tests

```rust
proptest! {
    #![proptest_config(ProptestConfig::with_cases(10000))]
    
    #[test]
    fn fuzz_new_input(
        data in prop::collection::vec(any::<u8>(), 0..256),
    ) {
        // Should not panic
        let _ = parse_something(&data);
    }
}
```
