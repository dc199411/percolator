//! PDA derivation utilities

use solana_sdk::pubkey::Pubkey;
use crate::constants::*;

/// Derive router registry PDA
pub fn derive_registry_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[REGISTRY_SEED], &ROUTER_PROGRAM_ID)
}

/// Derive router vault PDA
pub fn derive_vault_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[VAULT_SEED], &ROUTER_PROGRAM_ID)
}

/// Derive user portfolio PDA
pub fn derive_portfolio_pda(owner: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[PORTFOLIO_SEED, owner.as_ref()], &ROUTER_PROGRAM_ID)
}

/// Derive slab state PDA
pub fn derive_slab_pda(lp_owner: &Pubkey, slab_index: u8) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[SLAB_SEED, lp_owner.as_ref(), &[slab_index]],
        &SLAB_PROGRAM_ID,
    )
}

/// Derive insurance pool PDA
pub fn derive_insurance_pda(slab_state: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[INSURANCE_SEED, slab_state.as_ref()], &SLAB_PROGRAM_ID)
}

/// Derive vault authority PDA
pub fn derive_vault_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"vault_authority"], &ROUTER_PROGRAM_ID)
}

/// Derive slab vault PDA
pub fn derive_slab_vault_pda(slab_state: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"slab_vault", slab_state.as_ref()], &SLAB_PROGRAM_ID)
}

/// Derive insurance vault PDA
pub fn derive_insurance_vault_pda(insurance_pool: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"insurance_vault", insurance_pool.as_ref()],
        &SLAB_PROGRAM_ID,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_pda() {
        let (pda, bump) = derive_registry_pda();
        assert_ne!(bump, 0);
        assert_ne!(pda, Pubkey::default());
    }

    #[test]
    fn test_portfolio_pda_deterministic() {
        let owner = Pubkey::new_unique();
        let (pda1, bump1) = derive_portfolio_pda(&owner);
        let (pda2, bump2) = derive_portfolio_pda(&owner);
        assert_eq!(pda1, pda2);
        assert_eq!(bump1, bump2);
    }

    #[test]
    fn test_different_owners_different_pdas() {
        let owner1 = Pubkey::new_unique();
        let owner2 = Pubkey::new_unique();
        let (pda1, _) = derive_portfolio_pda(&owner1);
        let (pda2, _) = derive_portfolio_pda(&owner2);
        assert_ne!(pda1, pda2);
    }
}
