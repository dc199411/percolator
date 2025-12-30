/**
 * Percolator Protocol Types
 */
import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import { Side, TimeInForce, MakerClass, OrderStatus, ReservationStatus } from './constants';

// ============================================================================
// ACCOUNT TYPES
// ============================================================================

/** Router registry account */
export interface RouterRegistry {
  /** Registered slabs */
  slabs: SlabEntry[];
  /** Registry bump */
  bump: number;
  /** Admin authority */
  admin: PublicKey;
  /** Total collateral deposited */
  totalCollateral: BN;
  /** Next portfolio ID */
  nextPortfolioId: BN;
}

/** Slab entry in registry */
export interface SlabEntry {
  /** Slab program address */
  programId: PublicKey;
  /** Slab state PDA */
  stateAddress: PublicKey;
  /** Version for CPI compatibility */
  version: number;
  /** Active status */
  isActive: boolean;
  /** Total open interest */
  openInterest: BN;
}

/** User portfolio account */
export interface UserPortfolio {
  /** Owner pubkey */
  owner: PublicKey;
  /** Portfolio ID */
  portfolioId: BN;
  /** USDC collateral balance */
  collateralBalance: BN;
  /** Initial margin used */
  initialMarginUsed: BN;
  /** Maintenance margin used */
  maintenanceMarginUsed: BN;
  /** Unrealized PnL */
  unrealizedPnl: BN;
  /** Realized PnL */
  realizedPnl: BN;
  /** Active positions */
  positions: PositionInfo[];
  /** Bump seed */
  bump: number;
}

/** Position info */
export interface PositionInfo {
  /** Slab index */
  slabIndex: number;
  /** Instrument index */
  instrumentIndex: number;
  /** Signed quantity (positive = long, negative = short) */
  qty: BN;
  /** Entry price (scaled) */
  entryPrice: BN;
  /** Entry value */
  entryValue: BN;
  /** Last mark price */
  lastMarkPrice: BN;
  /** Unrealized PnL */
  unrealizedPnl: BN;
}

/** Slab header */
export interface SlabHeader {
  /** Magic bytes */
  magic: Uint8Array;
  /** Version */
  version: number;
  /** Sequence number */
  seqno: number;
  /** Slab program ID */
  programId: PublicKey;
  /** LP owner */
  lpOwner: PublicKey;
  /** Router ID */
  routerId: PublicKey;
  /** Initial margin ratio (bps) */
  imrBps: BN;
  /** Maintenance margin ratio (bps) */
  mmrBps: BN;
  /** Maker fee (bps, signed) */
  makerFeeBps: BN;
  /** Taker fee (bps) */
  takerFeeBps: BN;
  /** Batch window (ms) */
  batchMs: BN;
  /** Kill band (bps) */
  killBandBps: BN;
  /** Freeze levels */
  freezeLevels: number;
  /** JIT penalty enabled */
  jitPenaltyOn: boolean;
  /** Mark price */
  markPx: BN;
  /** Instrument count */
  instrumentCount: number;
  /** Order count */
  orderCount: number;
}

/** Insurance pool state */
export interface InsurancePool {
  /** Current balance */
  balance: BN;
  /** Target balance */
  targetBalance: BN;
  /** Contribution rate (bps) */
  contributionRateBps: BN;
  /** ADL threshold (bps) */
  adlThresholdBps: BN;
  /** Withdrawal timelock (seconds) */
  withdrawalTimelockSecs: BN;
  /** Pending withdrawal amount */
  pendingWithdrawal: BN;
  /** Pending withdrawal unlock timestamp */
  pendingWithdrawalUnlockTs: BN;
  /** LP owner */
  lpOwner: PublicKey;
  /** Total open interest */
  totalOpenInterest: BN;
  /** Statistics */
  stats: InsuranceStats;
}

/** Insurance statistics */
export interface InsuranceStats {
  /** Total contributions */
  totalContributions: BN;
  /** Total payouts */
  totalPayouts: BN;
  /** ADL events count */
  adlEvents: BN;
  /** Shortfall events count */
  shortfallEvents: BN;
  /** Maximum single payout */
  maxSinglePayout: BN;
  /** Last contribution timestamp */
  lastContributionTs: BN;
  /** Last payout timestamp */
  lastPayoutTs: BN;
}

// ============================================================================
// ORDER TYPES
// ============================================================================

/** Order parameters for placing orders */
export interface OrderParams {
  /** Instrument index */
  instrumentIndex: number;
  /** Order side */
  side: Side;
  /** Limit price (scaled by PRICE_SCALE) */
  price: BN;
  /** Quantity (scaled by QTY_SCALE) */
  qty: BN;
  /** Time in force */
  timeInForce: TimeInForce;
  /** Client order ID (optional) */
  clientOrderId?: BN;
  /** Reduce only flag */
  reduceOnly?: boolean;
}

/** Order in the book */
export interface Order {
  /** Order ID */
  orderId: BN;
  /** Client order ID */
  clientOrderId: BN;
  /** Owner account index */
  ownerIndex: number;
  /** Instrument index */
  instrumentIndex: number;
  /** Side */
  side: Side;
  /** Price (scaled) */
  price: BN;
  /** Original quantity */
  originalQty: BN;
  /** Remaining quantity */
  remainingQty: BN;
  /** Filled quantity */
  filledQty: BN;
  /** Order status */
  status: OrderStatus;
  /** Time in force */
  timeInForce: TimeInForce;
  /** Maker class */
  makerClass: MakerClass;
  /** Created timestamp */
  createdTs: BN;
  /** Updated timestamp */
  updatedTs: BN;
}

/** Reservation (for two-phase commit) */
export interface Reservation {
  /** Hold ID */
  holdId: BN;
  /** Owner account index */
  ownerIndex: number;
  /** Instrument index */
  instrumentIndex: number;
  /** Side */
  side: Side;
  /** Reserved quantity */
  qty: BN;
  /** Reserved price */
  price: BN;
  /** Status */
  status: ReservationStatus;
  /** Expiry timestamp */
  expiryTs: BN;
  /** Router request ID */
  routerRequestId: BN;
}

// ============================================================================
// INSTRUCTION PARAMETERS
// ============================================================================

/** Cross-slab order execution parameters */
export interface CrossSlabOrderParams {
  /** Order details per slab */
  slabOrders: SlabOrderParams[];
  /** Maximum total slippage (bps) */
  maxSlippageBps: number;
  /** Request expiry (Unix timestamp) */
  expiryTs: BN;
}

/** Order parameters for a single slab */
export interface SlabOrderParams {
  /** Slab index in registry */
  slabIndex: number;
  /** Instrument index */
  instrumentIndex: number;
  /** Side */
  side: Side;
  /** Quantity */
  qty: BN;
  /** Limit price */
  limitPrice: BN;
}

/** Multi-slab reserve parameters */
export interface MultiSlabReserveParams {
  /** Splits per slab */
  splits: SlabSplit[];
  /** Total quantity */
  totalQty: BN;
  /** Request ID */
  requestId: BN;
  /** Expiry timestamp */
  expiryTs: BN;
}

/** Slab split for multi-slab operations */
export interface SlabSplit {
  /** Slab index */
  slabIndex: number;
  /** Instrument index */
  instrumentIndex: number;
  /** Quantity for this slab */
  qty: BN;
  /** Limit price */
  limitPrice: BN;
}

/** Deposit parameters */
export interface DepositParams {
  /** Amount to deposit (in USDC, 6 decimals) */
  amount: BN;
}

/** Withdraw parameters */
export interface WithdrawParams {
  /** Amount to withdraw (in USDC, 6 decimals) */
  amount: BN;
}

/** Insurance initialization parameters */
export interface InitializeInsuranceParams {
  /** Contribution rate (bps) */
  contributionRateBps: number;
  /** ADL threshold (bps) */
  adlThresholdBps: number;
  /** Withdrawal timelock (seconds) */
  withdrawalTimelockSecs: number;
}

/** Insurance contribution parameters */
export interface ContributeInsuranceParams {
  /** Amount to contribute */
  amount: BN;
}

/** Insurance withdrawal parameters */
export interface InitiateWithdrawalParams {
  /** Amount to withdraw */
  amount: BN;
}

// ============================================================================
// RESPONSE TYPES
// ============================================================================

/** Reserve response */
export interface ReserveResponse {
  /** Hold ID */
  holdId: BN;
  /** Reserved quantity */
  reservedQty: BN;
  /** Reserved price */
  reservedPrice: BN;
  /** Expiry timestamp */
  expiryTs: BN;
}

/** Commit response */
export interface CommitResponse {
  /** Filled quantity */
  filledQty: BN;
  /** Average fill price */
  avgPrice: BN;
  /** Fees paid */
  fees: BN;
  /** New position size */
  newPositionSize: BN;
}

/** Liquidation response */
export interface LiquidationResponse {
  /** Liquidated quantity */
  liquidatedQty: BN;
  /** Liquidation price */
  liquidationPrice: BN;
  /** Insurance payout */
  insurancePayout: BN;
  /** Socialized loss (if any) */
  socializedLoss: BN;
}

/** Portfolio margin result */
export interface PortfolioMarginResult {
  /** Gross initial margin */
  grossIm: BN;
  /** Net initial margin (after netting) */
  netIm: BN;
  /** Gross maintenance margin */
  grossMm: BN;
  /** Net maintenance margin */
  netMm: BN;
  /** Netting benefit */
  nettingBenefit: BN;
  /** Available margin */
  availableMargin: BN;
  /** Margin ratio (bps) */
  marginRatioBps: number;
}
