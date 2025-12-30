'use client';

import { FC } from 'react';
import clsx from 'clsx';
import { useTradingStore } from '@/store/trading';
import { formatNumber } from '@/lib/utils';

export const RecentTrades: FC = () => {
  const { recentTrades, selectedMarket } = useTradingStore();
  const priceDecimals = selectedMarket && selectedMarket.lastPrice > 1000 ? 2 : 4;

  return (
    <div className="flex flex-col h-full bg-surface rounded-lg border border-border overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-border">
        <h3 className="text-sm font-medium">Recent Trades</h3>
      </div>

      {/* Column Headers */}
      <div className="grid grid-cols-3 px-3 py-1.5 text-xs text-text-muted border-b border-border">
        <span>Price (USD)</span>
        <span className="text-right">Size</span>
        <span className="text-right">Time</span>
      </div>

      {/* Trades List */}
      <div className="flex-1 overflow-y-auto">
        {recentTrades.map((trade, index) => (
          <div
            key={trade.id}
            className="grid grid-cols-3 px-3 py-1 text-xs font-mono hover:bg-surface-hover transition-colors"
          >
            <span className={clsx(
              trade.side === 'buy' ? 'text-success' : 'text-danger'
            )}>
              {formatNumber(trade.price, priceDecimals)}
            </span>
            <span className="text-right text-text-primary">
              {formatNumber(trade.size, 4)}
            </span>
            <span className="text-right text-text-secondary">
              {formatTime(trade.timestamp)}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
};

function formatTime(timestamp: number): string {
  const date = new Date(timestamp);
  return date.toLocaleTimeString('en-US', {
    hour12: false,
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}
