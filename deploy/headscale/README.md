# deploy/headscale: file map

What each file is, where it ends up, and which step of the [VPN guide](../../docs/setup/vpn.md)
uses it. The procedures are in the guide, not here. Public DNS records are in
[`../dns/`](../dns/README.md).

| File | Installed as | Guide step |
| --- | --- | --- |
| `config/config.yaml` | Template. Rendered on the VPS to `/opt/anima/headscale/config/config.yaml` | 2 |
| `config/render.sh` | Not installed. Run once from the copy in the home folder (as root) | 2 |
| `config/policy.hujson` | `/opt/anima/headscale/config/policy.hujson` | 2, 8 |
| `compose.yaml` | `/opt/anima/headscale/compose.yaml` | 2 |
| `vps/harden.sh` | Run from `/opt/anima/headscale/vps/` on the VPS | 4 |
| `vps/sshd-anima.conf` | `/etc/ssh/sshd_config.d/10-anima.conf` (by `harden.sh ssh`) | 4 |
| `vps/20auto-upgrades`, `vps/52anima-unattended-upgrades` | `/etc/apt/apt.conf.d/` (by `harden.sh updates`) | 4 |
| `vps/daemon.json` | Merged into `/etc/docker/daemon.json` (by `harden.sh docker`) | 4 |
| `vps/nftables.conf` | Applied live, then `/etc/nftables.conf` (by `harden.sh firewall`, `confirm`) | 4 |
| `backup/` | Units, scripts and secret files as listed in [`backup/README.md`](backup/README.md) | 9 |
| `src/`, `tests/`, `package.json`, `tsconfig.json` | Not installed. Local checks: `bun test deploy/headscale` | 8 |
