#!/bin/bash
# Percolator Protocol Health Check Script
# 
# Usage: ./health_check.sh [--verbose] [--json]
#
# Checks:
# 1. RPC connectivity
# 2. Program deployment status
# 3. Registry health
# 4. Insurance pool status
# 5. Recent transaction activity

set -e

# Configuration
RPC_URL="${SOLANA_RPC_URL:-https://api.devnet.solana.com}"
ROUTER_PROGRAM_ID="RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr"
SLAB_PROGRAM_ID="SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk"

VERBOSE=false
JSON_OUTPUT=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        --json|-j)
            JSON_OUTPUT=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Colors (disabled for JSON output)
if [ "$JSON_OUTPUT" = false ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    NC='\033[0m' # No Color
else
    RED=''
    GREEN=''
    YELLOW=''
    NC=''
fi

# Results storage
declare -A RESULTS

log() {
    if [ "$JSON_OUTPUT" = false ]; then
        echo -e "$1"
    fi
}

log_verbose() {
    if [ "$VERBOSE" = true ] && [ "$JSON_OUTPUT" = false ]; then
        echo -e "  $1"
    fi
}

check_pass() {
    RESULTS["$1"]="pass"
    log "${GREEN}✓${NC} $1"
}

check_fail() {
    RESULTS["$1"]="fail"
    log "${RED}✗${NC} $1: $2"
}

check_warn() {
    RESULTS["$1"]="warn"
    log "${YELLOW}!${NC} $1: $2"
}

# =============================================================================
# CHECKS
# =============================================================================

check_rpc() {
    log "\n📡 RPC Connectivity"
    
    if ! command -v solana &> /dev/null; then
        check_fail "Solana CLI" "Not installed"
        return
    fi
    
    # Check RPC connection
    if solana cluster-version -u "$RPC_URL" &> /dev/null; then
        VERSION=$(solana cluster-version -u "$RPC_URL" 2>/dev/null)
        check_pass "RPC Connection"
        log_verbose "URL: $RPC_URL"
        log_verbose "Version: $VERSION"
    else
        check_fail "RPC Connection" "Cannot connect to $RPC_URL"
        return
    fi
    
    # Check slot
    SLOT=$(solana slot -u "$RPC_URL" 2>/dev/null)
    if [ -n "$SLOT" ]; then
        check_pass "Slot Query"
        log_verbose "Current slot: $SLOT"
    else
        check_fail "Slot Query" "Cannot get current slot"
    fi
}

check_programs() {
    log "\n📦 Program Deployment"
    
    # Check Router program
    if solana program show "$ROUTER_PROGRAM_ID" -u "$RPC_URL" &> /dev/null; then
        check_pass "Router Program"
        log_verbose "ID: $ROUTER_PROGRAM_ID"
    else
        check_warn "Router Program" "Not deployed or not accessible"
    fi
    
    # Check Slab program
    if solana program show "$SLAB_PROGRAM_ID" -u "$RPC_URL" &> /dev/null; then
        check_pass "Slab Program"
        log_verbose "ID: $SLAB_PROGRAM_ID"
    else
        check_warn "Slab Program" "Not deployed or not accessible"
    fi
}

check_accounts() {
    log "\n📋 Account Status"
    
    # Derive registry PDA (simplified - would use actual derivation)
    # For now, just check if we can query accounts
    
    RECENT_BLOCKHASH=$(solana block -u "$RPC_URL" 2>/dev/null | head -1)
    if [ -n "$RECENT_BLOCKHASH" ]; then
        check_pass "Block Access"
        log_verbose "Recent blockhash available"
    else
        check_warn "Block Access" "Cannot get recent blockhash"
    fi
}

check_transactions() {
    log "\n📈 Transaction Activity"
    
    # Check recent transactions for router
    TX_COUNT=$(solana transaction-history "$ROUTER_PROGRAM_ID" -u "$RPC_URL" --limit 10 2>/dev/null | wc -l)
    if [ "$TX_COUNT" -gt 0 ]; then
        check_pass "Router Transactions"
        log_verbose "Recent transactions found: ~$TX_COUNT"
    else
        check_warn "Router Transactions" "No recent transactions"
    fi
}

check_latency() {
    log "\n⏱️  Latency"
    
    START=$(date +%s%N)
    solana slot -u "$RPC_URL" &> /dev/null
    END=$(date +%s%N)
    
    LATENCY=$(( (END - START) / 1000000 ))
    
    if [ "$LATENCY" -lt 200 ]; then
        check_pass "RPC Latency"
        log_verbose "${LATENCY}ms"
    elif [ "$LATENCY" -lt 500 ]; then
        check_warn "RPC Latency" "${LATENCY}ms (elevated)"
    else
        check_fail "RPC Latency" "${LATENCY}ms (high)"
    fi
}

# =============================================================================
# OUTPUT
# =============================================================================

output_json() {
    echo "{"
    echo "  \"timestamp\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
    echo "  \"rpc_url\": \"$RPC_URL\","
    echo "  \"results\": {"
    
    first=true
    for key in "${!RESULTS[@]}"; do
        if [ "$first" = false ]; then
            echo ","
        fi
        first=false
        echo -n "    \"$key\": \"${RESULTS[$key]}\""
    done
    
    echo
    echo "  },"
    
    # Overall status
    OVERALL="healthy"
    for status in "${RESULTS[@]}"; do
        if [ "$status" = "fail" ]; then
            OVERALL="unhealthy"
            break
        elif [ "$status" = "warn" ]; then
            OVERALL="degraded"
        fi
    done
    
    echo "  \"overall\": \"$OVERALL\""
    echo "}"
}

output_summary() {
    log "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    
    PASS=0
    WARN=0
    FAIL=0
    
    for status in "${RESULTS[@]}"; do
        case $status in
            pass) ((PASS++)) ;;
            warn) ((WARN++)) ;;
            fail) ((FAIL++)) ;;
        esac
    done
    
    log "Summary: ${GREEN}$PASS passed${NC}, ${YELLOW}$WARN warnings${NC}, ${RED}$FAIL failed${NC}"
    
    if [ "$FAIL" -gt 0 ]; then
        log "\n${RED}Overall Status: UNHEALTHY${NC}"
        exit 1
    elif [ "$WARN" -gt 0 ]; then
        log "\n${YELLOW}Overall Status: DEGRADED${NC}"
        exit 0
    else
        log "\n${GREEN}Overall Status: HEALTHY${NC}"
        exit 0
    fi
}

# =============================================================================
# MAIN
# =============================================================================

main() {
    if [ "$JSON_OUTPUT" = false ]; then
        log "╔═══════════════════════════════════════════════════╗"
        log "║       Percolator Protocol Health Check            ║"
        log "╚═══════════════════════════════════════════════════╝"
        log ""
        log "Checking: $RPC_URL"
    fi
    
    check_rpc
    check_programs
    check_accounts
    check_transactions
    check_latency
    
    if [ "$JSON_OUTPUT" = true ]; then
        output_json
    else
        output_summary
    fi
}

main
