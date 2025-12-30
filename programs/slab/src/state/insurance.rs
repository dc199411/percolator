//! Slab-Level Insurance Pool
//!
//! Per-slab insurance fund for covering liquidation shortfalls and
//! socializing losses within the slab's isolated risk boundary.

use percolator_common::*;

// ============================================================================
// CONSTANTS
// ============================================================================

/// Maximum number of insurance events to track in history
pub const INSURANCE_HISTORY_SIZE: usize = 100;

/// Default insurance contribution rate (basis points)
pub const DEFAULT_INSURANCE_RATE_BPS: u64 = 25; // 0.25%

/// Maximum insurance contribution rate (basis points)
pub const MAX_INSURANCE_RATE_BPS: u64 = 100; // 1.0%

/// Auto-deleverage trigger threshold (insurance < X% of open interest)
pub const ADL_TRIGGER_THRESHOLD_BPS: u64 = 50; // 0.5%

// ============================================================================
// TYPES
// ============================================================================

/// Insurance event type
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsuranceEventType {
    /// Contribution from liquidation fee
    LiquidationContribution = 0,
    /// Payout for liquidation shortfall
    ShortfallPayout = 1,
    /// ADL (auto-deleveraging) event
    AutoDeleverage = 2,
    /// Manual contribution by LP
    LpContribution = 3,
    /// Manual withdrawal by LP (with timelock)
    LpWithdrawal = 4,
    /// Socialized loss distribution
    SocializedLoss = 5,
}

impl Default for InsuranceEventType {
    fn default() -> Self {
        Self::LiquidationContribution
    }
}

/// Single insurance event record
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct InsuranceEvent {
    /// Event type
    pub event_type: InsuranceEventType,
    /// Padding for alignment
    pub _type_padding: [u8; 7],
    /// Timestamp of event
    pub timestamp: u64,
    /// Amount involved (positive for contribution, negative for payout)
    pub amount: i128,
    /// Balance after event
    pub balance_after: u128,
    /// Related account (liquidated user, LP, etc.)
    pub related_account: u32,
    /// Related instrument
    pub related_instrument: u16,
    /// Padding
    pub _padding: [u8; 2],
}

impl InsuranceEvent {
    pub const LEN: usize = core::mem::size_of::<Self>();
}

/// Insurance pool statistics
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct InsuranceStats {
    /// Total contributions received
    pub total_contributions: u128,
    /// Total payouts made
    pub total_payouts: u128,
    /// Total ADL events
    pub adl_events: u64,
    /// Total shortfall events
    pub shortfall_events: u64,
    /// Largest single payout
    pub max_single_payout: u128,
    /// Last contribution timestamp
    pub last_contribution_ts: u64,
    /// Last payout timestamp
    pub last_payout_ts: u64,
}

impl InsuranceStats {
    pub const LEN: usize = core::mem::size_of::<Self>();
}

/// Slab insurance pool state
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct InsurancePool {
    /// Current balance
    pub balance: u128,
    /// Target balance (based on open interest)
    pub target_balance: u128,
    /// Contribution rate (basis points)
    pub contribution_rate_bps: u64,
    /// ADL trigger threshold (basis points of OI)
    pub adl_threshold_bps: u64,
    /// LP withdrawal timelock (seconds)
    pub withdrawal_timelock_secs: u64,
    /// Pending withdrawal amount
    pub pending_withdrawal: u128,
    /// Pending withdrawal unlock timestamp
    pub pending_withdrawal_unlock_ts: u64,
    /// LP owner for withdrawals
    pub lp_owner: [u8; 32], // Pubkey bytes
    /// Open interest for threshold calculation
    pub total_open_interest: u128,
    /// Statistics
    pub stats: InsuranceStats,
    /// Event history (ring buffer)
    pub event_write_idx: u32,
    /// Padding
    pub _padding: [u8; 4],
    /// Event history
    pub events: [InsuranceEvent; INSURANCE_HISTORY_SIZE],
}

impl InsurancePool {
    pub const LEN: usize = core::mem::size_of::<Self>();

    /// Initialize new insurance pool
    pub fn new(lp_owner: [u8; 32]) -> Self {
        Self {
            balance: 0,
            target_balance: 0,
            contribution_rate_bps: DEFAULT_INSURANCE_RATE_BPS,
            adl_threshold_bps: ADL_TRIGGER_THRESHOLD_BPS,
            withdrawal_timelock_secs: 7 * 24 * 60 * 60, // 7 days default
            pending_withdrawal: 0,
            pending_withdrawal_unlock_ts: 0,
            lp_owner,
            total_open_interest: 0,
            stats: InsuranceStats::default(),
            event_write_idx: 0,
            _padding: [0; 4],
            events: [InsuranceEvent::default(); INSURANCE_HISTORY_SIZE],
        }
    }

    /// Initialize in-place to avoid stack allocation
    pub fn init_in_place(&mut self, lp_owner: [u8; 32]) {
        self.balance = 0;
        self.target_balance = 0;
        self.contribution_rate_bps = DEFAULT_INSURANCE_RATE_BPS;
        self.adl_threshold_bps = ADL_TRIGGER_THRESHOLD_BPS;
        self.withdrawal_timelock_secs = 7 * 24 * 60 * 60;
        self.pending_withdrawal = 0;
        self.pending_withdrawal_unlock_ts = 0;
        self.lp_owner = lp_owner;
        self.total_open_interest = 0;
        self.stats = InsuranceStats::default();
        self.event_write_idx = 0;
        self._padding = [0; 4];
        for i in 0..INSURANCE_HISTORY_SIZE {
            self.events[i] = InsuranceEvent::default();
        }
    }

    /// Add contribution to insurance pool
    pub fn contribute(
        &mut self,
        amount: u128,
        event_type: InsuranceEventType,
        related_account: u32,
        related_instrument: u16,
        timestamp: u64,
    ) {
        self.balance = self.balance.saturating_add(amount);
        self.stats.total_contributions = self.stats.total_contributions.saturating_add(amount);
        self.stats.last_contribution_ts = timestamp;

        // Record event
        self.record_event(InsuranceEvent {
            event_type,
            _type_padding: [0; 7],
            timestamp,
            amount: amount as i128,
            balance_after: self.balance,
            related_account,
            related_instrument,
            _padding: [0; 2],
        });
    }

    /// Pay out from insurance pool for shortfall
    pub fn payout(
        &mut self,
        amount: u128,
        event_type: InsuranceEventType,
        related_account: u32,
        related_instrument: u16,
        timestamp: u64,
    ) -> Result<u128, PercolatorError> {
        let actual_payout = amount.min(self.balance);
        
        if actual_payout == 0 {
            return Err(PercolatorError::InsufficientFunds);
        }

        self.balance = self.balance.saturating_sub(actual_payout);
        self.stats.total_payouts = self.stats.total_payouts.saturating_add(actual_payout);
        self.stats.last_payout_ts = timestamp;
        
        if actual_payout > self.stats.max_single_payout {
            self.stats.max_single_payout = actual_payout;
        }

        if event_type == InsuranceEventType::ShortfallPayout {
            self.stats.shortfall_events += 1;
        } else if event_type == InsuranceEventType::AutoDeleverage {
            self.stats.adl_events += 1;
        }

        // Record event
        self.record_event(InsuranceEvent {
            event_type,
            _type_padding: [0; 7],
            timestamp,
            amount: -(actual_payout as i128),
            balance_after: self.balance,
            related_account,
            related_instrument,
            _padding: [0; 2],
        });

        Ok(actual_payout)
    }

    /// Calculate contribution from liquidation
    pub fn calculate_liquidation_contribution(&self, liquidation_notional: u128) -> u128 {
        (liquidation_notional * self.contribution_rate_bps as u128) / 10_000
    }

    /// Check if ADL should be triggered
    pub fn should_trigger_adl(&self) -> bool {
        if self.total_open_interest == 0 {
            return false;
        }
        
        let threshold = (self.total_open_interest * self.adl_threshold_bps as u128) / 10_000;
        self.balance < threshold
    }

    /// Update open interest for ADL threshold calculation
    pub fn update_open_interest(&mut self, new_oi: u128) {
        self.total_open_interest = new_oi;
        // Update target balance (1% of OI as target)
        self.target_balance = new_oi / 100;
    }

    /// Get funding ratio (balance / target)
    pub fn funding_ratio_bps(&self) -> u64 {
        if self.target_balance == 0 {
            return 10_000; // 100% if no target
        }
        ((self.balance * 10_000) / self.target_balance) as u64
    }

    /// Initiate LP withdrawal (subject to timelock)
    pub fn initiate_withdrawal(
        &mut self,
        amount: u128,
        current_ts: u64,
    ) -> Result<(), PercolatorError> {
        if amount > self.balance {
            return Err(PercolatorError::InsufficientFunds);
        }

        // Cannot initiate if ADL threshold breached
        if self.should_trigger_adl() {
            return Err(PercolatorError::InsuranceBelowThreshold);
        }

        self.pending_withdrawal = amount;
        self.pending_withdrawal_unlock_ts = current_ts + self.withdrawal_timelock_secs;
        
        Ok(())
    }

    /// Complete LP withdrawal after timelock
    pub fn complete_withdrawal(&mut self, current_ts: u64) -> Result<u128, PercolatorError> {
        if self.pending_withdrawal == 0 {
            return Err(PercolatorError::NoPendingWithdrawal);
        }

        if current_ts < self.pending_withdrawal_unlock_ts {
            return Err(PercolatorError::WithdrawalLocked);
        }

        // Re-check ADL threshold at withdrawal time
        if self.balance.saturating_sub(self.pending_withdrawal) < 
           (self.total_open_interest * self.adl_threshold_bps as u128) / 10_000 {
            return Err(PercolatorError::InsuranceBelowThreshold);
        }

        let amount = self.pending_withdrawal;
        self.balance = self.balance.saturating_sub(amount);
        self.pending_withdrawal = 0;
        self.pending_withdrawal_unlock_ts = 0;

        Ok(amount)
    }

    /// Cancel pending withdrawal
    pub fn cancel_withdrawal(&mut self) {
        self.pending_withdrawal = 0;
        self.pending_withdrawal_unlock_ts = 0;
    }

    /// Record event in ring buffer
    fn record_event(&mut self, event: InsuranceEvent) {
        let idx = self.event_write_idx as usize % INSURANCE_HISTORY_SIZE;
        self.events[idx] = event;
        self.event_write_idx = self.event_write_idx.wrapping_add(1);
    }

    /// Get recent events (most recent first)
    pub fn recent_events(&self, count: usize) -> impl Iterator<Item = &InsuranceEvent> {
        let count = count.min(INSURANCE_HISTORY_SIZE);
        let start_idx = self.event_write_idx as usize;
        
        (0..count).map(move |i| {
            let idx = (start_idx.wrapping_sub(i + 1)) % INSURANCE_HISTORY_SIZE;
            &self.events[idx]
        })
    }

    /// Update contribution rate (LP configurable)
    pub fn set_contribution_rate(&mut self, rate_bps: u64) -> Result<(), PercolatorError> {
        if rate_bps > MAX_INSURANCE_RATE_BPS {
            return Err(PercolatorError::InvalidRiskParams);
        }
        self.contribution_rate_bps = rate_bps;
        Ok(())
    }

    /// Update ADL threshold (LP configurable with minimum)
    pub fn set_adl_threshold(&mut self, threshold_bps: u64) -> Result<(), PercolatorError> {
        // Minimum 0.1% threshold
        if threshold_bps < 10 {
            return Err(PercolatorError::InvalidRiskParams);
        }
        self.adl_threshold_bps = threshold_bps;
        Ok(())
    }
}

// ============================================================================
// ADL (AUTO-DELEVERAGE) LOGIC
// ============================================================================

/// ADL priority for position selection
#[derive(Debug, Clone, Copy)]
pub struct AdlPriority {
    /// Account index
    pub account_idx: u32,
    /// Position index
    pub position_idx: u32,
    /// Instrument index
    pub instrument_idx: u16,
    /// Position quantity (signed)
    pub qty: i64,
    /// Priority score (higher = selected first for ADL)
    pub priority_score: u64,
    /// Unrealized PnL (positive = profitable)
    pub unrealized_pnl: i128,
}

/// Calculate ADL priority score
/// Higher score = selected first for ADL
/// Priority based on: profitability + leverage
pub fn calculate_adl_priority(
    unrealized_pnl: i128,
    position_value: u128,
    margin_used: u128,
) -> u64 {
    if position_value == 0 || margin_used == 0 {
        return 0;
    }

    // Profitability component (0-5000 based on ROI%)
    let roi_bps = if unrealized_pnl >= 0 {
        ((unrealized_pnl as u128 * 10_000) / margin_used).min(5000) as u64
    } else {
        0
    };

    // Leverage component (0-5000 based on leverage ratio)
    let leverage = (position_value * 100 / margin_used) as u64;
    let leverage_score = leverage.min(5000);

    // Combined score
    roi_bps + leverage_score
}

/// Maximum positions that can be selected for ADL
pub const MAX_ADL_POSITIONS: usize = 16;

/// ADL selection result
#[derive(Debug, Clone, Copy, Default)]
pub struct AdlSelection {
    /// Account index
    pub account_idx: u32,
    /// Position index
    pub position_idx: u32,
    /// Quantity to ADL
    pub qty: u64,
}

/// ADL selection result
#[derive(Debug, Clone, Copy)]
pub struct AdlSelectionResult {
    /// Selected positions
    pub selections: [AdlSelection; MAX_ADL_POSITIONS],
    /// Number of valid selections
    pub count: usize,
    /// Remaining quantity not filled
    pub remaining_qty: u64,
}

impl Default for AdlSelectionResult {
    fn default() -> Self {
        Self {
            selections: [AdlSelection::default(); MAX_ADL_POSITIONS],
            count: 0,
            remaining_qty: 0,
        }
    }
}

/// Select positions for ADL based on priority
/// Returns a fixed-size result without heap allocation
pub fn select_adl_positions(
    priorities: &mut [AdlPriority],
    target_qty: u64,
    side: Side,
) -> AdlSelectionResult {
    // Sort by priority score descending (bubble sort for no_std)
    for i in 0..priorities.len() {
        for j in 0..(priorities.len() - i - 1) {
            if priorities[j].priority_score < priorities[j + 1].priority_score {
                priorities.swap(j, j + 1);
            }
        }
    }

    let mut result = AdlSelectionResult::default();
    let mut remaining = target_qty as i64;

    for p in priorities.iter() {
        if remaining <= 0 || result.count >= MAX_ADL_POSITIONS {
            break;
        }

        // Select positions on the opposite side
        let is_opposite = match side {
            Side::Buy => p.qty < 0,  // Select shorts for buy ADL
            Side::Sell => p.qty > 0, // Select longs for sell ADL
        };

        if is_opposite {
            let available = p.qty.abs();
            let take = (available as i64).min(remaining) as u64;
            result.selections[result.count] = AdlSelection {
                account_idx: p.account_idx,
                position_idx: p.position_idx,
                qty: take,
            };
            result.count += 1;
            remaining -= take as i64;
        }
    }

    result.remaining_qty = remaining.max(0) as u64;
    result
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insurance_pool_creation() {
        let pool = InsurancePool::new([0u8; 32]);
        assert_eq!(pool.balance, 0);
        assert_eq!(pool.contribution_rate_bps, DEFAULT_INSURANCE_RATE_BPS);
        assert_eq!(pool.adl_threshold_bps, ADL_TRIGGER_THRESHOLD_BPS);
    }

    #[test]
    fn test_contribution() {
        let mut pool = InsurancePool::new([0u8; 32]);
        
        pool.contribute(
            1_000_000_000, // 1000 USDC
            InsuranceEventType::LiquidationContribution,
            0,
            0,
            1000,
        );

        assert_eq!(pool.balance, 1_000_000_000);
        assert_eq!(pool.stats.total_contributions, 1_000_000_000);
    }

    #[test]
    fn test_payout() {
        let mut pool = InsurancePool::new([0u8; 32]);
        pool.balance = 10_000_000_000; // 10k USDC
        
        let payout = pool.payout(
            5_000_000_000, // 5k USDC
            InsuranceEventType::ShortfallPayout,
            0,
            0,
            1000,
        ).unwrap();

        assert_eq!(payout, 5_000_000_000);
        assert_eq!(pool.balance, 5_000_000_000);
        assert_eq!(pool.stats.shortfall_events, 1);
    }

    #[test]
    fn test_payout_insufficient() {
        let mut pool = InsurancePool::new([0u8; 32]);
        pool.balance = 100; // Very small balance
        
        let payout = pool.payout(
            1_000_000,
            InsuranceEventType::ShortfallPayout,
            0,
            0,
            1000,
        ).unwrap();

        // Should pay out available balance
        assert_eq!(payout, 100);
        assert_eq!(pool.balance, 0);
    }

    #[test]
    fn test_adl_trigger() {
        let mut pool = InsurancePool::new([0u8; 32]);
        pool.balance = 100_000_000; // 100 USDC
        pool.update_open_interest(100_000_000_000_000); // 100M OI
        
        // Balance is way below 0.5% threshold
        assert!(pool.should_trigger_adl());

        // Add more balance
        pool.balance = 1_000_000_000_000; // 1M USDC
        assert!(!pool.should_trigger_adl());
    }

    #[test]
    fn test_contribution_calculation() {
        let pool = InsurancePool::new([0u8; 32]);
        
        let contribution = pool.calculate_liquidation_contribution(1_000_000_000_000); // 1M notional
        
        // 0.25% of 1M = 2500
        assert_eq!(contribution, 2_500_000_000);
    }

    #[test]
    fn test_withdrawal_timelock() {
        let mut pool = InsurancePool::new([0u8; 32]);
        pool.balance = 10_000_000_000;
        pool.update_open_interest(100_000_000_000); // Small OI to avoid ADL trigger
        
        // Initiate withdrawal
        pool.initiate_withdrawal(1_000_000_000, 1000).unwrap();
        
        // Cannot complete before timelock
        assert!(pool.complete_withdrawal(1000 + 86400).is_err()); // 1 day later
        
        // Can complete after 7 days
        let amount = pool.complete_withdrawal(1000 + 7 * 86400 + 1).unwrap();
        assert_eq!(amount, 1_000_000_000);
    }

    #[test]
    fn test_adl_priority() {
        // Profitable position with high leverage
        let score1 = calculate_adl_priority(
            1_000_000_000, // 1k profit
            10_000_000_000, // 10k position
            1_000_000_000, // 1k margin (10x leverage)
        );

        // Less profitable position with low leverage
        let score2 = calculate_adl_priority(
            100_000_000, // 100 profit
            10_000_000_000, // 10k position
            5_000_000_000, // 5k margin (2x leverage)
        );

        // Higher profit + higher leverage = higher priority
        assert!(score1 > score2);
    }

    #[test]
    fn test_funding_ratio() {
        let mut pool = InsurancePool::new([0u8; 32]);
        pool.balance = 500_000_000; // 500 USDC
        pool.target_balance = 1_000_000_000; // 1000 USDC target
        
        assert_eq!(pool.funding_ratio_bps(), 5000); // 50%
    }

    #[test]
    fn test_event_size() {
        assert_eq!(InsuranceEvent::LEN, 48);
    }

    #[test]
    fn test_stats_size() {
        assert_eq!(InsuranceStats::LEN, 72);
    }
}
