'use client';

import { useEffect } from 'react';
import { useTradingStore } from '@/store/trading';
import { usePercolator, usePriceSimulation } from '@/hooks/usePercolator';
import {
  TradingHeader,
  MarketInfoBar,
  OrderBook,
  RecentTrades,
  OrderForm,
  PositionsPanel,
  TradingChart,
  AccountPanel,
} from '@/components/trading';

export default function TradePage() {
  const { markets, selectedMarket, setSelectedMarket } = useTradingStore();
  
  // Initialize percolator hooks
  usePercolator();
  usePriceSimulation();

  // Set default market on mount
  useEffect(() => {
    if (!selectedMarket && markets.length > 0) {
      setSelectedMarket(markets[0]);
    }
  }, [markets, selectedMarket, setSelectedMarket]);

  return (
    <div className="h-screen flex flex-col bg-background overflow-hidden">
      {/* Header */}
      <TradingHeader />

      {/* Market Info Bar */}
      <MarketInfoBar />

      {/* Main Content */}
      <div className="flex-1 flex overflow-hidden">
        {/* Desktop Layout */}
        <div className="hidden lg:flex flex-1 p-2 gap-2 overflow-hidden">
          {/* Left Column - Order Book & Trades */}
          <div className="w-72 flex flex-col gap-2 flex-shrink-0">
            <div className="flex-1 min-h-0">
              <OrderBook maxRows={15} />
            </div>
            <div className="h-48 flex-shrink-0">
              <RecentTrades />
            </div>
          </div>

          {/* Center Column - Chart & Positions */}
          <div className="flex-1 flex flex-col gap-2 min-w-0">
            <div className="flex-1 min-h-0">
              <TradingChart />
            </div>
            <div className="h-64 flex-shrink-0">
              <PositionsPanel />
            </div>
          </div>

          {/* Right Column - Order Form & Account */}
          <div className="w-80 flex flex-col gap-2 flex-shrink-0">
            <div className="flex-1 min-h-0">
              <OrderForm />
            </div>
            <div className="flex-shrink-0">
              <AccountPanel />
            </div>
          </div>
        </div>

        {/* Tablet Layout (md-lg) */}
        <div className="hidden md:flex lg:hidden flex-1 flex-col p-2 gap-2 overflow-hidden">
          <div className="flex gap-2 flex-1 min-h-0">
            {/* Left - Chart */}
            <div className="flex-1 min-w-0">
              <TradingChart />
            </div>
            {/* Right - Order Form */}
            <div className="w-72 flex-shrink-0">
              <OrderForm />
            </div>
          </div>
          <div className="flex gap-2 h-64">
            <div className="w-64 flex-shrink-0">
              <OrderBook maxRows={8} />
            </div>
            <div className="flex-1">
              <PositionsPanel />
            </div>
          </div>
        </div>

        {/* Mobile Layout */}
        <div className="flex md:hidden flex-col w-full overflow-hidden">
          <MobileTradingView />
        </div>
      </div>
    </div>
  );
}

// Mobile Trading View with tabs
function MobileTradingView() {
  const [activeTab, setActiveTab] = useState<'chart' | 'order' | 'positions'>('chart');

  return (
    <div className="flex flex-col h-full">
      {/* Tab Navigation */}
      <div className="flex border-b border-border bg-surface">
        {(['chart', 'order', 'positions'] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`flex-1 py-3 text-sm font-medium capitalize transition-colors relative ${
              activeTab === tab
                ? 'text-text-primary'
                : 'text-text-secondary'
            }`}
          >
            {tab === 'chart' ? 'Chart' : tab === 'order' ? 'Trade' : 'Positions'}
            {activeTab === tab && (
              <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-accent-primary" />
            )}
          </button>
        ))}
      </div>

      {/* Tab Content */}
      <div className="flex-1 overflow-hidden">
        {activeTab === 'chart' && (
          <div className="h-full p-2">
            <TradingChart />
          </div>
        )}
        {activeTab === 'order' && (
          <div className="h-full p-2 overflow-y-auto">
            <OrderForm />
            <div className="mt-2">
              <AccountPanel />
            </div>
          </div>
        )}
        {activeTab === 'positions' && (
          <div className="h-full p-2">
            <PositionsPanel />
          </div>
        )}
      </div>
    </div>
  );
}

import { useState } from 'react';
