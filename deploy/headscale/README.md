# deploy/headscale: file map

What each file is, where it ends up, and which step of the [VPN guide](../../docs/setup/vpn.md)
uses it. The procedures are in the guide, not here. Public DNS records are in
[`../dns/`](../dns/README.md).

| File | Installed as | Guide step |
| --- | --- | --- |
| `config/config.yaml` | Template. Rendered on the VPS to `/opt/anima/headscale/config/config.yaml` | 2 |
| `config/render.sh` | Not installed. Run once from the copy in the home folder (as root) | 2 |
| `config/policy.hujson` | `/opt/anima/headscale/config/policy.hujson` | 2, 8 |
| `compose.yaml` | `/opt/anima/headscale/compose.yaml`; its `config/` and `lib/` are labelled `container_file_t` by `harden.sh selinux` | 2 |
| `vps/harden.sh` | Run from `/opt/anima/headscale/vps/` on the VPS (AlmaLinux 10); `check` also from root's copy in step 2.0, `selinux` from the home copy in step 2.5 | 2.0, 2.5, 4 |
| `vps/inventory.sh`, `vps/firewall.sh` | Sourced by `harden.sh` (the read-only inventory and `check`; `firewall` and `confirm`) | 2.0, 4 |
| `vps/stock-units.txt` | Not installed. The enabled services `check` does not mark as extra | 2.0, 4 |
| `vps/sshd-anima.conf` | `/etc/ssh/sshd_config.d/00-anima.conf` (by `harden.sh ssh`) | 4 |
| `vps/dnf-automatic.conf` | `/etc/dnf/automatic.conf`, the original kept in `/var/lib/anima/` (by `harden.sh updates`) | 4 |
| `vps/dnf-automatic-timer.conf` | `/etc/systemd/system/dnf-automatic.timer.d/anima.conf` (by `harden.sh updates`) | 4 |
| `vps/daemon.json` | Merged into `/etc/docker/daemon.json` (by `harden.sh docker`) | 4 |
| `vps/nftables-unit.conf` | `/etc/systemd/system/nftables.service.d/anima.conf` (by `harden.sh confirm`) | 4 |
| `vps/nftables.conf` | Applied live, then `/etc/nftables/anima.nft`, included from `/etc/sysconfig/nftables.conf` (by `harden.sh firewall`, `confirm`) | 4 |
| (operator's file) | `/etc/anima/firewall-allow`: extra public ports to keep open, one `PORT/PROTO` per line; read by `harden.sh firewall` | 4 |
| `backup/` | Units, scripts and secret files as listed in [`backup/README.md`](backup/README.md) | 9 |
| `src/`, `tests/`, `package.json`, `tsconfig.json` | Not installed. Local checks: `bun test deploy/headscale` | 8 |
