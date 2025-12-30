//! Slab header - metadata and anti-toxicity parameters

use pinocchio::pubkey::Pubkey;

/// Slab header with full anti-toxicity parameters
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SlabHeader {
    /// Magic bytes for validation (b"PERP10\0\0")
    pub magic: [u8; 8],
    /// Version (=1)
    pub version: u32,
    /// Sequence number (incremented on any book/state change)
    pub seqno: u32,

    /// Slab program ID
    pub program_id: Pubkey,
    /// LP owner pubkey
    pub lp_owner: Pubkey,
    /// Router program ID (only router can call commit)
    pub router_id: Pubkey,

    // === Risk Parameters ===
    /// Initial margin ratio (basis points, e.g., 500 = 5%)
    pub imr_bps: u64,
    /// Maintenance margin ratio (basis points, e.g., 250 = 2.5%)
    pub mmr_bps: u64,
    /// Maker fee (signed, can be negative for rebates)
    pub maker_fee_bps: i64,
    /// Taker fee (basis points)
    pub taker_fee_bps: u64,

    // === Anti-Toxicity Parameters ===
    /// Batch window in milliseconds (e.g., 50-100 ms)
    pub batch_ms: u64,
    /// Kill band in basis points (reject if mark moved > this amount)
    pub kill_band_bps: u64,
    /// Freeze level count (top-K orders to freeze in batch)
    pub freeze_levels: u16,
    /// JIT penalty enabled flag
    pub jit_penalty_on: bool,
    /// Minimum maker age for rebate (milliseconds)
    pub maker_rebate_min_ms: u64,
    /// ARG (Aggressor Roundtrip Guard) enabled
    pub arg_enabled: bool,
    /// ARG tax rate (basis points)
    pub arg_tax_bps: u64,

    // === State Tracking ===
    /// Current epoch
    pub current_epoch: u64,
    /// Next order ID (monotonic)
    pub next_order_id: u64,
    /// Next hold ID for reservations
    pub next_hold_id: u64,
    /// Last batch open timestamp
    pub last_batch_open_ts: u64,
    /// Last funding update timestamp
    pub last_funding_ts: u64,
    /// Mark price from oracle (1e6 scale)
    pub mark_px: i64,
    /// Previous mark price (for kill band check)
    pub prev_mark_px: i64,

    // === Pool Counts ===
    /// Number of active instruments
    pub instrument_count: u16,
    /// Number of active accounts
    pub account_count: u16,
    /// Number of active orders
    pub order_count: u32,
    /// Number of active positions
    pub position_count: u32,
    /// Number of active reservations
    pub reservation_count: u32,
    /// Number of active slices
    pub slice_count: u32,
    /// Trade ring write index
    pub trade_write_idx: u32,

    // === Freelist Heads ===
    /// Order pool freelist head
    pub order_freelist_head: u32,
    /// Position pool freelist head
    pub position_freelist_head: u32,
    /// Reservation pool freelist head
    pub reservation_freelist_head: u32,
    /// Slice pool freelist head
    pub slice_freelist_head: u32,

    /// Bump seed
    pub bump: u8,
    /// Padding for alignment
    pub _padding: [u8; 7],
}

impl SlabHeader {
    pub const MAGIC: &'static [u8; 8] = b"PERP10\0\0";
    pub const VERSION: u32 = 1;
    pub const LEN: usize = core::mem::size_of::<Self>();
    pub const INVALID_INDEX: u32 = u32::MAX;

    /// Initialize new slab header with full parameters
    pub fn new(
        program_id: Pubkey,
        lp_owner: Pubkey,
        router_id: Pubkey,
        imr_bps: u64,
        mmr_bps: u64,
        maker_fee_bps: i64,
        taker_fee_bps: u64,
        batch_ms: u64,
        bump: u8,
    ) -> Self {
        Self {
            magic: *Self::MAGIC,
            version: Self::VERSION,
            seqno: 0,
            program_id,
            lp_owner,
            router_id,
            // Risk params
            imr_bps,
            mmr_bps,
            maker_fee_bps,
            taker_fee_bps,
            // Anti-toxicity defaults
            batch_ms,
            kill_band_bps: 100, // 1% default kill band
            freeze_levels: 3,   // Freeze top 3 levels
            jit_penalty_on: true,
            maker_rebate_min_ms: 50, // 50ms min age for rebate
            arg_enabled: true,
            arg_tax_bps: 50, // 0.5% ARG tax
            // State
            current_epoch: 0,
            next_order_id: 1,
            next_hold_id: 1,
            last_batch_open_ts: 0,
            last_funding_ts: 0,
            mark_px: 0,
            prev_mark_px: 0,
            // Pool counts
            instrument_count: 0,
            account_count: 0,
            order_count: 0,
            position_count: 0,
            reservation_count: 0,
            slice_count: 0,
            trade_write_idx: 0,
            // Freelist heads (initialized to INVALID_INDEX = empty)
            order_freelist_head: Self::INVALID_INDEX,
            position_freelist_head: Self::INVALID_INDEX,
            reservation_freelist_head: Self::INVALID_INDEX,
            slice_freelist_head: Self::INVALID_INDEX,
            bump,
            _padding: [0; 7],
        }
    }

    /// Validate magic and version
    pub fn validate(&self) -> bool {
        &self.magic == Self::MAGIC && self.version == Self::VERSION
    }

    /// Increment sequence number (on any book change)
    pub fn increment_seqno(&mut self) -> u32 {
        self.seqno = self.seqno.wrapping_add(1);
        self.seqno
    }

    /// Allocate next order ID
    pub fn next_order_id(&mut self) -> u64 {
        let id = self.next_order_id;
        self.next_order_id = self.next_order_id.wrapping_add(1);
        id
    }

    /// Allocate next hold ID for reservations
    pub fn next_hold_id(&mut self) -> u64 {
        let id = self.next_hold_id;
        self.next_hold_id = self.next_hold_id.wrapping_add(1);
        id
    }

    /// Check if mark price moved beyond kill band
    pub fn check_kill_band(&self, new_mark_px: i64) -> bool {
        if self.prev_mark_px == 0 {
            return true; // No previous price, allow
        }

        let diff = (new_mark_px - self.prev_mark_px).abs();
        let threshold = (self.prev_mark_px.abs() as u64 * self.kill_band_bps) / 10_000;
        
        diff <= threshold as i64
    }

    /// Check if an order qualifies for JIT penalty (posted too recently)
    pub fn is_jit_order(&self, order_created_ts: u64, current_ts: u64) -> bool {
        if !self.jit_penalty_on {
            return false;
        }
        current_ts.saturating_sub(order_created_ts) < self.maker_rebate_min_ms
    }

    /// Update mark price (stores previous for kill band check)
    pub fn update_mark_px(&mut self, new_mark_px: i64) {
        self.prev_mark_px = self.mark_px;
        self.mark_px = new_mark_px;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_validation() {
        let header = SlabHeader::new(
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            500,  // 5% IMR
            250,  // 2.5% MMR
            -5,   // -0.05% maker rebate
            20,   // 0.2% taker fee
            100,  // 100ms batch
            255,
        );

        assert!(header.validate());
        assert_eq!(header.seqno, 0);
        assert_eq!(header.version, 1);
        assert_eq!(header.magic, *SlabHeader::MAGIC);
    }

    #[test]
    fn test_seqno_increment() {
        let mut header = SlabHeader::new(
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            500, 250, -5, 20, 100, 255,
        );

        assert_eq!(header.seqno, 0);
        assert_eq!(header.increment_seqno(), 1);
        assert_eq!(header.increment_seqno(), 2);
        assert_eq!(header.seqno, 2);
    }

    #[test]
    fn test_order_id_allocation() {
        let mut header = SlabHeader::new(
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            500, 250, -5, 20, 100, 255,
        );

        assert_eq!(header.next_order_id(), 1);
        assert_eq!(header.next_order_id(), 2);
        assert_eq!(header.next_order_id(), 3);
    }

    #[test]
    fn test_hold_id_allocation() {
        let mut header = SlabHeader::new(
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            500, 250, -5, 20, 100, 255,
        );

        assert_eq!(header.next_hold_id(), 1);
        assert_eq!(header.next_hold_id(), 2);
    }

    #[test]
    fn test_kill_band_check() {
        let mut header = SlabHeader::new(
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            500, 250, -5, 20, 100, 255,
        );

        // Set initial mark price
        header.update_mark_px(50_000_000_000); // $50,000
        
        // Move within kill band (1%)
        assert!(header.check_kill_band(50_400_000_000)); // $50,400 (0.8% move)
        
        // Move beyond kill band (>1%)
        header.update_mark_px(50_400_000_000);
        assert!(!header.check_kill_band(51_500_000_000)); // $51,500 (2.2% from prev)
    }

    #[test]
    fn test_jit_detection() {
        let header = SlabHeader::new(
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            500, 250, -5, 20, 100, 255,
        );

        // Order created 10ms ago (should be JIT, min is 50ms)
        assert!(header.is_jit_order(100, 110));
        
        // Order created 60ms ago (should not be JIT)
        assert!(!header.is_jit_order(100, 160));
    }

    #[test]
    fn test_anti_toxicity_defaults() {
        let header = SlabHeader::new(
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            500, 250, -5, 20, 100, 255,
        );

        assert_eq!(header.kill_band_bps, 100); // 1%
        assert_eq!(header.freeze_levels, 3);
        assert!(header.jit_penalty_on);
        assert_eq!(header.maker_rebate_min_ms, 50);
        assert!(header.arg_enabled);
        assert_eq!(header.arg_tax_bps, 50); // 0.5%
    }
}
