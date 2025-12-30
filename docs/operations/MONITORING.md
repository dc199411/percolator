# Percolator Protocol Monitoring Guide

This guide covers monitoring setup, metrics collection, and alerting for Percolator Protocol.

## Quick Start

```bash
# Start monitoring stack (Docker)
cd scripts/monitoring
docker-compose up -d

# Access dashboards
# Grafana: http://localhost:3000 (admin/admin)
# Prometheus: http://localhost:9090
```

## Architecture

```
┌───────────────┐     ┌───────────────┐     ┌───────────────┐
│   Percolator  │────▶│  Prometheus   │────▶│   Grafana     │
│   Exporter    │     │               │     │               │
└───────────────┘     └───────────────┘     └───────────────┘
        │                                           │
        ▼                                           ▼
┌───────────────┐                          ┌───────────────┐
│ Solana RPC    │                          │  AlertManager │
│ (data source) │                          │  (PagerDuty)  │
└───────────────┘                          └───────────────┘
```

## Metrics Reference

### Protocol Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `percolator_tvl_usdc` | Gauge | Total USDC in router vault |
| `percolator_open_interest_usdc` | Gauge | Total open interest |
| `percolator_volume_24h_usdc` | Counter | 24-hour trading volume |
| `percolator_portfolios_total` | Gauge | Total user portfolios |
| `percolator_slabs_active` | Gauge | Number of active slabs |

### Slab Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `percolator_slab_oi` | Gauge | slab, instrument | Open interest per instrument |
| `percolator_slab_volume` | Counter | slab, instrument | Volume per instrument |
| `percolator_slab_orders` | Gauge | slab, side | Open orders count |
| `percolator_slab_spread_bps` | Gauge | slab | Bid-ask spread |
| `percolator_slab_depth_usdc` | Gauge | slab, side, pct | Orderbook depth |

### Insurance Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `percolator_insurance_balance` | Gauge | slab | Insurance pool balance |
| `percolator_insurance_ratio` | Gauge | slab | Balance / OI ratio |
| `percolator_insurance_contributions` | Counter | slab | Total contributions |
| `percolator_insurance_payouts` | Counter | slab | Total payouts |
| `percolator_insurance_adl_events` | Counter | slab | ADL events count |

### Performance Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `percolator_tx_total` | Counter | status | Transaction count |
| `percolator_tx_latency` | Histogram | | Transaction latency |
| `percolator_cu_used` | Histogram | instruction | Compute units used |
| `percolator_rpc_latency` | Histogram | | RPC call latency |

### Risk Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `percolator_liquidations_total` | Counter | Total liquidations |
| `percolator_liquidation_volume_usdc` | Counter | Liquidation volume |
| `percolator_portfolios_at_risk` | Gauge | Portfolios near liquidation |
| `percolator_funding_rate` | Gauge | Current funding rate |

## Grafana Dashboards

### Dashboard 1: Protocol Overview

```json
{
  "title": "Percolator - Protocol Overview",
  "panels": [
    {
      "title": "Total Value Locked",
      "type": "stat",
      "targets": [{"expr": "percolator_tvl_usdc"}]
    },
    {
      "title": "24h Volume",
      "type": "stat",
      "targets": [{"expr": "increase(percolator_volume_24h_usdc[24h])"}]
    },
    {
      "title": "Open Interest",
      "type": "timeseries",
      "targets": [{"expr": "percolator_open_interest_usdc"}]
    },
    {
      "title": "Active Users",
      "type": "stat",
      "targets": [{"expr": "percolator_portfolios_total"}]
    }
  ]
}
```

### Dashboard 2: Risk Monitoring

```json
{
  "title": "Percolator - Risk Dashboard",
  "panels": [
    {
      "title": "Insurance Coverage",
      "type": "gauge",
      "targets": [{"expr": "percolator_insurance_ratio * 100"}],
      "thresholds": [{"value": 1, "color": "red"}, {"value": 2, "color": "yellow"}, {"value": 5, "color": "green"}]
    },
    {
      "title": "Liquidations per Hour",
      "type": "timeseries",
      "targets": [{"expr": "rate(percolator_liquidations_total[1h]) * 3600"}]
    },
    {
      "title": "Portfolios at Risk",
      "type": "stat",
      "targets": [{"expr": "percolator_portfolios_at_risk"}]
    },
    {
      "title": "Funding Rate",
      "type": "timeseries",
      "targets": [{"expr": "percolator_funding_rate * 100"}]
    }
  ]
}
```

### Dashboard 3: Slab Performance

```json
{
  "title": "Percolator - Slab Performance",
  "panels": [
    {
      "title": "Orderbook Depth",
      "type": "timeseries",
      "targets": [
        {"expr": "percolator_slab_depth_usdc{side='bid',pct='1'}"},
        {"expr": "percolator_slab_depth_usdc{side='ask',pct='1'}"}
      ]
    },
    {
      "title": "Spread",
      "type": "timeseries",
      "targets": [{"expr": "percolator_slab_spread_bps"}]
    },
    {
      "title": "Order Count",
      "type": "timeseries",
      "targets": [{"expr": "percolator_slab_orders"}]
    }
  ]
}
```

## Alert Rules

### Critical Alerts (P1)

```yaml
groups:
  - name: percolator_critical
    rules:
      # Insurance depleted
      - alert: InsuranceDepleted
        expr: percolator_insurance_balance == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Insurance pool depleted for {{ $labels.slab }}"
          description: "Insurance pool has zero balance. ADL will be triggered."

      # Mass liquidations
      - alert: MassLiquidations
        expr: rate(percolator_liquidations_total[5m]) * 60 > 20
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Mass liquidation event detected"
          description: "More than 20 liquidations per minute in the last 5 minutes."

      # Program unavailable
      - alert: ProgramUnavailable
        expr: up{job="percolator"} == 0
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "Percolator program unreachable"
          description: "Cannot connect to Percolator programs for 2+ minutes."
```

### Warning Alerts (P2)

```yaml
groups:
  - name: percolator_warning
    rules:
      # Low insurance coverage
      - alert: LowInsuranceCoverage
        expr: percolator_insurance_ratio < 0.02
        for: 15m
        labels:
          severity: warning
        annotations:
          summary: "Low insurance coverage for {{ $labels.slab }}"
          description: "Insurance coverage below 2% of open interest."

      # High liquidation rate
      - alert: HighLiquidationRate
        expr: rate(percolator_liquidations_total[1h]) * 3600 > 10
        for: 30m
        labels:
          severity: warning
        annotations:
          summary: "Elevated liquidation rate"
          description: "More than 10 liquidations per hour."

      # Wide spread
      - alert: WideSpread
        expr: percolator_slab_spread_bps > 50
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Wide spread on {{ $labels.slab }}"
          description: "Bid-ask spread exceeding 50 basis points."

      # Low orderbook depth
      - alert: LowOrderbookDepth
        expr: percolator_slab_depth_usdc{pct="1"} < 10000
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Low orderbook depth on {{ $labels.slab }}"
          description: "Less than $10K liquidity within 1% of mid price."
```

### Info Alerts (P3/P4)

```yaml
groups:
  - name: percolator_info
    rules:
      # High funding rate
      - alert: HighFundingRate
        expr: abs(percolator_funding_rate) > 0.001
        for: 1h
        labels:
          severity: info
        annotations:
          summary: "High funding rate on {{ $labels.instrument }}"
          description: "Funding rate exceeds 0.1% (8h)."

      # Transaction failures
      - alert: TransactionFailures
        expr: rate(percolator_tx_total{status="failed"}[5m]) > 0.1
        for: 10m
        labels:
          severity: info
        annotations:
          summary: "Elevated transaction failures"
          description: "More than 10% of transactions failing."
```

## Exporter Implementation

### Rust Exporter Service

```rust
// scripts/monitoring/src/main.rs
use prometheus::{Encoder, GaugeVec, TextEncoder, Registry};
use solana_client::rpc_client::RpcClient;
use std::net::SocketAddr;
use warp::Filter;

#[tokio::main]
async fn main() {
    let registry = Registry::new();
    
    // Define metrics
    let tvl_gauge = GaugeVec::new(
        prometheus::Opts::new("percolator_tvl_usdc", "Total value locked in USDC"),
        &[]
    ).unwrap();
    registry.register(Box::new(tvl_gauge.clone())).unwrap();
    
    let insurance_gauge = GaugeVec::new(
        prometheus::Opts::new("percolator_insurance_balance", "Insurance pool balance"),
        &["slab"]
    ).unwrap();
    registry.register(Box::new(insurance_gauge.clone())).unwrap();
    
    // Start metrics server
    let metrics_route = warp::path!("metrics")
        .map(move || {
            let encoder = TextEncoder::new();
            let metric_families = registry.gather();
            let mut buffer = Vec::new();
            encoder.encode(&metric_families, &mut buffer).unwrap();
            String::from_utf8(buffer).unwrap()
        });
    
    let addr: SocketAddr = "0.0.0.0:9090".parse().unwrap();
    warp::serve(metrics_route).run(addr).await;
}
```

### Python Exporter (Alternative)

```python
#!/usr/bin/env python3
# scripts/monitoring/exporter.py

from prometheus_client import start_http_server, Gauge
from solana.rpc.api import Client
import time

# Metrics
tvl = Gauge('percolator_tvl_usdc', 'Total value locked')
insurance = Gauge('percolator_insurance_balance', 'Insurance balance', ['slab'])
oi = Gauge('percolator_open_interest_usdc', 'Open interest')
liquidations = Gauge('percolator_liquidations_total', 'Total liquidations')

def collect_metrics():
    client = Client("https://api.mainnet-beta.solana.com")
    
    # Fetch vault balance
    vault_info = client.get_account_info(VAULT_PUBKEY)
    tvl.set(vault_info.value.lamports / 1e6)
    
    # Fetch insurance balances
    for slab_name, slab_addr in SLABS.items():
        insurance_info = client.get_account_info(slab_addr)
        insurance.labels(slab=slab_name).set(parse_insurance_balance(insurance_info))

if __name__ == '__main__':
    start_http_server(9090)
    while True:
        collect_metrics()
        time.sleep(30)
```

## Docker Compose Setup

```yaml
# scripts/monitoring/docker-compose.yml
version: '3.8'

services:
  prometheus:
    image: prom/prometheus:latest
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - ./alerts:/etc/prometheus/alerts
      - prometheus_data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--web.enable-lifecycle'
    ports:
      - "9090:9090"

  grafana:
    image: grafana/grafana:latest
    volumes:
      - ./grafana/provisioning:/etc/grafana/provisioning
      - grafana_data:/var/lib/grafana
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
      - GF_USERS_ALLOW_SIGN_UP=false
    ports:
      - "3000:3000"
    depends_on:
      - prometheus

  alertmanager:
    image: prom/alertmanager:latest
    volumes:
      - ./alertmanager.yml:/etc/alertmanager/alertmanager.yml
    ports:
      - "9093:9093"

  percolator-exporter:
    build: ./exporter
    environment:
      - SOLANA_RPC_URL=${SOLANA_RPC_URL}
    ports:
      - "9091:9091"

volumes:
  prometheus_data:
  grafana_data:
```

## Prometheus Configuration

```yaml
# scripts/monitoring/prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

alerting:
  alertmanagers:
    - static_configs:
        - targets:
          - alertmanager:9093

rule_files:
  - "/etc/prometheus/alerts/*.yml"

scrape_configs:
  - job_name: 'percolator'
    static_configs:
      - targets: ['percolator-exporter:9091']

  - job_name: 'prometheus'
    static_configs:
      - targets: ['localhost:9090']
```

## AlertManager Configuration

```yaml
# scripts/monitoring/alertmanager.yml
global:
  resolve_timeout: 5m

route:
  group_by: ['alertname', 'severity']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 1h
  receiver: 'default'
  routes:
    - match:
        severity: critical
      receiver: 'pagerduty-critical'
    - match:
        severity: warning
      receiver: 'slack-warning'

receivers:
  - name: 'default'
    slack_configs:
      - api_url: '${SLACK_WEBHOOK_URL}'
        channel: '#percolator-alerts'

  - name: 'pagerduty-critical'
    pagerduty_configs:
      - service_key: '${PAGERDUTY_SERVICE_KEY}'
        severity: critical

  - name: 'slack-warning'
    slack_configs:
      - api_url: '${SLACK_WEBHOOK_URL}'
        channel: '#percolator-alerts'
```

## Health Check Endpoints

```bash
# Check exporter health
curl http://localhost:9091/health

# Check metrics
curl http://localhost:9091/metrics

# Check Prometheus targets
curl http://localhost:9090/api/v1/targets

# Check AlertManager status
curl http://localhost:9093/api/v2/status
```

## Runbook Integration

See [RUNBOOK.md](./RUNBOOK.md) for:
- Alert response procedures
- Escalation paths
- Emergency contacts
