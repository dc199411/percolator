'use client';

import { motion } from 'framer-motion';
import Link from 'next/link';
import { 
  ArrowRight, 
  Shield, 
  Zap, 
  Layers, 
  TrendingUp,
  ChevronDown,
  ExternalLink,
  Github
} from 'lucide-react';

const features = [
  {
    icon: Layers,
    title: 'Sharded Architecture',
    description: 'Multiple liquidity pools (slabs) working in harmony for deep liquidity and fast execution.',
  },
  {
    icon: Shield,
    title: 'Portfolio Margining',
    description: 'Capital-efficient cross-margin across all positions. Long + Short = Near-zero margin.',
  },
  {
    icon: Zap,
    title: 'Lightning Fast',
    description: 'Sub-second execution with atomic cross-slab fills. No more partial fills or slippage.',
  },
  {
    icon: TrendingUp,
    title: 'Deep Liquidity',
    description: 'Aggregate liquidity from multiple market makers for tight spreads and minimal impact.',
  },
];

const stats = [
  { label: 'Capital Efficiency', value: '∞', suffix: '' },
  { label: 'Execution Speed', value: '<400', suffix: 'ms' },
  { label: 'Max Leverage', value: '20', suffix: 'x' },
  { label: 'Markets', value: '8', suffix: '+' },
];

export default function HomePage() {
  return (
    <main className="min-h-screen bg-background overflow-hidden">
      {/* Navigation */}
      <nav className="fixed top-0 left-0 right-0 z-50 glass border-b border-border">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex items-center justify-between h-16">
            <div className="flex items-center gap-2">
              <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-accent-primary to-accent-secondary flex items-center justify-center">
                <span className="text-background font-bold text-sm">P</span>
              </div>
              <span className="font-semibold text-lg">Percolator</span>
            </div>
            
            <div className="hidden md:flex items-center gap-8">
              <a href="#features" className="text-text-secondary hover:text-text-primary transition-colors">
                Features
              </a>
              <a href="#how-it-works" className="text-text-secondary hover:text-text-primary transition-colors">
                How it Works
              </a>
              <a 
                href="https://github.com/percolator-protocol" 
                target="_blank" 
                rel="noopener noreferrer"
                className="text-text-secondary hover:text-text-primary transition-colors flex items-center gap-1"
              >
                <Github className="w-4 h-4" />
                Docs
              </a>
            </div>

            <Link
              href="/trade"
              className="px-4 py-2 bg-gradient-to-r from-accent-primary to-accent-secondary rounded-lg font-medium text-background hover:shadow-glow transition-all duration-300 hover:-translate-y-0.5"
            >
              Launch App
            </Link>
          </div>
        </div>
      </nav>

      {/* Hero Section */}
      <section className="relative pt-32 pb-20 px-4 sm:px-6 lg:px-8 overflow-hidden">
        {/* Background Effects */}
        <div className="absolute inset-0 overflow-hidden">
          <div className="absolute top-1/4 left-1/4 w-96 h-96 bg-accent-primary/10 rounded-full blur-3xl animate-pulse-slow" />
          <div className="absolute bottom-1/4 right-1/4 w-96 h-96 bg-accent-secondary/10 rounded-full blur-3xl animate-pulse-slow delay-1000" />
          <div className="absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 w-[800px] h-[800px]">
            <div className="absolute inset-0 border border-border/20 rounded-full" />
            <div className="absolute inset-8 border border-border/20 rounded-full" />
            <div className="absolute inset-16 border border-border/20 rounded-full" />
          </div>
        </div>

        <div className="max-w-7xl mx-auto relative">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.6 }}
            className="text-center max-w-4xl mx-auto"
          >
            <motion.div
              initial={{ opacity: 0, scale: 0.9 }}
              animate={{ opacity: 1, scale: 1 }}
              transition={{ delay: 0.2 }}
              className="inline-flex items-center gap-2 px-4 py-2 rounded-full bg-surface border border-border mb-6"
            >
              <span className="w-2 h-2 rounded-full bg-accent-primary animate-pulse" />
              <span className="text-sm text-text-secondary">Live on Solana Devnet</span>
            </motion.div>

            <h1 className="text-5xl sm:text-6xl lg:text-7xl font-bold mb-6 leading-tight">
              <span className="bg-clip-text text-transparent bg-gradient-to-r from-white via-white to-text-secondary">
                Trade Perpetuals
              </span>
              <br />
              <span className="bg-clip-text text-transparent bg-gradient-to-r from-accent-primary to-accent-secondary glow-text">
                Infinitely Efficient
              </span>
            </h1>

            <p className="text-lg sm:text-xl text-text-secondary max-w-2xl mx-auto mb-8">
              The first sharded perpetual exchange on Solana. Portfolio margining across multiple 
              liquidity pools means your capital works harder than ever before.
            </p>

            <div className="flex flex-col sm:flex-row items-center justify-center gap-4">
              <Link
                href="/trade"
                className="group px-8 py-4 bg-gradient-to-r from-accent-primary to-accent-secondary rounded-xl font-semibold text-background hover:shadow-glow transition-all duration-300 hover:-translate-y-1 flex items-center gap-2"
              >
                Start Trading
                <ArrowRight className="w-5 h-5 group-hover:translate-x-1 transition-transform" />
              </Link>
              <a
                href="#how-it-works"
                className="px-8 py-4 rounded-xl font-semibold border border-border hover:border-accent-primary/50 hover:bg-surface transition-all duration-300 flex items-center gap-2"
              >
                Learn More
                <ChevronDown className="w-5 h-5" />
              </a>
            </div>
          </motion.div>

          {/* Stats */}
          <motion.div
            initial={{ opacity: 0, y: 40 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.4, duration: 0.6 }}
            className="mt-20 grid grid-cols-2 md:grid-cols-4 gap-4 sm:gap-8"
          >
            {stats.map((stat, index) => (
              <div
                key={stat.label}
                className="text-center p-6 rounded-2xl bg-surface/50 border border-border hover:border-accent-primary/30 transition-colors"
              >
                <div className="text-3xl sm:text-4xl font-bold text-accent-primary mb-1">
                  {stat.value}
                  <span className="text-text-secondary text-lg">{stat.suffix}</span>
                </div>
                <div className="text-sm text-text-secondary">{stat.label}</div>
              </div>
            ))}
          </motion.div>
        </div>
      </section>

      {/* Features Section */}
      <section id="features" className="py-20 px-4 sm:px-6 lg:px-8">
        <div className="max-w-7xl mx-auto">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            className="text-center mb-16"
          >
            <h2 className="text-3xl sm:text-4xl font-bold mb-4">Why Percolator?</h2>
            <p className="text-text-secondary max-w-2xl mx-auto">
              Built from the ground up for capital efficiency and speed. No compromises.
            </p>
          </motion.div>

          <div className="grid md:grid-cols-2 lg:grid-cols-4 gap-6">
            {features.map((feature, index) => (
              <motion.div
                key={feature.title}
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true }}
                transition={{ delay: index * 0.1 }}
                className="group p-6 rounded-2xl bg-surface border border-border hover:border-accent-primary/30 transition-all duration-300 hover:-translate-y-1"
              >
                <div className="w-12 h-12 rounded-xl bg-accent-primary/10 flex items-center justify-center mb-4 group-hover:bg-accent-primary/20 transition-colors">
                  <feature.icon className="w-6 h-6 text-accent-primary" />
                </div>
                <h3 className="font-semibold text-lg mb-2">{feature.title}</h3>
                <p className="text-text-secondary text-sm">{feature.description}</p>
              </motion.div>
            ))}
          </div>
        </div>
      </section>

      {/* How It Works Section */}
      <section id="how-it-works" className="py-20 px-4 sm:px-6 lg:px-8 bg-surface/30">
        <div className="max-w-7xl mx-auto">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            className="text-center mb-16"
          >
            <h2 className="text-3xl sm:text-4xl font-bold mb-4">How It Works</h2>
            <p className="text-text-secondary max-w-2xl mx-auto">
              A revolutionary approach to perpetual trading
            </p>
          </motion.div>

          <div className="grid lg:grid-cols-3 gap-8">
            {/* Step 1 */}
            <motion.div
              initial={{ opacity: 0, x: -20 }}
              whileInView={{ opacity: 1, x: 0 }}
              viewport={{ once: true }}
              className="relative"
            >
              <div className="absolute -left-4 top-0 w-8 h-8 rounded-full bg-accent-primary flex items-center justify-center text-background font-bold">
                1
              </div>
              <div className="pl-8">
                <h3 className="font-semibold text-xl mb-3">Connect & Deposit</h3>
                <p className="text-text-secondary">
                  Connect your Solana wallet and deposit USDC collateral. Your funds are secured in the protocol vault with full transparency.
                </p>
              </div>
            </motion.div>

            {/* Step 2 */}
            <motion.div
              initial={{ opacity: 0, x: -20 }}
              whileInView={{ opacity: 1, x: 0 }}
              viewport={{ once: true }}
              transition={{ delay: 0.1 }}
              className="relative"
            >
              <div className="absolute -left-4 top-0 w-8 h-8 rounded-full bg-accent-primary flex items-center justify-center text-background font-bold">
                2
              </div>
              <div className="pl-8">
                <h3 className="font-semibold text-xl mb-3">Place Orders</h3>
                <p className="text-text-secondary">
                  The router automatically splits your orders across multiple slabs for best execution. Atomic fills ensure no partial orders.
                </p>
              </div>
            </motion.div>

            {/* Step 3 */}
            <motion.div
              initial={{ opacity: 0, x: -20 }}
              whileInView={{ opacity: 1, x: 0 }}
              viewport={{ once: true }}
              transition={{ delay: 0.2 }}
              className="relative"
            >
              <div className="absolute -left-4 top-0 w-8 h-8 rounded-full bg-accent-primary flex items-center justify-center text-background font-bold">
                3
              </div>
              <div className="pl-8">
                <h3 className="font-semibold text-xl mb-3">Maximize Efficiency</h3>
                <p className="text-text-secondary">
                  Your positions are netted across all slabs. Long on Slab A + Short on Slab B = zero margin requirement. Pure capital efficiency.
                </p>
              </div>
            </motion.div>
          </div>

          {/* Visual Diagram */}
          <motion.div
            initial={{ opacity: 0, y: 40 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            className="mt-16 p-8 rounded-2xl bg-surface border border-border"
          >
            <div className="grid md:grid-cols-3 gap-8 items-center">
              {/* User */}
              <div className="text-center">
                <div className="w-16 h-16 mx-auto rounded-2xl bg-gradient-to-br from-accent-primary/20 to-accent-secondary/20 flex items-center justify-center mb-4">
                  <span className="text-2xl">👤</span>
                </div>
                <h4 className="font-semibold mb-1">Trader</h4>
                <p className="text-sm text-text-secondary">Single deposit, multiple positions</p>
              </div>

              {/* Router */}
              <div className="text-center">
                <div className="w-16 h-16 mx-auto rounded-2xl bg-gradient-to-br from-accent-primary to-accent-secondary flex items-center justify-center mb-4">
                  <Layers className="w-8 h-8 text-background" />
                </div>
                <h4 className="font-semibold mb-1">Router</h4>
                <p className="text-sm text-text-secondary">Splits orders, nets positions, manages margin</p>
              </div>

              {/* Slabs */}
              <div className="text-center">
                <div className="flex justify-center gap-2 mb-4">
                  {[1, 2, 3].map((i) => (
                    <div
                      key={i}
                      className="w-12 h-12 rounded-xl bg-surface-light border border-border flex items-center justify-center text-sm font-mono text-text-secondary"
                    >
                      S{i}
                    </div>
                  ))}
                </div>
                <h4 className="font-semibold mb-1">Slabs</h4>
                <p className="text-sm text-text-secondary">Independent liquidity pools</p>
              </div>
            </div>
          </motion.div>
        </div>
      </section>

      {/* CTA Section */}
      <section className="py-20 px-4 sm:px-6 lg:px-8">
        <div className="max-w-4xl mx-auto text-center">
          <motion.div
            initial={{ opacity: 0, scale: 0.95 }}
            whileInView={{ opacity: 1, scale: 1 }}
            viewport={{ once: true }}
            className="p-8 sm:p-12 rounded-3xl bg-gradient-to-br from-surface to-surface-light border border-border relative overflow-hidden"
          >
            <div className="absolute inset-0 bg-gradient-to-br from-accent-primary/5 to-transparent" />
            <div className="relative">
              <h2 className="text-3xl sm:text-4xl font-bold mb-4">
                Ready to Trade?
              </h2>
              <p className="text-text-secondary mb-8 max-w-xl mx-auto">
                Experience the future of perpetual trading. Maximum capital efficiency, 
                lightning-fast execution, and deep liquidity await.
              </p>
              <Link
                href="/trade"
                className="inline-flex items-center gap-2 px-8 py-4 bg-gradient-to-r from-accent-primary to-accent-secondary rounded-xl font-semibold text-background hover:shadow-glow transition-all duration-300 hover:-translate-y-1"
              >
                Launch Trading App
                <ArrowRight className="w-5 h-5" />
              </Link>
            </div>
          </motion.div>
        </div>
      </section>

      {/* Footer */}
      <footer className="border-t border-border py-12 px-4 sm:px-6 lg:px-8">
        <div className="max-w-7xl mx-auto">
          <div className="flex flex-col md:flex-row items-center justify-between gap-6">
            <div className="flex items-center gap-2">
              <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-accent-primary to-accent-secondary flex items-center justify-center">
                <span className="text-background font-bold text-sm">P</span>
              </div>
              <span className="font-semibold">Percolator</span>
            </div>
            
            <div className="flex items-center gap-6 text-sm text-text-secondary">
              <a href="#" className="hover:text-text-primary transition-colors">Terms</a>
              <a href="#" className="hover:text-text-primary transition-colors">Privacy</a>
              <a 
                href="https://github.com/percolator-protocol" 
                target="_blank" 
                rel="noopener noreferrer"
                className="hover:text-text-primary transition-colors flex items-center gap-1"
              >
                <Github className="w-4 h-4" />
                GitHub
              </a>
            </div>

            <div className="text-sm text-text-muted">
              © 2025 Percolator Protocol. All rights reserved.
            </div>
          </div>
        </div>
      </footer>
    </main>
  );
}
