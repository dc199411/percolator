#!/bin/bash
# Deploy Percolator programs to local test validator
#
# Prerequisites:
# 1. Solana CLI installed (solana --version)
# 2. Local validator running (solana-test-validator)
# 3. Programs built (cargo build-sbf)

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}=== Percolator Local Deployment ===${NC}"

# Check if solana CLI is available
if ! command -v solana &> /dev/null; then
    echo -e "${RED}Error: Solana CLI not found. Please install it first.${NC}"
    echo "Run: sh -c \"\$(curl -sSfL https://release.anza.xyz/v2.1.0/install)\""
    exit 1
fi

# Check if programs are built
SLAB_SO="./target/deploy/percolator_slab.so"
ROUTER_SO="./target/deploy/percolator_router.so"

if [ ! -f "$SLAB_SO" ] || [ ! -f "$ROUTER_SO" ]; then
    echo -e "${YELLOW}Programs not built. Building now...${NC}"
    cargo build-sbf --manifest-path programs/slab/Cargo.toml --features bpf-entrypoint
    cargo build-sbf --manifest-path programs/router/Cargo.toml --features bpf-entrypoint
fi

# Configure for localhost
solana config set --url localhost

# Check if validator is running
if ! solana cluster-version &> /dev/null; then
    echo -e "${RED}Error: Local validator not running.${NC}"
    echo "Start it with: solana-test-validator"
    exit 1
fi

# Get airdrop for deployment
echo -e "${YELLOW}Requesting airdrop...${NC}"
solana airdrop 10 || true

# Display program sizes
echo -e "${GREEN}Program sizes:${NC}"
ls -la "$SLAB_SO" "$ROUTER_SO"

# Deploy Slab program
echo -e "${YELLOW}Deploying Slab program...${NC}"
SLAB_PROGRAM_ID=$(solana program deploy "$SLAB_SO" --output json | jq -r '.programId')
echo -e "${GREEN}Slab Program ID: $SLAB_PROGRAM_ID${NC}"

# Deploy Router program
echo -e "${YELLOW}Deploying Router program...${NC}"
ROUTER_PROGRAM_ID=$(solana program deploy "$ROUTER_SO" --output json | jq -r '.programId')
echo -e "${GREEN}Router Program ID: $ROUTER_PROGRAM_ID${NC}"

# Save program IDs
echo "SLAB_PROGRAM_ID=$SLAB_PROGRAM_ID" > .env.local
echo "ROUTER_PROGRAM_ID=$ROUTER_PROGRAM_ID" >> .env.local

echo -e "${GREEN}=== Deployment Complete ===${NC}"
echo "Program IDs saved to .env.local"
echo ""
echo "Next steps:"
echo "1. Initialize the Router registry"
echo "2. Register the Slab program with the Router"
echo "3. Create test portfolios and execute trades"
