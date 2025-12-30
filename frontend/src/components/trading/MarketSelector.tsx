'use client';

import { FC, useState, useRef, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { ChevronDown, Search, Star, TrendingUp, TrendingDown } from 'lucide-react';
import { useTradingStore, Market } from '@/store/trading';
import { formatNumber, formatCurrency, formatPercent } from '@/lib/utils';
import clsx from 'clsx';

export const MarketSelector: FC = () => {
  const [isOpen, setIsOpen] = useState(false);
  const [search, setSearch] = useState('');
  const dropdownRef = useRef<HTMLDivElement>(null);
  
  const { selectedMarket, markets, setSelectedMarket } = useTradingStore();

  // Close dropdown when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const filteredMarkets = markets.filter(market =>
    market.symbol.toLowerCase().includes(search.toLowerCase()) ||
    market.name.toLowerCase().includes(search.toLowerCase())
  );

  const handleSelectMarket = (market: Market) => {
    setSelectedMarket(market);
    setIsOpen(false);
    setSearch('');
  };

  if (!selectedMarket) return null;

  return (
    <div className="relative" ref={dropdownRef}>
      {/* Selected Market Button */}
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="flex items-center gap-3 px-4 py-3 hover:bg-surface-hover rounded-lg transition-colors"
      >
        <div className="flex items-center gap-2">
          <div className="w-8 h-8 rounded-full bg-gradient-to-br from-accent-primary/20 to-accent-secondary/20 flex items-center justify-center">
            <span className="text-xs font-bold text-accent-primary">
              {selectedMarket.baseAsset.slice(0, 2)}
            </span>
          </div>
          <div>
            <div className="flex items-center gap-2">
              <span className="font-semibold">{selectedMarket.symbol}</span>
              <span className="text-xs px-1.5 py-0.5 rounded bg-surface-light text-text-secondary">
                20x
              </span>
            </div>
          </div>
        </div>
        <ChevronDown className={clsx(
          'w-4 h-4 text-text-secondary transition-transform',
          isOpen && 'rotate-180'
        )} />
      </button>

      {/* Dropdown */}
      <AnimatePresence>
        {isOpen && (
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -10 }}
            transition={{ duration: 0.15 }}
            className="absolute top-full left-0 mt-1 w-96 bg-surface border border-border rounded-xl shadow-xl z-50 overflow-hidden"
          >
            {/* Search */}
            <div className="p-3 border-b border-border">
              <div className="relative">
                <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 w-4 h-4 text-text-muted" />
                <input
                  type="text"
                  placeholder="Search markets..."
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                  className="w-full pl-10 pr-4 py-2 bg-surface-light rounded-lg text-sm focus:outline-none focus:ring-1 focus:ring-accent-primary"
                />
              </div>
            </div>

            {/* Market List */}
            <div className="max-h-80 overflow-y-auto">
              {filteredMarkets.map((market) => (
                <button
                  key={market.symbol}
                  onClick={() => handleSelectMarket(market)}
                  className={clsx(
                    'w-full flex items-center justify-between px-4 py-3 hover:bg-surface-hover transition-colors',
                    market.symbol === selectedMarket.symbol && 'bg-surface-light'
                  )}
                >
                  <div className="flex items-center gap-3">
                    <button 
                      onClick={(e) => e.stopPropagation()}
                      className="text-text-muted hover:text-warning"
                    >
                      <Star className="w-4 h-4" />
                    </button>
                    <div className="w-8 h-8 rounded-full bg-gradient-to-br from-accent-primary/20 to-accent-secondary/20 flex items-center justify-center">
                      <span className="text-xs font-bold text-accent-primary">
                        {market.baseAsset.slice(0, 2)}
                      </span>
                    </div>
                    <div className="text-left">
                      <div className="font-medium">{market.symbol}</div>
                      <div className="text-xs text-text-secondary">{market.name}</div>
                    </div>
                  </div>
                  <div className="text-right">
                    <div className="font-mono font-medium">
                      ${formatNumber(market.lastPrice, market.lastPrice > 1000 ? 2 : 4)}
                    </div>
                    <div className={clsx(
                      'flex items-center justify-end gap-1 text-xs',
                      market.change24h >= 0 ? 'text-success' : 'text-danger'
                    )}>
                      {market.change24h >= 0 ? (
                        <TrendingUp className="w-3 h-3" />
                      ) : (
                        <TrendingDown className="w-3 h-3" />
                      )}
                      {market.change24h >= 0 ? '+' : ''}{market.change24h.toFixed(2)}%
                    </div>
                  </div>
                </button>
              ))}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
};

// Compact Market Info Bar
export const MarketInfoBar: FC = () => {
  const { selectedMarket } = useTradingStore();

  if (!selectedMarket) return null;

  return (
    <div className="flex items-center gap-6 px-4 py-2 bg-surface border-b border-border overflow-x-auto">
      <MarketSelector />
      
      <div className="flex items-center gap-6 text-sm">
        {/* Last Price */}
        <div>
          <div className={clsx(
            'font-mono text-lg font-bold',
            selectedMarket.change24h >= 0 ? 'text-success' : 'text-danger'
          )}>
            ${formatNumber(selectedMarket.lastPrice, selectedMarket.lastPrice > 1000 ? 2 : 4)}
          </div>
        </div>

        {/* 24h Change */}
        <div>
          <div className="text-xs text-text-muted mb-0.5">24h Change</div>
          <div className={clsx(
            'font-mono font-medium',
            selectedMarket.change24h >= 0 ? 'text-success' : 'text-danger'
          )}>
            {selectedMarket.change24h >= 0 ? '+' : ''}{selectedMarket.change24h.toFixed(2)}%
          </div>
        </div>

        {/* 24h High */}
        <div>
          <div className="text-xs text-text-muted mb-0.5">24h High</div>
          <div className="font-mono font-medium">
            ${formatNumber(selectedMarket.high24h, 2)}
          </div>
        </div>

        {/* 24h Low */}
        <div>
          <div className="text-xs text-text-muted mb-0.5">24h Low</div>
          <div className="font-mono font-medium">
            ${formatNumber(selectedMarket.low24h, 2)}
          </div>
        </div>

        {/* 24h Volume */}
        <div>
          <div className="text-xs text-text-muted mb-0.5">24h Volume</div>
          <div className="font-mono font-medium">
            ${formatNumber(selectedMarket.volume24h / 1_000_000, 2)}M
          </div>
        </div>

        {/* Open Interest */}
        <div>
          <div className="text-xs text-text-muted mb-0.5">Open Interest</div>
          <div className="font-mono font-medium">
            ${formatNumber(selectedMarket.openInterest / 1_000_000, 2)}M
          </div>
        </div>

        {/* Funding Rate */}
        <div>
          <div className="text-xs text-text-muted mb-0.5">Funding / Countdown</div>
          <div className="flex items-center gap-2">
            <span className={clsx(
              'font-mono font-medium',
              selectedMarket.fundingRate >= 0 ? 'text-success' : 'text-danger'
            )}>
              {selectedMarket.fundingRate >= 0 ? '+' : ''}{(selectedMarket.fundingRate * 100).toFixed(4)}%
            </span>
            <span className="text-text-muted">|</span>
            <FundingCountdown nextFunding={selectedMarket.nextFunding} />
          </div>
        </div>
      </div>
    </div>
  );
};

// Funding countdown component
const FundingCountdown: FC<{ nextFunding: number }> = ({ nextFunding }) => {
  const [timeLeft, setTimeLeft] = useState('');

  useEffect(() => {
    const updateCountdown = () => {
      const now = Date.now();
      const diff = nextFunding - now;
      
      if (diff <= 0) {
        setTimeLeft('00:00:00');
        return;
      }

      const hours = Math.floor(diff / (1000 * 60 * 60));
      const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60));
      const seconds = Math.floor((diff % (1000 * 60)) / 1000);

      setTimeLeft(
        `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`
      );
    };

    updateCountdown();
    const interval = setInterval(updateCountdown, 1000);
    return () => clearInterval(interval);
  }, [nextFunding]);

  return <span className="font-mono text-text-secondary">{timeLeft}</span>;
};
