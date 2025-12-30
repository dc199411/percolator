import type { Config } from 'tailwindcss';

const config: Config = {
  content: [
    './src/pages/**/*.{js,ts,jsx,tsx,mdx}',
    './src/components/**/*.{js,ts,jsx,tsx,mdx}',
    './src/app/**/*.{js,ts,jsx,tsx,mdx}',
  ],
  theme: {
    extend: {
      colors: {
        // Hyperliquid-inspired dark theme colors
        background: '#0a0a0f',
        surface: {
          DEFAULT: '#12121a',
          light: '#1a1a24',
          hover: '#222230',
        },
        border: {
          DEFAULT: '#2a2a3a',
          light: '#3a3a4a',
        },
        accent: {
          primary: '#00d4aa',
          secondary: '#00b894',
          hover: '#00e6bb',
        },
        success: '#00d4aa',
        danger: '#ff4757',
        warning: '#ffa502',
        text: {
          primary: '#ffffff',
          secondary: '#8b8b9e',
          muted: '#5a5a6e',
        },
        buy: {
          DEFAULT: '#00d4aa',
          light: 'rgba(0, 212, 170, 0.1)',
          hover: 'rgba(0, 212, 170, 0.2)',
        },
        sell: {
          DEFAULT: '#ff4757',
          light: 'rgba(255, 71, 87, 0.1)',
          hover: 'rgba(255, 71, 87, 0.2)',
        },
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace'],
      },
      fontSize: {
        '2xs': ['0.625rem', { lineHeight: '0.75rem' }],
      },
      animation: {
        'pulse-slow': 'pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite',
        'slide-up': 'slideUp 0.3s ease-out',
        'fade-in': 'fadeIn 0.2s ease-out',
      },
      keyframes: {
        slideUp: {
          '0%': { transform: 'translateY(10px)', opacity: '0' },
          '100%': { transform: 'translateY(0)', opacity: '1' },
        },
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
      },
      boxShadow: {
        'glow': '0 0 20px rgba(0, 212, 170, 0.3)',
        'glow-sm': '0 0 10px rgba(0, 212, 170, 0.2)',
      },
    },
  },
  plugins: [],
};

export default config;
