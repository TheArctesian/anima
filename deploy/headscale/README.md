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
| `vps/sshd.sh`, `vps/selinux.sh`, `vps/open-ports.sh`, `vps/firewall.sh`, `vps/inventory.sh` | Sourced by `harden.sh`: `ssh`; `selinux`; which ports are open to the internet and the guard against closing them; `firewall` and `confirm`; the read-only inventory that `check` prints first | 2.0, 3, 4 |
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

## Agent devices (MET-020, R-084)

`config/policy.hujson` carries `tag:agent` for a device that runs `anima-agent` and nothing
else. One grant names it: `tag:agent` to `tag:hub`, `tcp:443`. No grant has it as a
destination (the agent listens on no port), and no other grant gives it reach, so it cannot
touch the hub's SSH, a site or another device. Like every tag, its owner list is empty:
only the operator applies it, with `headscale nodes tag -i <id> -t tag:agent` or a pre-auth
key created with `--tags tag:agent` (a proposal, see below). A tagged node stops being a
personal node and does not expire (VPN-003).

A device that is also an operator machine (a laptop or the Mac) stays a personal node of
`operator`, untagged: it keeps its operator reach, and its agent still only dials the hub on
443, which `operator@` already allows. Do not tag it, or it would lose that reach.

Applying the policy on the live Headscale is the operator's step (VPS-005). The tests block
does not name `tag:agent` yet: Headscale refuses a policy that names a tag no node carries.
Its rows wait in `AGENT_ROWS` in `tests/policy.test.ts` and move into the policy once the
first agent-only device is tagged.
