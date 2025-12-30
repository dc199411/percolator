#!/bin/bash
# Start local Solana test validator with Percolator programs pre-loaded
#
# Usage:
#   ./scripts/start-validator.sh           # Start with default settings
#   ./scripts/start-validator.sh --quiet   # Start without verbose logging
#
# Prerequisites:
#   - Solana CLI installed (solana --version)
#   - Programs built (cargo build-sbf)

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
QUIET_MODE=false
for arg in "$@"; do
    case $arg in
        --quiet|-q)
            QUIET_MODE=true
            shift
            ;;
    esac
done

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║         PERCOLATOR LOCAL VALIDATOR                         ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check if Solana CLI is available
if ! command -v solana-test-validator &> /dev/null; then
    echo -e "${RED}Error: solana-test-validator not found.${NC}"
    echo "Install Solana CLI:"
    echo "  sh -c \"\$(curl -sSfL https://release.anza.xyz/v2.1.0/install)\""
    echo "  export PATH=\"\$HOME/.local/share/solana/install/active_release/bin:\$PATH\""
    exit 1
fi

# Check if programs are built
if [ ! -f "$SLAB_SO" ] || [ ! -f "$ROUTER_SO" ]; then
    echo -e "${YELLOW}Programs not built. Building now...${NC}"
    cd "$PROJECT_ROOT"
    
    echo "Building Slab program..."
    cargo build-sbf --manifest-path programs/slab/Cargo.toml --features bpf-entrypoint
    
    echo "Building Router program..."
    cargo build-sbf --manifest-path programs/router/Cargo.toml --features bpf-entrypoint
    
    echo -e "${GREEN}Build complete!${NC}"
fi

# Display program info
echo -e "${GREEN}Program Configuration:${NC}"
echo "  Slab Program:"
echo "    ID: $SLAB_PROGRAM_ID"
echo "    Size: $(du -h "$SLAB_SO" | cut -f1)"
echo "  Router Program:"
echo "    ID: $ROUTER_PROGRAM_ID"
echo "    Size: $(du -h "$ROUTER_SO" | cut -f1)"
echo ""

# Validator configuration
COMPUTE_LIMIT=1400000
LEDGER_DIR="$PROJECT_ROOT/.ledger"

echo -e "${GREEN}Validator Configuration:${NC}"
echo "  Compute Unit Limit: $COMPUTE_LIMIT"
echo "  Ledger Directory: $LEDGER_DIR"
echo ""

# Cleanup old ledger
if [ -d "$LEDGER_DIR" ]; then
    echo -e "${YELLOW}Removing old ledger...${NC}"
    rm -rf "$LEDGER_DIR"
fi

echo -e "${GREEN}Starting validator...${NC}"
echo -e "${YELLOW}Press Ctrl+C to stop${NC}"
echo ""

# Start validator
if [ "$QUIET_MODE" = true ]; then
    solana-test-validator \
        --bpf-program "$SLAB_PROGRAM_ID" "$SLAB_SO" \
        --bpf-program "$ROUTER_PROGRAM_ID" "$ROUTER_SO" \
        --compute-unit-limit "$COMPUTE_LIMIT" \
        --ledger "$LEDGER_DIR" \
        --reset \
        --quiet
else
    solana-test-validator \
        --bpf-program "$SLAB_PROGRAM_ID" "$SLAB_SO" \
        --bpf-program "$ROUTER_PROGRAM_ID" "$ROUTER_SO" \
        --compute-unit-limit "$COMPUTE_LIMIT" \
        --ledger "$LEDGER_DIR" \
        --reset \
        --log
fi
