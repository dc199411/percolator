# Percolator Protocol Operations Runbook

This document provides operational procedures for running and maintaining Percolator Protocol infrastructure.

## Table of Contents

1. [System Overview](#system-overview)
2. [Deployment Procedures](#deployment-procedures)
3. [Monitoring & Alerting](#monitoring--alerting)
4. [Incident Response](#incident-response)
5. [Maintenance Procedures](#maintenance-procedures)
6. [Emergency Procedures](#emergency-procedures)

---

## System Overview

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Solana Cluster                         │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │   Router    │───▶│   Slab 1    │    │   Slab 2    │     │
│  │   Program   │───▶│  (BTC-PERP) │    │  (ETH-PERP) │     │
│  └─────────────┘    └─────────────┘    └─────────────┘     │
│         │                  │                  │             │
│         ▼                  ▼                  ▼             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │   Registry  │    │  Insurance  │    │  Insurance  │     │
│  │    Vault    │    │    Pool 1   │    │    Pool 2   │     │
│  └─────────────┘    └─────────────┘    └─────────────┘     │
└─────────────────────────────────────────────────────────────┘
```

### Program IDs

| Program | Devnet | Mainnet |
|---------|--------|---------|
| Router | `RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr` | TBD |
| Slab | `SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk` | TBD |

### Key Accounts

- **Router Registry**: Stores registered slabs and global state
- **Router Vault**: Holds USDC collateral for all users
- **Slab State**: Per-slab orderbook and position data
- **Insurance Pool**: Per-slab insurance fund

---

## Deployment Procedures

### Pre-Deployment Checklist

- [ ] All tests passing (`cargo test --release`)
- [ ] BPF build successful (`cargo build-sbf`)
- [ ] Security audit complete (for mainnet)
- [ ] Multisig configured for upgrades
- [ ] Monitoring dashboards ready
- [ ] Alert thresholds configured
- [ ] Incident response team notified

### Deploy to Devnet

```bash
# 1. Build programs
cargo build-sbf

# 2. Deploy Router
solana program deploy \
  --program-id RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr \
  target/deploy/percolator_router.so

# 3. Deploy Slab
solana program deploy \
  --program-id SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk \
  target/deploy/percolator_slab.so

# 4. Initialize Router Registry
percolator config set-rpc https://api.devnet.solana.com
percolator portfolio init  # Creates admin portfolio first
```

### Deploy to Mainnet

```bash
# 1. Verify program binaries (reproducible build)
./scripts/verify_build.sh

# 2. Deploy via multisig (Squads or similar)
# - Create proposal to deploy program
# - Collect required signatures
# - Execute deployment

# 3. Initialize with production parameters
# - Set appropriate IMR/MMR
# - Configure insurance pool
# - Add approved instruments
```

### Program Upgrade Procedure

```bash
# 1. Build new version
cargo build-sbf

# 2. Create buffer account
solana program write-buffer target/deploy/percolator_router.so

# 3. Set buffer authority (multisig)
solana program set-buffer-authority <BUFFER_PUBKEY> \
  --new-buffer-authority <MULTISIG_PUBKEY>

# 4. Initiate upgrade via multisig
# 5. After approval, finalize upgrade
solana program deploy --buffer <BUFFER_PUBKEY> \
  --program-id <PROGRAM_ID>
```

---

## Monitoring & Alerting

### Key Metrics

#### Protocol Health

| Metric | Description | Alert Threshold |
|--------|-------------|-----------------|
| `total_tvl` | Total USDC in router vault | < 80% of ATH |
| `insurance_coverage` | Insurance / OI ratio | < 1% |
| `liquidation_rate` | Liquidations per hour | > 10/hour |
| `failed_transactions` | TX failures per minute | > 5/min |

#### Slab Health

| Metric | Description | Alert Threshold |
|--------|-------------|-----------------|
| `open_interest` | Total OI per slab | N/A (monitoring) |
| `orderbook_depth` | Bid/ask liquidity | < $10K within 1% |
| `spread_bps` | Bid-ask spread | > 50 bps |
| `funding_rate` | 8h funding rate | > 0.1% |

#### System Health

| Metric | Description | Alert Threshold |
|--------|-------------|-----------------|
| `rpc_latency_ms` | RPC response time | > 500ms |
| `slot_behind` | Slots behind tip | > 10 slots |
| `cu_usage` | Average CU per TX | > 400K |

### Grafana Dashboard Queries

```promql
# Total Value Locked
percolator_vault_balance_usdc

# Insurance Coverage Ratio
percolator_insurance_balance / percolator_open_interest

# Liquidation Rate (per hour)
rate(percolator_liquidations_total[1h]) * 3600

# Transaction Success Rate
rate(percolator_tx_success_total[5m]) / 
rate(percolator_tx_total[5m]) * 100
```

### Alert Definitions

```yaml
# alerts.yaml
groups:
  - name: percolator
    rules:
      - alert: InsuranceCoverageLow
        expr: percolator_insurance_ratio < 0.01
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: Insurance coverage below 1%
          
      - alert: HighLiquidationRate
        expr: rate(percolator_liquidations_total[1h]) > 10
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: High liquidation rate detected
          
      - alert: RPCLatencyHigh
        expr: percolator_rpc_latency_p99 > 0.5
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: RPC latency exceeding 500ms
```

---

## Incident Response

### Severity Levels

| Level | Description | Response Time | Examples |
|-------|-------------|---------------|----------|
| P1 | Critical - Protocol at risk | < 15 min | Exploit, total halt |
| P2 | High - Degraded service | < 1 hour | Liquidation cascade |
| P3 | Medium - Partial impact | < 4 hours | Single slab issue |
| P4 | Low - Minor issue | < 24 hours | UI bug |

### Incident Response Procedure

#### 1. Detection & Triage
```
1. Acknowledge alert
2. Assess severity level
3. Create incident channel (#incident-YYYYMMDD)
4. Notify on-call team
```

#### 2. Investigation
```
1. Gather logs: solana logs <PROGRAM_ID> --follow
2. Check transaction history
3. Review recent deployments
4. Analyze affected accounts
```

#### 3. Mitigation
```
# If exploit detected:
1. Pause affected slab (if possible via governance)
2. Notify users via status page
3. Begin root cause analysis

# If network issue:
1. Check RPC provider status
2. Switch to backup RPC if needed
3. Monitor recovery
```

#### 4. Resolution & Post-mortem
```
1. Implement fix
2. Deploy (following upgrade procedure)
3. Verify resolution
4. Update status page
5. Schedule post-mortem (within 48h)
```

### Emergency Contacts

| Role | Contact | Escalation Path |
|------|---------|-----------------|
| On-call Engineer | PagerDuty | Slack → Phone |
| Protocol Lead | Direct | Slack → Phone |
| Security Team | security@percolator.xyz | Email → Phone |

---

## Maintenance Procedures

### Daily Tasks

- [ ] Review overnight alerts
- [ ] Check insurance pool balances
- [ ] Verify funding rate calculations
- [ ] Monitor orderbook depth

### Weekly Tasks

- [ ] Review liquidation reports
- [ ] Analyze fee revenue
- [ ] Check LP withdrawals queue
- [ ] Review CU optimization opportunities

### Monthly Tasks

- [ ] Security audit review
- [ ] Parameter optimization analysis
- [ ] Dependency updates
- [ ] Disaster recovery drill

### Slab Parameter Updates

```bash
# Update IMR/MMR for a slab
percolator slab update <SLAB_ADDRESS> \
  --imr-bps 600 \
  --mmr-bps 300

# Add new instrument
percolator slab add-instrument <SLAB_ADDRESS> \
  SOL-PERP \
  --tick-size 0.001 \
  --lot-size 1
```

### Insurance Pool Management

```bash
# Check insurance status
percolator insurance status <SLAB_ADDRESS>

# LP contribution
percolator insurance contribute <SLAB_ADDRESS> 10000

# Initiate withdrawal (starts 7-day timelock)
percolator insurance initiate-withdraw <SLAB_ADDRESS> 5000

# Complete withdrawal (after timelock)
percolator insurance complete-withdraw <SLAB_ADDRESS>
```

---

## Emergency Procedures

### Protocol Pause (Nuclear Option)

**Only use in extreme circumstances (active exploit)**

```bash
# 1. Document decision and get approval from 2+ team leads
# 2. Execute pause via multisig governance
# 3. Notify all users immediately

# If governance pause not available:
# Contact Solana Foundation for program freeze (last resort)
```

### Liquidation Cascade Response

```bash
# 1. Monitor liquidation queue
percolator info liquidatable

# 2. If cascade detected:
#    - Alert trading desk
#    - Prepare additional liquidity
#    - Consider temporary parameter adjustments

# 3. Post-cascade analysis
#    - Review affected portfolios
#    - Analyze insurance fund usage
#    - Report to governance
```

### Insurance Fund Depletion

```bash
# If insurance fund drops below ADL threshold:

# 1. ADL will automatically trigger
percolator info stats  # Check ADL status

# 2. Notify affected users
# 3. Prepare LP contribution if needed

# 4. Post-event:
#    - Review ADL parameters
#    - Consider contribution rate increase
#    - Report to governance
```

### RPC Provider Failure

```bash
# 1. Switch to backup RPC
export SOLANA_RPC_URL=https://backup-rpc.example.com
percolator config set-rpc $SOLANA_RPC_URL

# 2. Verify connectivity
solana cluster-version

# 3. Update all services to use backup RPC
# 4. Contact primary RPC provider
# 5. Monitor for recovery
```

---

## Appendix

### Useful Commands

```bash
# Check account balance
solana balance <PUBKEY>

# Get account info
solana account <PUBKEY>

# View recent transactions
solana transaction-history <PUBKEY> --limit 10

# Monitor logs
solana logs <PROGRAM_ID> --follow

# Get slot
solana slot

# Check cluster version
solana cluster-version
```

### Log Locations

| Service | Location |
|---------|----------|
| Validator | `/var/log/solana/validator.log` |
| Monitor | `/var/log/percolator/monitor.log` |
| Alerts | Grafana/PagerDuty |

### Related Documentation

- [Architecture Overview](./ARCHITECTURE.md)
- [Security Model](./SECURITY.md)
- [API Reference](./API.md)
- [SDK Documentation](../sdk/README.md)
