'use client';

import { FC, useState } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import { motion, AnimatePresence } from 'framer-motion';
import clsx from 'clsx';
import { useTradingStore } from '@/store/trading';
import { usePercolator } from '@/hooks/usePercolator';
import { formatNumber, shortenAddress } from '@/lib/utils';
import { 
  Wallet, 
  ArrowDownToLine, 
  ArrowUpFromLine, 
  RefreshCw,
  Copy,
  ExternalLink,
  Check,
  AlertTriangle,
} from 'lucide-react';
import toast from 'react-hot-toast';

export const AccountPanel: FC = () => {
  const { connected, publicKey } = useWallet();
  const { portfolio } = useTradingStore();
  const { deposit, withdraw, isLoading, fetchPortfolio } = usePercolator();
  
  const [activeTab, setActiveTab] = useState<'overview' | 'deposit' | 'withdraw'>('overview');
  const [amount, setAmount] = useState('');
  const [copied, setCopied] = useState(false);

  const handleCopyAddress = () => {
    if (publicKey) {
      navigator.clipboard.writeText(publicKey.toBase58());
      setCopied(true);
      toast.success('Address copied!');
      setTimeout(() => setCopied(false), 2000);
    }
  };

  const handleDeposit = async () => {
    const value = parseFloat(amount);
    if (value > 0) {
      await deposit(value);
      setAmount('');
      setActiveTab('overview');
    }
  };

  const handleWithdraw = async () => {
    const value = parseFloat(amount);
    if (value > 0) {
      await withdraw(value);
      setAmount('');
      setActiveTab('overview');
    }
  };

  if (!connected) {
    return (
      <div className="bg-surface rounded-lg border border-border p-4">
        <div className="flex items-center gap-2 text-text-secondary text-sm">
          <Wallet className="w-4 h-4" />
          <span>Connect wallet to view account</span>
        </div>
      </div>
    );
  }

  return (
    <div className="bg-surface rounded-lg border border-border overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-border">
        <div className="flex items-center gap-2">
          <div className="w-8 h-8 rounded-full bg-gradient-to-br from-accent-primary to-accent-secondary flex items-center justify-center">
            <Wallet className="w-4 h-4 text-background" />
          </div>
          <div>
            <div className="text-sm font-medium">Account</div>
            <button 
              onClick={handleCopyAddress}
              className="flex items-center gap-1 text-xs text-text-secondary hover:text-text-primary transition-colors"
            >
              {shortenAddress(publicKey?.toBase58() || '', 4)}
              {copied ? (
                <Check className="w-3 h-3 text-success" />
              ) : (
                <Copy className="w-3 h-3" />
              )}
            </button>
          </div>
        </div>
        <button 
          onClick={fetchPortfolio}
          className="p-1.5 hover:bg-surface-light rounded-lg transition-colors"
        >
          <RefreshCw className={clsx('w-4 h-4 text-text-secondary', isLoading && 'animate-spin')} />
        </button>
      </div>

      {/* Tabs */}
      <div className="flex border-b border-border">
        {(['overview', 'deposit', 'withdraw'] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={clsx(
              'flex-1 px-4 py-2 text-xs font-medium capitalize transition-colors relative',
              activeTab === tab
                ? 'text-text-primary'
                : 'text-text-secondary hover:text-text-primary'
            )}
          >
            {tab}
            {activeTab === tab && (
              <motion.div 
                layoutId="account-tab"
                className="absolute bottom-0 left-0 right-0 h-0.5 bg-accent-primary" 
              />
            )}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="p-4">
        <AnimatePresence mode="wait">
          {activeTab === 'overview' && (
            <motion.div
              key="overview"
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -10 }}
              className="space-y-4"
            >
              {/* Equity */}
              <div className="p-3 bg-surface-light rounded-lg">
                <div className="text-xs text-text-muted mb-1">Total Equity</div>
                <div className="text-xl font-bold font-mono">
                  ${formatNumber((portfolio?.collateralBalance || 0) + (portfolio?.unrealizedPnl || 0), 2)}
                </div>
              </div>

              {/* Stats Grid */}
              <div className="grid grid-cols-2 gap-3">
                <div className="p-3 bg-surface-light rounded-lg">
                  <div className="text-xs text-text-muted mb-1">Collateral</div>
                  <div className="font-mono font-medium">
                    ${formatNumber(portfolio?.collateralBalance || 0, 2)}
                  </div>
                </div>
                <div className="p-3 bg-surface-light rounded-lg">
                  <div className="text-xs text-text-muted mb-1">Available</div>
                  <div className="font-mono font-medium text-success">
                    ${formatNumber(portfolio?.availableMargin || 0, 2)}
                  </div>
                </div>
                <div className="p-3 bg-surface-light rounded-lg">
                  <div className="text-xs text-text-muted mb-1">Unrealized P&L</div>
                  <div className={clsx(
                    'font-mono font-medium',
                    (portfolio?.unrealizedPnl || 0) >= 0 ? 'text-success' : 'text-danger'
                  )}>
                    {(portfolio?.unrealizedPnl || 0) >= 0 ? '+' : ''}
                    ${formatNumber(portfolio?.unrealizedPnl || 0, 2)}
                  </div>
                </div>
                <div className="p-3 bg-surface-light rounded-lg">
                  <div className="text-xs text-text-muted mb-1">Realized P&L</div>
                  <div className={clsx(
                    'font-mono font-medium',
                    (portfolio?.realizedPnl || 0) >= 0 ? 'text-success' : 'text-danger'
                  )}>
                    {(portfolio?.realizedPnl || 0) >= 0 ? '+' : ''}
                    ${formatNumber(portfolio?.realizedPnl || 0, 2)}
                  </div>
                </div>
              </div>

              {/* Margin Health */}
              <div className="p-3 bg-surface-light rounded-lg">
                <div className="flex items-center justify-between mb-2">
                  <div className="text-xs text-text-muted">Margin Usage</div>
                  <div className={clsx(
                    'text-xs font-mono font-medium',
                    (portfolio?.marginRatio || 0) < 50 ? 'text-success' :
                    (portfolio?.marginRatio || 0) < 80 ? 'text-warning' : 'text-danger'
                  )}>
                    {formatNumber(portfolio?.marginRatio || 0, 1)}%
                  </div>
                </div>
                <div className="h-2 bg-background rounded-full overflow-hidden">
                  <div 
                    className={clsx(
                      'h-full transition-all duration-300',
                      (portfolio?.marginRatio || 0) < 50 ? 'bg-success' :
                      (portfolio?.marginRatio || 0) < 80 ? 'bg-warning' : 'bg-danger'
                    )}
                    style={{ width: `${Math.min(portfolio?.marginRatio || 0, 100)}%` }}
                  />
                </div>
                {(portfolio?.marginRatio || 0) > 80 && (
                  <div className="flex items-center gap-1 mt-2 text-2xs text-danger">
                    <AlertTriangle className="w-3 h-3" />
                    High margin usage - reduce positions or add collateral
                  </div>
                )}
              </div>
            </motion.div>
          )}

          {(activeTab === 'deposit' || activeTab === 'withdraw') && (
            <motion.div
              key={activeTab}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -10 }}
              className="space-y-4"
            >
              <div>
                <label className="text-xs text-text-secondary mb-2 block">
                  Amount (USDC)
                </label>
                <div className="relative">
                  <input
                    type="number"
                    value={amount}
                    onChange={(e) => setAmount(e.target.value)}
                    placeholder="0.00"
                    className="w-full px-4 py-3 bg-surface-light rounded-lg text-right font-mono text-lg focus:outline-none focus:ring-1 focus:ring-accent-primary"
                  />
                  <span className="absolute left-4 top-1/2 -translate-y-1/2 text-sm text-text-muted">
                    USDC
                  </span>
                </div>
              </div>

              {/* Quick amount buttons */}
              <div className="grid grid-cols-4 gap-2">
                {[100, 500, 1000, 5000].map((value) => (
                  <button
                    key={value}
                    onClick={() => setAmount(value.toString())}
                    className="px-2 py-2 text-xs bg-surface-light rounded-lg text-text-secondary hover:text-text-primary hover:bg-surface-hover transition-colors"
                  >
                    ${value}
                  </button>
                ))}
              </div>

              {activeTab === 'withdraw' && (
                <button
                  onClick={() => setAmount(portfolio?.availableMargin?.toString() || '0')}
                  className="w-full px-4 py-2 text-xs bg-surface-light rounded-lg text-text-secondary hover:text-text-primary transition-colors"
                >
                  Max: ${formatNumber(portfolio?.availableMargin || 0, 2)}
                </button>
              )}

              <button
                onClick={activeTab === 'deposit' ? handleDeposit : handleWithdraw}
                disabled={isLoading || !amount || parseFloat(amount) <= 0}
                className={clsx(
                  'w-full py-3 rounded-lg font-semibold transition-all flex items-center justify-center gap-2',
                  'disabled:opacity-50 disabled:cursor-not-allowed',
                  activeTab === 'deposit'
                    ? 'bg-success hover:bg-success/90 text-background'
                    : 'bg-danger hover:bg-danger/90 text-white'
                )}
              >
                {isLoading ? (
                  <RefreshCw className="w-4 h-4 animate-spin" />
                ) : activeTab === 'deposit' ? (
                  <>
                    <ArrowDownToLine className="w-4 h-4" />
                    Deposit
                  </>
                ) : (
                  <>
                    <ArrowUpFromLine className="w-4 h-4" />
                    Withdraw
                  </>
                )}
              </button>
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </div>
  );
};
