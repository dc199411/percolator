#!/bin/bash
# Percolator Protocol Alert Handler
#
# Receives alerts from AlertManager and executes appropriate actions
#
# Usage: ./alert_handler.sh <alert_json>

set -e

ALERT_JSON="$1"

if [ -z "$ALERT_JSON" ]; then
    echo "Usage: $0 <alert_json>"
    exit 1
fi

# Parse alert details
ALERT_NAME=$(echo "$ALERT_JSON" | jq -r '.commonLabels.alertname // "Unknown"')
SEVERITY=$(echo "$ALERT_JSON" | jq -r '.commonLabels.severity // "info"')
STATUS=$(echo "$ALERT_JSON" | jq -r '.status // "unknown"')
DESCRIPTION=$(echo "$ALERT_JSON" | jq -r '.commonAnnotations.description // "No description"')

log() {
    echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] $1"
}

# =============================================================================
# ALERT HANDLERS
# =============================================================================

handle_insurance_depleted() {
    log "CRITICAL: Insurance pool depleted!"
    log "Action: Notifying on-call team immediately"
    
    # Send critical notification
    if [ -n "$PAGERDUTY_API_KEY" ]; then
        curl -X POST https://events.pagerduty.com/v2/enqueue \
            -H "Content-Type: application/json" \
            -d "{
                \"routing_key\": \"$PAGERDUTY_API_KEY\",
                \"event_action\": \"trigger\",
                \"payload\": {
                    \"summary\": \"Percolator Insurance Pool Depleted\",
                    \"severity\": \"critical\",
                    \"source\": \"percolator-monitor\"
                }
            }"
    fi
    
    # Log to incident channel
    if [ -n "$SLACK_WEBHOOK_URL" ]; then
        curl -X POST "$SLACK_WEBHOOK_URL" \
            -H "Content-Type: application/json" \
            -d "{
                \"channel\": \"#incidents\",
                \"text\": \"🚨 CRITICAL: Percolator insurance pool depleted. ADL triggered.\",
                \"attachments\": [{
                    \"color\": \"danger\",
                    \"text\": \"$DESCRIPTION\"
                }]
            }"
    fi
}

handle_mass_liquidations() {
    log "CRITICAL: Mass liquidation event detected!"
    log "Action: Monitoring cascade and preparing response"
    
    # Check current liquidation queue
    # ./health_check.sh --json | jq '.liquidation_rate'
    
    # Notify trading desk
    if [ -n "$SLACK_WEBHOOK_URL" ]; then
        curl -X POST "$SLACK_WEBHOOK_URL" \
            -H "Content-Type: application/json" \
            -d "{
                \"channel\": \"#trading-desk\",
                \"text\": \"⚠️ Mass liquidation event in progress\",
                \"attachments\": [{
                    \"color\": \"warning\",
                    \"text\": \"$DESCRIPTION\"
                }]
            }"
    fi
}

handle_low_insurance_coverage() {
    log "WARNING: Low insurance coverage detected"
    log "Action: Alerting LP team for potential contribution"
    
    if [ -n "$SLACK_WEBHOOK_URL" ]; then
        curl -X POST "$SLACK_WEBHOOK_URL" \
            -H "Content-Type: application/json" \
            -d "{
                \"channel\": \"#percolator-ops\",
                \"text\": \"📊 Insurance coverage below threshold\",
                \"attachments\": [{
                    \"color\": \"warning\",
                    \"text\": \"$DESCRIPTION. Consider LP contribution.\"
                }]
            }"
    fi
}

handle_wide_spread() {
    log "WARNING: Wide spread detected"
    log "Action: Monitoring liquidity conditions"
    
    # Could trigger market maker alerts here
    if [ -n "$SLACK_WEBHOOK_URL" ]; then
        curl -X POST "$SLACK_WEBHOOK_URL" \
            -H "Content-Type: application/json" \
            -d "{
                \"channel\": \"#percolator-ops\",
                \"text\": \"📈 Wide spread alert\",
                \"attachments\": [{
                    \"color\": \"warning\",
                    \"text\": \"$DESCRIPTION\"
                }]
            }"
    fi
}

handle_low_orderbook_depth() {
    log "WARNING: Low orderbook depth"
    log "Action: Notifying market makers"
    
    if [ -n "$SLACK_WEBHOOK_URL" ]; then
        curl -X POST "$SLACK_WEBHOOK_URL" \
            -H "Content-Type: application/json" \
            -d "{
                \"channel\": \"#market-makers\",
                \"text\": \"📉 Low orderbook depth alert\",
                \"attachments\": [{
                    \"color\": \"warning\",
                    \"text\": \"$DESCRIPTION\"
                }]
            }"
    fi
}

handle_program_unavailable() {
    log "CRITICAL: Program unavailable!"
    log "Action: Checking RPC and initiating failover"
    
    # Try backup RPC
    BACKUP_RPC="${BACKUP_SOLANA_RPC_URL:-}"
    if [ -n "$BACKUP_RPC" ]; then
        log "Checking backup RPC: $BACKUP_RPC"
        if solana cluster-version -u "$BACKUP_RPC" &> /dev/null; then
            log "Backup RPC available - consider failover"
        else
            log "Backup RPC also unavailable"
        fi
    fi
    
    # Critical alert
    if [ -n "$PAGERDUTY_API_KEY" ]; then
        curl -X POST https://events.pagerduty.com/v2/enqueue \
            -H "Content-Type: application/json" \
            -d "{
                \"routing_key\": \"$PAGERDUTY_API_KEY\",
                \"event_action\": \"trigger\",
                \"payload\": {
                    \"summary\": \"Percolator Program Unavailable\",
                    \"severity\": \"critical\",
                    \"source\": \"percolator-monitor\"
                }
            }"
    fi
}

handle_resolved() {
    log "RESOLVED: $ALERT_NAME"
    
    if [ -n "$SLACK_WEBHOOK_URL" ]; then
        curl -X POST "$SLACK_WEBHOOK_URL" \
            -H "Content-Type: application/json" \
            -d "{
                \"channel\": \"#percolator-ops\",
                \"text\": \"✅ Alert resolved: $ALERT_NAME\"
            }"
    fi
}

# =============================================================================
# MAIN
# =============================================================================

log "Received alert: $ALERT_NAME (severity: $SEVERITY, status: $STATUS)"

if [ "$STATUS" = "resolved" ]; then
    handle_resolved
    exit 0
fi

case "$ALERT_NAME" in
    InsuranceDepleted)
        handle_insurance_depleted
        ;;
    MassLiquidations)
        handle_mass_liquidations
        ;;
    LowInsuranceCoverage)
        handle_low_insurance_coverage
        ;;
    WideSpread)
        handle_wide_spread
        ;;
    LowOrderbookDepth)
        handle_low_orderbook_depth
        ;;
    ProgramUnavailable)
        handle_program_unavailable
        ;;
    *)
        log "No specific handler for $ALERT_NAME, sending generic notification"
        if [ -n "$SLACK_WEBHOOK_URL" ]; then
            curl -X POST "$SLACK_WEBHOOK_URL" \
                -H "Content-Type: application/json" \
                -d "{
                    \"channel\": \"#percolator-ops\",
                    \"text\": \"⚠️ Alert: $ALERT_NAME\",
                    \"attachments\": [{
                        \"color\": \"warning\",
                        \"text\": \"$DESCRIPTION\"
                    }]
                }"
        fi
        ;;
esac

log "Alert handling complete"
