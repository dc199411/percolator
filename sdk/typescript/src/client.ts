/**
 * Percolator Protocol Client
 */
import {
  Connection,
  PublicKey,
  Transaction,
  TransactionInstruction,
  Signer,
  Commitment,
  SendOptions,
} from '@solana/web3.js';
import { getAssociatedTokenAddress, TOKEN_PROGRAM_ID } from '@solana/spl-token';
import BN from 'bn.js';
import {
  ROUTER_PROGRAM_ID,
  SLAB_PROGRAM_ID,
  USDC_SCALE,
  PRICE_SCALE,
  QTY_SCALE,
  Side,
  TimeInForce,
} from './constants';
import {
  deriveRegistryPda,
  deriveVaultPda,
  derivePortfolioPda,
  deriveInsurancePda,
} from './pda';
import {
  createInitializePortfolioInstruction,
  createDepositInstruction,
  createWithdrawInstruction,
  createMultiSlabReserveInstruction,
  createMultiSlabCommitInstruction,
  createGlobalLiquidationInstruction,
  createInitializeInsuranceInstruction,
  createContributeInsuranceInstruction,
  createInitiateInsuranceWithdrawalInstruction,
  createCompleteInsuranceWithdrawalInstruction,
} from './instructions';
import type {
  UserPortfolio,
  SlabHeader,
  InsurancePool,
  OrderParams,
  MultiSlabReserveParams,
  SlabSplit,
  DepositParams,
  WithdrawParams,
  InitializeInsuranceParams,
  ContributeInsuranceParams,
  InitiateWithdrawalParams,
  PortfolioMarginResult,
} from './types';

// ============================================================================
// CLIENT OPTIONS
// ============================================================================

export interface PercolatorClientOptions {
  /** Solana RPC connection */
  connection: Connection;
  /** USDC mint address */
  usdcMint: PublicKey;
  /** Default commitment level */
  commitment?: Commitment;
}

// ============================================================================
// PERCOLATOR CLIENT
// ============================================================================

/**
 * Main client for interacting with Percolator Protocol
 */
export class PercolatorClient {
  readonly connection: Connection;
  readonly usdcMint: PublicKey;
  readonly commitment: Commitment;

  // Cached PDAs
  private _registryPda?: PublicKey;
  private _vaultPda?: PublicKey;

  constructor(options: PercolatorClientOptions) {
    this.connection = options.connection;
    this.usdcMint = options.usdcMint;
    this.commitment = options.commitment ?? 'confirmed';
  }

  // ==========================================================================
  // PDA GETTERS
  // ==========================================================================

  /** Get router registry PDA */
  get registryPda(): PublicKey {
    if (!this._registryPda) {
      [this._registryPda] = deriveRegistryPda();
    }
    return this._registryPda;
  }

  /** Get router vault PDA */
  get vaultPda(): PublicKey {
    if (!this._vaultPda) {
      [this._vaultPda] = deriveVaultPda();
    }
    return this._vaultPda;
  }

  /** Derive portfolio PDA for user */
  getPortfolioPda(owner: PublicKey): PublicKey {
    const [pda] = derivePortfolioPda(owner);
    return pda;
  }

  /** Derive insurance pool PDA for slab */
  getInsurancePda(slabState: PublicKey): PublicKey {
    const [pda] = deriveInsurancePda(slabState);
    return pda;
  }

  // ==========================================================================
  // ACCOUNT FETCHING
  // ==========================================================================

  /**
   * Fetch user portfolio
   */
  async getPortfolio(owner: PublicKey): Promise<UserPortfolio | null> {
    const portfolioPda = this.getPortfolioPda(owner);
    const account = await this.connection.getAccountInfo(portfolioPda, this.commitment);
    
    if (!account) {
      return null;
    }

    return this.deserializePortfolio(account.data);
  }

  /**
   * Fetch slab header
   */
  async getSlabHeader(slabState: PublicKey): Promise<SlabHeader | null> {
    const account = await this.connection.getAccountInfo(slabState, this.commitment);
    
    if (!account) {
      return null;
    }

    return this.deserializeSlabHeader(account.data);
  }

  /**
   * Fetch insurance pool
   */
  async getInsurancePool(slabState: PublicKey): Promise<InsurancePool | null> {
    const insurancePda = this.getInsurancePda(slabState);
    const account = await this.connection.getAccountInfo(insurancePda, this.commitment);
    
    if (!account) {
      return null;
    }

    return this.deserializeInsurancePool(account.data);
  }

  // ==========================================================================
  // TRANSACTION BUILDERS
  // ==========================================================================

  /**
   * Initialize user portfolio
   */
  async initializePortfolio(owner: PublicKey): Promise<TransactionInstruction> {
    return createInitializePortfolioInstruction(owner);
  }

  /**
   * Deposit USDC collateral
   */
  async deposit(
    owner: PublicKey,
    amount: BN | number | string
  ): Promise<TransactionInstruction> {
    const userTokenAccount = await getAssociatedTokenAddress(this.usdcMint, owner);
    
    const params: DepositParams = {
      amount: new BN(amount),
    };

    return createDepositInstruction(owner, userTokenAccount, params);
  }

  /**
   * Withdraw USDC collateral
   */
  async withdraw(
    owner: PublicKey,
    amount: BN | number | string
  ): Promise<TransactionInstruction> {
    const userTokenAccount = await getAssociatedTokenAddress(this.usdcMint, owner);
    
    const params: WithdrawParams = {
      amount: new BN(amount),
    };

    return createWithdrawInstruction(owner, userTokenAccount, params);
  }

  /**
   * Place a cross-slab order (reserve + commit in one transaction)
   */
  async placeOrder(
    owner: PublicKey,
    slabAccounts: PublicKey[],
    splits: SlabSplit[],
    requestId?: BN
  ): Promise<TransactionInstruction[]> {
    const id = requestId ?? new BN(Date.now());
    const expiryTs = new BN(Math.floor(Date.now() / 1000) + 60); // 60 second expiry

    const totalQty = splits.reduce(
      (sum, split) => sum.add(split.qty),
      new BN(0)
    );

    const reserveParams: MultiSlabReserveParams = {
      splits,
      totalQty,
      requestId: id,
      expiryTs,
    };

    const reserveIx = createMultiSlabReserveInstruction(
      owner,
      slabAccounts,
      reserveParams
    );

    // Note: In production, you'd wait for reserve to succeed and get hold IDs
    // before building commit. Here we're building both in one go for simplicity.
    const holdIds = splits.map((_, i) => new BN(i + 1)); // Placeholder

    const commitIx = createMultiSlabCommitInstruction(
      owner,
      slabAccounts,
      id,
      holdIds
    );

    return [reserveIx, commitIx];
  }

  /**
   * Liquidate an undercollateralized portfolio
   */
  async liquidate(
    liquidator: PublicKey,
    targetOwner: PublicKey,
    slabAccounts: PublicKey[]
  ): Promise<TransactionInstruction> {
    const targetPortfolio = this.getPortfolioPda(targetOwner);
    return createGlobalLiquidationInstruction(liquidator, targetPortfolio, slabAccounts);
  }

  // ==========================================================================
  // INSURANCE OPERATIONS
  // ==========================================================================

  /**
   * Initialize insurance pool for a slab
   */
  async initializeInsurance(
    slabState: PublicKey,
    lpOwner: PublicKey,
    params: InitializeInsuranceParams
  ): Promise<TransactionInstruction> {
    return createInitializeInsuranceInstruction(slabState, lpOwner, params);
  }

  /**
   * Contribute to insurance pool
   */
  async contributeInsurance(
    slabState: PublicKey,
    lpOwner: PublicKey,
    amount: BN | number | string,
    insuranceVault: PublicKey
  ): Promise<TransactionInstruction> {
    const lpTokenAccount = await getAssociatedTokenAddress(this.usdcMint, lpOwner);
    
    const params: ContributeInsuranceParams = {
      amount: new BN(amount),
    };

    return createContributeInsuranceInstruction(
      slabState,
      lpOwner,
      lpTokenAccount,
      insuranceVault,
      params
    );
  }

  /**
   * Initiate insurance withdrawal (starts timelock)
   */
  async initiateInsuranceWithdrawal(
    slabState: PublicKey,
    lpOwner: PublicKey,
    amount: BN | number | string
  ): Promise<TransactionInstruction> {
    const params: InitiateWithdrawalParams = {
      amount: new BN(amount),
    };

    return createInitiateInsuranceWithdrawalInstruction(slabState, lpOwner, params);
  }

  /**
   * Complete insurance withdrawal (after timelock)
   */
  async completeInsuranceWithdrawal(
    slabState: PublicKey,
    lpOwner: PublicKey,
    insuranceVault: PublicKey,
    vaultAuthority: PublicKey
  ): Promise<TransactionInstruction> {
    const lpTokenAccount = await getAssociatedTokenAddress(this.usdcMint, lpOwner);

    return createCompleteInsuranceWithdrawalInstruction(
      slabState,
      lpOwner,
      lpTokenAccount,
      insuranceVault,
      vaultAuthority
    );
  }

  // ==========================================================================
  // MARGIN CALCULATIONS
  // ==========================================================================

  /**
   * Calculate portfolio margin requirements
   */
  calculatePortfolioMargin(portfolio: UserPortfolio): PortfolioMarginResult {
    let grossIm = new BN(0);
    let grossMm = new BN(0);

    // Calculate gross margins per position
    for (const pos of portfolio.positions) {
      if (pos.qty.isZero()) continue;

      const notional = pos.qty.abs().mul(pos.lastMarkPrice).div(PRICE_SCALE);
      const imr = 500; // 5% IMR in bps
      const mmr = 250; // 2.5% MMR in bps

      grossIm = grossIm.add(notional.muln(imr).divn(10000));
      grossMm = grossMm.add(notional.muln(mmr).divn(10000));
    }

    // Calculate net margins (with netting benefit)
    // Group positions by instrument for netting
    const netExposures = new Map<string, BN>();
    
    for (const pos of portfolio.positions) {
      const key = `${pos.slabIndex}-${pos.instrumentIndex}`;
      const existing = netExposures.get(key) ?? new BN(0);
      netExposures.set(key, existing.add(pos.qty));
    }

    let netIm = new BN(0);
    let netMm = new BN(0);

    for (const [key, netQty] of netExposures) {
      if (netQty.isZero()) continue;

      // Use last mark price from first matching position
      const pos = portfolio.positions.find(
        (p) => `${p.slabIndex}-${p.instrumentIndex}` === key
      );
      if (!pos) continue;

      const notional = netQty.abs().mul(pos.lastMarkPrice).div(PRICE_SCALE);
      const imr = 500;
      const mmr = 250;

      netIm = netIm.add(notional.muln(imr).divn(10000));
      netMm = netMm.add(notional.muln(mmr).divn(10000));
    }

    const nettingBenefit = grossIm.sub(netIm);
    const availableMargin = portfolio.collateralBalance.add(portfolio.unrealizedPnl).sub(netIm);

    let marginRatioBps = 0;
    if (!netIm.isZero()) {
      marginRatioBps = portfolio.collateralBalance
        .add(portfolio.unrealizedPnl)
        .muln(10000)
        .div(netIm)
        .toNumber();
    }

    return {
      grossIm,
      netIm,
      grossMm,
      netMm,
      nettingBenefit,
      availableMargin,
      marginRatioBps,
    };
  }

  // ==========================================================================
  // HELPER METHODS
  // ==========================================================================

  /**
   * Send and confirm transaction
   */
  async sendTransaction(
    transaction: Transaction,
    signers: Signer[],
    options?: SendOptions
  ): Promise<string> {
    const latestBlockhash = await this.connection.getLatestBlockhash(this.commitment);
    transaction.recentBlockhash = latestBlockhash.blockhash;
    transaction.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;

    transaction.sign(...signers);

    const signature = await this.connection.sendRawTransaction(
      transaction.serialize(),
      options
    );

    await this.connection.confirmTransaction(
      {
        signature,
        blockhash: latestBlockhash.blockhash,
        lastValidBlockHeight: latestBlockhash.lastValidBlockHeight,
      },
      this.commitment
    );

    return signature;
  }

  // ==========================================================================
  // DESERIALIZATION (simplified - actual impl would be more robust)
  // ==========================================================================

  private deserializePortfolio(data: Buffer): UserPortfolio {
    // Simplified deserialization - real impl would handle full struct
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
      positions: [], // Would deserialize position array
      bump: readU8(),
    };
  }

  private deserializeSlabHeader(data: Buffer): SlabHeader {
    // Simplified - real impl would deserialize full header
    const offset = { value: 0 };

    const magic = data.slice(0, 8);
    offset.value = 8;

    const version = data.readUInt32LE(offset.value);
    offset.value += 4;

    const seqno = data.readUInt32LE(offset.value);
    offset.value += 4;

    const programId = new PublicKey(data.slice(offset.value, offset.value + 32));
    offset.value += 32;

    const lpOwner = new PublicKey(data.slice(offset.value, offset.value + 32));
    offset.value += 32;

    const routerId = new PublicKey(data.slice(offset.value, offset.value + 32));
    offset.value += 32;

    const readU64 = (): BN => {
      const value = new BN(data.slice(offset.value, offset.value + 8), 'le');
      offset.value += 8;
      return value;
    };

    return {
      magic: new Uint8Array(magic),
      version,
      seqno,
      programId,
      lpOwner,
      routerId,
      imrBps: readU64(),
      mmrBps: readU64(),
      makerFeeBps: readU64(),
      takerFeeBps: readU64(),
      batchMs: readU64(),
      killBandBps: readU64(),
      freezeLevels: 0,
      jitPenaltyOn: false,
      markPx: new BN(0),
      instrumentCount: 0,
      orderCount: 0,
    };
  }

  private deserializeInsurancePool(data: Buffer): InsurancePool {
    // Simplified deserialization
    const offset = { value: 0 };

    const readU128 = (): BN => {
      const value = new BN(data.slice(offset.value, offset.value + 16), 'le');
      offset.value += 16;
      return value;
    };

    const readU64 = (): BN => {
      const value = new BN(data.slice(offset.value, offset.value + 8), 'le');
      offset.value += 8;
      return value;
    };

    const read32Bytes = (): PublicKey => {
      const bytes = data.slice(offset.value, offset.value + 32);
      offset.value += 32;
      return new PublicKey(bytes);
    };

    return {
      balance: readU128(),
      targetBalance: readU128(),
      contributionRateBps: readU64(),
      adlThresholdBps: readU64(),
      withdrawalTimelockSecs: readU64(),
      pendingWithdrawal: readU128(),
      pendingWithdrawalUnlockTs: readU64(),
      lpOwner: read32Bytes(),
      totalOpenInterest: readU128(),
      stats: {
        totalContributions: readU128(),
        totalPayouts: readU128(),
        adlEvents: readU64(),
        shortfallEvents: readU64(),
        maxSinglePayout: readU128(),
        lastContributionTs: readU64(),
        lastPayoutTs: readU64(),
      },
    };
  }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/**
 * Convert human-readable USDC amount to on-chain format
 */
export function usdcToRaw(amount: number | string): BN {
  const parts = amount.toString().split('.');
  const wholePart = parts[0];
  const decimalPart = (parts[1] ?? '').padEnd(6, '0').slice(0, 6);
  return new BN(wholePart + decimalPart);
}

/**
 * Convert on-chain USDC amount to human-readable format
 */
export function usdcFromRaw(rawAmount: BN): string {
  const str = rawAmount.toString().padStart(7, '0');
  const wholePart = str.slice(0, -6) || '0';
  const decimalPart = str.slice(-6);
  return `${wholePart}.${decimalPart}`;
}

/**
 * Convert human-readable price to on-chain format
 */
export function priceToRaw(price: number | string): BN {
  const parts = price.toString().split('.');
  const wholePart = parts[0];
  const decimalPart = (parts[1] ?? '').padEnd(6, '0').slice(0, 6);
  return new BN(wholePart + decimalPart);
}

/**
 * Convert on-chain price to human-readable format
 */
export function priceFromRaw(rawPrice: BN): string {
  const str = rawPrice.toString().padStart(7, '0');
  const wholePart = str.slice(0, -6) || '0';
  const decimalPart = str.slice(-6);
  return `${wholePart}.${decimalPart}`;
}

/**
 * Convert human-readable quantity to on-chain format
 */
export function qtyToRaw(qty: number | string): BN {
  const parts = qty.toString().split('.');
  const wholePart = parts[0];
  const decimalPart = (parts[1] ?? '').padEnd(6, '0').slice(0, 6);
  return new BN(wholePart + decimalPart);
}

/**
 * Convert on-chain quantity to human-readable format
 */
export function qtyFromRaw(rawQty: BN): string {
  const str = rawQty.toString().padStart(7, '0');
  const wholePart = str.slice(0, -6) || '0';
  const decimalPart = str.slice(-6);
  return `${wholePart}.${decimalPart}`;
}

/**
 * Calculate PnL for a position
 */
export function calculatePnl(
  qty: BN,
  entryPrice: BN,
  currentPrice: BN
): BN {
  const priceDiff = currentPrice.sub(entryPrice);
  return qty.mul(priceDiff).div(PRICE_SCALE);
}

/**
 * Check if a portfolio is liquidatable
 */
export function isLiquidatable(
  collateral: BN,
  unrealizedPnl: BN,
  maintenanceMargin: BN
): boolean {
  const equity = collateral.add(unrealizedPnl);
  return equity.lt(maintenanceMargin);
}
