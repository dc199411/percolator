# Percolator Tests

Integration tests, CU measurement tests, and property-based tests.

## Quick Start

```bash
# Build BPF programs
cargo build-sbf --manifest-path programs/slab/Cargo.toml --features bpf-entrypoint
cargo build-sbf --manifest-path programs/router/Cargo.toml --features bpf-entrypoint

# Run CU measurement tests
SBF_OUT_DIR=target/deploy cargo test --test cu_measurement -- --nocapture

# Run all library tests (71 tests)
cargo test --workspace --lib
```

## Test Files

| File | Status | Description |
|------|--------|-------------|
| `cu_measurement.rs` | ✅ Working | CU budget verification |
| `common/mod.rs` | ✅ Working | Shared test utilities |
| `integration_*.rs` | 📝 Template | Integration test templates |
| `property_invariants.rs` | 📝 Template | Property-based tests |

## CU Budget Summary

| Instruction | Typical | Max | % of 1.4M |
|-------------|---------|-----|-----------|
| Initialize | 15,000 | 30,000 | 2.14% |
| Add Instrument | 8,000 | 15,000 | 1.07% |
| Reserve | 75,000 | 150,000 | 10.71% |
| Commit | 50,000 | 100,000 | 7.14% |
| Cancel | 20,000 | 40,000 | 2.86% |
| Batch Open | 40,000 | 80,000 | 5.71% |
| Update Funding | 60,000 | 150,000 | 10.71% |
| Liquidation | 100,000 | 200,000 | 14.29% |
| **Reserve+Commit** | **150,000** | **300,000** | **21.43%** |

## Program Sizes

- **Slab:** ~66KB (0.63% of 10MB limit)
- **Router:** ~31KB (0.29% of 10MB limit)
