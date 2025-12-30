'use client';

import { FC, useMemo } from 'react';
import clsx from 'clsx';
import { useTradingStore } from '@/store/trading';
import { formatNumber } from '@/lib/utils';

interface OrderBookProps {
  maxRows?: number;
}

export const OrderBook: FC<OrderBookProps> = ({ maxRows = 12 }) => {
  const { bids, asks, spread, spreadPercent, selectedMarket, setOrderFormPrice } = useTradingStore();

  const maxTotal = useMemo(() => {
    const maxBidTotal = bids[bids.length - 1]?.total || 0;
    const maxAskTotal = asks[asks.length - 1]?.total || 0;
    return Math.max(maxBidTotal, maxAskTotal);
  }, [bids, asks]);

  const displayedAsks = asks.slice(0, maxRows).reverse();
  const displayedBids = bids.slice(0, maxRows);

  const priceDecimals = selectedMarket && selectedMarket.lastPrice > 1000 ? 2 : 4;

  return (
    <div className="flex flex-col h-full bg-surface rounded-lg border border-border overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-border">
        <h3 className="text-sm font-medium">Order Book</h3>
        <div className="flex items-center gap-2">
          <button className="px-2 py-1 text-xs bg-surface-light rounded text-text-secondary hover:text-text-primary transition-colors">
            0.01
          </button>
          <button className="px-2 py-1 text-xs bg-surface-light rounded text-text-secondary hover:text-text-primary transition-colors">
            0.1
          </button>
          <button className="px-2 py-1 text-xs bg-accent-primary/20 text-accent-primary rounded">
            1
          </button>
        </div>
      </div>

      {/* Column Headers */}
      <div className="grid grid-cols-3 px-3 py-1.5 text-xs text-text-muted border-b border-border">
        <span>Price (USD)</span>
        <span className="text-right">Size</span>
        <span className="text-right">Total</span>
      </div>

      {/* Asks (Sells) */}
      <div className="flex-1 overflow-hidden">
        <div className="h-1/2 flex flex-col justify-end overflow-hidden">
          {displayedAsks.map((ask, index) => (
            <OrderBookRow
              key={`ask-${index}`}
              price={ask.price}
              size={ask.size}
              total={ask.total}
              maxTotal={maxTotal}
              side="sell"
              priceDecimals={priceDecimals}
              onClick={() => setOrderFormPrice(ask.price.toFixed(priceDecimals))}
            />
          ))}
        </div>

        {/* Spread */}
        <div className="flex items-center justify-between px-3 py-2 bg-surface-light border-y border-border">
          <span className="text-xs text-text-muted">Spread</span>
          <div className="flex items-center gap-2 text-xs">
            <span className="font-mono">${formatNumber(spread, priceDecimals)}</span>
            <span className="text-text-muted">({spreadPercent.toFixed(3)}%)</span>
          </div>
        </div>

        {/* Bids (Buys) */}
        <div className="h-1/2 overflow-hidden">
          {displayedBids.map((bid, index) => (
            <OrderBookRow
              key={`bid-${index}`}
              price={bid.price}
              size={bid.size}
              total={bid.total}
              maxTotal={maxTotal}
              side="buy"
              priceDecimals={priceDecimals}
              onClick={() => setOrderFormPrice(bid.price.toFixed(priceDecimals))}
            />
          ))}
        </div>
      </div>
    </div>
  );
};

interface OrderBookRowProps {
  price: number;
  size: number;
  total: number;
  maxTotal: number;
  side: 'buy' | 'sell';
  priceDecimals: number;
  onClick: () => void;
}

const OrderBookRow: FC<OrderBookRowProps> = ({
  price,
  size,
  total,
  maxTotal,
  side,
  priceDecimals,
  onClick,
}) => {
  const percentage = (total / maxTotal) * 100;

  return (
    <button
      onClick={onClick}
      className="relative grid grid-cols-3 px-3 py-1 text-xs font-mono hover:bg-surface-hover transition-colors cursor-pointer w-full group"
    >
      {/* Background bar */}
      <div
        className={clsx(
          'absolute inset-y-0 right-0 opacity-20 transition-all',
          side === 'buy' ? 'bg-success' : 'bg-danger'
        )}
        style={{ width: `${percentage}%` }}
      />
      
      {/* Content */}
      <span className={clsx(
        'relative z-10',
        side === 'buy' ? 'text-success' : 'text-danger'
      )}>
        {formatNumber(price, priceDecimals)}
      </span>
      <span className="relative z-10 text-right text-text-primary group-hover:text-text-primary">
        {formatNumber(size, 4)}
      </span>
      <span className="relative z-10 text-right text-text-secondary group-hover:text-text-primary">
        {formatNumber(total, 4)}
      </span>
    </button>
  );
};

// Compact horizontal order book for mobile
export const OrderBookCompact: FC = () => {
  const { bids, asks, selectedMarket } = useTradingStore();
  const priceDecimals = selectedMarket && selectedMarket.lastPrice > 1000 ? 2 : 4;

  const topBid = bids[0];
  const topAsk = asks[0];

  if (!topBid || !topAsk) return null;

  return (
    <div className="flex items-center gap-4 px-4 py-2 bg-surface border-b border-border">
      <div>
        <div className="text-xs text-text-muted mb-0.5">Best Bid</div>
        <div className="font-mono text-sm text-success">
          ${formatNumber(topBid.price, priceDecimals)}
        </div>
      </div>
      <div className="h-8 w-px bg-border" />
      <div>
        <div className="text-xs text-text-muted mb-0.5">Best Ask</div>
        <div className="font-mono text-sm text-danger">
          ${formatNumber(topAsk.price, priceDecimals)}
        </div>
      </div>
    </div>
  );
};
