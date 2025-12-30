'use client';

import { FC, useState, useCallback, useMemo } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import { WalletMultiButton } from '@solana/wallet-adapter-react-ui';
import clsx from 'clsx';
import toast from 'react-hot-toast';
import { useTradingStore } from '@/store/trading';
import { usePercolator } from '@/hooks/usePercolator';
import { formatNumber } from '@/lib/utils';
import { Info, Minus, Plus } from 'lucide-react';

export const OrderForm: FC = () => {
  const { connected, publicKey } = useWallet();
  const {
    selectedMarket,
    portfolio,
    orderFormSide,
    orderFormType,
    orderFormPrice,
    orderFormSize,
    orderFormLeverage,
    setOrderFormSide,
    setOrderFormType,
    setOrderFormPrice,
    setOrderFormSize,
    setOrderFormLeverage,
  } = useTradingStore();

  const { hasPortfolio, initializePortfolio, isLoading } = usePercolator();
  const [isSubmitting, setIsSubmitting] = useState(false);

  // Calculate order details
  const orderDetails = useMemo(() => {
    const price = parseFloat(orderFormPrice) || 0;
    const size = parseFloat(orderFormSize) || 0;
    const notional = price * size;
    const margin = notional / orderFormLeverage;
    const fee = notional * 0.0005; // 0.05% taker fee
    
    return {
      price,
      size,
      notional,
      margin,
      fee,
      total: margin + fee,
    };
  }, [orderFormPrice, orderFormSize, orderFormLeverage]);

  // Size presets
  const sizePresets = [25, 50, 75, 100];

  const handleSizePreset = (percent: number) => {
    if (!portfolio || !selectedMarket) return;
    const maxNotional = portfolio.availableMargin * orderFormLeverage;
    const maxSize = maxNotional / selectedMarket.lastPrice;
    setOrderFormSize((maxSize * (percent / 100)).toFixed(6));
  };

  const handleSubmitOrder = async () => {
    if (!connected || !selectedMarket) {
      toast.error('Please connect your wallet');
      return;
    }

    if (!orderDetails.size || !orderDetails.price) {
      toast.error('Please enter valid price and size');
      return;
    }

    if (orderDetails.margin > (portfolio?.availableMargin || 0)) {
      toast.error('Insufficient margin');
      return;
    }

    setIsSubmitting(true);
    const loadingToast = toast.loading(
      `Placing ${orderFormSide.toUpperCase()} ${orderFormType} order...`
    );

    try {
      // Simulate order placement
      await new Promise(resolve => setTimeout(resolve, 1500));
      
      toast.success(
        `${orderFormSide.toUpperCase()} ${orderDetails.size} ${selectedMarket.baseAsset} @ $${formatNumber(orderDetails.price, 2)}`,
        { id: loadingToast }
      );

      // Reset form
      setOrderFormSize('');
    } catch (error: any) {
      toast.error(error.message || 'Failed to place order', { id: loadingToast });
    } finally {
      setIsSubmitting(false);
    }
  };

  if (!connected) {
    return (
      <div className="flex flex-col h-full bg-surface rounded-lg border border-border overflow-hidden">
        <div className="flex items-center justify-between px-3 py-2 border-b border-border">
          <h3 className="text-sm font-medium">Place Order</h3>
        </div>
        <div className="flex-1 flex flex-col items-center justify-center p-6">
          <p className="text-text-secondary text-sm mb-4 text-center">
            Connect your wallet to start trading
          </p>
          <WalletMultiButton className="!rounded-lg" />
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full bg-surface rounded-lg border border-border overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-border">
        <h3 className="text-sm font-medium">Place Order</h3>
      </div>

      {/* Side Toggle */}
      <div className="grid grid-cols-2 m-3 bg-surface-light rounded-lg p-1">
        <button
          onClick={() => setOrderFormSide('buy')}
          className={clsx(
            'py-2 text-sm font-medium rounded-md transition-all',
            orderFormSide === 'buy'
              ? 'bg-success text-background shadow-sm'
              : 'text-text-secondary hover:text-text-primary'
          )}
        >
          Buy / Long
        </button>
        <button
          onClick={() => setOrderFormSide('sell')}
          className={clsx(
            'py-2 text-sm font-medium rounded-md transition-all',
            orderFormSide === 'sell'
              ? 'bg-danger text-white shadow-sm'
              : 'text-text-secondary hover:text-text-primary'
          )}
        >
          Sell / Short
        </button>
      </div>

      {/* Order Type Toggle */}
      <div className="flex gap-2 px-3">
        {(['limit', 'market'] as const).map((type) => (
          <button
            key={type}
            onClick={() => setOrderFormType(type)}
            className={clsx(
              'px-3 py-1.5 text-xs font-medium rounded-lg transition-colors',
              orderFormType === type
                ? 'bg-surface-light text-text-primary'
                : 'text-text-secondary hover:text-text-primary'
            )}
          >
            {type.charAt(0).toUpperCase() + type.slice(1)}
          </button>
        ))}
      </div>

      {/* Form Fields */}
      <div className="flex-1 p-3 space-y-3 overflow-y-auto">
        {/* Price Input */}
        {orderFormType === 'limit' && (
          <div>
            <label className="text-xs text-text-secondary mb-1 block">
              Price (USD)
            </label>
            <div className="relative">
              <input
                type="number"
                value={orderFormPrice}
                onChange={(e) => setOrderFormPrice(e.target.value)}
                placeholder="0.00"
                className="w-full px-3 py-2 bg-surface-light rounded-lg text-right font-mono focus:outline-none focus:ring-1 focus:ring-accent-primary"
              />
              <button
                onClick={() => selectedMarket && setOrderFormPrice(selectedMarket.lastPrice.toFixed(2))}
                className="absolute left-2 top-1/2 -translate-y-1/2 text-xs text-accent-primary hover:text-accent-hover"
              >
                Last
              </button>
            </div>
          </div>
        )}

        {/* Size Input */}
        <div>
          <label className="text-xs text-text-secondary mb-1 block">
            Size ({selectedMarket?.baseAsset || 'BTC'})
          </label>
          <div className="relative">
            <input
              type="number"
              value={orderFormSize}
              onChange={(e) => setOrderFormSize(e.target.value)}
              placeholder="0.00"
              className="w-full px-3 py-2 bg-surface-light rounded-lg text-right font-mono focus:outline-none focus:ring-1 focus:ring-accent-primary"
            />
            <span className="absolute left-3 top-1/2 -translate-y-1/2 text-xs text-text-muted">
              {selectedMarket?.baseAsset || 'BTC'}
            </span>
          </div>
        </div>

        {/* Size Presets */}
        <div className="grid grid-cols-4 gap-2">
          {sizePresets.map((preset) => (
            <button
              key={preset}
              onClick={() => handleSizePreset(preset)}
              className="px-2 py-1.5 text-xs bg-surface-light rounded-lg text-text-secondary hover:text-text-primary hover:bg-surface-hover transition-colors"
            >
              {preset}%
            </button>
          ))}
        </div>

        {/* Leverage Slider */}
        <div>
          <div className="flex items-center justify-between mb-2">
            <label className="text-xs text-text-secondary">Leverage</label>
            <span className="text-xs font-mono text-accent-primary">
              {orderFormLeverage}x
            </span>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={() => setOrderFormLeverage(Math.max(1, orderFormLeverage - 1))}
              className="p-1.5 bg-surface-light rounded-lg hover:bg-surface-hover transition-colors"
            >
              <Minus className="w-3 h-3" />
            </button>
            <input
              type="range"
              min="1"
              max="20"
              value={orderFormLeverage}
              onChange={(e) => setOrderFormLeverage(parseInt(e.target.value))}
              className="flex-1 h-1 bg-surface-light rounded-lg appearance-none cursor-pointer accent-accent-primary"
            />
            <button
              onClick={() => setOrderFormLeverage(Math.min(20, orderFormLeverage + 1))}
              className="p-1.5 bg-surface-light rounded-lg hover:bg-surface-hover transition-colors"
            >
              <Plus className="w-3 h-3" />
            </button>
          </div>
          <div className="flex justify-between mt-1 text-2xs text-text-muted">
            <span>1x</span>
            <span>5x</span>
            <span>10x</span>
            <span>15x</span>
            <span>20x</span>
          </div>
        </div>

        {/* Order Summary */}
        {orderDetails.size > 0 && (
          <div className="p-3 bg-surface-light rounded-lg space-y-2 text-xs">
            <div className="flex justify-between">
              <span className="text-text-secondary">Notional Value</span>
              <span className="font-mono">${formatNumber(orderDetails.notional, 2)}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-text-secondary">Required Margin</span>
              <span className="font-mono">${formatNumber(orderDetails.margin, 2)}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-text-secondary">Est. Fee (0.05%)</span>
              <span className="font-mono">${formatNumber(orderDetails.fee, 2)}</span>
            </div>
            <div className="border-t border-border pt-2 flex justify-between font-medium">
              <span className="text-text-secondary">Total Cost</span>
              <span className="font-mono">${formatNumber(orderDetails.total, 2)}</span>
            </div>
          </div>
        )}
      </div>

      {/* Submit Button */}
      <div className="p-3 border-t border-border">
        {!hasPortfolio ? (
          <button
            onClick={initializePortfolio}
            disabled={isLoading}
            className="w-full py-3 bg-gradient-to-r from-accent-primary to-accent-secondary rounded-lg font-semibold text-background hover:shadow-glow transition-all disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isLoading ? 'Initializing...' : 'Initialize Portfolio'}
          </button>
        ) : (
          <button
            onClick={handleSubmitOrder}
            disabled={isSubmitting || orderDetails.size <= 0}
            className={clsx(
              'w-full py-3 rounded-lg font-semibold transition-all disabled:opacity-50 disabled:cursor-not-allowed',
              orderFormSide === 'buy'
                ? 'bg-success hover:bg-success/90 text-background'
                : 'bg-danger hover:bg-danger/90 text-white'
            )}
          >
            {isSubmitting ? 'Placing Order...' : (
              orderFormSide === 'buy' 
                ? `Buy / Long ${selectedMarket?.baseAsset || 'BTC'}` 
                : `Sell / Short ${selectedMarket?.baseAsset || 'BTC'}`
            )}
          </button>
        )}
      </div>

      {/* Available Margin */}
      <div className="px-3 pb-3">
        <div className="flex items-center justify-between text-xs text-text-secondary">
          <span className="flex items-center gap-1">
            Available
            <Info className="w-3 h-3" />
          </span>
          <span className="font-mono text-success">
            ${formatNumber(portfolio?.availableMargin || 0, 2)} USDC
          </span>
        </div>
      </div>
    </div>
  );
};
