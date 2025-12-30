#!/bin/bash
# Start local Solana test validator with Percolator programs pre-loaded
#
# This script starts the validator with:
# - Slab and Router programs pre-deployed
# - Increased compute budget for testing
# - Verbose logging enabled

set -e

SLAB_SO="./target/deploy/percolator_slab.so"
ROUTER_SO="./target/deploy/percolator_router.so"

# Check if programs are built
if [ ! -f "$SLAB_SO" ] || [ ! -f "$ROUTER_SO" ]; then
    echo "Programs not built. Building now..."
    cargo build-sbf --manifest-path programs/slab/Cargo.toml --features bpf-entrypoint
    cargo build-sbf --manifest-path programs/router/Cargo.toml --features bpf-entrypoint
fi

# Program IDs from lib.rs
SLAB_PROGRAM_ID="SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk"
ROUTER_PROGRAM_ID="RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr"

echo "Starting local validator..."
echo "Slab Program ID: $SLAB_PROGRAM_ID"
echo "Router Program ID: $ROUTER_PROGRAM_ID"
echo ""
echo "Press Ctrl+C to stop the validator"
echo ""

solana-test-validator \
    --bpf-program $SLAB_PROGRAM_ID $SLAB_SO \
    --bpf-program $ROUTER_PROGRAM_ID $ROUTER_SO \
    --compute-unit-limit 1400000 \
    --reset \
    --log
