# Read metrics with Prometheus

The hub gives the state of every check at `GET /metrics` in the Prometheus text format. Phase 3 does
not deploy a Prometheus server. This page shows how to point your own one at the hub. Spec: INT-001,
INT-002. Decision: D-24.

Prometheus needs API keys (WP-43, WP-50) and `/metrics` (WP-46) to be deployed in the hub image.

## 1. Make a read API key

1. Open Settings in the Anima client and make an API key.
2. Give it a name that says what it is for, for example `prometheus`.
3. Set the scope to READ. The key needs no more (D-23).
4. Set an expiry date. It must be at most one year from now.
5. Copy the secret. It starts with `anima_` and the hub shows it once. The hub keeps only its SHA-256.

Revoke the key in Settings to stop it at once.

## 2. Store the key on the scraper

Put the secret in a file that only the Prometheus user can read. The command is for a Linux scraper:
`install` reading `/dev/stdin` needs GNU coreutils, and the folder `/etc/prometheus` must exist
first.

```bash
# Run in bash, with sudo, on the Linux machine that runs Prometheus. Type the secret; do not paste it into chat.
sudo mkdir -p /etc/prometheus
read -rs -p 'API key: ' value; echo
printf '%s' "$value" | sudo install -m 0600 -o prometheus -g prometheus /dev/stdin /etc/prometheus/anima_api_key
unset value
```

The command gives the file to the user that runs Prometheus (`prometheus` here; use your own user name)
with mode 0600. Never commit it.

## 3. Add the scrape job

```yaml
# In prometheus.yml on the scraper.
scrape_configs:
  - job_name: anima
    scheme: https
    metrics_path: /metrics
    scrape_interval: 30s
    authorization:
      type: Bearer
      credentials_file: /etc/prometheus/anima_api_key
    static_configs:
      - targets: ["hub.anima.ing:443"]
```

The hub caches the result for 10 seconds, so a shorter `scrape_interval` only reads the same numbers.
Use 30s or 60s. A request without a token, or with a token below READ, is refused.

## 4. Series

The content type is `text/plain; version=0.0.4; charset=utf-8`. The series come from K7 in
[phase-3-contracts.md](../programme/phase-3-contracts.md).

| Series | Type | Value |
| --- | --- | --- |
| `anima_check_status` | gauge | 0 Down, 1 Up, 2 Degraded, 3 Maintenance, 4 Unknown (also paused) |
| `anima_check_up` | gauge | 1 for Up or Degraded, else 0 |
| `anima_check_response_time_seconds` | gauge | The last heartbeat; absent when the check has none |
| `anima_check_uptime_ratio` | gauge | 0 to 1, with the label `window` set to `1d`, `7d` or `30d` |
| `anima_check_certificate_days_left` | gauge | Only for checks with a certificate |
| `anima_check_domain_days_left` | gauge | Only for domain expiry checks |
| `anima_device_status` | gauge | Same values as `anima_check_status`; labels `device_id`, `device_name`, `site` |

Labels of the check series: `check_id`, `check_name`, `check_type`, `device_id`, `device_name`, `site`.
No label holds a status, a time, an address or a message, so a series does not change identity when
a check changes state.

## 5. The scraper must be on the VPN

`hub.anima.ing` answers only machines on the VPN.

- A machine of the Headscale user `operator@` needs no change. The policy already lets `operator@`
  reach `tag:hub` on tcp:443.
- A machine with a tag needs a Headscale grant to `tag:hub` on tcp:443. That is a change to the VPN
  policy (VPN-004) and needs your confirmation (VPS-005). The grant has this shape, with your own tag:

  ```jsonc
  // Example only. Add to "grants" in deploy/headscale/config/policy.hujson after you confirm it.
  {"src": ["tag:prometheus"], "dst": ["tag:hub"], "ip": ["tcp:443"]}
  ```

  The tag `tag:prometheus` must also exist in `tagOwners`. Check the policy with
  `headscale policy check` before you install it, as the VPN guide describes.
