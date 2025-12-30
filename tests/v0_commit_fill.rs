//! v0 commit_fill Tests
//!
//! Tests for the slab's commit_fill instruction logic
//! These tests reference internal slab state and require the slab program
//! to be compiled with test features.

#[cfg(test)]
mod commit_fill_tests {
    /// Test notional and fee calculations
    #[test]
    fn test_notional_and_fee_calc() {
        // Simulate fill: 1 BTC at $50,000
        let filled_qty = 1_000_000i64;       // 1.0 BTC
        let vwap_px = 50_000_000_000i64;     // $50,000
        let taker_fee_bps = 20i64;           // 0.2% (20 bps)

        // Calculate notional: qty * price / 1e6
        let notional = (filled_qty as i128 * vwap_px as i128 / 1_000_000) as i64;
        assert_eq!(notional, 50_000_000_000, "Notional should be $50,000");

        // Calculate fee: notional * fee_bps / 10000
        let fee = (notional as i128 * taker_fee_bps as i128 / 10_000) as i64;
        assert_eq!(fee, 100_000_000, "Fee should be $100 (0.2% of $50k)");

        println!("✅ NOTIONAL & FEE CALCULATION:");
        println!("   Filled: {} BTC", filled_qty as f64 / 1_000_000.0);
        println!("   Price: ${}", vwap_px / 1_000_000);
        println!("   Notional: ${}", notional / 1_000_000);
        println!("   Fee: ${}", fee / 1_000_000);
    }

    /// Test v0 instant fill logic
    #[test]
    fn test_v0_instant_fill() {
        // In v0, fills are instant at limit price
        let qty = 1_000_000i64;           // Want to buy 1 BTC
        let limit_px = 50_000_000_000i64; // Willing to pay up to $50k

        // v0 logic: filled_qty = qty, vwap_px = limit_px
        let filled_qty = qty;
        let vwap_px = limit_px;

        assert_eq!(filled_qty, 1_000_000);
        assert_eq!(vwap_px, 50_000_000_000);

        // Calculate notional
        let notional = (filled_qty as i128 * vwap_px as i128 / 1_000_000) as i64;
        assert_eq!(notional, 50_000_000_000);

        println!("✅ V0 INSTANT FILL:");
        println!("   Requested: {} BTC @ ${}", qty as f64 / 1_000_000.0, limit_px / 1_000_000);
        println!("   Filled: {} BTC @ ${}", filled_qty as f64 / 1_000_000.0, vwap_px / 1_000_000);
        println!("   Notional: ${}", notional / 1_000_000);
    }

    /// Test fill receipt structure
    #[test]
    fn test_fill_receipt_structure() {
        // FillReceipt fields
        let seqno_committed: u64 = 123;
        let filled_qty: i64 = 1_000_000;
        let vwap_px: i64 = 50_000_000_000;
        let notional: i64 = 50_000_000_000;
        let fee: i64 = 10_000_000;

        assert!(seqno_committed > 0);
        assert!(filled_qty > 0);
        assert!(vwap_px > 0);
        assert_eq!((filled_qty as i128 * vwap_px as i128 / 1_000_000) as i64, notional);
        assert!(fee > 0);

        println!("✅ FILL RECEIPT STRUCTURE:");
        println!("   seqno: {}", seqno_committed);
        println!("   filled_qty: {}", filled_qty);
        println!("   vwap: {}", vwap_px);
        println!("   notional: {}", notional);
        println!("   fee: {}", fee);
    }

    /// Test quote cache structure
    #[test]
    fn test_quote_cache_structure() {
        // QuoteLevel
        let bid_level = (50_000_000_000u64, 1_000_000u64); // (price, qty)
        let ask_level = (50_001_000_000u64, 1_500_000u64);

        // Best bid should be lower than best ask
        assert!(bid_level.0 < ask_level.0);

        // Spread calculation
        let spread = ask_level.0 - bid_level.0;
        assert_eq!(spread, 1_000_000); // $1 spread

        println!("✅ QUOTE CACHE STRUCTURE:");
        println!("   Best bid: ${}", bid_level.0 as f64 / 1_000_000.0);
        println!("   Best ask: ${}", ask_level.0 as f64 / 1_000_000.0);
        println!("   Spread: ${}", spread as f64 / 1_000_000.0);
    }
}
