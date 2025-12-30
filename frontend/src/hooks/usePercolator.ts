'use client';

import { useCallback, useEffect, useMemo, useState } from 'react';
import { useConnection, useWallet } from '@solana/wallet-adapter-react';
import { Transaction } from '@solana/web3.js';
import BN from 'bn.js';
import toast from 'react-hot-toast';
import {
  PercolatorClient,
  UserPortfolio,
  usdcFromRaw,
  USDC_MINT,
} from '@/lib/percolator';
import { useTradingStore } from '@/store/trading';

export function usePercolator() {
  const { connection } = useConnection();
  const { publicKey, signTransaction, connected } = useWallet();
  const { setPortfolio, setIsConnected, setPositions } = useTradingStore();
  
  const [isInitialized, setIsInitialized] = useState(false);
  const [hasPortfolio, setHasPortfolio] = useState(false);
  const [isLoading, setIsLoading] = useState(false);

  const client = useMemo(() => {
    return new PercolatorClient(connection, USDC_MINT);
  }, [connection]);

  // Check if user has a portfolio
  const checkPortfolio = useCallback(async () => {
    if (!publicKey) {
      setHasPortfolio(false);
      return;
    }

    try {
      const exists = await client.hasPortfolio(publicKey);
      setHasPortfolio(exists);
      
      // Note: We intentionally don't include fetchPortfolio in deps
      // to avoid circular dependency - it's called only when exists is true
    } catch (error) {
      console.error('Error checking portfolio:', error);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [publicKey, client]);

  // Fetch portfolio data
  const fetchPortfolio = useCallback(async () => {
    if (!publicKey) return;

    try {
      const portfolio = await client.getPortfolio(publicKey);
      
      if (portfolio) {
        const collateralBalance = parseFloat(usdcFromRaw(portfolio.collateralBalance));
        const unrealizedPnl = parseFloat(usdcFromRaw(portfolio.unrealizedPnl));
        const realizedPnl = parseFloat(usdcFromRaw(portfolio.realizedPnl));
        const initialMargin = parseFloat(usdcFromRaw(portfolio.initialMarginUsed));
        const maintenanceMargin = parseFloat(usdcFromRaw(portfolio.maintenanceMarginUsed));
        
        const availableMargin = collateralBalance + unrealizedPnl - initialMargin;
        const totalValue = collateralBalance + unrealizedPnl;
        const marginRatio = totalValue > 0 ? (initialMargin / totalValue) * 100 : 0;

        setPortfolio({
          collateralBalance,
          unrealizedPnl,
          realizedPnl,
          initialMargin,
          maintenanceMargin,
          availableMargin,
          marginRatio,
          leverage: initialMargin > 0 ? totalValue / availableMargin : 1,
          totalPositionValue: initialMargin * 20, // Assuming 5% IM = 20x max
        });

        // Convert positions
        const positions = portfolio.positions.map((pos, idx) => ({
          id: `pos-${idx}`,
          market: 'BTC-PERP', // Would map from instrument index
          side: pos.qty.gt(new BN(0)) ? 'long' as const : 'short' as const,
          size: Math.abs(parseFloat(pos.qty.toString()) / 1_000_000),
          entryPrice: parseFloat(usdcFromRaw(pos.entryPrice)),
          markPrice: parseFloat(usdcFromRaw(pos.lastMarkPrice)),
          liquidationPrice: 0, // Calculate based on margin
          pnl: parseFloat(usdcFromRaw(pos.unrealizedPnl)),
          pnlPercent: 0,
          margin: 0,
          leverage: 5,
          slabIndex: pos.slabIndex,
          instrumentIndex: pos.instrumentIndex,
        }));

        setPositions(positions);
      }
    } catch (error) {
      console.error('Error fetching portfolio:', error);
    }
  }, [publicKey, client, setPortfolio, setPositions]);

  // Initialize portfolio
  const initializePortfolio = useCallback(async () => {
    if (!publicKey || !signTransaction) {
      toast.error('Please connect your wallet');
      return;
    }

    setIsLoading(true);
    const loadingToast = toast.loading('Initializing portfolio...');

    try {
      const tx = await client.buildInitializePortfolioTx(publicKey);
      tx.feePayer = publicKey;
      tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;

      const signedTx = await signTransaction(tx);
      const signature = await connection.sendRawTransaction(signedTx.serialize());
      
      await connection.confirmTransaction(signature, 'confirmed');

      toast.success('Portfolio initialized!', { id: loadingToast });
      setHasPortfolio(true);
      await fetchPortfolio();
    } catch (error: any) {
      console.error('Error initializing portfolio:', error);
      toast.error(error.message || 'Failed to initialize portfolio', { id: loadingToast });
    } finally {
      setIsLoading(false);
    }
  }, [publicKey, signTransaction, client, connection, fetchPortfolio]);

  // Deposit collateral
  const deposit = useCallback(async (amount: number) => {
    if (!publicKey || !signTransaction) {
      toast.error('Please connect your wallet');
      return;
    }

    if (amount <= 0) {
      toast.error('Invalid amount');
      return;
    }

    setIsLoading(true);
    const loadingToast = toast.loading(`Depositing ${amount} USDC...`);

    try {
      const tx = await client.buildDepositTx(publicKey, amount);
      tx.feePayer = publicKey;
      tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;

      const signedTx = await signTransaction(tx);
      const signature = await connection.sendRawTransaction(signedTx.serialize());
      
      await connection.confirmTransaction(signature, 'confirmed');

      toast.success(`Deposited ${amount} USDC!`, { id: loadingToast });
      await fetchPortfolio();
    } catch (error: any) {
      console.error('Error depositing:', error);
      toast.error(error.message || 'Failed to deposit', { id: loadingToast });
    } finally {
      setIsLoading(false);
    }
  }, [publicKey, signTransaction, client, connection, fetchPortfolio]);

  // Withdraw collateral
  const withdraw = useCallback(async (amount: number) => {
    if (!publicKey || !signTransaction) {
      toast.error('Please connect your wallet');
      return;
    }

    if (amount <= 0) {
      toast.error('Invalid amount');
      return;
    }

    setIsLoading(true);
    const loadingToast = toast.loading(`Withdrawing ${amount} USDC...`);

    try {
      const tx = await client.buildWithdrawTx(publicKey, amount);
      tx.feePayer = publicKey;
      tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;

      const signedTx = await signTransaction(tx);
      const signature = await connection.sendRawTransaction(signedTx.serialize());
      
      await connection.confirmTransaction(signature, 'confirmed');

      toast.success(`Withdrew ${amount} USDC!`, { id: loadingToast });
      await fetchPortfolio();
    } catch (error: any) {
      console.error('Error withdrawing:', error);
      toast.error(error.message || 'Failed to withdraw', { id: loadingToast });
    } finally {
      setIsLoading(false);
    }
  }, [publicKey, signTransaction, client, connection, fetchPortfolio]);

  // Update connection status
  useEffect(() => {
    setIsConnected(connected);
    
    if (connected && publicKey) {
      checkPortfolio();
    } else {
      setHasPortfolio(false);
      setPortfolio(null as any);
    }
  }, [connected, publicKey, checkPortfolio, setIsConnected, setPortfolio]);

  // Set up mock portfolio for demo
  useEffect(() => {
    if (connected) {
      // Set demo portfolio data
      setPortfolio({
        collateralBalance: 10000,
        unrealizedPnl: 234.56,
        realizedPnl: 1234.56,
        initialMargin: 2500,
        maintenanceMargin: 1250,
        availableMargin: 7734.56,
        marginRatio: 24.5,
        leverage: 1.3,
        totalPositionValue: 50000,
      });

      // Set demo positions
      setPositions([
        {
          id: 'pos-1',
          market: 'BTC-PERP',
          side: 'long',
          size: 0.15,
          entryPrice: 95420.50,
          markPrice: 97245.50,
          liquidationPrice: 82000.00,
          pnl: 273.75,
          pnlPercent: 1.91,
          margin: 715.65,
          leverage: 20,
          slabIndex: 0,
          instrumentIndex: 0,
        },
        {
          id: 'pos-2',
          market: 'ETH-PERP',
          side: 'short',
          size: 2.5,
          entryPrice: 3520.00,
          markPrice: 3456.78,
          liquidationPrice: 3950.00,
          pnl: 158.05,
          pnlPercent: 1.79,
          margin: 440.00,
          leverage: 20,
          slabIndex: 1,
          instrumentIndex: 1,
        },
      ]);
    }
  }, [connected, setPortfolio, setPositions]);

  return {
    client,
    isInitialized,
    hasPortfolio,
    isLoading,
    initializePortfolio,
    deposit,
    withdraw,
    fetchPortfolio,
    checkPortfolio,
  };
}

// Price simulation hook for demo
export function usePriceSimulation() {
  const { selectedMarket, updateMarketPrice, setOrderBook } = useTradingStore();

  useEffect(() => {
    if (!selectedMarket) return;

    const interval = setInterval(() => {
      const change = (Math.random() - 0.5) * selectedMarket.lastPrice * 0.0002;
      const newPrice = selectedMarket.lastPrice + change;
      updateMarketPrice(selectedMarket.symbol, newPrice);
    }, 2000);

    return () => clearInterval(interval);
  }, [selectedMarket, updateMarketPrice]);
}
