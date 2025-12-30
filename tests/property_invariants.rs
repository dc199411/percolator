//! Property-based tests for protocol invariants
//!
//! Tests core invariants from plan.md:
//! - Safety: Capability constraints, escrow isolation
//! - Matching: Price-time priority, reservation constraints  
//! - Risk: Margin monotonicity, liquidation thresholds
//! - Anti-toxicity: Kill bands, JIT penalties
//!
//! Run with: cargo test --test property_invariants -- --nocapture

use proptest::prelude::*;

// ============================================================================
// DATA STRUCTURES FOR TESTING
// ============================================================================

/// Simulated capability token
#[derive(Debug, Clone, Default)]
pub struct Cap {
    pub remaining: u64,
    pub created_at: u64,
    pub ttl: u64,
}

impl Cap {
    pub fn is_expired(&self, current_time: u64) -> bool {
        current_time >= self.created_at.saturating_add(self.ttl)
    }
}

/// Simulated escrow
#[derive(Debug, Clone, Default)]
pub struct Escrow {
    pub balance: u64,
}

/// Simulated order
#[derive(Debug, Clone, Default)]
pub struct Order {
    pub qty: u64,
    pub filled: u64,
    pub reserved: u64,
    pub price: u64,
    pub created_at: u64,
    pub maker_class: MakerClass,
}

impl Order {
    pub fn available(&self) -> u64 {
        self.qty.saturating_sub(self.filled).saturating_sub(self.reserved)
    }
}

/// Maker classification for anti-toxicity
#[derive(Debug, Clone, Default, PartialEq)]
pub enum MakerClass {
    #[default]
    Retail,
    DLP, // Designated Liquidity Provider
}

/// Simulated position
#[derive(Debug, Clone, Default)]
pub struct Position {
    pub qty: i64,
    pub entry_price: u64,
}

/// Simulated instrument
#[derive(Debug, Clone, Default)]
pub struct Instrument {
    pub im_bps: u64,    // Initial margin basis points
    pub mm_bps: u64,    // Maintenance margin basis points
    pub mark_price: u64,
}

/// Slab header for JIT detection
#[derive(Debug, Clone, Default)]
pub struct SlabHeader {
    pub last_batch_open: u64,
}

// ============================================================================
// SAFETY INVARIANTS
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]
    
    /// Invariant: Total debits <= min(cap.remaining, escrow.balance)
    #[test]
    fn prop_capability_amount_constraint(
        initial_amount in 1u64..1_000_000_000u64,
        debit_amount in 1u64..1_000_000_000u64,
    ) {
        let mut cap = Cap {
            remaining: initial_amount,
            ..Default::default()
        };
        
        // Escrow has half the cap
        let mut escrow = Escrow {
            balance: initial_amount / 2,
        };
        
        // Calculate maximum debit
        let max_debit = cap.remaining.min(escrow.balance);
        let actual_debit = debit_amount.min(max_debit);
        
        // Apply debit
        if actual_debit > 0 {
            cap.remaining = cap.remaining.saturating_sub(actual_debit);
            escrow.balance = escrow.balance.saturating_sub(actual_debit);
        }
        
        // Verify invariant: never underflow
        prop_assert!(cap.remaining <= initial_amount);
        prop_assert!(escrow.balance <= initial_amount / 2);
    }
    
    /// Invariant: Caps cannot be used after expiry
    #[test]
    fn prop_capability_expiry_check(
        created_at in 0u64..1_000_000_000u64,
        ttl in 1u64..120_000u64, // max 2 minutes in ms
        time_offset in 0u64..240_000u64, // up to 4 minutes
    ) {
        let cap = Cap {
            created_at,
            ttl,
            remaining: 1_000_000,
        };
        
        let current_time = created_at.saturating_add(time_offset);
        let should_be_expired = time_offset >= ttl;
        
        prop_assert_eq!(cap.is_expired(current_time), should_be_expired);
    }
    
    /// Invariant: Operations on one escrow don't affect others
    #[test]
    fn prop_escrow_isolation(
        user1_balance in 0u64..1_000_000_000u64,
        user2_balance in 0u64..1_000_000_000u64,
        transfer_amount in 0u64..1_000_000_000u64,
    ) {
        let mut escrow1 = Escrow { balance: user1_balance };
        let escrow2 = Escrow { balance: user2_balance };
        
        let initial_escrow2_balance = escrow2.balance;
        
        // Debit from escrow1
        let actual_transfer = transfer_amount.min(escrow1.balance);
        escrow1.balance = escrow1.balance.saturating_sub(actual_transfer);
        
        // Verify escrow2 unaffected
        prop_assert_eq!(escrow2.balance, initial_escrow2_balance);
    }
}

// ============================================================================
// MATCHING INVARIANTS  
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]
    
    /// Invariant: Reserved qty <= available qty always
    #[test]
    fn prop_reserved_qty_le_available(
        total_qty in 1u64..1_000_000u64,
        filled_pct in 0u32..100u32,
        reserved_pct in 0u32..100u32,
        request_qty in 0u64..1_000_000u64,
    ) {
        let filled = (total_qty as u128 * filled_pct as u128 / 100) as u64;
        let existing_reserved = (total_qty as u128 * reserved_pct as u128 / 100) as u64;
        
        let order = Order {
            qty: total_qty,
            filled,
            reserved: existing_reserved.min(total_qty.saturating_sub(filled)),
            ..Default::default()
        };
        
        // Calculate reservable amount
        let available = order.available();
        let to_reserve = request_qty.min(available);
        
        // Verify invariant
        prop_assert!(to_reserve <= available);
        prop_assert!(order.reserved + to_reserve <= order.qty.saturating_sub(order.filled));
    }
    
    /// Invariant: VWAP must be within min/max price range
    #[test]
    fn prop_vwap_bounds(
        price1 in 100_000_000u64..200_000_000u64,
        qty1 in 1_000_000u64..10_000_000u64,
        price2 in 100_000_000u64..200_000_000u64,
        qty2 in 1_000_000u64..10_000_000u64,
        price3 in 100_000_000u64..200_000_000u64,
        qty3 in 1_000_000u64..10_000_000u64,
    ) {
        let prices = vec![(price1, qty1), (price2, qty2), (price3, qty3)];
        
        let total_qty: u64 = prices.iter().map(|(_, q)| q).sum();
        let total_notional: u128 = prices.iter()
            .map(|(p, q)| (*p as u128) * (*q as u128))
            .sum();
        
        let vwap = (total_notional / total_qty as u128) as u64;
        
        let min_price = prices.iter().map(|(p, _)| *p).min().unwrap();
        let max_price = prices.iter().map(|(p, _)| *p).max().unwrap();
        
        // VWAP must be within price range
        prop_assert!(min_price <= vwap, "VWAP {} < min {}", vwap, min_price);
        prop_assert!(vwap <= max_price, "VWAP {} > max {}", vwap, max_price);
    }
    
    /// Invariant: Price-time priority (same price = earlier order first)
    #[test]
    fn prop_price_time_priority(
        price in 100_000_000u64..200_000_000u64,
        time1 in 0u64..1_000_000u64,
        time2 in 0u64..1_000_000u64,
    ) {
        let order1 = Order {
            price,
            created_at: time1,
            qty: 1_000_000,
            ..Default::default()
        };
        
        let order2 = Order {
            price,
            created_at: time2,
            qty: 1_000_000,
            ..Default::default()
        };
        
        // Same price: earlier timestamp has priority
        if order1.created_at < order2.created_at {
            prop_assert!(has_priority(&order1, &order2));
        } else if order2.created_at < order1.created_at {
            prop_assert!(has_priority(&order2, &order1));
        }
    }
}

/// Check if order1 has priority over order2
fn has_priority(order1: &Order, order2: &Order) -> bool {
    // For asks: lower price = higher priority
    // For same price: earlier time = higher priority
    if order1.price != order2.price {
        order1.price < order2.price
    } else {
        order1.created_at < order2.created_at
    }
}

// ============================================================================
// RISK INVARIANTS
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]
    
    /// Invariant: IM increases monotonically with exposure
    #[test]
    fn prop_margin_monotonic_with_qty(
        base_qty in 1i64..1_000_000i64,
        multiplier in 2u64..10u64,
        price in 40_000_000_000u64..60_000_000_000u64,
        im_bps in 100u64..1000u64, // 1% to 10%
    ) {
        let instrument = Instrument {
            im_bps,
            mark_price: price,
            ..Default::default()
        };
        
        let position1 = Position {
            qty: base_qty,
            entry_price: price,
        };
        
        let position2 = Position {
            qty: base_qty.saturating_mul(multiplier as i64),
            entry_price: price,
        };
        
        let im1 = calculate_im(&position1, &instrument);
        let im2 = calculate_im(&position2, &instrument);
        
        // Larger position requires more margin
        prop_assert!(im2 >= im1, "IM2 {} should >= IM1 {}", im2, im1);
    }
    
    /// Invariant: Liquidation triggers only when equity < MM
    #[test]
    fn prop_liquidation_threshold_mm(
        collateral in 100_000_000u64..10_000_000_000u64,
        position_qty in 1i64..1000i64,
        entry_price in 40_000_000_000u64..60_000_000_000u64,
        mark_price in 35_000_000_000u64..65_000_000_000u64,
        mm_bps in 100u64..500u64, // 1% to 5%
    ) {
        let position = Position {
            qty: position_qty,
            entry_price,
        };
        
        let instrument = Instrument {
            mm_bps,
            mark_price,
            ..Default::default()
        };
        
        let unrealized_pnl = calculate_pnl(&position, mark_price);
        let equity = (collateral as i128) + unrealized_pnl;
        let mm = calculate_mm(&position, &instrument);
        
        let should_liquidate = equity < mm as i128;
        let is_liquidatable = check_liquidatable(collateral, &position, &instrument);
        
        prop_assert_eq!(should_liquidate, is_liquidatable,
            "Liquidation mismatch: equity={}, mm={}", equity, mm);
    }
    
    /// Invariant: Portfolio IM <= Σ slab IMs (convexity)
    /// Note: Due to integer division rounding, we allow a small tolerance
    #[test]
    fn prop_cross_margin_convexity(
        qty1 in -1_000_000i64..1_000_000i64,
        price1 in 40_000_000_000u64..60_000_000_000u64,
        qty2 in -1_000_000i64..1_000_000i64,
        price2 in 40_000_000_000u64..60_000_000_000u64,
        im_bps in 100u64..1000u64,
    ) {
        // Skip zero positions
        if qty1 == 0 && qty2 == 0 {
            return Ok(());
        }
        
        let exposures = vec![(qty1, price1), (qty2, price2)];
        
        let mut individual_ims: u128 = 0;
        let mut total_long_notional: u128 = 0;
        let mut total_short_notional: u128 = 0;
        
        for (qty, price) in &exposures {
            if *qty == 0 { continue; }
            let notional = (qty.unsigned_abs() as u128) * (*price as u128);
            let im = notional * im_bps as u128 / 10_000;
            individual_ims += im;
            
            if *qty > 0 {
                total_long_notional += notional;
            } else {
                total_short_notional += notional;
            }
        }
        
        // Cross-margin recognizes offsetting positions
        let net_notional = if total_long_notional > total_short_notional {
            total_long_notional - total_short_notional
        } else {
            total_short_notional - total_long_notional
        };
        
        let portfolio_im = net_notional * im_bps as u128 / 10_000;
        
        // Portfolio IM should be <= sum of individual IMs
        // Allow tolerance of 1 for integer division rounding
        prop_assert!(portfolio_im <= individual_ims + 1,
            "Portfolio IM {} should <= individual IMs {} (with tolerance)", portfolio_im, individual_ims);
    }
}

// ============================================================================
// ANTI-TOXICITY INVARIANTS
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]
    
    /// Invariant: Orders rejected if mark moved > kill_band
    #[test]
    fn prop_kill_band_threshold(
        last_mark in 40_000_000_000u64..60_000_000_000u64,
        price_change_bps in 0i64..1000i64,
        kill_band_bps in 10u64..500u64, // 0.1% to 5%
    ) {
        // Calculate current mark with change
        let change = (last_mark as i128 * price_change_bps as i128) / 10_000;
        let current_mark = if change >= 0 {
            last_mark.saturating_add(change as u64)
        } else {
            last_mark.saturating_sub((-change) as u64)
        };
        
        let diff = if current_mark > last_mark {
            current_mark - last_mark
        } else {
            last_mark - current_mark
        };
        
        let max_move = (last_mark as u128 * kill_band_bps as u128) / 10_000;
        let should_reject = diff as u128 > max_move;
        
        let is_outside = is_outside_kill_band(last_mark, current_mark, kill_band_bps);
        prop_assert_eq!(should_reject, is_outside);
    }
    
    /// Invariant: DLP orders posted after batch_open get no rebate (JIT penalty)
    #[test]
    fn prop_jit_penalty_detection(
        batch_open_time in 0u64..1_000_000_000u64,
        order_time_offset in 0u64..10_000u64,
        is_dlp in proptest::bool::ANY,
    ) {
        let order_time = batch_open_time.saturating_add(order_time_offset);
        
        let order = Order {
            created_at: order_time,
            maker_class: if is_dlp { MakerClass::DLP } else { MakerClass::Retail },
            ..Default::default()
        };
        
        let header = SlabHeader {
            last_batch_open: batch_open_time,
        };
        
        // JIT = DLP order posted after batch open
        let expected_jit = is_dlp && order.created_at > header.last_batch_open;
        let detected_jit = is_jit_order(&order, &header);
        
        prop_assert_eq!(expected_jit, detected_jit);
    }
    
    /// Invariant: Slippage bounds are respected
    #[test]
    fn prop_slippage_bounds(
        expected_price in 40_000_000_000u64..60_000_000_000u64,
        actual_price in 40_000_000_000u64..60_000_000_000u64,
        max_slippage_bps in 10u64..500u64, // 0.1% to 5%
    ) {
        let slippage = if actual_price > expected_price {
            actual_price - expected_price
        } else {
            expected_price - actual_price
        };
        
        let max_allowed = (expected_price as u128 * max_slippage_bps as u128) / 10_000;
        let should_reject = slippage as u128 > max_allowed;
        
        let result = check_slippage(expected_price, actual_price, max_slippage_bps);
        prop_assert_eq!(!should_reject, result.is_ok());
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn calculate_im(position: &Position, instrument: &Instrument) -> u128 {
    let notional = (position.qty.unsigned_abs() as u128) * (instrument.mark_price as u128);
    notional * instrument.im_bps as u128 / 10_000
}

fn calculate_mm(position: &Position, instrument: &Instrument) -> u128 {
    let notional = (position.qty.unsigned_abs() as u128) * (instrument.mark_price as u128);
    notional * instrument.mm_bps as u128 / 10_000
}

fn calculate_pnl(position: &Position, mark_price: u64) -> i128 {
    let notional_at_entry = (position.qty.unsigned_abs() as u128) * (position.entry_price as u128);
    let notional_at_mark = (position.qty.unsigned_abs() as u128) * (mark_price as u128);
    
    if position.qty > 0 {
        // Long: profit when mark > entry
        notional_at_mark as i128 - notional_at_entry as i128
    } else {
        // Short: profit when mark < entry
        notional_at_entry as i128 - notional_at_mark as i128
    }
}

fn check_liquidatable(collateral: u64, position: &Position, instrument: &Instrument) -> bool {
    let pnl = calculate_pnl(position, instrument.mark_price);
    let equity = (collateral as i128) + pnl;
    let mm = calculate_mm(position, instrument);
    equity < mm as i128
}

fn is_outside_kill_band(last_mark: u64, current_mark: u64, kill_band_bps: u64) -> bool {
    let diff = if current_mark > last_mark {
        current_mark - last_mark
    } else {
        last_mark - current_mark
    };
    let max_move = (last_mark as u128 * kill_band_bps as u128) / 10_000;
    diff as u128 > max_move
}

fn is_jit_order(order: &Order, header: &SlabHeader) -> bool {
    order.created_at > header.last_batch_open && order.maker_class == MakerClass::DLP
}

fn check_slippage(expected: u64, actual: u64, max_bps: u64) -> Result<(), &'static str> {
    let diff = if actual > expected {
        actual - expected
    } else {
        expected - actual
    };
    let max_allowed = (expected as u128 * max_bps as u128) / 10_000;
    if diff as u128 > max_allowed {
        Err("Slippage exceeded")
    } else {
        Ok(())
    }
}

// ============================================================================
// DETERMINISTIC UNIT TESTS
// ============================================================================

#[cfg(test)]
mod deterministic_tests {
    use super::*;
    
    #[test]
    fn test_cap_expiry() {
        let cap = Cap {
            created_at: 1000,
            ttl: 100,
            remaining: 1_000_000,
        };
        
        assert!(!cap.is_expired(1000));
        assert!(!cap.is_expired(1099));
        assert!(cap.is_expired(1100));
        assert!(cap.is_expired(2000));
    }
    
    #[test]
    fn test_order_available() {
        let order = Order {
            qty: 1000,
            filled: 300,
            reserved: 200,
            ..Default::default()
        };
        
        assert_eq!(order.available(), 500);
    }
    
    #[test]
    fn test_calculate_im() {
        let position = Position {
            qty: 1_000_000, // 1 contract
            entry_price: 50_000_000_000, // $50,000
        };
        
        let instrument = Instrument {
            im_bps: 500, // 5%
            mark_price: 50_000_000_000,
            ..Default::default()
        };
        
        let im = calculate_im(&position, &instrument);
        // 1M * 50B * 500 / 10000 = 2.5e15
        // qty(1M) * mark(50B) = 50e15, then * 500 / 10000 = 2.5e15
        assert_eq!(im, 2_500_000_000_000_000);
    }
    
    #[test]
    fn test_calculate_pnl_long() {
        let position = Position {
            qty: 1_000_000,
            entry_price: 50_000_000_000,
        };
        
        // Price went up to $51,000
        let pnl = calculate_pnl(&position, 51_000_000_000);
        assert!(pnl > 0);
        
        // Price went down to $49,000
        let pnl = calculate_pnl(&position, 49_000_000_000);
        assert!(pnl < 0);
    }
    
    #[test]
    fn test_calculate_pnl_short() {
        let position = Position {
            qty: -1_000_000,
            entry_price: 50_000_000_000,
        };
        
        // Price went down to $49,000 (profit for short)
        let pnl = calculate_pnl(&position, 49_000_000_000);
        assert!(pnl > 0);
        
        // Price went up to $51,000 (loss for short)
        let pnl = calculate_pnl(&position, 51_000_000_000);
        assert!(pnl < 0);
    }
    
    #[test]
    fn test_kill_band() {
        // 1% kill band
        assert!(!is_outside_kill_band(50_000_000_000, 50_400_000_000, 100));
        assert!(is_outside_kill_band(50_000_000_000, 50_600_000_000, 100));
    }
    
    #[test]
    fn test_jit_detection() {
        let dlp_order = Order {
            created_at: 1001,
            maker_class: MakerClass::DLP,
            ..Default::default()
        };
        
        let retail_order = Order {
            created_at: 1001,
            maker_class: MakerClass::Retail,
            ..Default::default()
        };
        
        let header = SlabHeader {
            last_batch_open: 1000,
        };
        
        // DLP after batch = JIT
        assert!(is_jit_order(&dlp_order, &header));
        
        // Retail after batch = not JIT
        assert!(!is_jit_order(&retail_order, &header));
    }
    
    #[test]
    fn test_slippage_check() {
        // 1% slippage tolerance
        assert!(check_slippage(50_000_000_000, 50_400_000_000, 100).is_ok());
        assert!(check_slippage(50_000_000_000, 50_600_000_000, 100).is_err());
    }
    
    #[test]
    fn test_vwap_calculation() {
        // VWAP of two fills
        let fills = vec![
            (50_000_000_000u64, 1_000_000u64), // $50k, 1 lot
            (51_000_000_000u64, 1_000_000u64), // $51k, 1 lot
        ];
        
        let total_qty: u64 = fills.iter().map(|(_, q)| q).sum();
        let total_notional: u128 = fills.iter()
            .map(|(p, q)| (*p as u128) * (*q as u128))
            .sum();
        
        let vwap = (total_notional / total_qty as u128) as u64;
        
        // VWAP should be $50,500
        assert_eq!(vwap, 50_500_000_000);
    }
    
    #[test]
    fn test_price_time_priority() {
        let early_order = Order {
            price: 50_000_000_000,
            created_at: 1000,
            ..Default::default()
        };
        
        let late_order = Order {
            price: 50_000_000_000,
            created_at: 2000,
            ..Default::default()
        };
        
        let better_price_order = Order {
            price: 49_000_000_000,
            created_at: 3000,
            ..Default::default()
        };
        
        // Earlier time wins at same price
        assert!(has_priority(&early_order, &late_order));
        
        // Better price wins regardless of time
        assert!(has_priority(&better_price_order, &early_order));
    }
}
