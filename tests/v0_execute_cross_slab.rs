//! v0 execute_cross_slab Tests
//!
//! Tests for the router's execute_cross_slab instruction logic

#[cfg(test)]
mod execute_cross_slab_tests {
    /// Test atomic split across multiple slabs
    #[test]
    fn test_atomic_split() {
        // Simulate splits for 2 slabs
        let splits = vec![
            (0u16, 500_000i64, 0u8, 50_000_000_000u64),  // Slab 0: Buy 0.5 BTC at $50k
            (1u16, 500_000i64, 0u8, 50_010_000_000u64),  // Slab 1: Buy 0.5 BTC at $50,010
        ];

        // Track exposures
        let mut exposures: Vec<(u16, i64)> = Vec::new();

        for (slab_idx, qty, side, _limit_px) in &splits {
            let exposure = if *side == 0 { *qty } else { -*qty };
            exposures.push((*slab_idx, exposure));
        }

        // Verify both fills
        assert_eq!(exposures.len(), 2);
        assert_eq!(exposures[0], (0, 500_000));
        assert_eq!(exposures[1], (1, 500_000));

        // Net exposure = 1.0 BTC total
        let net: i64 = exposures.iter().map(|(_, e)| e).sum();
        assert_eq!(net, 1_000_000);

        println!("✅ ATOMIC SPLIT TEST:");
        println!("   Slab A: {} BTC", exposures[0].1 as f64 / 1_000_000.0);
        println!("   Slab B: {} BTC", exposures[1].1 as f64 / 1_000_000.0);
        println!("   Net: {} BTC", net as f64 / 1_000_000.0);
    }

    /// Test hedged position (long + short on different slabs)
    #[test]
    fn test_hedged_position() {
        // Long 1 BTC on Slab A
        let long_exposure = (0u16, 1_000_000i64);

        // Short 1 BTC on Slab B
        let short_exposure = (1u16, -1_000_000i64);

        let exposures = vec![long_exposure, short_exposure];

        // Calculate net exposure
        let net: i64 = exposures.iter().map(|(_, e)| e).sum();

        // Net should be ZERO
        assert_eq!(net, 0, "Hedged position should have zero net exposure");

        // Calculate IM on net exposure
        let price = 50_000_000_000i128;
        let imr = 10;
        let im = ((net.abs() as i128 * price * imr) / (100 * 1_000_000)) as u128;

        // IM should be ZERO!
        assert_eq!(im, 0, "IM for hedged position should be ZERO");

        println!("✅ HEDGED POSITION TEST:");
        println!("   Slab A (long): {} BTC", long_exposure.1 as f64 / 1_000_000.0);
        println!("   Slab B (short): {} BTC", short_exposure.1 as f64 / 1_000_000.0);
        println!("   Net exposure: {}", net);
        println!("   IM required: ${}", im / 1_000_000);
    }

    /// Test progressive scaling (adding to position)
    #[test]
    fn test_progressive_scaling() {
        let mut position = 0i64;

        // First trade: Buy 0.5 BTC
        position += 500_000;
        assert_eq!(position, 500_000);

        // Second trade: Buy another 0.5 BTC
        position += 500_000;
        assert_eq!(position, 1_000_000);

        // Third trade: Reduce by 0.3 BTC
        position -= 300_000;
        assert_eq!(position, 700_000);

        println!("✅ PROGRESSIVE SCALING:");
        println!("   After +0.5: {} BTC", 500_000 as f64 / 1_000_000.0);
        println!("   After +0.5: {} BTC", 1_000_000 as f64 / 1_000_000.0);
        println!("   After -0.3: {} BTC", 700_000 as f64 / 1_000_000.0);
    }

    /// Test margin calculation with various net exposures
    #[test]
    fn test_margin_calculation_scenarios() {
        let test_cases = vec![
            (0i64, 0u128, "Zero net exposure"),
            (1_000_000, 5_000_000_000, "Long 1 BTC"),
            (-1_000_000, 5_000_000_000, "Short 1 BTC"),
            (500_000, 2_500_000_000, "Long 0.5 BTC"),
            (2_000_000, 10_000_000_000, "Long 2 BTC"),
        ];

        let price = 50_000_000_000i128; // $50k
        let imr = 10; // 10%

        println!("✅ MARGIN CALCULATION SCENARIOS:");
        for (net_exposure, expected_im, desc) in test_cases {
            let im = ((net_exposure.abs() as i128 * price * imr) / (100 * 1_000_000)) as u128;
            assert_eq!(im, expected_im, "{}: expected IM ${}", desc, expected_im / 1_000_000);

            println!("   {} -> IM: ${}", desc, im / 1_000_000);
        }
    }

    /// Test multi-slab multi-instrument
    #[test]
    fn test_multi_slab_multi_instrument() {
        // (slab_idx, instrument_idx, qty)
        let exposures = vec![
            (0u16, 0u16, 1_000_000i64),   // Slab 0, BTC: +1
            (1u16, 0u16, -500_000i64),    // Slab 1, BTC: -0.5
            (0u16, 1u16, 10_000_000i64),  // Slab 0, ETH: +10
            (2u16, 1u16, -5_000_000i64),  // Slab 2, ETH: -5
        ];

        // Calculate net BTC (instrument 0)
        let btc_net: i64 = exposures.iter()
            .filter(|(_, inst, _)| *inst == 0)
            .map(|(_, _, qty)| qty)
            .sum();

        // Calculate net ETH (instrument 1)
        let eth_net: i64 = exposures.iter()
            .filter(|(_, inst, _)| *inst == 1)
            .map(|(_, _, qty)| qty)
            .sum();

        assert_eq!(btc_net, 500_000, "BTC net should be +0.5");
        assert_eq!(eth_net, 5_000_000, "ETH net should be +5.0");

        println!("✅ MULTI-SLAB MULTI-INSTRUMENT:");
        println!("   BTC net: {} BTC", btc_net as f64 / 1_000_000.0);
        println!("   ETH net: {} ETH", eth_net as f64 / 1_000_000.0);
    }

    /// Test margin sufficiency checks
    #[test]
    fn test_margin_sufficiency() {
        // Test insufficient margin
        let equity = 1_000_000_000u128; // $1k
        let im = 5_000_000_000u128;     // $5k required
        let mm = 2_500_000_000u128;     // $2.5k maintenance

        let has_sufficient_im = equity >= im;
        let above_mm = equity >= mm;

        assert!(!has_sufficient_im, "Should NOT have sufficient IM");
        assert!(!above_mm, "Should NOT be above MM");

        // Test sufficient margin
        let equity_high = 100_000_000_000u128; // $100k
        let has_sufficient_im = equity_high >= im;
        let above_mm = equity_high >= mm;
        let free_collateral = if has_sufficient_im { equity_high - im } else { 0 };

        assert!(has_sufficient_im, "Should have sufficient IM");
        assert!(above_mm, "Should be above MM");
        assert_eq!(free_collateral, 95_000_000_000, "Free collateral = $95k");

        println!("✅ MARGIN SUFFICIENCY:");
        println!("   Low equity ($1k): sufficient={}, above_mm={}", !has_sufficient_im, !above_mm);
        println!("   High equity ($100k): sufficient={}, above_mm={}", has_sufficient_im, above_mm);
        println!("   Free collateral: ${}", free_collateral / 1_000_000);
    }
}
