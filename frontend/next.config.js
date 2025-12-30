const path = require('path');

/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  
  // Use Turbopack configuration for Next.js 16+
  turbopack: {
    root: path.resolve(__dirname),
    resolveAlias: {
      // Add any custom aliases if needed
    },
  },
  
  // Transpile wallet adapter packages
  transpilePackages: [
    '@solana/wallet-adapter-base',
    '@solana/wallet-adapter-react',
    '@solana/wallet-adapter-react-ui',
    '@solana/wallet-adapter-wallets',
  ],
};

module.exports = nextConfig;
