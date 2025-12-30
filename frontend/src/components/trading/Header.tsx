'use client';

import { FC } from 'react';
import Link from 'next/link';
import { useWallet } from '@solana/wallet-adapter-react';
import { WalletMultiButton } from '@solana/wallet-adapter-react-ui';
import { 
  Home, 
  TrendingUp, 
  Wallet,
  Settings,
  Bell,
  ChevronDown,
  ExternalLink,
} from 'lucide-react';
import { useTradingStore } from '@/store/trading';
import { formatNumber, formatCurrency } from '@/lib/utils';

export const TradingHeader: FC = () => {
  const { connected } = useWallet();
  const { portfolio, selectedMarket } = useTradingStore();

  return (
    <header className="h-14 bg-surface border-b border-border flex items-center justify-between px-4">
      {/* Left Section - Logo & Navigation */}
      <div className="flex items-center gap-6">
        <Link href="/" className="flex items-center gap-2">
          <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-accent-primary to-accent-secondary flex items-center justify-center">
            <span className="text-background font-bold text-sm">P</span>
          </div>
          <span className="font-semibold text-lg hidden sm:block">Percolator</span>
        </Link>

        <nav className="hidden md:flex items-center gap-1">
          <Link 
            href="/trade" 
            className="px-3 py-1.5 rounded-lg bg-surface-light text-text-primary text-sm font-medium"
          >
            Trade
          </Link>
          <Link 
            href="/trade" 
            className="px-3 py-1.5 rounded-lg text-text-secondary hover:text-text-primary hover:bg-surface-light text-sm font-medium transition-colors"
          >
            Portfolio
          </Link>
          <Link 
            href="/trade" 
            className="px-3 py-1.5 rounded-lg text-text-secondary hover:text-text-primary hover:bg-surface-light text-sm font-medium transition-colors"
          >
            Earn
          </Link>
        </nav>
      </div>

      {/* Center Section - Market Info */}
      {selectedMarket && (
        <div className="hidden lg:flex items-center gap-6">
          <div className="flex items-center gap-2">
            <span className="text-sm text-text-secondary">Mark</span>
            <span className="text-sm font-mono font-medium">
              ${formatNumber(selectedMarket.markPrice, 2)}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-sm text-text-secondary">Index</span>
            <span className="text-sm font-mono font-medium">
              ${formatNumber(selectedMarket.indexPrice, 2)}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-sm text-text-secondary">Funding</span>
            <span className={`text-sm font-mono font-medium ${
              selectedMarket.fundingRate >= 0 ? 'text-success' : 'text-danger'
            }`}>
              {selectedMarket.fundingRate >= 0 ? '+' : ''}{(selectedMarket.fundingRate * 100).toFixed(4)}%
            </span>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-sm text-text-secondary">OI</span>
            <span className="text-sm font-mono font-medium">
              ${formatNumber(selectedMarket.openInterest / 1_000_000, 1)}M
            </span>
          </div>
        </div>
      )}

      {/* Right Section - Account & Wallet */}
      <div className="flex items-center gap-3">
        {connected && portfolio && (
          <div className="hidden sm:flex items-center gap-4 mr-2">
            <div className="text-right">
              <div className="text-xs text-text-secondary">Equity</div>
              <div className="text-sm font-mono font-medium">
                ${formatNumber(portfolio.collateralBalance + portfolio.unrealizedPnl, 2)}
              </div>
            </div>
            <div className="text-right">
              <div className="text-xs text-text-secondary">Available</div>
              <div className="text-sm font-mono font-medium text-success">
                ${formatNumber(portfolio.availableMargin, 2)}
              </div>
            </div>
          </div>
        )}

        <button className="p-2 rounded-lg hover:bg-surface-light text-text-secondary hover:text-text-primary transition-colors">
          <Bell className="w-5 h-5" />
        </button>

        <button className="p-2 rounded-lg hover:bg-surface-light text-text-secondary hover:text-text-primary transition-colors">
          <Settings className="w-5 h-5" />
        </button>

        <WalletMultiButton className="!h-9 !px-4 !rounded-lg !text-sm !font-medium" />
      </div>
    </header>
  );
};
