/**
 * Percolator Protocol Constants
 */
import { PublicKey } from '@solana/web3.js';

// ============================================================================
// PROGRAM IDS
// ============================================================================

/** Slab Program ID */
export const SLAB_PROGRAM_ID = new PublicKey('SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk');

/** Router Program ID */
export const ROUTER_PROGRAM_ID = new PublicKey('RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr');

// ============================================================================
// PDA SEEDS
// ============================================================================

export const SEEDS = {
  /** Router registry PDA seed */
  REGISTRY: Buffer.from('registry'),
  /** Router vault PDA seed */
  VAULT: Buffer.from('vault'),
  /** User portfolio PDA seed */
  PORTFOLIO: Buffer.from('portfolio'),
  /** Slab state PDA seed */
  SLAB: Buffer.from('slab'),
  /** Insurance pool PDA seed */
  INSURANCE: Buffer.from('insurance'),
} as const;

// ============================================================================
// SCALING FACTORS
// ============================================================================

/** Price scale (1e6) - prices represented as 6 decimal places */
export const PRICE_SCALE = 1_000_000n;

/** Quantity scale (1e6) - quantities represented as 6 decimal places */
export const QTY_SCALE = 1_000_000n;

/** Token scale for USDC (1e6) */
export const USDC_SCALE = 1_000_000n;

/** Basis points scale (10000 = 100%) */
export const BPS_SCALE = 10_000n;

// ============================================================================
// INSTRUCTION DISCRIMINATORS
// ============================================================================

/** Router instruction discriminators */
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

/** Slab instruction discriminators */
export const SlabInstruction = {
  Reserve: 0,
  Commit: 1,
  Cancel: 2,
  BatchOpen: 3,
  Initialize: 4,
  AddInstrument: 5,
  UpdateFunding: 6,
  Liquidation: 7,
  InitializeInsurance: 8,
  ContributeInsurance: 9,
  InitiateInsuranceWithdrawal: 10,
  CompleteInsuranceWithdrawal: 11,
  CancelInsuranceWithdrawal: 12,
  UpdateInsuranceConfig: 13,
} as const;

// ============================================================================
// LIMITS AND DEFAULTS
// ============================================================================

/** Maximum slabs in router registry */
export const MAX_SLABS = 8;

/** Maximum instruments per slab */
export const MAX_INSTRUMENTS = 8;

/** Maximum positions per user */
export const MAX_POSITIONS = 16;

/** Default batch window (milliseconds) */
export const DEFAULT_BATCH_MS = 100;

/** Default IMR (basis points) */
export const DEFAULT_IMR_BPS = 500; // 5%

/** Default MMR (basis points) */
export const DEFAULT_MMR_BPS = 250; // 2.5%

/** Default maker fee (basis points, negative = rebate) */
export const DEFAULT_MAKER_FEE_BPS = -5; // -0.05%

/** Default taker fee (basis points) */
export const DEFAULT_TAKER_FEE_BPS = 20; // 0.2%

/** Default insurance contribution rate (basis points) */
export const DEFAULT_INSURANCE_RATE_BPS = 25; // 0.25%

/** Insurance pool withdrawal timelock (seconds) */
export const INSURANCE_WITHDRAWAL_TIMELOCK_SECS = 7 * 24 * 60 * 60; // 7 days

// ============================================================================
// ORDER TYPES
// ============================================================================

/** Order side */
export enum Side {
  Buy = 0,
  Sell = 1,
}

/** Time in force */
export enum TimeInForce {
  GTC = 0, // Good til cancelled
  IOC = 1, // Immediate or cancel
  FOK = 2, // Fill or kill
  POST = 3, // Post only (maker only)
}

/** Maker class for anti-toxicity */
export enum MakerClass {
  Retail = 0,
  Informed = 1,
  MM = 2,
}

/** Order status */
export enum OrderStatus {
  Pending = 0,
  Open = 1,
  PartiallyFilled = 2,
  Filled = 3,
  Cancelled = 4,
  Expired = 5,
}

/** Reservation status */
export enum ReservationStatus {
  Active = 0,
  Committed = 1,
  Cancelled = 2,
  Expired = 3,
}
