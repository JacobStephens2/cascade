# Observability for cascade-sync

Monitoring-as-code for the listening-time sync service: the app exposes
Prometheus metrics, and this directory has the scrape jobs, alert rules, and a
Grafana dashboard to consume them. It replaces ad-hoc "curl /health in a loop"
checking with the standard Prometheus + Grafana + Alertmanager stack.

## Signals

The service emits (see `server/src/main.rs`):

- `cascade_sync_http_requests_total{method,path,status}` — request counter,
  labelled by the *matched route template* so cardinality stays bounded.
- `cascade_sync_http_request_duration_seconds{...}` — latency.

`/metrics` is bound to localhost and is **not** exposed through Apache, so
Prometheus scrapes it directly on `127.0.0.1:3470`. An external blackbox probe
of the public `/health` covers the Apache/TLS/DNS layer the app can't see.

## Files

| File | Purpose |
|---|---|
| `prometheus/cascade-sync.scrape.yml` | scrape jobs (app + blackbox probe) |
| `prometheus/cascade-sync.rules.yml`  | alerts: down, probe-failing, cert-expiry, 5xx rate, latency |
| `grafana/cascade-sync-dashboard.json` | dashboard (up, probe, cert days, req rate by route, status mix, error ratio, mean latency) |

## Apply

To an existing Prometheus (`/etc/prometheus/prometheus.yml`):

```yaml
scrape_config_files:
  - /etc/prometheus/cascade-sync.scrape.yml
rule_files:
  - /etc/prometheus/cascade-sync.rules.yml
```

then `promtool check rules cascade-sync.rules.yml && systemctl reload prometheus`.
Import the dashboard JSON in Grafana (it prompts for the Prometheus data source).

The blackbox job assumes `prometheus-blackbox-exporter` on `127.0.0.1:9115`
with an `http_2xx` module (the package default).

## Provisioning with Ansible

On Ubuntu the stack is `apt install prometheus prometheus-blackbox-exporter
grafana` (Grafana via its apt repo), then drop these files into
`/etc/prometheus/` and Grafana's provisioning dir. This mirrors the
`cascade_sync` role pattern; wire it as an `observability` role when the stack
is stood up. Until then the app already emits metrics and these configs are
ready to point at it.
