//! Initialize instruction tests
//!
//! Tests for slab initialization and header setup.

#[cfg(test)]
mod initialize_tests {
    use crate::state::{SlabHeader, SlabState};
    use pinocchio::pubkey::Pubkey;

    #[test]
    fn test_slab_header_initialization() {
        // Full initialization with risk and anti-toxicity params
        let program_id = Pubkey::default();
        let lp_owner = Pubkey::from([1; 32]);
        let router_id = Pubkey::from([2; 32]);
        let imr_bps = 500u64;      // 5% IMR
        let mmr_bps = 250u64;      // 2.5% MMR
        let maker_fee_bps = -5i64; // -0.05% rebate
        let taker_fee_bps = 20u64; // 0.2% fee
        let batch_ms = 100u64;     // 100ms batch
        let bump = 255u8;

        let header = SlabHeader::new(
            program_id,
            lp_owner,
            router_id,
            imr_bps,
            mmr_bps,
            maker_fee_bps,
            taker_fee_bps,
            batch_ms,
            bump,
        );

        // Verify magic bytes and version
        assert_eq!(header.magic, *SlabHeader::MAGIC);
        assert_eq!(header.version, SlabHeader::VERSION);
        assert!(header.validate());

        // Verify parameters
        assert_eq!(header.program_id, program_id);
        assert_eq!(header.lp_owner, lp_owner);
        assert_eq!(header.router_id, router_id);
        assert_eq!(header.imr_bps, imr_bps);
        assert_eq!(header.mmr_bps, mmr_bps);
        assert_eq!(header.maker_fee_bps, maker_fee_bps);
        assert_eq!(header.taker_fee_bps, taker_fee_bps);
        assert_eq!(header.batch_ms, batch_ms);
        assert_eq!(header.bump, bump);

        // Verify seqno starts at 0
        assert_eq!(header.seqno, 0);
        
        // Verify IDs start at 1
        assert_eq!(header.next_order_id, 1);
        assert_eq!(header.next_hold_id, 1);
    }

    #[test]
    fn test_header_anti_toxicity_defaults() {
        let header = SlabHeader::new(
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            500, 250, -5, 20, 100, 255,
        );

        // Verify anti-toxicity defaults
        assert_eq!(header.kill_band_bps, 100);  // 1%
        assert_eq!(header.freeze_levels, 3);
        assert!(header.jit_penalty_on);
        assert_eq!(header.maker_rebate_min_ms, 50);
        assert!(header.arg_enabled);
    }

    #[test]
    fn test_slab_state_size() {
        // SlabState with full pools is large
        // Just verify it's a reasonable size
        use core::mem::size_of;
        let actual_size = size_of::<SlabState>();

        // Should be at least 1MB with pools
        assert!(actual_size > 1_000_000, "SlabState too small: {} bytes", actual_size);
    }

    #[test]
    fn test_header_size_matches() {
        use core::mem::size_of;
        let actual_size = size_of::<SlabHeader>();
        assert_eq!(actual_size, SlabHeader::LEN);
    }
}
