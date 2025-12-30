# Percolator

A sharded perpetual exchange protocol for Solana, implementing the design from `plan.md`.

## Architecture

Percolator consists of two main on-chain programs:

### 1. Router Program
The global coordinator managing collateral, portfolio margin, and cross-slab routing.

**Program ID:** `RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr`

**State structures:**
- `Vault` - Collateral custody per asset mint
- `Escrow` - Per (user, slab, mint) pledges with anti-replay nonces
- `Cap` (Capability) - Time-limited, scoped debit authorization tokens (max 2 minutes TTL)
- `Portfolio` - Cross-margin tracking with exposure aggregation across slabs
- `SlabRegistry` - Governance-controlled registry with version validation

**PDA Derivations:**
- Vault: `[b"vault", mint]`
- Escrow: `[b"escrow", user, slab, mint]`
- Capability: `[b"cap", user, slab, mint, nonce_u64]`
- Portfolio: `[b"portfolio", user]`
- Registry: `[b"registry"]`

### 2. Slab Program
LP-run perp engines with 10 MB state budget, fully self-contained matching and settlement.

**Program ID:** `SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk`

**State structures:**
- `SlabHeader` - Metadata, risk params, anti-toxicity settings
- `Instrument` - Contract specs, oracle prices, funding rates, book heads
- `Order` - Price-time sorted orders with reservation tracking
- `Position` - User positions with VWAP entry prices
- `Reservation` - Reserve-commit two-phase execution state
- `Slice` - Sub-order fragments locked during reservation
- `Trade` - Ring buffer of executed trades
- `AggressorEntry` - Anti-sandwich tracking per batch

**PDA Derivations:**
- Slab State: `[b"slab", market_id]`
- Authority: `[b"authority", slab]`

## Key Features Implemented

### ✅ Memory Management
- **10 MB budget** strictly enforced at compile time
- O(1) freelist-based allocation for all pools
- Zero allocations after initialization
- Pool sizes (tuned to fit within 10 MB):
  - Accounts: 5,000
  - Orders: 30,000
  - Positions: 30,000
  - Reservations: 4,000
  - Slices: 16,000
  - Trades: 10,000 (ring buffer)
  - Instruments: 32
  - DLP accounts: 100
  - Aggressor entries: 4,000

### ✅ Matching Engine
- **Price-time priority** with strict FIFO at same price level
- **Reserve operation**: Walk book, lock slices, calculate VWAP/worst price
- **Commit operation**: Execute at captured maker prices
- **Cancel operation**: Release reservations
- **Pending queue promotion**: Non-DLP orders wait one batch epoch
- **Order book management**: Insert, remove, promote with proper linking

### ✅ Risk Management
- **Local (slab) margin**: IM/MM calculated per position
- **Global (router) margin**: Cross-slab portfolio netting
- Equity calculation with unrealized PnL and funding payments
- Pre-trade margin checks
- Liquidation detection

### ✅ Capability Security
- Time-limited caps (max 2 minutes TTL)
- Scoped to (user, slab, mint) triplet
- Anti-replay with nonces
- Remaining amount tracking
- Automatic expiry checks

### ✅ Fixed-Point Math
- 6-decimal precision for prices
- VWAP calculations
- PnL computation
- Funding payment tracking
- Margin calculations in basis points

### ✅ PDA Derivation Helpers
- Router: Vault, Escrow, Capability, Portfolio, Registry PDAs
- Slab: Slab State, Authority PDAs
- Verification functions for account validation
- Comprehensive seed management

### ✅ Instruction Dispatching
- 6 instruction types: Reserve, Commit, Cancel, BatchOpen, Initialize, AddInstrument
- Discriminator-based routing
- Error handling for invalid instructions
- Account validation framework ready

### ✅ Anti-Toxicity Infrastructure
- Batch windows (`batch_ms`)
- Delayed maker posting (pending → live promotion)
- JIT penalty detection
- Kill band parameters
- Freeze levels configuration
- Aggressor roundtrip guard (ARG) data structures

### ✅ BPF Build Support
- Panic handlers for no_std builds
- `panic = "abort"` configuration
- Pinocchio integration for zero-dependency Solana programs

## Test Coverage

**71 tests passing** across all packages:

### percolator-common (35 tests)
- ✅ VWAP calculations (single/multiple fills, zero quantity)
- ✅ PnL calculations (long/short profit/loss, no change)
- ✅ Funding payment calculations
- ✅ Tick/lot alignment and rounding
- ✅ Margin calculations (IM/MM, scaling with quantity/price)
- ✅ Type defaults (Side, TimeInForce, MakerClass, OrderState, Order, Position)
- ✅ Instruction reader parsing (u8, u16, u32, u64, u128, bytes, side)

### percolator-router (6 tests)
- ✅ Vault pledge/unpledge operations
- ✅ Escrow credit/debit with nonce validation
- ✅ Capability lifecycle (creation, usage, expiry)
- ✅ Portfolio exposure tracking
- ✅ Portfolio margin aggregation
- ✅ Registry operations (add/validate slabs)

### percolator-slab (30 tests)
- ✅ Pool allocation/free operations
- ✅ Pool capacity limits and reuse
- ✅ Header validation and monotonic IDs
- ✅ Order/hold ID allocation
- ✅ JIT penalty detection
- ✅ Kill band checks
- ✅ Anti-toxicity parameter defaults
- ✅ Reserve result sizing
- ✅ Commit result sizing
- ✅ Funding rate calculations
- ✅ Liquidation price calculations (long/short)
- ✅ Instrument summary sizing
- ✅ Batch status tracking
- ✅ Quote cache operations

**Note:** PDA tests require Solana syscalls and are marked `#[cfg(target_os = "solana")]`. They run during BPF builds.

## Building and Testing

### Build
```bash
# Build all programs (libraries)
cargo build

# Build in release mode
cargo build --release

# Build specific package
cargo build --package percolator-slab
```

### Unit Testing
```bash
# Run all tests
cargo test

# Run only library tests
cargo test --lib

# Run tests for specific package
cargo test --package percolator-common
cargo test --package percolator-router
cargo test --package percolator-slab

# Run specific test
cargo test test_vwap_calculation

# Run tests with output
cargo test -- --nocapture

# Run tests in release mode (faster)
cargo test --release
```

### Build for Solana BPF
```bash
# Install Solana toolchain (if not already installed)
sh -c "$(curl -sSfL https://release.anza.xyz/v2.1.0/install)"
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"

# Build BPF programs
cargo build-sbf --manifest-path programs/slab/Cargo.toml --features bpf-entrypoint
cargo build-sbf --manifest-path programs/router/Cargo.toml --features bpf-entrypoint

# Programs are output to target/deploy/
ls -la target/deploy/*.so
# percolator_router.so (~31KB)
# percolator_slab.so (~66KB)
```

### Local Validator Testing
```bash
# Start local validator with programs pre-loaded
./scripts/start-validator.sh

# In another terminal, deploy programs manually
./scripts/deploy-local.sh

# Or use solana CLI directly
solana-test-validator --bpf-program SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk target/deploy/percolator_slab.so \
                      --bpf-program RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr target/deploy/percolator_router.so
```

### Compute Unit (CU) Budgets

Expected CU consumption for each instruction (see `tests/cu_measurement.rs`):

| Instruction | Typical CU | Max CU | Notes |
|-------------|------------|--------|-------|
| Reserve | 75,000 | 150,000 | Depends on book depth |
| Commit | 50,000 | 100,000 | Depends on fill count |
| Cancel | 20,000 | 40,000 | Releases slices |
| Batch Open | 40,000 | 80,000 | Depends on pending queue |
| Initialize | 15,000 | 30,000 | One-time setup |
| Add Instrument | 8,000 | 15,000 | Per instrument |
| Update Funding | 60,000 | 150,000 | Depends on position count |
| Liquidation | 100,000 | 200,000 | Depends on positions to close |
| **Reserve+Commit Flow** | **150,000** | **300,000** | **Full trade execution** |

**Integration and Property Tests:**

The `tests/` directory contains templates for integration tests and property-based tests. See [`tests/README.md`](tests/README.md) for details on:
- Integration test scenarios (15+ tests across 3 files)
- Property-based invariant tests
- Setup instructions for Surfpool
- How to enable and run the tests

## Surfpool Integration

[Surfpool](https://github.com/txtx/surfpool) provides a local Solana test validator with mainnet state access for realistic integration testing.

### Setup Surfpool

```bash
# Clone surfpool
git clone https://github.com/txtx/surfpool
cd surfpool

# Install dependencies
npm install

# Start local validator
npm run validator
```

### Integration Test Structure

Create `tests/integration/` directory for surfpool-based tests:

```rust
// tests/integration/test_reserve_commit.rs
use surfpool::prelude::*;
use percolator_slab::*;
use percolator_router::*;

#[surfpool::test]
async fn test_reserve_and_commit_flow() {
    // Initialize test environment
    let mut context = SurfpoolContext::new().await;

    // Deploy programs
    let router_program = context.deploy_program("percolator_router").await;
    let slab_program = context.deploy_program("percolator_slab").await;

    // Initialize slab state (10 MB account)
    let slab_pda = derive_slab_pda(b"BTC-PERP", &slab_program.id());
    context.create_account(&slab_pda, 10 * 1024 * 1024, &slab_program.id()).await;

    // Initialize router accounts
    let vault_pda = derive_vault_pda(&usdc_mint, &router_program.id());
    // ... setup vault, escrow, portfolio

    // Test reserve operation
    let reserve_ix = create_reserve_instruction(/* ... */);
    context.send_transaction(&[reserve_ix]).await.unwrap();

    // Verify reservation created
    let slab_state = context.get_account::<SlabState>(&slab_pda).await;
    assert!(slab_state.reservations.used() > 0);

    // Test commit operation
    let commit_ix = create_commit_instruction(/* ... */);
    context.send_transaction(&[commit_ix]).await.unwrap();

    // Verify trade executed
    assert_eq!(slab_state.trade_count, 1);
}
```

### Running Integration Tests

```bash
# Start surfpool validator (terminal 1)
cd surfpool && npm run validator

# Run integration tests (terminal 2)
cargo test --test integration

# Run specific integration test
cargo test --test integration test_reserve_and_commit_flow
```

### Example Test Scenarios

1. **Order Matching**
   - Place limit orders on both sides
   - Execute market order
   - Verify VWAP calculation and position updates

2. **Reserve-Commit Flow**
   - Reserve liquidity for aggregator order
   - Verify slices locked correctly
   - Commit at reserved prices
   - Check trades executed at expected prices

3. **Cross-Slab Portfolio**
   - Open positions on multiple slabs
   - Verify router aggregates exposures
   - Check cross-margin calculation

4. **Capability Security**
   - Create time-limited cap
   - Use cap to debit escrow
   - Verify expiry enforcement

5. **Anti-Toxicity**
   - Post pending order
   - Open batch window
   - Verify promotion after epoch
   - Test JIT penalty application

6. **Liquidation**
   - Open underwater position
   - Trigger liquidation
   - Verify position closure and PnL settlement

## Design Invariants (from plan.md)

**Safety:**
1. Slabs cannot access Router vaults directly
2. Slabs can only debit via unexpired, correctly scoped Caps
3. Total debits ≤ min(cap.remaining, escrow.balance)
4. No cross-contamination: slab cannot move funds for (user', slab') ≠ (user, slab)

**Matching:**
1. Price-time priority strictly maintained
2. Reserved qty ≤ available qty always
3. Book links acyclic and consistent
4. Pending orders never match before promotion

**Risk:**
1. IM monotone: increasing exposure increases margin
2. Portfolio IM ≤ Σ slab IMs (convexity not double-counted)
3. Liquidation triggers only when equity < MM

**Anti-Toxicity:**
1. Kill band: reject if mark moved > threshold
2. JIT penalty: DLP orders posted after batch_open get no rebate
3. ARG: roundtrip trades within batch are taxed/clipped

## Current Status

### ✅ Phase 1 Complete - Core Program Logic
- **Core data structures** (Router & Slab) with full pools
- **Memory pools** with O(1) freelists (orders, positions, reservations, slices)
- **Order book management** (insert, remove, promote, price-time priority)
- **Reserve operation** (walk book, lock slices, calculate VWAP/max_charge)
- **Commit operation** (execute trades at maker prices, apply fees, update positions)
- **Cancel operation** (release reservations, restore available qty)
- **Batch open** (promote pending orders, increment epoch, freeze windows)
- **Anti-toxicity mechanisms**:
  - Kill band (reject if mark moved beyond threshold)
  - JIT penalty (no rebate for orders posted too recently)
  - Freeze levels (top-K order protection)
  - ARG parameters (aggressor roundtrip guard)
- **Funding rate updates** (time-weighted calculations, position funding accrual)
- **Liquidation execution** (position closure, PnL settlement, price bands)
- **Risk calculations** (equity, IM/MM, liquidation checks)
- **Capability system** (time-limited scoped debits)
- **Fixed-point math utilities** (VWAP, PnL, margin)
- **PDA derivation helpers** (all account types)
- **Instruction dispatching** (8 slab instructions, 5 router instructions)
- **BPF build support** (panic handlers, no_std)
- **Comprehensive unit tests** (71 tests passing)
- Integration test templates with Surfpool (3 test files with 15+ scenarios)
- Property-based test framework with invariant checks

### ✅ Phase 2 Complete - Build and Deploy
- **Solana Platform Tools** installed (v2.1.0)
- **BPF builds** working with `cargo build-sbf`
- **Program sizes**: Router ~31KB, Slab ~66KB (well under 10MB limit)
- **Stack overflow fixes** for large struct initialization
- **Deployment scripts** created (`scripts/deploy-local.sh`, `scripts/start-validator.sh`)
- **CU measurement framework** with budget estimates (`tests/cu_measurement.rs`)

### 🚧 In Progress
- Integration testing infrastructure (Surfpool setup and runbook development)

### 📋 Next Steps (Priority Order)

**Phase 3: Advanced Testing**
- Complete integration tests using `solana-program-test` crate
- Uncomment and run property-based tests with proptest
- Add fuzz tests for instruction parsing and edge cases
- Implement chaos/soak tests (24-72h load testing)
- Benchmark actual CU consumption on local validator

**Phase 4: Multi-Slab Coordination**
- Router orchestration (multi-slab reserve/commit atomicity)
- Cross-slab portfolio margin calculations
- Global liquidation coordination
- CPI integration between Router and Slab programs

**Phase 5: Production Readiness**
- Slab-level insurance pools (v1 feature)
- Client SDK (TypeScript/Rust)
- CLI tools for LP operations
- Operational runbooks and monitoring
- Security audits
- Documentation and examples

### Architecture Notes

**v0 Simplifications:**
- ✅ No router-level insurance pool (each slab manages its own isolated insurance fund)
- Individual slabs will implement their own insurance pools in v1
- This maintains full isolation between slabs and simplifies router logic

## Technology Stack

- **Framework**: [Pinocchio](https://github.com/anza-xyz/pinocchio) v0.9.2 - Zero-dependency Solana SDK
- **Testing**: [Surfpool](https://github.com/txtx/surfpool) - Local Solana test validator with mainnet state
- **Language**: Rust (no_std, zero allocations, panic = abort)

## Testing Infrastructure

### Local Validator Scripts

**`scripts/start-validator.sh`** - Starts local Solana test validator with programs pre-loaded:
```bash
./scripts/start-validator.sh
# Starts validator with:
# - Slab program at SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk
# - Router program at RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr
# - Increased compute budget for testing
# - Verbose logging enabled
```

**`scripts/deploy-local.sh`** - Deploys programs to a running validator:
```bash
./scripts/deploy-local.sh
# Deploys both programs and saves program IDs to .env.local
```

### CU Measurement Framework

The `tests/cu_measurement.rs` file provides:
- CU budget constants for each instruction
- `CuMeasurement` struct for recording and reporting CU usage
- Framework for integration CU tests (requires `solana-program-test`)

### Surfpool Integration (Optional)

Surfpool can be used for more advanced integration testing with mainnet state:

1. **Setup**: `git clone https://github.com/txtx/surfpool && cd surfpool && npm install`
2. **Configuration**: `Surfpool.toml` in project root
3. **Runbooks**: `.surfpool/runbooks/` for test scenarios

For now, the recommended approach is using the standard `solana-test-validator` with the provided scripts.

## References

- [Plan Document](./plan.md) - Full protocol specification
- [Pinocchio Docs](https://docs.rs/pinocchio/)
- [Surfpool](https://github.com/txtx/surfpool)
- [Solana Cookbook](https://solanacookbook.com/)

## License

Apache-2.0

---

**Status**: Phase 1-2 Complete ✅ | 71 unit tests passing ✅ | BPF builds working ✅ | Phase 3 (testing) next 🚀

**Last Updated**: December 30, 2025
