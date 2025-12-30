'use client';

import { FC, useEffect, useRef, useState } from 'react';
import { useTradingStore } from '@/store/trading';
import clsx from 'clsx';

// Time frame options
const timeframes = [
  { label: '1m', value: '1' },
  { label: '5m', value: '5' },
  { label: '15m', value: '15' },
  { label: '1H', value: '60' },
  { label: '4H', value: '240' },
  { label: '1D', value: 'D' },
  { label: '1W', value: 'W' },
];

// Chart types
const chartTypes = ['Candles', 'Line', 'Area'];

export const TradingChart: FC = () => {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const [selectedTimeframe, setSelectedTimeframe] = useState('15');
  const [selectedChartType, setSelectedChartType] = useState('Candles');
  const { selectedMarket } = useTradingStore();

  return (
    <div className="flex flex-col h-full bg-surface rounded-lg border border-border overflow-hidden">
      {/* Chart Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-border">
        <div className="flex items-center gap-2">
          {/* Timeframe selector */}
          <div className="flex items-center gap-1 bg-surface-light rounded-lg p-0.5">
            {timeframes.map((tf) => (
              <button
                key={tf.value}
                onClick={() => setSelectedTimeframe(tf.value)}
                className={clsx(
                  'px-2 py-1 text-xs rounded-md transition-colors',
                  selectedTimeframe === tf.value
                    ? 'bg-surface text-text-primary'
                    : 'text-text-secondary hover:text-text-primary'
                )}
              >
                {tf.label}
              </button>
            ))}
          </div>
        </div>

        <div className="flex items-center gap-2">
          {/* Chart type selector */}
          <div className="flex items-center gap-1">
            {chartTypes.map((type) => (
              <button
                key={type}
                onClick={() => setSelectedChartType(type)}
                className={clsx(
                  'px-2 py-1 text-xs rounded-md transition-colors',
                  selectedChartType === type
                    ? 'bg-surface-light text-text-primary'
                    : 'text-text-secondary hover:text-text-primary'
                )}
              >
                {type}
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* Chart Container */}
      <div 
        ref={chartContainerRef} 
        className="flex-1 relative min-h-0"
      >
        <SimulatedChart 
          symbol={selectedMarket?.symbol || 'BTC-PERP'}
          basePrice={selectedMarket?.lastPrice || 97245.50}
          timeframe={selectedTimeframe}
        />
      </div>
    </div>
  );
};

// Simulated chart component for demo
const SimulatedChart: FC<{ symbol: string; basePrice: number; timeframe: string }> = ({ 
  symbol, 
  basePrice,
  timeframe 
}) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [dimensions, setDimensions] = useState({ width: 0, height: 0 });

  useEffect(() => {
    const updateDimensions = () => {
      if (containerRef.current) {
        setDimensions({
          width: containerRef.current.clientWidth,
          height: containerRef.current.clientHeight,
        });
      }
    };

    updateDimensions();
    window.addEventListener('resize', updateDimensions);
    return () => window.removeEventListener('resize', updateDimensions);
  }, []);

  useEffect(() => {
    if (!canvasRef.current || dimensions.width === 0 || dimensions.height === 0) return;

    const canvas = canvasRef.current;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Set canvas size
    const dpr = window.devicePixelRatio || 1;
    canvas.width = dimensions.width * dpr;
    canvas.height = dimensions.height * dpr;
    ctx.scale(dpr, dpr);

    // Generate candle data (inline to avoid dependency issues)
    const generateCandles = (count: number, price: number) => {
      const candles = [];
      let currentPrice = price;
      const now = Date.now();
      const interval = parseInt(timeframe) * 60 * 1000 || 3600000;

      for (let i = count - 1; i >= 0; i--) {
        const volatility = currentPrice * 0.002;
        const open = currentPrice;
        const change = (Math.random() - 0.48) * volatility;
        const high = open + Math.random() * volatility;
        const low = open - Math.random() * volatility;
        const close = open + change;
        
        candles.push({
          time: now - i * interval,
          open,
          high: Math.max(open, close, high),
          low: Math.min(open, close, low),
          close,
          volume: Math.random() * 1000000,
        });
        
        currentPrice = close;
      }
      return candles;
    };

    const candles = generateCandles(100, basePrice);

    // Calculate price range
    const prices = candles.flatMap(c => [c.high, c.low]);
    const minPrice = Math.min(...prices);
    const maxPrice = Math.max(...prices);
    const priceRange = maxPrice - minPrice;
    const padding = priceRange * 0.1;

    // Chart dimensions
    const chartPadding = { top: 20, right: 80, bottom: 30, left: 10 };
    const chartWidth = dimensions.width - chartPadding.left - chartPadding.right;
    const chartHeight = dimensions.height - chartPadding.top - chartPadding.bottom;

    // Clear canvas
    ctx.fillStyle = '#0a0a0f';
    ctx.fillRect(0, 0, dimensions.width, dimensions.height);

    // Draw grid lines
    ctx.strokeStyle = 'rgba(42, 42, 58, 0.5)';
    ctx.lineWidth = 1;

    // Horizontal grid lines
    const priceStep = priceRange / 5;
    for (let i = 0; i <= 5; i++) {
      const price = minPrice - padding + (priceRange + padding * 2) * (i / 5);
      const y = chartPadding.top + chartHeight - ((price - (minPrice - padding)) / (priceRange + padding * 2)) * chartHeight;
      
      ctx.beginPath();
      ctx.moveTo(chartPadding.left, y);
      ctx.lineTo(dimensions.width - chartPadding.right, y);
      ctx.stroke();

      // Price label
      ctx.fillStyle = '#8b8b9e';
      ctx.font = '10px Inter';
      ctx.textAlign = 'left';
      ctx.fillText(`$${price.toFixed(2)}`, dimensions.width - chartPadding.right + 5, y + 3);
    }

    // Draw candles
    const candleWidth = chartWidth / candles.length;
    const candleBodyWidth = Math.max(candleWidth * 0.7, 2);

    candles.forEach((candle, i) => {
      const x = chartPadding.left + i * candleWidth + candleWidth / 2;
      
      const scaleY = (price: number) => {
        return chartPadding.top + chartHeight - ((price - (minPrice - padding)) / (priceRange + padding * 2)) * chartHeight;
      };

      const openY = scaleY(candle.open);
      const closeY = scaleY(candle.close);
      const highY = scaleY(candle.high);
      const lowY = scaleY(candle.low);

      const isGreen = candle.close >= candle.open;
      const color = isGreen ? '#00d4aa' : '#ff4757';

      // Draw wick
      ctx.strokeStyle = color;
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(x, highY);
      ctx.lineTo(x, lowY);
      ctx.stroke();

      // Draw body
      ctx.fillStyle = color;
      const bodyTop = Math.min(openY, closeY);
      const bodyHeight = Math.max(Math.abs(closeY - openY), 1);
      ctx.fillRect(x - candleBodyWidth / 2, bodyTop, candleBodyWidth, bodyHeight);
    });

    // Draw current price line
    const lastCandle = candles[candles.length - 1];
    const currentY = chartPadding.top + chartHeight - ((lastCandle.close - (minPrice - padding)) / (priceRange + padding * 2)) * chartHeight;
    
    ctx.setLineDash([5, 5]);
    ctx.strokeStyle = lastCandle.close >= candles[0].open ? '#00d4aa' : '#ff4757';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(chartPadding.left, currentY);
    ctx.lineTo(dimensions.width - chartPadding.right, currentY);
    ctx.stroke();
    ctx.setLineDash([]);

    // Current price label
    ctx.fillStyle = lastCandle.close >= candles[0].open ? '#00d4aa' : '#ff4757';
    ctx.fillRect(dimensions.width - chartPadding.right, currentY - 10, chartPadding.right - 5, 20);
    ctx.fillStyle = '#0a0a0f';
    ctx.font = 'bold 10px Inter';
    ctx.textAlign = 'center';
    ctx.fillText(`$${lastCandle.close.toFixed(2)}`, dimensions.width - chartPadding.right / 2, currentY + 4);

  }, [dimensions, basePrice, timeframe]);

  return (
    <div ref={containerRef} className="w-full h-full">
      <canvas
        ref={canvasRef}
        style={{ width: '100%', height: '100%' }}
      />
    </div>
  );
};
