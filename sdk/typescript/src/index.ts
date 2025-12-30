/**
 * Percolator Protocol SDK
 *
 * TypeScript SDK for interacting with the Percolator perpetual futures protocol on Solana.
 *
 * @example
 * ```typescript
 * import { PercolatorClient, usdcToRaw, Side } from '@percolator/sdk';
 * import { Connection, PublicKey, Keypair } from '@solana/web3.js';
 *
 * const connection = new Connection('https://api.devnet.solana.com');
 * const client = new PercolatorClient({
 *   connection,
 *   usdcMint: new PublicKey('...'),
 * });
 *
 * // Initialize portfolio
 * const initIx = await client.initializePortfolio(wallet.publicKey);
 *
 * // Deposit collateral
 * const depositIx = await client.deposit(wallet.publicKey, usdcToRaw(1000));
 *
 * // Place an order
 * const orderIxs = await client.placeOrder(
 *   wallet.publicKey,
 *   [slabAddress],
 *   [{
 *     slabIndex: 0,
 *     instrumentIndex: 0, // BTC-PERP
 *     side: Side.Buy,
 *     qty: qtyToRaw(0.1),
 *     limitPrice: priceToRaw(50000),
 *   }]
 * );
 * ```
 *
 * @packageDocumentation
 */

// Constants
export {
  SLAB_PROGRAM_ID,
  ROUTER_PROGRAM_ID,
  SEEDS,
  PRICE_SCALE,
  QTY_SCALE,
  USDC_SCALE,
  BPS_SCALE,
  RouterInstruction,
  SlabInstruction,
  MAX_SLABS,
  MAX_INSTRUMENTS,
  MAX_POSITIONS,
  DEFAULT_BATCH_MS,
  DEFAULT_IMR_BPS,
  DEFAULT_MMR_BPS,
  DEFAULT_MAKER_FEE_BPS,
  DEFAULT_TAKER_FEE_BPS,
  DEFAULT_INSURANCE_RATE_BPS,
  INSURANCE_WITHDRAWAL_TIMELOCK_SECS,
  Side,
  TimeInForce,
  MakerClass,
  OrderStatus,
  ReservationStatus,
} from './constants';

// Types
export type {
  RouterRegistry,
  SlabEntry,
  UserPortfolio,
  PositionInfo,
  SlabHeader,
  InsurancePool,
  InsuranceStats,
  OrderParams,
  Order,
  Reservation,
  CrossSlabOrderParams,
  SlabOrderParams,
  MultiSlabReserveParams,
  SlabSplit,
  DepositParams,
  WithdrawParams,
  InitializeInsuranceParams,
  ContributeInsuranceParams,
  InitiateWithdrawalParams,
  ReserveResponse,
  CommitResponse,
  LiquidationResponse,
  PortfolioMarginResult,
} from './types';

// PDA Derivation
export {
  deriveRegistryPda,
  deriveVaultPda,
  derivePortfolioPda,
  deriveSlabPda,
  deriveInsurancePda,
  deriveVaultAuthorityPda,
  deriveSlabVaultPda,
  deriveInsuranceVaultPda,
} from './pda';

// Instructions
export {
  createInitializeRouterInstruction,
  createInitializePortfolioInstruction,
  createDepositInstruction,
  createWithdrawInstruction,
  createMultiSlabReserveInstruction,
  createMultiSlabCommitInstruction,
  createGlobalLiquidationInstruction,
  createReserveInstruction,
  createCommitInstruction,
  createCancelInstruction,
  createInitializeInsuranceInstruction,
  createContributeInsuranceInstruction,
  createInitiateInsuranceWithdrawalInstruction,
  createCompleteInsuranceWithdrawalInstruction,
  createCancelInsuranceWithdrawalInstruction,
} from './instructions';

// Client
export {
  PercolatorClient,
  type PercolatorClientOptions,
  usdcToRaw,
  usdcFromRaw,
  priceToRaw,
  priceFromRaw,
  qtyToRaw,
  qtyFromRaw,
  calculatePnl,
  isLiquidatable,
} from './client';
