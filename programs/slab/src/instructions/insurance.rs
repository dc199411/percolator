//! Insurance Pool Instructions
//!
//! Instructions for managing the slab-level insurance pool.

use pinocchio::{
    account_info::AccountInfo,
    msg,
    program_error::ProgramError,
};
use percolator_common::*;
use crate::state::insurance::*;

// ============================================================================
// INSTRUCTION DATA
// ============================================================================

/// Initialize insurance pool parameters
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct InitializeInsuranceParams {
    /// Contribution rate in basis points
    pub contribution_rate_bps: u64,
    /// ADL trigger threshold in basis points
    pub adl_threshold_bps: u64,
    /// Withdrawal timelock in seconds
    pub withdrawal_timelock_secs: u64,
}

/// LP contribution parameters
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ContributeInsuranceParams {
    /// Amount to contribute
    pub amount: u64,
}

/// LP withdrawal initiation parameters
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct InitiateWithdrawalParams {
    /// Amount to withdraw
    pub amount: u64,
}

/// Update insurance config parameters
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UpdateInsuranceConfigParams {
    /// New contribution rate (optional, 0 = no change)
    pub contribution_rate_bps: u64,
    /// New ADL threshold (optional, 0 = no change)
    pub adl_threshold_bps: u64,
}

// ============================================================================
// PROCESS FUNCTIONS
// ============================================================================

/// Initialize insurance pool for a slab
pub fn process_initialize_insurance(
    accounts: &[AccountInfo],
    params: &InitializeInsuranceParams,
) -> Result<(), ProgramError> {
    // Account indices:
    // 0: Slab state (mut)
    // 1: Insurance pool state (mut, PDA)
    // 2: LP owner (signer)
    // 3: System program
    
    if accounts.len() < 4 {
        return Err(PercolatorError::InvalidAccount.into());
    }

    let _slab_info = &accounts[0];
    let insurance_info = &accounts[1];
    let lp_owner = &accounts[2];

    // Verify LP owner is signer
    if !lp_owner.is_signer() {
        return Err(PercolatorError::Unauthorized.into());
    }

    // Validate contribution rate
    if params.contribution_rate_bps > MAX_INSURANCE_RATE_BPS {
        return Err(PercolatorError::InvalidRiskParams.into());
    }

    // Validate ADL threshold (minimum 0.1%)
    if params.adl_threshold_bps < 10 {
        return Err(PercolatorError::InvalidRiskParams.into());
    }

    msg!("Initializing insurance pool");

    // Get mutable reference to insurance data
    let insurance_data = unsafe {
        &mut *(insurance_info.borrow_mut_data_unchecked().as_mut_ptr() as *mut InsurancePool)
    };

    // Initialize in place to avoid stack allocation
    let mut owner_bytes = [0u8; 32];
    owner_bytes.copy_from_slice(lp_owner.key().as_ref());
    insurance_data.init_in_place(owner_bytes);
    
    // Set custom parameters
    insurance_data.contribution_rate_bps = params.contribution_rate_bps;
    insurance_data.adl_threshold_bps = params.adl_threshold_bps;
    insurance_data.withdrawal_timelock_secs = params.withdrawal_timelock_secs;

    msg!("Insurance pool initialized");
    Ok(())
}

/// LP contributes to insurance pool
pub fn process_contribute_insurance(
    accounts: &[AccountInfo],
    params: &ContributeInsuranceParams,
    timestamp: u64,
) -> Result<(), ProgramError> {
    // Account indices:
    // 0: Insurance pool state (mut)
    // 1: LP token account (mut)
    // 2: Insurance vault (mut)
    // 3: LP owner (signer)
    // 4: Token program
    
    if accounts.len() < 5 {
        return Err(PercolatorError::InvalidAccount.into());
    }

    let insurance_info = &accounts[0];
    let _lp_token = &accounts[1];
    let _insurance_vault = &accounts[2];
    let lp_owner = &accounts[3];

    // Verify LP owner is signer
    if !lp_owner.is_signer() {
        return Err(PercolatorError::Unauthorized.into());
    }

    msg!("Processing LP contribution to insurance");

    let insurance_data = unsafe {
        &mut *(insurance_info.borrow_mut_data_unchecked().as_mut_ptr() as *mut InsurancePool)
    };

    // Verify LP owner matches
    if insurance_data.lp_owner != lp_owner.key().as_ref()[..32] {
        return Err(PercolatorError::Unauthorized.into());
    }

    // Add contribution
    insurance_data.contribute(
        params.amount as u128,
        InsuranceEventType::LpContribution,
        0, // No related account for LP contribution
        0,
        timestamp,
    );

    // Token transfer would happen here via CPI
    // transfer_tokens(lp_token, insurance_vault, params.amount)?;

    msg!("LP contribution processed");
    Ok(())
}

/// Record insurance contribution from liquidation
pub fn process_liquidation_contribution(
    insurance_pool: &mut InsurancePool,
    liquidation_notional: u128,
    related_account: u32,
    related_instrument: u16,
    timestamp: u64,
) -> u128 {
    let contribution = insurance_pool.calculate_liquidation_contribution(liquidation_notional);
    
    if contribution > 0 {
        insurance_pool.contribute(
            contribution,
            InsuranceEventType::LiquidationContribution,
            related_account,
            related_instrument,
            timestamp,
        );
    }
    
    contribution
}

/// Initiate LP withdrawal (subject to timelock)
pub fn process_initiate_withdrawal(
    accounts: &[AccountInfo],
    params: &InitiateWithdrawalParams,
    current_ts: u64,
) -> Result<(), ProgramError> {
    // Account indices:
    // 0: Insurance pool state (mut)
    // 1: LP owner (signer)
    
    if accounts.len() < 2 {
        return Err(PercolatorError::InvalidAccount.into());
    }

    let insurance_info = &accounts[0];
    let lp_owner = &accounts[1];

    if !lp_owner.is_signer() {
        return Err(PercolatorError::Unauthorized.into());
    }

    msg!("Initiating insurance withdrawal");

    let insurance_data = unsafe {
        &mut *(insurance_info.borrow_mut_data_unchecked().as_mut_ptr() as *mut InsurancePool)
    };

    // Verify LP owner matches
    if insurance_data.lp_owner != lp_owner.key().as_ref()[..32] {
        return Err(PercolatorError::Unauthorized.into());
    }

    insurance_data.initiate_withdrawal(params.amount as u128, current_ts)?;

    msg!("Withdrawal initiated with timelock");
    Ok(())
}

/// Complete LP withdrawal after timelock
pub fn process_complete_withdrawal(
    accounts: &[AccountInfo],
    current_ts: u64,
) -> Result<u64, ProgramError> {
    // Account indices:
    // 0: Insurance pool state (mut)
    // 1: Insurance vault (mut)
    // 2: LP token account (mut)
    // 3: LP owner (signer)
    // 4: Vault authority PDA
    // 5: Token program
    
    if accounts.len() < 6 {
        return Err(PercolatorError::InvalidAccount.into());
    }

    let insurance_info = &accounts[0];
    let lp_owner = &accounts[3];

    if !lp_owner.is_signer() {
        return Err(PercolatorError::Unauthorized.into());
    }

    msg!("Completing insurance withdrawal");

    let insurance_data = unsafe {
        &mut *(insurance_info.borrow_mut_data_unchecked().as_mut_ptr() as *mut InsurancePool)
    };

    // Verify LP owner matches
    if insurance_data.lp_owner != lp_owner.key().as_ref()[..32] {
        return Err(PercolatorError::Unauthorized.into());
    }

    let amount = insurance_data.complete_withdrawal(current_ts)?;

    // Token transfer would happen here via CPI
    // transfer_tokens_with_authority(insurance_vault, lp_token, amount, authority_bump)?;

    msg!("Withdrawal completed");
    Ok(amount as u64)
}

/// Cancel pending withdrawal
pub fn process_cancel_withdrawal(
    accounts: &[AccountInfo],
) -> Result<(), ProgramError> {
    // Account indices:
    // 0: Insurance pool state (mut)
    // 1: LP owner (signer)
    
    if accounts.len() < 2 {
        return Err(PercolatorError::InvalidAccount.into());
    }

    let insurance_info = &accounts[0];
    let lp_owner = &accounts[1];

    if !lp_owner.is_signer() {
        return Err(PercolatorError::Unauthorized.into());
    }

    let insurance_data = unsafe {
        &mut *(insurance_info.borrow_mut_data_unchecked().as_mut_ptr() as *mut InsurancePool)
    };

    // Verify LP owner matches
    if insurance_data.lp_owner != lp_owner.key().as_ref()[..32] {
        return Err(PercolatorError::Unauthorized.into());
    }

    insurance_data.cancel_withdrawal();

    msg!("Withdrawal cancelled");
    Ok(())
}

/// Pay out insurance for liquidation shortfall
pub fn process_insurance_payout(
    insurance_pool: &mut InsurancePool,
    shortfall_amount: u128,
    related_account: u32,
    related_instrument: u16,
    timestamp: u64,
) -> Result<(u128, bool), ProgramError> {
    // Attempt payout from insurance
    let payout = insurance_pool.payout(
        shortfall_amount,
        InsuranceEventType::ShortfallPayout,
        related_account,
        related_instrument,
        timestamp,
    )?;

    // Check if ADL is required for remaining shortfall
    let remaining_shortfall = shortfall_amount.saturating_sub(payout);
    let adl_required = remaining_shortfall > 0 || insurance_pool.should_trigger_adl();

    Ok((payout, adl_required))
}

/// Update insurance configuration
pub fn process_update_insurance_config(
    accounts: &[AccountInfo],
    params: &UpdateInsuranceConfigParams,
) -> Result<(), ProgramError> {
    // Account indices:
    // 0: Insurance pool state (mut)
    // 1: LP owner (signer)
    
    if accounts.len() < 2 {
        return Err(PercolatorError::InvalidAccount.into());
    }

    let insurance_info = &accounts[0];
    let lp_owner = &accounts[1];

    if !lp_owner.is_signer() {
        return Err(PercolatorError::Unauthorized.into());
    }

    let insurance_data = unsafe {
        &mut *(insurance_info.borrow_mut_data_unchecked().as_mut_ptr() as *mut InsurancePool)
    };

    // Verify LP owner matches
    if insurance_data.lp_owner != lp_owner.key().as_ref()[..32] {
        return Err(PercolatorError::Unauthorized.into());
    }

    // Update contribution rate if provided
    if params.contribution_rate_bps > 0 {
        insurance_data.set_contribution_rate(params.contribution_rate_bps)?;
    }

    // Update ADL threshold if provided
    if params.adl_threshold_bps > 0 {
        insurance_data.set_adl_threshold(params.adl_threshold_bps)?;
    }

    msg!("Insurance config updated");
    Ok(())
}

/// Update insurance pool open interest
pub fn process_update_insurance_oi(
    insurance_pool: &mut InsurancePool,
    new_open_interest: u128,
) {
    insurance_pool.update_open_interest(new_open_interest);
}

// ============================================================================
// INSTRUCTION DISCRIMINATORS
// ============================================================================

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsuranceInstruction {
    /// Initialize insurance pool
    Initialize = 0,
    /// LP contribute to insurance
    Contribute = 1,
    /// Initiate withdrawal
    InitiateWithdrawal = 2,
    /// Complete withdrawal
    CompleteWithdrawal = 3,
    /// Cancel withdrawal
    CancelWithdrawal = 4,
    /// Update config
    UpdateConfig = 5,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_liquidation_contribution() {
        let mut pool = InsurancePool::new([0u8; 32]);
        
        let contribution = process_liquidation_contribution(
            &mut pool,
            1_000_000_000_000, // 1M notional
            0,
            0,
            1000,
        );

        // 0.25% of 1M = 2500
        assert_eq!(contribution, 2_500_000_000);
        assert_eq!(pool.balance, 2_500_000_000);
    }

    #[test]
    fn test_insurance_payout() {
        let mut pool = InsurancePool::new([0u8; 32]);
        pool.balance = 10_000_000_000;
        pool.update_open_interest(100_000_000_000);

        let (payout, adl_required) = process_insurance_payout(
            &mut pool,
            5_000_000_000,
            0,
            0,
            1000,
        ).unwrap();

        assert_eq!(payout, 5_000_000_000);
        assert!(!adl_required); // Still above threshold
    }

    #[test]
    fn test_adl_triggered() {
        let mut pool = InsurancePool::new([0u8; 32]);
        pool.balance = 100_000; // Very low balance
        pool.update_open_interest(100_000_000_000_000); // 100M OI

        let (payout, adl_required) = process_insurance_payout(
            &mut pool,
            1_000_000,
            0,
            0,
            1000,
        ).unwrap();

        assert_eq!(payout, 100_000); // Only pays what's available
        assert!(adl_required); // ADL required
    }

    #[test]
    fn test_update_insurance_oi() {
        let mut pool = InsurancePool::new([0u8; 32]);
        
        process_update_insurance_oi(&mut pool, 1_000_000_000_000);
        
        assert_eq!(pool.total_open_interest, 1_000_000_000_000);
        assert_eq!(pool.target_balance, 10_000_000_000); // 1% of OI
    }
}
