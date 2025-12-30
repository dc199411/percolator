//! Instruction builders

use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
    sysvar,
};
use crate::constants::*;
use crate::pda::*;
use crate::types::*;

// ============================================================================
// ROUTER INSTRUCTIONS
// ============================================================================

/// Create initialize router instruction
pub fn create_initialize_router_instruction(admin: &Pubkey, usdc_mint: &Pubkey) -> Instruction {
    let (registry_pda, _) = derive_registry_pda();
    let (vault_pda, _) = derive_vault_pda();
    let (vault_authority, _) = derive_vault_authority_pda();

    let data = vec![RouterInstruction::Initialize as u8];

    let accounts = vec![
        AccountMeta::new(registry_pda, false),
        AccountMeta::new(vault_pda, false),
        AccountMeta::new_readonly(vault_authority, false),
        AccountMeta::new(*admin, true),
        AccountMeta::new_readonly(*usdc_mint, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(sysvar::rent::ID, false),
    ];

    Instruction::new_with_bytes(ROUTER_PROGRAM_ID, &data, accounts)
}

/// Create initialize portfolio instruction
pub fn create_initialize_portfolio_instruction(owner: &Pubkey) -> Instruction {
    let (registry_pda, _) = derive_registry_pda();
    let (portfolio_pda, _) = derive_portfolio_pda(owner);

    let data = vec![RouterInstruction::InitializePortfolio as u8];

    let accounts = vec![
        AccountMeta::new(registry_pda, false),
        AccountMeta::new(portfolio_pda, false),
        AccountMeta::new(*owner, true),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    Instruction::new_with_bytes(ROUTER_PROGRAM_ID, &data, accounts)
}

/// Create deposit instruction
pub fn create_deposit_instruction(
    owner: &Pubkey,
    user_token_account: &Pubkey,
    params: &DepositParams,
) -> Instruction {
    let (registry_pda, _) = derive_registry_pda();
    let (vault_pda, _) = derive_vault_pda();
    let (portfolio_pda, _) = derive_portfolio_pda(owner);

    let mut data = vec![RouterInstruction::Deposit as u8];
    data.extend_from_slice(&params.amount.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(registry_pda, false),
        AccountMeta::new(portfolio_pda, false),
        AccountMeta::new(vault_pda, false),
        AccountMeta::new(*user_token_account, false),
        AccountMeta::new_readonly(*owner, true),
        AccountMeta::new_readonly(spl_token::ID, false),
    ];

    Instruction::new_with_bytes(ROUTER_PROGRAM_ID, &data, accounts)
}

/// Create withdraw instruction
pub fn create_withdraw_instruction(
    owner: &Pubkey,
    user_token_account: &Pubkey,
    params: &WithdrawParams,
) -> Instruction {
    let (registry_pda, _) = derive_registry_pda();
    let (vault_pda, _) = derive_vault_pda();
    let (vault_authority, _) = derive_vault_authority_pda();
    let (portfolio_pda, _) = derive_portfolio_pda(owner);

    let mut data = vec![RouterInstruction::Withdraw as u8];
    data.extend_from_slice(&params.amount.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(registry_pda, false),
        AccountMeta::new(portfolio_pda, false),
        AccountMeta::new(vault_pda, false),
        AccountMeta::new(*user_token_account, false),
        AccountMeta::new_readonly(vault_authority, false),
        AccountMeta::new_readonly(*owner, true),
        AccountMeta::new_readonly(spl_token::ID, false),
    ];

    Instruction::new_with_bytes(ROUTER_PROGRAM_ID, &data, accounts)
}

/// Create multi-slab reserve instruction
pub fn create_multi_slab_reserve_instruction(
    owner: &Pubkey,
    slab_accounts: &[Pubkey],
    params: &MultiSlabReserveParams,
) -> Instruction {
    let (registry_pda, _) = derive_registry_pda();
    let (portfolio_pda, _) = derive_portfolio_pda(owner);

    let mut data = vec![RouterInstruction::MultiSlabReserve as u8];
    data.push(params.splits.len() as u8);

    for split in &params.splits {
        data.push(split.slab_index);
        data.push(split.instrument_index);
        data.extend_from_slice(&split.qty.to_le_bytes());
        data.extend_from_slice(&split.limit_price.to_le_bytes());
    }

    data.extend_from_slice(&params.total_qty.to_le_bytes());
    data.extend_from_slice(&params.request_id.to_le_bytes());
    data.extend_from_slice(&params.expiry_ts.to_le_bytes());

    let mut accounts = vec![
        AccountMeta::new(registry_pda, false),
        AccountMeta::new(portfolio_pda, false),
        AccountMeta::new_readonly(*owner, true),
    ];

    for slab in slab_accounts {
        accounts.push(AccountMeta::new(*slab, false));
    }

    Instruction::new_with_bytes(ROUTER_PROGRAM_ID, &data, accounts)
}

/// Create multi-slab commit instruction
pub fn create_multi_slab_commit_instruction(
    owner: &Pubkey,
    slab_accounts: &[Pubkey],
    request_id: u64,
    hold_ids: &[u64],
) -> Instruction {
    let (registry_pda, _) = derive_registry_pda();
    let (portfolio_pda, _) = derive_portfolio_pda(owner);

    let mut data = vec![RouterInstruction::MultiSlabCommit as u8];
    data.extend_from_slice(&request_id.to_le_bytes());
    data.push(hold_ids.len() as u8);

    for hold_id in hold_ids {
        data.extend_from_slice(&hold_id.to_le_bytes());
    }

    let mut accounts = vec![
        AccountMeta::new(registry_pda, false),
        AccountMeta::new(portfolio_pda, false),
        AccountMeta::new_readonly(*owner, true),
    ];

    for slab in slab_accounts {
        accounts.push(AccountMeta::new(*slab, false));
    }

    Instruction::new_with_bytes(ROUTER_PROGRAM_ID, &data, accounts)
}

/// Create global liquidation instruction
pub fn create_global_liquidation_instruction(
    liquidator: &Pubkey,
    target_portfolio: &Pubkey,
    slab_accounts: &[Pubkey],
) -> Instruction {
    let (registry_pda, _) = derive_registry_pda();

    let data = vec![RouterInstruction::GlobalLiquidation as u8];

    let mut accounts = vec![
        AccountMeta::new(registry_pda, false),
        AccountMeta::new(*target_portfolio, false),
        AccountMeta::new_readonly(*liquidator, true),
    ];

    for slab in slab_accounts {
        accounts.push(AccountMeta::new(*slab, false));
    }

    Instruction::new_with_bytes(ROUTER_PROGRAM_ID, &data, accounts)
}

// ============================================================================
// SLAB INSTRUCTIONS
// ============================================================================

/// Create reserve instruction
pub fn create_reserve_instruction(
    slab_state: &Pubkey,
    router: &Pubkey,
    params: &OrderParams,
    request_id: u64,
    expiry_ts: u64,
) -> Instruction {
    let mut data = vec![SlabInstruction::Reserve as u8];
    data.push(params.instrument_index);
    data.push(params.side as u8);
    data.extend_from_slice(&params.price.to_le_bytes());
    data.extend_from_slice(&params.qty.to_le_bytes());
    data.push(params.time_in_force as u8);
    data.extend_from_slice(&request_id.to_le_bytes());
    data.extend_from_slice(&expiry_ts.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(*slab_state, false),
        AccountMeta::new_readonly(*router, true),
    ];

    Instruction::new_with_bytes(SLAB_PROGRAM_ID, &data, accounts)
}

/// Create commit instruction
pub fn create_commit_instruction(
    slab_state: &Pubkey,
    router: &Pubkey,
    hold_id: u64,
) -> Instruction {
    let mut data = vec![SlabInstruction::Commit as u8];
    data.extend_from_slice(&hold_id.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(*slab_state, false),
        AccountMeta::new_readonly(*router, true),
    ];

    Instruction::new_with_bytes(SLAB_PROGRAM_ID, &data, accounts)
}

/// Create cancel instruction
pub fn create_cancel_instruction(
    slab_state: &Pubkey,
    router: &Pubkey,
    hold_id: u64,
) -> Instruction {
    let mut data = vec![SlabInstruction::Cancel as u8];
    data.extend_from_slice(&hold_id.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(*slab_state, false),
        AccountMeta::new_readonly(*router, true),
    ];

    Instruction::new_with_bytes(SLAB_PROGRAM_ID, &data, accounts)
}

// ============================================================================
// INSURANCE INSTRUCTIONS
// ============================================================================

/// Create initialize insurance instruction
pub fn create_initialize_insurance_instruction(
    slab_state: &Pubkey,
    lp_owner: &Pubkey,
    params: &InitializeInsuranceParams,
) -> Instruction {
    let (insurance_pda, _) = derive_insurance_pda(slab_state);

    let mut data = vec![SlabInstruction::InitializeInsurance as u8];
    data.extend_from_slice(&params.contribution_rate_bps.to_le_bytes());
    data.extend_from_slice(&params.adl_threshold_bps.to_le_bytes());
    data.extend_from_slice(&params.withdrawal_timelock_secs.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(*slab_state, false),
        AccountMeta::new(insurance_pda, false),
        AccountMeta::new(*lp_owner, true),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    Instruction::new_with_bytes(SLAB_PROGRAM_ID, &data, accounts)
}

/// Create contribute insurance instruction
pub fn create_contribute_insurance_instruction(
    slab_state: &Pubkey,
    lp_owner: &Pubkey,
    lp_token_account: &Pubkey,
    insurance_vault: &Pubkey,
    params: &ContributeInsuranceParams,
) -> Instruction {
    let (insurance_pda, _) = derive_insurance_pda(slab_state);

    let mut data = vec![SlabInstruction::ContributeInsurance as u8];
    data.extend_from_slice(&params.amount.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(insurance_pda, false),
        AccountMeta::new(*lp_token_account, false),
        AccountMeta::new(*insurance_vault, false),
        AccountMeta::new_readonly(*lp_owner, true),
        AccountMeta::new_readonly(spl_token::ID, false),
    ];

    Instruction::new_with_bytes(SLAB_PROGRAM_ID, &data, accounts)
}

/// Create initiate insurance withdrawal instruction
pub fn create_initiate_insurance_withdrawal_instruction(
    slab_state: &Pubkey,
    lp_owner: &Pubkey,
    params: &InitiateWithdrawalParams,
) -> Instruction {
    let (insurance_pda, _) = derive_insurance_pda(slab_state);

    let mut data = vec![SlabInstruction::InitiateInsuranceWithdrawal as u8];
    data.extend_from_slice(&params.amount.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(insurance_pda, false),
        AccountMeta::new_readonly(*lp_owner, true),
    ];

    Instruction::new_with_bytes(SLAB_PROGRAM_ID, &data, accounts)
}

/// Create complete insurance withdrawal instruction
pub fn create_complete_insurance_withdrawal_instruction(
    slab_state: &Pubkey,
    lp_owner: &Pubkey,
    lp_token_account: &Pubkey,
    insurance_vault: &Pubkey,
    vault_authority: &Pubkey,
) -> Instruction {
    let (insurance_pda, _) = derive_insurance_pda(slab_state);

    let data = vec![SlabInstruction::CompleteInsuranceWithdrawal as u8];

    let accounts = vec![
        AccountMeta::new(insurance_pda, false),
        AccountMeta::new(*insurance_vault, false),
        AccountMeta::new(*lp_token_account, false),
        AccountMeta::new_readonly(*lp_owner, true),
        AccountMeta::new_readonly(*vault_authority, false),
        AccountMeta::new_readonly(spl_token::ID, false),
    ];

    Instruction::new_with_bytes(SLAB_PROGRAM_ID, &data, accounts)
}

/// Create cancel insurance withdrawal instruction
pub fn create_cancel_insurance_withdrawal_instruction(
    slab_state: &Pubkey,
    lp_owner: &Pubkey,
) -> Instruction {
    let (insurance_pda, _) = derive_insurance_pda(slab_state);

    let data = vec![SlabInstruction::CancelInsuranceWithdrawal as u8];

    let accounts = vec![
        AccountMeta::new(insurance_pda, false),
        AccountMeta::new_readonly(*lp_owner, true),
    ];

    Instruction::new_with_bytes(SLAB_PROGRAM_ID, &data, accounts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deposit_instruction() {
        let owner = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();
        let params = DepositParams { amount: 1_000_000 };

        let ix = create_deposit_instruction(&owner, &token_account, &params);

        assert_eq!(ix.program_id, ROUTER_PROGRAM_ID);
        assert!(!ix.data.is_empty());
        assert_eq!(ix.data[0], RouterInstruction::Deposit as u8);
    }

    #[test]
    fn test_reserve_instruction() {
        let slab = Pubkey::new_unique();
        let router = Pubkey::new_unique();
        let params = OrderParams {
            instrument_index: 0,
            side: Side::Buy,
            price: 50_000_000_000,
            qty: 1_000_000,
            time_in_force: TimeInForce::GTC,
            ..Default::default()
        };

        let ix = create_reserve_instruction(&slab, &router, &params, 1, 0);

        assert_eq!(ix.program_id, SLAB_PROGRAM_ID);
        assert_eq!(ix.data[0], SlabInstruction::Reserve as u8);
    }
}
