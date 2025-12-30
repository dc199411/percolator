#!/bin/bash
# Deploy Percolator programs to local test validator
#
# Usage:
#   ./scripts/deploy-local.sh              # Deploy to localhost
#   ./scripts/deploy-local.sh --devnet     # Deploy to devnet (requires SOL)
#
# Prerequisites:
#   1. Solana CLI installed (solana --version)
#   2. Local validator running (./scripts/start-validator.sh)
#      OR connected to devnet/testnet
#   3. Programs built (cargo build-sbf)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Program paths
SLAB_SO="$PROJECT_ROOT/target/deploy/percolator_slab.so"
ROUTER_SO="$PROJECT_ROOT/target/deploy/percolator_router.so"

# Program IDs (from lib.rs declare_id!)
SLAB_PROGRAM_ID="SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk"
ROUTER_PROGRAM_ID="RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr"

# Parse arguments
NETWORK="localhost"
for arg in "$@"; do
    case $arg in
        --devnet)
            NETWORK="devnet"
            shift
            ;;
        --testnet)
            NETWORK="testnet"
            shift
            ;;
        --mainnet)
            echo -e "${RED}ERROR: Mainnet deployment requires additional safety checks.${NC}"
            echo "This script does not support mainnet deployment."
            exit 1
            ;;
    esac
done

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║         PERCOLATOR DEPLOYMENT                              ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check if Solana CLI is available
if ! command -v solana &> /dev/null; then
    echo -e "${RED}Error: Solana CLI not found.${NC}"
    echo "Install with:"
    echo "  sh -c \"\$(curl -sSfL https://release.anza.xyz/v2.1.0/install)\""
    exit 1
fi

echo -e "${GREEN}Solana CLI version:${NC} $(solana --version)"
echo ""

# Check if programs are built
if [ ! -f "$SLAB_SO" ]; then
    echo -e "${RED}Error: Slab program not built at $SLAB_SO${NC}"
    echo "Run: cargo build-sbf --manifest-path programs/slab/Cargo.toml --features bpf-entrypoint"
    exit 1
fi

if [ ! -f "$ROUTER_SO" ]; then
    echo -e "${RED}Error: Router program not built at $ROUTER_SO${NC}"
    echo "Run: cargo build-sbf --manifest-path programs/router/Cargo.toml --features bpf-entrypoint"
    exit 1
fi

# Configure network
echo -e "${YELLOW}Configuring for $NETWORK...${NC}"
solana config set --url "$NETWORK" > /dev/null

# Verify connection
echo -e "${GREEN}Network:${NC} $(solana config get | grep 'RPC URL')"

if ! solana cluster-version &> /dev/null; then
    echo -e "${RED}Error: Cannot connect to $NETWORK${NC}"
    if [ "$NETWORK" = "localhost" ]; then
        echo "Start the local validator first:"
        echo "  ./scripts/start-validator.sh"
    fi
    exit 1
fi

echo -e "${GREEN}Cluster version:${NC} $(solana cluster-version)"
echo ""

# Check wallet balance
BALANCE=$(solana balance 2>/dev/null | grep -oP '\d+\.?\d*' || echo "0")
echo -e "${GREEN}Wallet balance:${NC} $BALANCE SOL"

if (( $(echo "$BALANCE < 1" | bc -l) )); then
    if [ "$NETWORK" = "localhost" ]; then
        echo -e "${YELLOW}Requesting airdrop...${NC}"
        solana airdrop 10 || true
        sleep 2
        BALANCE=$(solana balance | grep -oP '\d+\.?\d*')
        echo -e "${GREEN}New balance:${NC} $BALANCE SOL"
    else
        echo -e "${RED}Warning: Low balance. Deployment may fail.${NC}"
    fi
fi
echo ""

# Display program sizes
echo -e "${GREEN}Program Sizes:${NC}"
echo "  Slab:   $(du -h "$SLAB_SO" | cut -f1) ($SLAB_SO)"
echo "  Router: $(du -h "$ROUTER_SO" | cut -f1) ($ROUTER_SO)"
echo ""

# Check if programs are already deployed
check_program_deployed() {
    local program_id=$1
    if solana program show "$program_id" &> /dev/null; then
        return 0
    else
        return 1
    fi
}

# Deploy Slab program
echo -e "${YELLOW}Deploying Slab program...${NC}"
echo "  Program ID: $SLAB_PROGRAM_ID"

if check_program_deployed "$SLAB_PROGRAM_ID"; then
    echo -e "${YELLOW}  Program already deployed. Upgrading...${NC}"
    solana program deploy "$SLAB_SO" --program-id "$SLAB_PROGRAM_ID" --upgrade-authority "$(solana address)"
else
    # For fresh deployment, we need the program keypair
    SLAB_KEYPAIR="$PROJECT_ROOT/target/deploy/percolator_slab-keypair.json"
    if [ -f "$SLAB_KEYPAIR" ]; then
        solana program deploy "$SLAB_SO" --program-id "$SLAB_KEYPAIR"
    else
        solana program deploy "$SLAB_SO"
    fi
fi

echo -e "${GREEN}  ✓ Slab program deployed${NC}"
echo ""

# Deploy Router program
echo -e "${YELLOW}Deploying Router program...${NC}"
echo "  Program ID: $ROUTER_PROGRAM_ID"

if check_program_deployed "$ROUTER_PROGRAM_ID"; then
    echo -e "${YELLOW}  Program already deployed. Upgrading...${NC}"
    solana program deploy "$ROUTER_SO" --program-id "$ROUTER_PROGRAM_ID" --upgrade-authority "$(solana address)"
else
    ROUTER_KEYPAIR="$PROJECT_ROOT/target/deploy/percolator_router-keypair.json"
    if [ -f "$ROUTER_KEYPAIR" ]; then
        solana program deploy "$ROUTER_SO" --program-id "$ROUTER_KEYPAIR"
    else
        solana program deploy "$ROUTER_SO"
    fi
fi

echo -e "${GREEN}  ✓ Router program deployed${NC}"
echo ""

# Save deployment info
DEPLOY_INFO="$PROJECT_ROOT/.env.local"
cat > "$DEPLOY_INFO" << EOF
# Percolator Deployment Info
# Generated: $(date -u +"%Y-%m-%d %H:%M:%S UTC")
# Network: $NETWORK

SLAB_PROGRAM_ID=$SLAB_PROGRAM_ID
ROUTER_PROGRAM_ID=$ROUTER_PROGRAM_ID
NETWORK=$NETWORK
EOF

echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║         DEPLOYMENT COMPLETE                                ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo "Program IDs:"
echo "  Slab:   $SLAB_PROGRAM_ID"
echo "  Router: $ROUTER_PROGRAM_ID"
echo ""
echo "Deployment info saved to: $DEPLOY_INFO"
echo ""
echo -e "${BLUE}Next steps:${NC}"
echo "  1. Initialize the Router registry"
echo "  2. Register the Slab program with the Router"
echo "  3. Create test portfolios and execute trades"
echo ""
echo "Monitor logs:"
echo "  solana logs | grep -E '(SLabZ|RoutR)'"
