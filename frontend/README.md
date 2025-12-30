# Percolator Frontend

A professional, Hyperliquid-inspired trading interface for the Percolator Protocol - a sharded perpetual exchange on Solana.

## Features

- **🎨 Modern UI/UX** - Clean, dark theme with professional trading interface
- **📱 Mobile Responsive** - Works seamlessly on desktop, tablet, and mobile devices
- **💼 Wallet Integration** - Supports Phantom, Solflare, Ledger, and other popular Solana wallets
- **📊 Real-time Data** - Live order book, price charts, and position tracking
- **⚡ Fast Execution** - Optimized for quick order placement and management

## Pages

### Landing Page (`/`)
- Hero section with protocol overview
- Key features and benefits
- How it works section
- Call-to-action to launch trading app

### Trading Interface (`/trade`)
- **Market Selector** - Switch between trading pairs (BTC-PERP, ETH-PERP, SOL-PERP, etc.)
- **Order Book** - Real-time bid/ask depth visualization
- **Price Chart** - Candlestick charts with multiple timeframes
- **Order Form** - Market and limit orders with leverage slider
- **Positions Panel** - Open positions, orders, and trade history
- **Account Panel** - Deposit, withdraw, and portfolio overview

## Tech Stack

- **Framework**: Next.js 16 with App Router
- **Language**: TypeScript
- **Styling**: Tailwind CSS
- **State Management**: Zustand
- **Wallet Integration**: Solana Wallet Adapter
- **Animations**: Framer Motion
- **Charts**: Custom Canvas-based implementation
- **Notifications**: React Hot Toast

## Getting Started

### Prerequisites

- Node.js 18+
- npm or yarn

### Installation

```bash
# Navigate to frontend directory
cd frontend

# Install dependencies
npm install

# Create environment file
cp .env.local.example .env.local
```

### Development

```bash
# Start development server
npm run dev

# Open http://localhost:3000
```

### Production Build

```bash
# Build for production
npm run build

# Start production server
npm start
```

## Environment Variables

Create a `.env.local` file with:

```env
# Solana RPC URL
NEXT_PUBLIC_RPC_URL=https://api.devnet.solana.com

# Network (devnet | mainnet-beta)
NEXT_PUBLIC_NETWORK=devnet
```

## Project Structure

```
frontend/
├── src/
│   ├── app/                    # Next.js App Router pages
│   │   ├── page.tsx           # Landing page
│   │   ├── trade/
│   │   │   └── page.tsx       # Trading interface
│   │   ├── layout.tsx         # Root layout
│   │   └── globals.css        # Global styles
│   ├── components/
│   │   ├── providers.tsx      # Context providers
│   │   └── trading/           # Trading components
│   │       ├── Header.tsx
│   │       ├── MarketSelector.tsx
│   │       ├── OrderBook.tsx
│   │       ├── OrderForm.tsx
│   │       ├── Positions.tsx
│   │       ├── Chart.tsx
│   │       ├── AccountPanel.tsx
│   │       └── RecentTrades.tsx
│   ├── hooks/
│   │   └── usePercolator.ts   # Protocol integration hook
│   ├── lib/
│   │   ├── percolator.ts      # Protocol SDK client
│   │   └── utils.ts           # Utility functions
│   └── store/
│       └── trading.ts         # Zustand store
├── public/                     # Static assets
├── next.config.js             # Next.js configuration
├── tailwind.config.ts         # Tailwind configuration
├── tsconfig.json              # TypeScript configuration
└── package.json               # Dependencies
```

## Protocol Integration

The frontend integrates with the Percolator Protocol through:

1. **PercolatorClient** (`src/lib/percolator.ts`)
   - Builds and sends transactions
   - Fetches portfolio and position data
   - Handles PDA derivation

2. **usePercolator Hook** (`src/hooks/usePercolator.ts`)
   - Manages wallet connection state
   - Provides deposit/withdraw functions
   - Handles portfolio initialization

## Design System

### Colors

- **Background**: `#0a0a0f`
- **Surface**: `#12121a`
- **Accent (Success/Buy)**: `#00d4aa`
- **Danger (Sell)**: `#ff4757`
- **Text Primary**: `#ffffff`
- **Text Secondary**: `#8b8b9e`

### Typography

- **Sans**: Inter
- **Mono**: JetBrains Mono

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run `npm run build` to verify
5. Submit a pull request

## License

MIT License - see LICENSE file for details
