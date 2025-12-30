'use client';

import { FC, useState } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import clsx from 'clsx';
import { useTradingStore } from '@/store/trading';
import { formatNumber, formatPercent } from '@/lib/utils';
import { X, ChevronDown, ChevronUp } from 'lucide-react';
import toast from 'react-hot-toast';

type TabType = 'positions' | 'orders' | 'history';

export const PositionsPanel: FC = () => {
  const [activeTab, setActiveTab] = useState<TabType>('positions');
  const { connected } = useWallet();
  const { positions, openOrders, portfolio } = useTradingStore();

  const tabs: { key: TabType; label: string; count?: number }[] = [
    { key: 'positions', label: 'Positions', count: positions.length },
    { key: 'orders', label: 'Open Orders', count: openOrders.length },
    { key: 'history', label: 'Trade History' },
  ];

  return (
    <div className="flex flex-col h-full bg-surface rounded-lg border border-border overflow-hidden">
      {/* Tabs */}
      <div className="flex border-b border-border">
        {tabs.map((tab) => (
          <button
            key={tab.key}
            onClick={() => setActiveTab(tab.key)}
            className={clsx(
              'px-4 py-2.5 text-sm font-medium transition-colors relative',
              activeTab === tab.key
                ? 'text-text-primary'
                : 'text-text-secondary hover:text-text-primary'
            )}
          >
            {tab.label}
            {tab.count !== undefined && tab.count > 0 && (
              <span className="ml-1.5 px-1.5 py-0.5 text-2xs bg-surface-light rounded">
                {tab.count}
              </span>
            )}
            {activeTab === tab.key && (
              <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-accent-primary" />
            )}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-hidden">
        {activeTab === 'positions' && <PositionsTable />}
        {activeTab === 'orders' && <OrdersTable />}
        {activeTab === 'history' && <TradeHistory />}
      </div>

      {/* Portfolio Summary Footer */}
      {connected && portfolio && (
        <div className="border-t border-border p-3 bg-surface-light">
          <div className="flex items-center justify-between gap-6 text-xs">
            <div>
              <span className="text-text-muted">Total Unrealized P&L</span>
              <span className={clsx(
                'ml-2 font-mono font-medium',
                portfolio.unrealizedPnl >= 0 ? 'text-success' : 'text-danger'
              )}>
                {portfolio.unrealizedPnl >= 0 ? '+' : ''}${formatNumber(portfolio.unrealizedPnl, 2)}
              </span>
            </div>
            <div>
              <span className="text-text-muted">Margin Ratio</span>
              <span className={clsx(
                'ml-2 font-mono font-medium',
                portfolio.marginRatio < 50 ? 'text-success' : 
                portfolio.marginRatio < 80 ? 'text-warning' : 'text-danger'
              )}>
                {formatNumber(portfolio.marginRatio, 2)}%
              </span>
            </div>
            <div>
              <span className="text-text-muted">Available Margin</span>
              <span className="ml-2 font-mono font-medium text-success">
                ${formatNumber(portfolio.availableMargin, 2)}
              </span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

const PositionsTable: FC = () => {
  const { connected } = useWallet();
  const { positions } = useTradingStore();

  if (!connected) {
    return (
      <div className="flex items-center justify-center h-full text-text-secondary text-sm">
        Connect wallet to view positions
      </div>
    );
  }

  if (positions.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-text-secondary text-sm">
        No open positions
      </div>
    );
  }

  return (
    <div className="overflow-x-auto">
      <table className="w-full text-xs">
        <thead>
          <tr className="text-text-muted border-b border-border">
            <th className="px-3 py-2 text-left font-medium">Market</th>
            <th className="px-3 py-2 text-right font-medium">Size</th>
            <th className="px-3 py-2 text-right font-medium">Entry Price</th>
            <th className="px-3 py-2 text-right font-medium">Mark Price</th>
            <th className="px-3 py-2 text-right font-medium">Liq. Price</th>
            <th className="px-3 py-2 text-right font-medium">PnL</th>
            <th className="px-3 py-2 text-center font-medium">Actions</th>
          </tr>
        </thead>
        <tbody>
          {positions.map((position) => (
            <PositionRow key={position.id} position={position} />
          ))}
        </tbody>
      </table>
    </div>
  );
};

const PositionRow: FC<{ position: any }> = ({ position }) => {
  const [isClosing, setIsClosing] = useState(false);

  const handleClosePosition = async () => {
    setIsClosing(true);
    const loadingToast = toast.loading('Closing position...');
    
    try {
      await new Promise(resolve => setTimeout(resolve, 1500));
      toast.success('Position closed!', { id: loadingToast });
    } catch (error) {
      toast.error('Failed to close position', { id: loadingToast });
    } finally {
      setIsClosing(false);
    }
  };

  return (
    <tr className="border-b border-border/50 hover:bg-surface-hover transition-colors">
      <td className="px-3 py-3">
        <div className="flex items-center gap-2">
          <div className={clsx(
            'px-1.5 py-0.5 rounded text-2xs font-medium',
            position.side === 'long' ? 'bg-success/20 text-success' : 'bg-danger/20 text-danger'
          )}>
            {position.side.toUpperCase()}
          </div>
          <span className="font-medium">{position.market}</span>
          <span className="text-text-muted">{position.leverage}x</span>
        </div>
      </td>
      <td className="px-3 py-3 text-right font-mono">
        {formatNumber(position.size, 6)}
      </td>
      <td className="px-3 py-3 text-right font-mono">
        ${formatNumber(position.entryPrice, 2)}
      </td>
      <td className="px-3 py-3 text-right font-mono">
        ${formatNumber(position.markPrice, 2)}
      </td>
      <td className="px-3 py-3 text-right font-mono text-danger">
        ${formatNumber(position.liquidationPrice, 2)}
      </td>
      <td className="px-3 py-3 text-right">
        <div className={clsx(
          'font-mono font-medium',
          position.pnl >= 0 ? 'text-success' : 'text-danger'
        )}>
          {position.pnl >= 0 ? '+' : ''}${formatNumber(position.pnl, 2)}
        </div>
        <div className={clsx(
          'text-2xs',
          position.pnlPercent >= 0 ? 'text-success' : 'text-danger'
        )}>
          {formatPercent(position.pnlPercent)}
        </div>
      </td>
      <td className="px-3 py-3 text-center">
        <button
          onClick={handleClosePosition}
          disabled={isClosing}
          className="px-2 py-1 text-2xs bg-surface-light hover:bg-danger/20 hover:text-danger rounded transition-colors disabled:opacity-50"
        >
          {isClosing ? 'Closing...' : 'Close'}
        </button>
      </td>
    </tr>
  );
};

const OrdersTable: FC = () => {
  const { connected } = useWallet();
  const { openOrders } = useTradingStore();

  if (!connected) {
    return (
      <div className="flex items-center justify-center h-full text-text-secondary text-sm">
        Connect wallet to view orders
      </div>
    );
  }

  if (openOrders.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-text-secondary text-sm">
        No open orders
      </div>
    );
  }

  return (
    <div className="overflow-x-auto">
      <table className="w-full text-xs">
        <thead>
          <tr className="text-text-muted border-b border-border">
            <th className="px-3 py-2 text-left font-medium">Market</th>
            <th className="px-3 py-2 text-right font-medium">Type</th>
            <th className="px-3 py-2 text-right font-medium">Side</th>
            <th className="px-3 py-2 text-right font-medium">Price</th>
            <th className="px-3 py-2 text-right font-medium">Size</th>
            <th className="px-3 py-2 text-right font-medium">Filled</th>
            <th className="px-3 py-2 text-center font-medium">Actions</th>
          </tr>
        </thead>
        <tbody>
          {openOrders.map((order) => (
            <tr key={order.id} className="border-b border-border/50 hover:bg-surface-hover transition-colors">
              <td className="px-3 py-3 font-medium">{order.market}</td>
              <td className="px-3 py-3 text-right capitalize">{order.type}</td>
              <td className="px-3 py-3 text-right">
                <span className={clsx(
                  order.side === 'buy' ? 'text-success' : 'text-danger'
                )}>
                  {order.side.toUpperCase()}
                </span>
              </td>
              <td className="px-3 py-3 text-right font-mono">
                ${formatNumber(order.price, 2)}
              </td>
              <td className="px-3 py-3 text-right font-mono">
                {formatNumber(order.size, 6)}
              </td>
              <td className="px-3 py-3 text-right font-mono">
                {formatNumber(order.filled, 6)}
              </td>
              <td className="px-3 py-3 text-center">
                <button className="p-1 hover:bg-danger/20 hover:text-danger rounded transition-colors">
                  <X className="w-3.5 h-3.5" />
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
};

const TradeHistory: FC = () => {
  const { connected } = useWallet();

  if (!connected) {
    return (
      <div className="flex items-center justify-center h-full text-text-secondary text-sm">
        Connect wallet to view history
      </div>
    );
  }

  return (
    <div className="flex items-center justify-center h-full text-text-secondary text-sm">
      No trade history
    </div>
  );
};
