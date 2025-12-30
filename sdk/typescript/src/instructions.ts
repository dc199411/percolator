/**
 * Instruction Builders
 */
import {
  PublicKey,
  TransactionInstruction,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import BN from 'bn.js';
import {
  ROUTER_PROGRAM_ID,
  SLAB_PROGRAM_ID,
  RouterInstruction,
  SlabInstruction,
  Side,
  TimeInForce,
} from './constants';
import {
  deriveRegistryPda,
  deriveVaultPda,
  derivePortfolioPda,
  deriveInsurancePda,
  deriveVaultAuthorityPda,
} from './pda';
import type {
  OrderParams,
  MultiSlabReserveParams,
  DepositParams,
  WithdrawParams,
  InitializeInsuranceParams,
  ContributeInsuranceParams,
  InitiateWithdrawalParams,
} from './types';

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

function u8ToBuffer(value: number): Buffer {
  const buf = Buffer.alloc(1);
  buf.writeUInt8(value);
  return buf;
}

function u16ToBuffer(value: number): Buffer {
  const buf = Buffer.alloc(2);
  buf.writeUInt16LE(value);
  return buf;
}

function u64ToBuffer(value: BN): Buffer {
  return value.toArrayLike(Buffer, 'le', 8);
}

function i64ToBuffer(value: BN): Buffer {
  return value.toArrayLike(Buffer, 'le', 8);
}

function u128ToBuffer(value: BN): Buffer {
  return value.toArrayLike(Buffer, 'le', 16);
}

// ============================================================================
// ROUTER INSTRUCTIONS
// ============================================================================

/**
 * Create initialize router instruction
 */
export function createInitializeRouterInstruction(
  admin: PublicKey,
  usdcMint: PublicKey
): TransactionInstruction {
  const [registryPda] = deriveRegistryPda();
  const [vaultPda] = deriveVaultPda();
  const [vaultAuthority] = deriveVaultAuthorityPda();

  const data = Buffer.from([RouterInstruction.Initialize]);

  const keys = [
    { pubkey: registryPda, isSigner: false, isWritable: true },
    { pubkey: vaultPda, isSigner: false, isWritable: true },
    { pubkey: vaultAuthority, isSigner: false, isWritable: false },
    { pubkey: admin, isSigner: true, isWritable: true },
    { pubkey: usdcMint, isSigner: false, isWritable: false },
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
  ];

  return new TransactionInstruction({
    keys,
    programId: ROUTER_PROGRAM_ID,
    data,
  });
}

/**
 * Create initialize portfolio instruction
 */
export function createInitializePortfolioInstruction(
  owner: PublicKey
): TransactionInstruction {
  const [registryPda] = deriveRegistryPda();
  const [portfolioPda] = derivePortfolioPda(owner);

  const data = Buffer.from([RouterInstruction.InitializePortfolio]);

  const keys = [
    { pubkey: registryPda, isSigner: false, isWritable: true },
    { pubkey: portfolioPda, isSigner: false, isWritable: true },
    { pubkey: owner, isSigner: true, isWritable: true },
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
  ];

  return new TransactionInstruction({
    keys,
    programId: ROUTER_PROGRAM_ID,
    data,
  });
}

/**
 * Create deposit instruction
 */
export function createDepositInstruction(
  owner: PublicKey,
  userTokenAccount: PublicKey,
  params: DepositParams
): TransactionInstruction {
  const [registryPda] = deriveRegistryPda();
  const [vaultPda] = deriveVaultPda();
  const [portfolioPda] = derivePortfolioPda(owner);

  const data = Buffer.concat([
    u8ToBuffer(RouterInstruction.Deposit),
    u64ToBuffer(params.amount),
  ]);

  const keys = [
    { pubkey: registryPda, isSigner: false, isWritable: true },
    { pubkey: portfolioPda, isSigner: false, isWritable: true },
    { pubkey: vaultPda, isSigner: false, isWritable: true },
    { pubkey: userTokenAccount, isSigner: false, isWritable: true },
    { pubkey: owner, isSigner: true, isWritable: false },
    { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
  ];

  return new TransactionInstruction({
    keys,
    programId: ROUTER_PROGRAM_ID,
    data,
  });
}

/**
 * Create withdraw instruction
 */
export function createWithdrawInstruction(
  owner: PublicKey,
  userTokenAccount: PublicKey,
  params: WithdrawParams
): TransactionInstruction {
  const [registryPda] = deriveRegistryPda();
  const [vaultPda] = deriveVaultPda();
  const [vaultAuthority] = deriveVaultAuthorityPda();
  const [portfolioPda] = derivePortfolioPda(owner);

  const data = Buffer.concat([
    u8ToBuffer(RouterInstruction.Withdraw),
    u64ToBuffer(params.amount),
  ]);

  const keys = [
    { pubkey: registryPda, isSigner: false, isWritable: true },
    { pubkey: portfolioPda, isSigner: false, isWritable: true },
    { pubkey: vaultPda, isSigner: false, isWritable: true },
    { pubkey: userTokenAccount, isSigner: false, isWritable: true },
    { pubkey: vaultAuthority, isSigner: false, isWritable: false },
    { pubkey: owner, isSigner: true, isWritable: false },
    { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
  ];

  return new TransactionInstruction({
    keys,
    programId: ROUTER_PROGRAM_ID,
    data,
  });
}

/**
 * Create multi-slab reserve instruction
 */
export function createMultiSlabReserveInstruction(
  owner: PublicKey,
  slabAccounts: PublicKey[],
  params: MultiSlabReserveParams
): TransactionInstruction {
  const [registryPda] = deriveRegistryPda();
  const [portfolioPda] = derivePortfolioPda(owner);

  // Serialize splits
  const splitBuffers = params.splits.map((split) =>
    Buffer.concat([
      u8ToBuffer(split.slabIndex),
      u8ToBuffer(split.instrumentIndex),
      u64ToBuffer(split.qty),
      u64ToBuffer(split.limitPrice),
    ])
  );

  const data = Buffer.concat([
    u8ToBuffer(RouterInstruction.MultiSlabReserve),
    u8ToBuffer(params.splits.length),
    ...splitBuffers,
    u64ToBuffer(params.totalQty),
    u64ToBuffer(params.requestId),
    u64ToBuffer(params.expiryTs),
  ]);

  const keys = [
    { pubkey: registryPda, isSigner: false, isWritable: true },
    { pubkey: portfolioPda, isSigner: false, isWritable: true },
    { pubkey: owner, isSigner: true, isWritable: false },
    ...slabAccounts.map((pubkey) => ({
      pubkey,
      isSigner: false,
      isWritable: true,
    })),
  ];

  return new TransactionInstruction({
    keys,
    programId: ROUTER_PROGRAM_ID,
    data,
  });
}

/**
 * Create multi-slab commit instruction
 */
export function createMultiSlabCommitInstruction(
  owner: PublicKey,
  slabAccounts: PublicKey[],
  requestId: BN,
  holdIds: BN[]
): TransactionInstruction {
  const [registryPda] = deriveRegistryPda();
  const [portfolioPda] = derivePortfolioPda(owner);

  const data = Buffer.concat([
    u8ToBuffer(RouterInstruction.MultiSlabCommit),
    u64ToBuffer(requestId),
    u8ToBuffer(holdIds.length),
    ...holdIds.map((id) => u64ToBuffer(id)),
  ]);

  const keys = [
    { pubkey: registryPda, isSigner: false, isWritable: true },
    { pubkey: portfolioPda, isSigner: false, isWritable: true },
    { pubkey: owner, isSigner: true, isWritable: false },
    ...slabAccounts.map((pubkey) => ({
      pubkey,
      isSigner: false,
      isWritable: true,
    })),
  ];

  return new TransactionInstruction({
    keys,
    programId: ROUTER_PROGRAM_ID,
    data,
  });
}

/**
 * Create global liquidation instruction
 */
export function createGlobalLiquidationInstruction(
  liquidator: PublicKey,
  targetPortfolio: PublicKey,
  slabAccounts: PublicKey[]
): TransactionInstruction {
  const [registryPda] = deriveRegistryPda();

  const data = Buffer.from([RouterInstruction.GlobalLiquidation]);

  const keys = [
    { pubkey: registryPda, isSigner: false, isWritable: true },
    { pubkey: targetPortfolio, isSigner: false, isWritable: true },
    { pubkey: liquidator, isSigner: true, isWritable: false },
    ...slabAccounts.map((pubkey) => ({
      pubkey,
      isSigner: false,
      isWritable: true,
    })),
  ];

  return new TransactionInstruction({
    keys,
    programId: ROUTER_PROGRAM_ID,
    data,
  });
}

// ============================================================================
// SLAB INSTRUCTIONS
// ============================================================================

/**
 * Create reserve instruction
 */
export function createReserveInstruction(
  slabState: PublicKey,
  router: PublicKey,
  params: OrderParams,
  requestId: BN,
  expiryTs: BN
): TransactionInstruction {
  const data = Buffer.concat([
    u8ToBuffer(SlabInstruction.Reserve),
    u8ToBuffer(params.instrumentIndex),
    u8ToBuffer(params.side),
    u64ToBuffer(params.price),
    u64ToBuffer(params.qty),
    u8ToBuffer(params.timeInForce),
    u64ToBuffer(requestId),
    u64ToBuffer(expiryTs),
  ]);

  const keys = [
    { pubkey: slabState, isSigner: false, isWritable: true },
    { pubkey: router, isSigner: true, isWritable: false },
  ];

  return new TransactionInstruction({
    keys,
    programId: SLAB_PROGRAM_ID,
    data,
  });
}

/**
 * Create commit instruction
 */
export function createCommitInstruction(
  slabState: PublicKey,
  router: PublicKey,
  holdId: BN
): TransactionInstruction {
  const data = Buffer.concat([
    u8ToBuffer(SlabInstruction.Commit),
    u64ToBuffer(holdId),
  ]);

  const keys = [
    { pubkey: slabState, isSigner: false, isWritable: true },
    { pubkey: router, isSigner: true, isWritable: false },
  ];

  return new TransactionInstruction({
    keys,
    programId: SLAB_PROGRAM_ID,
    data,
  });
}

/**
 * Create cancel instruction
 */
export function createCancelInstruction(
  slabState: PublicKey,
  router: PublicKey,
  holdId: BN
): TransactionInstruction {
  const data = Buffer.concat([
    u8ToBuffer(SlabInstruction.Cancel),
    u64ToBuffer(holdId),
  ]);

  const keys = [
    { pubkey: slabState, isSigner: false, isWritable: true },
    { pubkey: router, isSigner: true, isWritable: false },
  ];

  return new TransactionInstruction({
    keys,
    programId: SLAB_PROGRAM_ID,
    data,
  });
}

// ============================================================================
// INSURANCE INSTRUCTIONS
// ============================================================================

/**
 * Create initialize insurance instruction
 */
export function createInitializeInsuranceInstruction(
  slabState: PublicKey,
  lpOwner: PublicKey,
  params: InitializeInsuranceParams
): TransactionInstruction {
  const [insurancePda] = deriveInsurancePda(slabState);

  const data = Buffer.concat([
    u8ToBuffer(SlabInstruction.InitializeInsurance),
    u64ToBuffer(new BN(params.contributionRateBps)),
    u64ToBuffer(new BN(params.adlThresholdBps)),
    u64ToBuffer(new BN(params.withdrawalTimelockSecs)),
  ]);

  const keys = [
    { pubkey: slabState, isSigner: false, isWritable: true },
    { pubkey: insurancePda, isSigner: false, isWritable: true },
    { pubkey: lpOwner, isSigner: true, isWritable: true },
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
  ];

  return new TransactionInstruction({
    keys,
    programId: SLAB_PROGRAM_ID,
    data,
  });
}

/**
 * Create contribute insurance instruction
 */
export function createContributeInsuranceInstruction(
  slabState: PublicKey,
  lpOwner: PublicKey,
  lpTokenAccount: PublicKey,
  insuranceVault: PublicKey,
  params: ContributeInsuranceParams
): TransactionInstruction {
  const [insurancePda] = deriveInsurancePda(slabState);

  const data = Buffer.concat([
    u8ToBuffer(SlabInstruction.ContributeInsurance),
    u64ToBuffer(params.amount),
  ]);

  const keys = [
    { pubkey: insurancePda, isSigner: false, isWritable: true },
    { pubkey: lpTokenAccount, isSigner: false, isWritable: true },
    { pubkey: insuranceVault, isSigner: false, isWritable: true },
    { pubkey: lpOwner, isSigner: true, isWritable: false },
    { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
  ];

  return new TransactionInstruction({
    keys,
    programId: SLAB_PROGRAM_ID,
    data,
  });
}

/**
 * Create initiate insurance withdrawal instruction
 */
export function createInitiateInsuranceWithdrawalInstruction(
  slabState: PublicKey,
  lpOwner: PublicKey,
  params: InitiateWithdrawalParams
): TransactionInstruction {
  const [insurancePda] = deriveInsurancePda(slabState);

  const data = Buffer.concat([
    u8ToBuffer(SlabInstruction.InitiateInsuranceWithdrawal),
    u64ToBuffer(params.amount),
  ]);

  const keys = [
    { pubkey: insurancePda, isSigner: false, isWritable: true },
    { pubkey: lpOwner, isSigner: true, isWritable: false },
  ];

  return new TransactionInstruction({
    keys,
    programId: SLAB_PROGRAM_ID,
    data,
  });
}

/**
 * Create complete insurance withdrawal instruction
 */
export function createCompleteInsuranceWithdrawalInstruction(
  slabState: PublicKey,
  lpOwner: PublicKey,
  lpTokenAccount: PublicKey,
  insuranceVault: PublicKey,
  vaultAuthority: PublicKey
): TransactionInstruction {
  const [insurancePda] = deriveInsurancePda(slabState);

  const data = Buffer.from([SlabInstruction.CompleteInsuranceWithdrawal]);

  const keys = [
    { pubkey: insurancePda, isSigner: false, isWritable: true },
    { pubkey: insuranceVault, isSigner: false, isWritable: true },
    { pubkey: lpTokenAccount, isSigner: false, isWritable: true },
    { pubkey: lpOwner, isSigner: true, isWritable: false },
    { pubkey: vaultAuthority, isSigner: false, isWritable: false },
    { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
  ];

  return new TransactionInstruction({
    keys,
    programId: SLAB_PROGRAM_ID,
    data,
  });
}

/**
 * Create cancel insurance withdrawal instruction
 */
export function createCancelInsuranceWithdrawalInstruction(
  slabState: PublicKey,
  lpOwner: PublicKey
): TransactionInstruction {
  const [insurancePda] = deriveInsurancePda(slabState);

  const data = Buffer.from([SlabInstruction.CancelInsuranceWithdrawal]);

  const keys = [
    { pubkey: insurancePda, isSigner: false, isWritable: true },
    { pubkey: lpOwner, isSigner: true, isWritable: false },
  ];

  return new TransactionInstruction({
    keys,
    programId: SLAB_PROGRAM_ID,
    data,
  });
}
