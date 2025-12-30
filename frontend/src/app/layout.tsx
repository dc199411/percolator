import type { Metadata } from 'next';
import { Providers } from '@/components/providers';
import './globals.css';

export const metadata: Metadata = {
  title: 'Percolator | Sharded Perpetual Exchange',
  description: 'Trade perpetual futures with maximum capital efficiency on Solana. A sharded exchange protocol with portfolio margining across multiple liquidity pools.',
  keywords: ['Solana', 'DeFi', 'perpetual futures', 'DEX', 'trading', 'percolator'],
  openGraph: {
    title: 'Percolator | Sharded Perpetual Exchange',
    description: 'Trade perpetual futures with maximum capital efficiency on Solana.',
    type: 'website',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Percolator | Sharded Perpetual Exchange',
    description: 'Trade perpetual futures with maximum capital efficiency on Solana.',
  },
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className="dark">
      <body className="bg-background text-text-primary antialiased">
        <Providers>
          {children}
        </Providers>
      </body>
    </html>
  );
}
