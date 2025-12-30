'use client';

import { FC, ReactNode, useMemo } from 'react';
import { ConnectionProvider, WalletProvider } from '@solana/wallet-adapter-react';
import { WalletModalProvider } from '@solana/wallet-adapter-react-ui';
import {
  PhantomWalletAdapter,
  SolflareWalletAdapter,
  TorusWalletAdapter,
  LedgerWalletAdapter,
  CoinbaseWalletAdapter,
} from '@solana/wallet-adapter-wallets';
import { Toaster } from 'react-hot-toast';

// Import wallet adapter CSS
import '@solana/wallet-adapter-react-ui/styles.css';

interface ProvidersProps {
  children: ReactNode;
}

export const Providers: FC<ProvidersProps> = ({ children }) => {
  // Network endpoint - use devnet for development, mainnet-beta for production
  const endpoint = useMemo(() => {
    return process.env.NEXT_PUBLIC_RPC_URL || 'https://api.devnet.solana.com';
  }, []);

  // Wallet adapters
  const wallets = useMemo(
    () => [
      new PhantomWalletAdapter(),
      new SolflareWalletAdapter(),
      new TorusWalletAdapter(),
      new LedgerWalletAdapter(),
      new CoinbaseWalletAdapter(),
    ],
    []
  );

  return (
    <ConnectionProvider endpoint={endpoint}>
      <WalletProvider wallets={wallets} autoConnect>
        <WalletModalProvider>
          {children}
          <Toaster
            position="bottom-right"
            toastOptions={{
              duration: 4000,
              style: {
                background: '#12121a',
                color: '#ffffff',
                border: '1px solid #2a2a3a',
                borderRadius: '12px',
                padding: '16px',
                fontSize: '14px',
              },
              success: {
                iconTheme: {
                  primary: '#00d4aa',
                  secondary: '#12121a',
                },
              },
              error: {
                iconTheme: {
                  primary: '#ff4757',
                  secondary: '#12121a',
                },
              },
            }}
          />
        </WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
};
