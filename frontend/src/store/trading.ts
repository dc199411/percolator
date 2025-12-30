import { create } from 'zustand';
import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';

// Types
export interface Market {
  symbol: string;
  name: string;
  baseAsset: string;
  quoteAsset: string;
  slabAddress: string;
  instrumentIndex: number;
  lastPrice: number;
  change24h: number;
  high24h: number;
  low24h: number;
  volume24h: number;
  markPrice: number;
  indexPrice: number;
  fundingRate: number;
  nextFunding: number;
  openInterest: number;
}

export interface OrderBookLevel {
  price: number;
  size: number;
  total: number;
}

export interface Position {
  id: string;
  market: string;
  side: 'long' | 'short';
  size: number;
  entryPrice: number;
  markPrice: number;
  liquidationPrice: number;
  pnl: number;
  pnlPercent: number;
  margin: number;
  leverage: number;
  slabIndex: number;
  instrumentIndex: number;
}

export interface Order {
  id: string;
  market: string;
  side: 'buy' | 'sell';
  type: 'limit' | 'market';
  price: number;
  size: number;
  filled: number;
  status: 'open' | 'partial' | 'filled' | 'cancelled';
  timestamp: number;
}

export interface Trade {
  id: string;
  market: string;
  side: 'buy' | 'sell';
  price: number;
  size: number;
  timestamp: number;
}

export interface Portfolio {
  collateralBalance: number;
  unrealizedPnl: number;
  realizedPnl: number;
  initialMargin: number;
  maintenanceMargin: number;
  availableMargin: number;
  marginRatio: number;
  leverage: number;
  totalPositionValue: number;
}

// Store interface
interface TradingStore {
  // Current market
  selectedMarket: Market | null;
  markets: Market[];
  
  // Order book
  bids: OrderBookLevel[];
  asks: OrderBookLevel[];
  spread: number;
  spreadPercent: number;
  
  // Trading data
  positions: Position[];
  openOrders: Order[];
  recentTrades: Trade[];
  
  // Portfolio
  portfolio: Portfolio | null;
  
  // UI state
  orderFormSide: 'buy' | 'sell';
  orderFormType: 'limit' | 'market';
  orderFormPrice: string;
  orderFormSize: string;
  orderFormLeverage: number;
  
  // Loading states
  isLoading: boolean;
  isConnected: boolean;
  
  // Actions
  setSelectedMarket: (market: Market) => void;
  setMarkets: (markets: Market[]) => void;
  setOrderBook: (bids: OrderBookLevel[], asks: OrderBookLevel[]) => void;
  setPositions: (positions: Position[]) => void;
  setOpenOrders: (orders: Order[]) => void;
  setRecentTrades: (trades: Trade[]) => void;
  setPortfolio: (portfolio: Portfolio) => void;
  setOrderFormSide: (side: 'buy' | 'sell') => void;
  setOrderFormType: (type: 'limit' | 'market') => void;
  setOrderFormPrice: (price: string) => void;
  setOrderFormSize: (size: string) => void;
  setOrderFormLeverage: (leverage: number) => void;
  setIsLoading: (loading: boolean) => void;
  setIsConnected: (connected: boolean) => void;
  updateMarketPrice: (symbol: string, price: number) => void;
}

// Mock data generators
const generateMockMarkets = (): Market[] => [
  {
    symbol: 'BTC-PERP',
    name: 'Bitcoin Perpetual',
    baseAsset: 'BTC',
    quoteAsset: 'USD',
    slabAddress: 'Slab1111111111111111111111111111111111111',
    instrumentIndex: 0,
    lastPrice: 97245.50,
    change24h: 2.34,
    high24h: 98500.00,
    low24h: 95000.00,
    volume24h: 245678900,
    markPrice: 97250.00,
    indexPrice: 97248.00,
    fundingRate: 0.0012,
    nextFunding: Date.now() + 3600000,
    openInterest: 125000000,
  },
  {
    symbol: 'ETH-PERP',
    name: 'Ethereum Perpetual',
    baseAsset: 'ETH',
    quoteAsset: 'USD',
    slabAddress: 'Slab2222222222222222222222222222222222222',
    instrumentIndex: 1,
    lastPrice: 3456.78,
    change24h: -1.23,
    high24h: 3550.00,
    low24h: 3400.00,
    volume24h: 89012345,
    markPrice: 3457.00,
    indexPrice: 3456.50,
    fundingRate: 0.0008,
    nextFunding: Date.now() + 3600000,
    openInterest: 45000000,
  },
  {
    symbol: 'SOL-PERP',
    name: 'Solana Perpetual',
    baseAsset: 'SOL',
    quoteAsset: 'USD',
    slabAddress: 'Slab3333333333333333333333333333333333333',
    instrumentIndex: 2,
    lastPrice: 198.45,
    change24h: 5.67,
    high24h: 205.00,
    low24h: 185.00,
    volume24h: 34567890,
    markPrice: 198.50,
    indexPrice: 198.48,
    fundingRate: 0.0025,
    nextFunding: Date.now() + 3600000,
    openInterest: 18000000,
  },
  {
    symbol: 'ARB-PERP',
    name: 'Arbitrum Perpetual',
    baseAsset: 'ARB',
    quoteAsset: 'USD',
    slabAddress: 'Slab4444444444444444444444444444444444444',
    instrumentIndex: 3,
    lastPrice: 1.23,
    change24h: 3.45,
    high24h: 1.30,
    low24h: 1.18,
    volume24h: 12345678,
    markPrice: 1.235,
    indexPrice: 1.232,
    fundingRate: 0.0015,
    nextFunding: Date.now() + 3600000,
    openInterest: 5000000,
  },
];

const generateMockOrderBook = (basePrice: number): { bids: OrderBookLevel[], asks: OrderBookLevel[] } => {
  const bids: OrderBookLevel[] = [];
  const asks: OrderBookLevel[] = [];
  let bidTotal = 0;
  let askTotal = 0;
  
  for (let i = 0; i < 15; i++) {
    const bidSize = Math.random() * 10 + 0.5;
    const askSize = Math.random() * 10 + 0.5;
    bidTotal += bidSize;
    askTotal += askSize;
    
    bids.push({
      price: basePrice * (1 - (i + 1) * 0.0002),
      size: bidSize,
      total: bidTotal,
    });
    
    asks.push({
      price: basePrice * (1 + (i + 1) * 0.0002),
      size: askSize,
      total: askTotal,
    });
  }
  
  return { bids, asks };
};

const generateMockTrades = (symbol: string, basePrice: number): Trade[] => {
  const trades: Trade[] = [];
  const now = Date.now();
  
  for (let i = 0; i < 20; i++) {
    trades.push({
      id: `trade-${i}`,
      market: symbol,
      side: Math.random() > 0.5 ? 'buy' : 'sell',
      price: basePrice * (1 + (Math.random() - 0.5) * 0.001),
      size: Math.random() * 5 + 0.1,
      timestamp: now - i * 5000,
    });
  }
  
  return trades;
};

// Create store
export const useTradingStore = create<TradingStore>((set, get) => ({
  // Initial state
  selectedMarket: null,
  markets: generateMockMarkets(),
  bids: [],
  asks: [],
  spread: 0,
  spreadPercent: 0,
  positions: [],
  openOrders: [],
  recentTrades: [],
  portfolio: null,
  orderFormSide: 'buy',
  orderFormType: 'limit',
  orderFormPrice: '',
  orderFormSize: '',
  orderFormLeverage: 5,
  isLoading: false,
  isConnected: false,
  
  // Actions
  setSelectedMarket: (market) => {
    const { bids, asks } = generateMockOrderBook(market.lastPrice);
    const trades = generateMockTrades(market.symbol, market.lastPrice);
    const spread = asks[0]?.price - bids[0]?.price || 0;
    const spreadPercent = bids[0]?.price ? (spread / bids[0].price) * 100 : 0;
    
    set({
      selectedMarket: market,
      bids,
      asks,
      spread,
      spreadPercent,
      recentTrades: trades,
      orderFormPrice: market.lastPrice.toFixed(2),
    });
  },
  
  setMarkets: (markets) => set({ markets }),
  
  setOrderBook: (bids, asks) => {
    const spread = asks[0]?.price - bids[0]?.price || 0;
    const spreadPercent = bids[0]?.price ? (spread / bids[0].price) * 100 : 0;
    set({ bids, asks, spread, spreadPercent });
  },
  
  setPositions: (positions) => set({ positions }),
  
  setOpenOrders: (orders) => set({ openOrders: orders }),
  
  setRecentTrades: (trades) => set({ recentTrades: trades }),
  
  setPortfolio: (portfolio) => set({ portfolio }),
  
  setOrderFormSide: (side) => set({ orderFormSide: side }),
  
  setOrderFormType: (type) => set({ orderFormType: type }),
  
  setOrderFormPrice: (price) => set({ orderFormPrice: price }),
  
  setOrderFormSize: (size) => set({ orderFormSize: size }),
  
  setOrderFormLeverage: (leverage) => set({ orderFormLeverage: leverage }),
  
  setIsLoading: (loading) => set({ isLoading: loading }),
  
  setIsConnected: (connected) => set({ isConnected: connected }),
  
  updateMarketPrice: (symbol, price) => {
    const { markets, selectedMarket } = get();
    const updatedMarkets = markets.map(m => 
      m.symbol === symbol ? { ...m, lastPrice: price, markPrice: price } : m
    );
    set({ markets: updatedMarkets });
    
    if (selectedMarket?.symbol === symbol) {
      set({ selectedMarket: { ...selectedMarket, lastPrice: price, markPrice: price } });
    }
  },
}));
