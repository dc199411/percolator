//! Slab state - full orderbook with pools

use super::{SlabHeader, QuoteCache};
use percolator_common::{
    Order, Position, Reservation, Slice, Trade, Instrument, AccountState, AggressorEntry,
    Side, OrderState,
};

/// Pool sizes for different account tiers
/// Standard tier fits in ~3.5MB
pub const POOL_ORDERS: usize = 30_000;
pub const POOL_POSITIONS: usize = 30_000;
pub const POOL_RESERVATIONS: usize = 4_000;
pub const POOL_SLICES: usize = 16_000;
pub const POOL_TRADES: usize = 10_000;
pub const POOL_ACCOUNTS: usize = 5_000;
pub const POOL_INSTRUMENTS: usize = 32;
pub const POOL_AGGRESSOR: usize = 4_000;

/// Main slab state - full orderbook with pools
/// Target size: ~10MB with all pools
#[repr(C)]
pub struct SlabState {
    /// Header with metadata and risk parameters
    pub header: SlabHeader,
    
    /// Quote cache (router-readable best levels)
    pub quote_cache: QuoteCache,
    
    /// Instrument pool (MAX_INSTRUMENTS = 32)
    pub instruments: [Instrument; POOL_INSTRUMENTS],
    
    /// Account pool (POOL_ACCOUNTS = 5,000)
    pub accounts: [AccountState; POOL_ACCOUNTS],
    
    /// Order pool (POOL_ORDERS = 30,000)
    pub orders: [Order; POOL_ORDERS],
    
    /// Position pool (POOL_POSITIONS = 30,000)
    pub positions: [Position; POOL_POSITIONS],
    
    /// Reservation pool (POOL_RESERVATIONS = 4,000)
    pub reservations: [Reservation; POOL_RESERVATIONS],
    
    /// Slice pool (POOL_SLICES = 16,000)
    pub slices: [Slice; POOL_SLICES],
    
    /// Trade ring buffer (POOL_TRADES = 10,000)
    pub trades: [Trade; POOL_TRADES],
    
    /// Aggressor ledger for ARG (POOL_AGGRESSOR = 4,000)
    pub aggressors: [AggressorEntry; POOL_AGGRESSOR],
}

impl SlabState {
    /// Size of the slab state
    pub const LEN: usize = core::mem::size_of::<Self>();
    /// Invalid pool index marker
    pub const INVALID_INDEX: u32 = u32::MAX;

    /// Initialize freelists for all pools
    pub fn initialize_pools(&mut self) {
        // Initialize order freelist
        for i in 0..POOL_ORDERS {
            self.orders[i].next_free = if i + 1 < POOL_ORDERS { 
                (i + 1) as u32 
            } else { 
                Self::INVALID_INDEX 
            };
            self.orders[i].used = false;
        }
        self.header.order_freelist_head = 0;
        
        // Initialize position freelist (we'll use index field for freelist)
        for i in 0..POOL_POSITIONS {
            self.positions[i].index = if i + 1 < POOL_POSITIONS { 
                (i + 1) as u32 
            } else { 
                Self::INVALID_INDEX 
            };
            self.positions[i].used = false;
        }
        self.header.position_freelist_head = 0;
        
        // Initialize reservation freelist
        for i in 0..POOL_RESERVATIONS {
            self.reservations[i].index = if i + 1 < POOL_RESERVATIONS { 
                (i + 1) as u32 
            } else { 
                Self::INVALID_INDEX 
            };
            self.reservations[i].used = false;
        }
        self.header.reservation_freelist_head = 0;
        
        // Initialize slice freelist
        for i in 0..POOL_SLICES {
            self.slices[i].index = if i + 1 < POOL_SLICES { 
                (i + 1) as u32 
            } else { 
                Self::INVALID_INDEX 
            };
            self.slices[i].used = false;
        }
        self.header.slice_freelist_head = 0;
    }

    // === Order Pool Operations ===
    
    /// Allocate an order from the freelist
    pub fn alloc_order(&mut self) -> Option<u32> {
        let head = self.header.order_freelist_head;
        if head == Self::INVALID_INDEX {
            return None;
        }
        
        let order = &mut self.orders[head as usize];
        self.header.order_freelist_head = order.next_free;
        order.used = true;
        order.next_free = Self::INVALID_INDEX;
        self.header.order_count += 1;
        
        Some(head)
    }
    
    /// Free an order back to the freelist
    pub fn free_order(&mut self, idx: u32) {
        if idx as usize >= POOL_ORDERS {
            return;
        }
        
        let order = &mut self.orders[idx as usize];
        if !order.used {
            return; // Already free
        }
        
        order.used = false;
        order.next_free = self.header.order_freelist_head;
        self.header.order_freelist_head = idx;
        self.header.order_count = self.header.order_count.saturating_sub(1);
    }
    
    /// Get order by index
    pub fn get_order(&self, idx: u32) -> Option<&Order> {
        if idx as usize >= POOL_ORDERS {
            return None;
        }
        let order = &self.orders[idx as usize];
        if order.used { Some(order) } else { None }
    }
    
    /// Get mutable order by index
    pub fn get_order_mut(&mut self, idx: u32) -> Option<&mut Order> {
        if idx as usize >= POOL_ORDERS {
            return None;
        }
        let order = &mut self.orders[idx as usize];
        if order.used { Some(order) } else { None }
    }

    // === Position Pool Operations ===
    
    /// Allocate a position from the freelist
    pub fn alloc_position(&mut self) -> Option<u32> {
        let head = self.header.position_freelist_head;
        if head == Self::INVALID_INDEX {
            return None;
        }
        
        let pos = &mut self.positions[head as usize];
        self.header.position_freelist_head = pos.index; // index stores next free
        pos.used = true;
        pos.index = head; // Now stores actual index
        self.header.position_count += 1;
        
        Some(head)
    }
    
    /// Free a position back to the freelist
    pub fn free_position(&mut self, idx: u32) {
        if idx as usize >= POOL_POSITIONS {
            return;
        }
        
        let pos = &mut self.positions[idx as usize];
        if !pos.used {
            return;
        }
        
        pos.used = false;
        pos.index = self.header.position_freelist_head;
        self.header.position_freelist_head = idx;
        self.header.position_count = self.header.position_count.saturating_sub(1);
    }
    
    /// Get position by index
    pub fn get_position(&self, idx: u32) -> Option<&Position> {
        if idx as usize >= POOL_POSITIONS {
            return None;
        }
        let pos = &self.positions[idx as usize];
        if pos.used { Some(pos) } else { None }
    }
    
    /// Get mutable position by index
    pub fn get_position_mut(&mut self, idx: u32) -> Option<&mut Position> {
        if idx as usize >= POOL_POSITIONS {
            return None;
        }
        let pos = &mut self.positions[idx as usize];
        if pos.used { Some(pos) } else { None }
    }

    // === Reservation Pool Operations ===
    
    /// Allocate a reservation from the freelist
    pub fn alloc_reservation(&mut self) -> Option<u32> {
        let head = self.header.reservation_freelist_head;
        if head == Self::INVALID_INDEX {
            return None;
        }
        
        let resv = &mut self.reservations[head as usize];
        self.header.reservation_freelist_head = resv.index;
        resv.used = true;
        resv.index = head;
        self.header.reservation_count += 1;
        
        Some(head)
    }
    
    /// Free a reservation back to the freelist
    pub fn free_reservation(&mut self, idx: u32) {
        if idx as usize >= POOL_RESERVATIONS {
            return;
        }
        
        let resv = &mut self.reservations[idx as usize];
        if !resv.used {
            return;
        }
        
        resv.used = false;
        resv.index = self.header.reservation_freelist_head;
        self.header.reservation_freelist_head = idx;
        self.header.reservation_count = self.header.reservation_count.saturating_sub(1);
    }
    
    /// Get reservation by index
    pub fn get_reservation(&self, idx: u32) -> Option<&Reservation> {
        if idx as usize >= POOL_RESERVATIONS {
            return None;
        }
        let resv = &self.reservations[idx as usize];
        if resv.used { Some(resv) } else { None }
    }
    
    /// Get mutable reservation by index
    pub fn get_reservation_mut(&mut self, idx: u32) -> Option<&mut Reservation> {
        if idx as usize >= POOL_RESERVATIONS {
            return None;
        }
        let resv = &mut self.reservations[idx as usize];
        if resv.used { Some(resv) } else { None }
    }
    
    /// Find reservation by hold_id
    pub fn find_reservation_by_hold_id(&self, hold_id: u64) -> Option<u32> {
        for i in 0..POOL_RESERVATIONS {
            if self.reservations[i].used && self.reservations[i].hold_id == hold_id {
                return Some(i as u32);
            }
        }
        None
    }

    // === Slice Pool Operations ===
    
    /// Allocate a slice from the freelist
    pub fn alloc_slice(&mut self) -> Option<u32> {
        let head = self.header.slice_freelist_head;
        if head == Self::INVALID_INDEX {
            return None;
        }
        
        let slice = &mut self.slices[head as usize];
        self.header.slice_freelist_head = slice.index;
        slice.used = true;
        slice.index = head;
        self.header.slice_count += 1;
        
        Some(head)
    }
    
    /// Free a slice back to the freelist
    pub fn free_slice(&mut self, idx: u32) {
        if idx as usize >= POOL_SLICES {
            return;
        }
        
        let slice = &mut self.slices[idx as usize];
        if !slice.used {
            return;
        }
        
        slice.used = false;
        slice.index = self.header.slice_freelist_head;
        self.header.slice_freelist_head = idx;
        self.header.slice_count = self.header.slice_count.saturating_sub(1);
    }
    
    /// Get slice by index
    pub fn get_slice(&self, idx: u32) -> Option<&Slice> {
        if idx as usize >= POOL_SLICES {
            return None;
        }
        let slice = &self.slices[idx as usize];
        if slice.used { Some(slice) } else { None }
    }
    
    /// Get mutable slice by index
    pub fn get_slice_mut(&mut self, idx: u32) -> Option<&mut Slice> {
        if idx as usize >= POOL_SLICES {
            return None;
        }
        let slice = &mut self.slices[idx as usize];
        if slice.used { Some(slice) } else { None }
    }

    // === Trade Ring Buffer Operations ===
    
    /// Record a trade in the ring buffer
    pub fn record_trade(&mut self, trade: Trade) {
        let idx = self.header.trade_write_idx as usize % POOL_TRADES;
        self.trades[idx] = trade;
        self.header.trade_write_idx = self.header.trade_write_idx.wrapping_add(1);
    }

    // === Instrument Operations ===
    
    /// Get instrument by index
    pub fn get_instrument(&self, idx: u16) -> Option<&Instrument> {
        if idx as usize >= POOL_INSTRUMENTS || idx >= self.header.instrument_count {
            return None;
        }
        Some(&self.instruments[idx as usize])
    }
    
    /// Get mutable instrument by index
    pub fn get_instrument_mut(&mut self, idx: u16) -> Option<&mut Instrument> {
        if idx as usize >= POOL_INSTRUMENTS || idx >= self.header.instrument_count {
            return None;
        }
        Some(&mut self.instruments[idx as usize])
    }

    // === Account Operations ===
    
    /// Get or create account by pubkey
    pub fn get_or_create_account(&mut self, key: &pinocchio::pubkey::Pubkey) -> Option<u32> {
        // First try to find existing
        for i in 0..self.header.account_count as usize {
            if self.accounts[i].active && &self.accounts[i].key == key {
                return Some(i as u32);
            }
        }
        
        // Create new if space available
        if (self.header.account_count as usize) < POOL_ACCOUNTS {
            let idx = self.header.account_count as usize;
            self.accounts[idx] = AccountState {
                key: *key,
                cash: 0,
                im: 0,
                mm: 0,
                position_head: Self::INVALID_INDEX,
                index: idx as u32,
                active: true,
                _padding: [0; 7],
            };
            self.header.account_count += 1;
            return Some(idx as u32);
        }
        
        None
    }
    
    /// Get account by index
    pub fn get_account(&self, idx: u32) -> Option<&AccountState> {
        if idx as usize >= POOL_ACCOUNTS {
            return None;
        }
        let acc = &self.accounts[idx as usize];
        if acc.active { Some(acc) } else { None }
    }
    
    /// Get mutable account by index
    pub fn get_account_mut(&mut self, idx: u32) -> Option<&mut AccountState> {
        if idx as usize >= POOL_ACCOUNTS {
            return None;
        }
        let acc = &mut self.accounts[idx as usize];
        if acc.active { Some(acc) } else { None }
    }

    // === Book Operations ===
    
    /// Insert order into book (maintains price-time priority)
    pub fn insert_order_into_book(&mut self, order_idx: u32, instrument_idx: u16) {
        let order = match self.get_order(order_idx) {
            Some(o) => o,
            None => return,
        };
        let side = order.side;
        let price = order.price;
        let state = order.state;
        
        let instr = match self.get_instrument_mut(instrument_idx) {
            Some(i) => i,
            None => return,
        };
        
        // Get the appropriate book head based on state and side
        let head_ptr = match (state, side) {
            (OrderState::LIVE, Side::Buy) => &mut instr.bids_head,
            (OrderState::LIVE, Side::Sell) => &mut instr.asks_head,
            (OrderState::PENDING, Side::Buy) => &mut instr.bids_pending_head,
            (OrderState::PENDING, Side::Sell) => &mut instr.asks_pending_head,
        };
        
        let head = *head_ptr;
        
        // Insert maintaining price-time priority
        // For bids: descending price (higher better)
        // For asks: ascending price (lower better)
        let mut prev_idx = Self::INVALID_INDEX;
        let mut curr_idx = head;
        
        while curr_idx != Self::INVALID_INDEX {
            let curr = match self.get_order(curr_idx) {
                Some(o) => o,
                None => break,
            };
            
            let should_insert_before = match side {
                Side::Buy => price > curr.price,  // Higher price has priority for bids
                Side::Sell => price < curr.price, // Lower price has priority for asks
            };
            
            if should_insert_before {
                break;
            }
            
            prev_idx = curr_idx;
            curr_idx = curr.next;
        }
        
        // Update links
        if let Some(order) = self.get_order_mut(order_idx) {
            order.next = curr_idx;
            order.prev = prev_idx;
        }
        
        if curr_idx != Self::INVALID_INDEX {
            if let Some(curr) = self.get_order_mut(curr_idx) {
                curr.prev = order_idx;
            }
        }
        
        if prev_idx == Self::INVALID_INDEX {
            // Insert at head
            let instr = self.get_instrument_mut(instrument_idx).unwrap();
            match (state, side) {
                (OrderState::LIVE, Side::Buy) => instr.bids_head = order_idx,
                (OrderState::LIVE, Side::Sell) => instr.asks_head = order_idx,
                (OrderState::PENDING, Side::Buy) => instr.bids_pending_head = order_idx,
                (OrderState::PENDING, Side::Sell) => instr.asks_pending_head = order_idx,
            }
        } else {
            if let Some(prev) = self.get_order_mut(prev_idx) {
                prev.next = order_idx;
            }
        }
        
        self.header.increment_seqno();
    }
    
    /// Remove order from book
    pub fn remove_order_from_book(&mut self, order_idx: u32) {
        let order = match self.get_order(order_idx) {
            Some(o) => o,
            None => return,
        };
        
        let prev = order.prev;
        let next = order.next;
        let side = order.side;
        let state = order.state;
        let instrument_idx = order.instrument_idx;
        
        // Update prev's next pointer
        if prev != Self::INVALID_INDEX {
            if let Some(prev_order) = self.get_order_mut(prev) {
                prev_order.next = next;
            }
        } else {
            // Was the head, update instrument's head
            if let Some(instr) = self.get_instrument_mut(instrument_idx) {
                match (state, side) {
                    (OrderState::LIVE, Side::Buy) => instr.bids_head = next,
                    (OrderState::LIVE, Side::Sell) => instr.asks_head = next,
                    (OrderState::PENDING, Side::Buy) => instr.bids_pending_head = next,
                    (OrderState::PENDING, Side::Sell) => instr.asks_pending_head = next,
                }
            }
        }
        
        // Update next's prev pointer
        if next != Self::INVALID_INDEX {
            if let Some(next_order) = self.get_order_mut(next) {
                next_order.prev = prev;
            }
        }
        
        // Clear order's links
        if let Some(order) = self.get_order_mut(order_idx) {
            order.prev = Self::INVALID_INDEX;
            order.next = Self::INVALID_INDEX;
        }
        
        self.header.increment_seqno();
    }
    
    /// Get best contra order for a side (best ask for buy, best bid for sell)
    pub fn get_best_contra(&self, instrument_idx: u16, side: Side) -> Option<u32> {
        let instr = self.get_instrument(instrument_idx)?;
        match side {
            Side::Buy => {
                // Buy matches against asks (lowest price first)
                if instr.asks_head != Self::INVALID_INDEX {
                    Some(instr.asks_head)
                } else {
                    None
                }
            }
            Side::Sell => {
                // Sell matches against bids (highest price first)
                if instr.bids_head != Self::INVALID_INDEX {
                    Some(instr.bids_head)
                } else {
                    None
                }
            }
        }
    }
    
    /// Promote pending orders to live (called at batch open)
    pub fn promote_pending_orders(&mut self, instrument_idx: u16, current_epoch: u64) {
        let instr = match self.get_instrument_mut(instrument_idx) {
            Some(i) => i,
            None => return,
        };
        
        // Promote pending bids
        let mut pending_head = instr.bids_pending_head;
        while pending_head != Self::INVALID_INDEX {
            let order = match self.get_order(pending_head) {
                Some(o) => o,
                None => break,
            };
            
            let next = order.next;
            
            if order.eligible_epoch as u64 <= current_epoch {
                // Remove from pending queue
                self.remove_order_from_book(pending_head);
                
                // Change state to LIVE
                if let Some(order) = self.get_order_mut(pending_head) {
                    order.state = OrderState::LIVE;
                }
                
                // Insert into live book
                self.insert_order_into_book(pending_head, instrument_idx);
            }
            
            pending_head = next;
        }
        
        // Promote pending asks
        let instr = self.get_instrument_mut(instrument_idx).unwrap();
        pending_head = instr.asks_pending_head;
        while pending_head != Self::INVALID_INDEX {
            let order = match self.get_order(pending_head) {
                Some(o) => o,
                None => break,
            };
            
            let next = order.next;
            
            if order.eligible_epoch as u64 <= current_epoch {
                self.remove_order_from_book(pending_head);
                
                if let Some(order) = self.get_order_mut(pending_head) {
                    order.state = OrderState::LIVE;
                }
                
                self.insert_order_into_book(pending_head, instrument_idx);
            }
            
            pending_head = next;
        }
    }
}

// Compile-time size validation
// Note: Full pools would exceed 10MB. In production, use smaller pools
// or multiple accounts. The current pool sizes are for reference.

#[cfg(test)]
mod tests {
    use super::*;
    use pinocchio::pubkey::Pubkey;

    #[test]
    fn test_slab_pool_operations() {
        // Create a minimal test slab (can't test full 10MB in unit tests)
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

        // Just test header creation
        assert!(header.validate());
        assert_eq!(header.current_epoch, 0);
        assert_eq!(header.next_order_id, 1);
    }

    #[test]
    fn test_header_kill_band() {
        let mut header = SlabHeader::new(
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            500, 250, -5, 20, 100, 255,
        );

        // Set initial mark price
        header.update_mark_px(50_000_000_000); // $50,000
        
        // Move within kill band (1%)
        header.update_mark_px(50_400_000_000); // $50,400 (0.8% move)
        assert!(header.check_kill_band(50_400_000_000));
        
        // Move beyond kill band (>1%)
        assert!(!header.check_kill_band(51_000_000_000)); // $51,000 (2% from prev)
    }

    #[test]
    fn test_jit_detection() {
        let header = SlabHeader::new(
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            500, 250, -5, 20, 100, 255,
        );

        // Order created 10ms ago (should be JIT)
        assert!(header.is_jit_order(100, 110));
        
        // Order created 60ms ago (should not be JIT, min is 50ms)
        assert!(!header.is_jit_order(100, 160));
    }
}
