/**
 * Percolator Protocol Integration
 * Client-side SDK wrapper for interacting with Percolator smart contracts
 */

import {
  Connection,
  PublicKey,
  Transaction,
  TransactionInstruction,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID, getAssociatedTokenAddress } from '@solana/spl-token';
import BN from 'bn.js';

// Program IDs
export const ROUTER_PROGRAM_ID = new PublicKey('RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr');
export const SLAB_PROGRAM_ID = new PublicKey('SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk');

// USDC Mint (Devnet)
export const USDC_MINT = new PublicKey('4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU');

// Seeds for PDA derivation
const SEEDS = {
  REGISTRY: Buffer.from('registry'),
  VAULT: Buffer.from('vault'),
  PORTFOLIO: Buffer.from('portfolio'),
  SLAB: Buffer.from('slab'),
  INSURANCE: Buffer.from('insurance'),
};

// Scaling factors
export const PRICE_SCALE = BigInt(1_000_000);
export const QTY_SCALE = BigInt(1_000_000);
export const USDC_SCALE = BigInt(1_000_000);

// Instruction discriminators
export const RouterInstruction = {
  Initialize: 0,
  InitializePortfolio: 1,
  Deposit: 2,
  Withdraw: 3,
  ExecuteCrossSlab: 4,
  MultiSlabReserve: 5,
  MultiSlabCommit: 6,
  MultiSlabCancel: 7,
  GlobalLiquidation: 8,
  MarkToMarket: 9,
} as const;

// Types
export interface UserPortfolio {
  owner: PublicKey;
  portfolioId: BN;
  collateralBalance: BN;
  initialMarginUsed: BN;
  maintenanceMarginUsed: BN;
  unrealizedPnl: BN;
  realizedPnl: BN;
  positions: PositionInfo[];
  bump: number;
}

export interface PositionInfo {
  slabIndex: number;
  instrumentIndex: number;
  qty: BN;
  entryPrice: BN;
  entryValue: BN;
  lastMarkPrice: BN;
  unrealizedPnl: BN;
}

export interface SlabSplit {
  slabIndex: number;
  instrumentIndex: number;
  qty: BN;
  limitPrice: BN;
}

// PDA Derivation
export function deriveRegistryPda(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync([SEEDS.REGISTRY], ROUTER_PROGRAM_ID);
}

export function deriveVaultPda(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync([SEEDS.VAULT], ROUTER_PROGRAM_ID);
}

export function derivePortfolioPda(owner: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEEDS.PORTFOLIO, owner.toBuffer()],
    ROUTER_PROGRAM_ID
  );
}

export function deriveVaultAuthorityPda(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from('vault_authority')],
    ROUTER_PROGRAM_ID
  );
}

// Helper functions
function u8ToBuffer(value: number): Buffer {
  const buf = Buffer.alloc(1);
  buf.writeUInt8(value);
  return buf;
}

function u64ToBuffer(value: BN): Buffer {
  return value.toArrayLike(Buffer, 'le', 8);
}

// Instruction builders
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

export function createDepositInstruction(
  owner: PublicKey,
  userTokenAccount: PublicKey,
  amount: BN
): TransactionInstruction {
  const [registryPda] = deriveRegistryPda();
  const [vaultPda] = deriveVaultPda();
  const [portfolioPda] = derivePortfolioPda(owner);

  const data = Buffer.concat([
    u8ToBuffer(RouterInstruction.Deposit),
    u64ToBuffer(amount),
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

export function createWithdrawInstruction(
  owner: PublicKey,
  userTokenAccount: PublicKey,
  amount: BN
): TransactionInstruction {
  const [registryPda] = deriveRegistryPda();
  const [vaultPda] = deriveVaultPda();
  const [vaultAuthority] = deriveVaultAuthorityPda();
  const [portfolioPda] = derivePortfolioPda(owner);

  const data = Buffer.concat([
    u8ToBuffer(RouterInstruction.Withdraw),
    u64ToBuffer(amount),
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

export function createMultiSlabReserveInstruction(
  owner: PublicKey,
  slabAccounts: PublicKey[],
  splits: SlabSplit[],
  totalQty: BN,
  requestId: BN,
  expiryTs: BN
): TransactionInstruction {
  const [registryPda] = deriveRegistryPda();
  const [portfolioPda] = derivePortfolioPda(owner);

  const splitBuffers = splits.map((split) =>
    Buffer.concat([
      u8ToBuffer(split.slabIndex),
      u8ToBuffer(split.instrumentIndex),
      u64ToBuffer(split.qty),
      u64ToBuffer(split.limitPrice),
    ])
  );

  const data = Buffer.concat([
    u8ToBuffer(RouterInstruction.MultiSlabReserve),
    u8ToBuffer(splits.length),
    ...splitBuffers,
    u64ToBuffer(totalQty),
    u64ToBuffer(requestId),
    u64ToBuffer(expiryTs),
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

// Utility functions
export function usdcToRaw(amount: number | string): BN {
  const parts = amount.toString().split('.');
  const wholePart = parts[0];
  const decimalPart = (parts[1] ?? '').padEnd(6, '0').slice(0, 6);
  return new BN(wholePart + decimalPart);
}

export function usdcFromRaw(rawAmount: BN): string {
  const str = rawAmount.toString().padStart(7, '0');
  const wholePart = str.slice(0, -6) || '0';
  const decimalPart = str.slice(-6);
  return `${wholePart}.${decimalPart}`;
}

export function priceToRaw(price: number | string): BN {
  const parts = price.toString().split('.');
  const wholePart = parts[0];
  const decimalPart = (parts[1] ?? '').padEnd(6, '0').slice(0, 6);
  return new BN(wholePart + decimalPart);
}

export function priceFromRaw(rawPrice: BN): string {
  const str = rawPrice.toString().padStart(7, '0');
  const wholePart = str.slice(0, -6) || '0';
  const decimalPart = str.slice(-6);
  return `${wholePart}.${decimalPart}`;
}

export function qtyToRaw(qty: number | string): BN {
  const parts = qty.toString().split('.');
  const wholePart = parts[0];
  const decimalPart = (parts[1] ?? '').padEnd(6, '0').slice(0, 6);
  return new BN(wholePart + decimalPart);
}

export function qtyFromRaw(rawQty: BN): string {
  const str = rawQty.toString().padStart(7, '0');
  const wholePart = str.slice(0, -6) || '0';
  const decimalPart = str.slice(-6);
  return `${wholePart}.${decimalPart}`;
}

// Percolator Client Class
export class PercolatorClient {
  readonly connection: Connection;
  readonly usdcMint: PublicKey;

  constructor(connection: Connection, usdcMint: PublicKey = USDC_MINT) {
    this.connection = connection;
    this.usdcMint = usdcMint;
  }

  async getPortfolio(owner: PublicKey): Promise<UserPortfolio | null> {
    const [portfolioPda] = derivePortfolioPda(owner);
    const account = await this.connection.getAccountInfo(portfolioPda);
    
    if (!account) {
      return null;
    }

    // Simplified deserialization - in production would be more robust
    return this.deserializePortfolio(account.data);
  }

  async hasPortfolio(owner: PublicKey): Promise<boolean> {
    const [portfolioPda] = derivePortfolioPda(owner);
    const account = await this.connection.getAccountInfo(portfolioPda);
    return account !== null;
  }

  async getUserTokenAccount(owner: PublicKey): Promise<PublicKey> {
    return getAssociatedTokenAddress(this.usdcMint, owner);
  }

  async buildInitializePortfolioTx(owner: PublicKey): Promise<Transaction> {
    const tx = new Transaction();
    tx.add(createInitializePortfolioInstruction(owner));
    return tx;
  }

  async buildDepositTx(owner: PublicKey, amount: number): Promise<Transaction> {
    const userTokenAccount = await this.getUserTokenAccount(owner);
    const rawAmount = usdcToRaw(amount);
    
    const tx = new Transaction();
    tx.add(createDepositInstruction(owner, userTokenAccount, rawAmount));
    return tx;
  }

  async buildWithdrawTx(owner: PublicKey, amount: number): Promise<Transaction> {
    const userTokenAccount = await this.getUserTokenAccount(owner);
    const rawAmount = usdcToRaw(amount);
    
    const tx = new Transaction();
    tx.add(createWithdrawInstruction(owner, userTokenAccount, rawAmount));
    return tx;
  }

  private deserializePortfolio(data: Buffer): UserPortfolio {
    const offset = { value: 0 };

    const read32Bytes = (): PublicKey => {
      const bytes = data.slice(offset.value, offset.value + 32);
      offset.value += 32;
      return new PublicKey(bytes);
    };

    const readU64 = (): BN => {
      const value = new BN(data.slice(offset.value, offset.value + 8), 'le');
      offset.value += 8;
      return value;
    };

    const readI64 = (): BN => {
      const value = new BN(data.slice(offset.value, offset.value + 8), 'le');
      offset.value += 8;
      return value;
    };

    const readU8 = (): number => {
      const value = data[offset.value];
      offset.value += 1;
      return value;
    };

    return {
      owner: read32Bytes(),
      portfolioId: readU64(),
      collateralBalance: readU64(),
      initialMarginUsed: readU64(),
      maintenanceMarginUsed: readU64(),
      unrealizedPnl: readI64(),
      realizedPnl: readI64(),
      positions: [],
      bump: readU8(),
    };
  }
}
