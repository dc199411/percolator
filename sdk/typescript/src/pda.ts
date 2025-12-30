/**
 * PDA Derivation Utilities
 */
import { PublicKey } from '@solana/web3.js';
import { ROUTER_PROGRAM_ID, SLAB_PROGRAM_ID, SEEDS } from './constants';

/**
 * Derive router registry PDA
 */
export function deriveRegistryPda(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEEDS.REGISTRY],
    ROUTER_PROGRAM_ID
  );
}

/**
 * Derive router vault PDA
 */
export function deriveVaultPda(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEEDS.VAULT],
    ROUTER_PROGRAM_ID
  );
}

/**
 * Derive user portfolio PDA
 */
export function derivePortfolioPda(owner: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEEDS.PORTFOLIO, owner.toBuffer()],
    ROUTER_PROGRAM_ID
  );
}

/**
 * Derive slab state PDA
 */
export function deriveSlabPda(
  lpOwner: PublicKey,
  slabIndex: number
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEEDS.SLAB, lpOwner.toBuffer(), Buffer.from([slabIndex])],
    SLAB_PROGRAM_ID
  );
}

/**
 * Derive insurance pool PDA
 */
export function deriveInsurancePda(slabState: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEEDS.INSURANCE, slabState.toBuffer()],
    SLAB_PROGRAM_ID
  );
}

/**
 * Derive vault authority PDA (for token transfers)
 */
export function deriveVaultAuthorityPda(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from('vault_authority')],
    ROUTER_PROGRAM_ID
  );
}

/**
 * Derive slab vault PDA
 */
export function deriveSlabVaultPda(slabState: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from('slab_vault'), slabState.toBuffer()],
    SLAB_PROGRAM_ID
  );
}

/**
 * Derive insurance vault PDA
 */
export function deriveInsuranceVaultPda(insurancePool: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from('insurance_vault'), insurancePool.toBuffer()],
    SLAB_PROGRAM_ID
  );
}
