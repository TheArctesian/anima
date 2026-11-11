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
| `vps/broker.sh` | Sourced by `harden.sh`: `broker`, the broker install (see "Installing the broker" below) | 11 |
| `broker/anima-broker.socket`, `broker/anima-broker@.unit` | `/etc/systemd/system/anima-broker.socket` and `anima-broker@.service` (by `harden.sh broker`, from `/opt/anima/headscale/broker/`) | 11 |
| `broker/*.ts`, `bin/broker-build.ts` | Not installed. Compiled on the Mac to one Linux binary, `/usr/local/libexec/anima-broker` | 11 |
| `src/`, `tests/`, `package.json`, `tsconfig.json` | Not installed. Local checks: `bun test deploy/headscale` | 8 |

## Agent devices (MET-020, R-084)

`config/policy.hujson` carries `tag:agent` for a device that runs `anima-agent` and nothing
else. One grant names it: `tag:agent` to `tag:hub`, `tcp:443`. No grant has it as a
destination (the agent listens on no port) while every `device-*` host entry is kept current
(see "Removing a client device" below). No other grant gives it reach, so it cannot touch the
hub's SSH, a site or another device.

Like every tag, its owner list is empty. The operator applies the tag in one of two ways, both
operator steps (VPS-005): with `headscale nodes tag -i <id> -t tag:agent` on an existing node,
or with a pre-auth key made with `--tags tag:agent` when the device registers.

A tagged node stops being a personal node, and tagged nodes do not expire in Headscale.
VPN-003 lists agent-only devices with the hub and routers as nodes whose keys do not expire
(R-105): an agent node reaches only the hub on TCP 443, and every route there needs the
device's agent token (K8), so there is no re-key schedule.

Revoking the agent token on the device's page stops its pushes, but not its VPN reach to the
hub on 443. To cut an agent device off the tailnet, the operator deletes the node
(`headscale nodes delete`) or removes the tag (VPS-005).

A device that is also an operator machine (a laptop or the Mac) stays a personal node of
`operator`, untagged: it keeps its operator reach, and its agent still only dials the hub on
443, which `operator@` already allows. Do not tag it, or it would lose that reach.

Applying the policy on the live Headscale is the operator's step (VPS-005). The tests block
does not name `tag:agent` yet: Headscale refuses a policy that names a tag no node carries.
Its rows wait in `AGENT_ROWS` in `tests/policy-fixture.ts`.

### First agent device

1. Tag the enrolled node, or register it with a pre-auth key made with `--tags tag:agent`
   (both operator steps, VPS-005).
2. Run `headscale nodes list`. The node must show `tag:agent` and no user. Do not continue
   if it shows a user: it is not an agent-only device.
3. Move `AGENT_ROWS` from `tests/policy-fixture.ts` into the policy `tests` block, then apply
   the policy on the live Headscale (VPS-005).

### Removing a client device

When the operator removes a client device, delete its `device-*` host entry and its grant from
`config/policy.hujson` too. The hub's device grants match by address, so a reused address could
otherwise reach an agent. `tests/policy-rules.test.ts` checks that no host `/32` equals the address of
a tagged node in its test inventory, but the test cannot see the live tailnet, so keep the
entries current.

## Installing the broker (R-055, VP-8)

The broker is the only path from the hub to Headscale (vpn-build.md Section 7, B1 and B2). It
listens on `/run/anima-broker/broker.sock`; systemd starts one `anima-broker` process for each
connection, one at a time. It accepts six verbs (`nodes`, `personal-key`, `router-key`,
`approve-route`, `register`, `renew`), at most 3 keys and 3 registrations in 24 hours, and no new
node at `max_nodes` (`/etc/anima/broker.conf`). It logs one line per request to journald and
never the pre-auth key, the auth ID or a node name. Installing it is a VPS change (VPS-005).

1. On the Mac, build the binary (the first build downloads the Bun runtime for Linux):

   ```sh
   bun deploy/headscale/bin/broker-build.ts          # x86_64; add arm64 for an aarch64 VPS
   ```

2. Copy it and the units to the VPS. The units go in `/opt/anima/headscale/broker/`, beside
   `vps/`:

   ```sh
   scp deploy/headscale/broker/dist/anima-broker <admin>@<vps>:/tmp/anima-broker
   scp deploy/headscale/broker/anima-broker.socket 'deploy/headscale/broker/anima-broker@.unit' <admin>@<vps>:/tmp/
   ssh <admin>@<vps>
   sudo install -m 0700 /tmp/anima-broker /usr/local/libexec/anima-broker.new
   sudo install -D -m 0644 -t /opt/anima/headscale/broker /tmp/anima-broker.socket /tmp/anima-broker@.unit
   rm /tmp/anima-broker /tmp/anima-broker.socket /tmp/anima-broker@.unit
   ```

3. Install it with one command; try it with `--dry-run` first:

   ```sh
   sudo /opt/anima/headscale/vps/harden.sh --dry-run broker
   sudo /opt/anima/headscale/vps/harden.sh broker
   ```

   It refuses when `anima-broker.new` is missing, the machine is not x86_64 or aarch64, or the
   binary cannot count the nodes. It writes `/etc/anima/broker.conf` (`max_nodes` = the node
   count + 5) only if the file is absent, installs the binary and the two units, enables
   `anima-broker.socket` and prints the broker's own answer, `broker: ok, <n> nodes`.

4. To raise the node limit, edit `max_nodes=` in `/etc/anima/broker.conf`; the next request
   reads it. To update the broker, repeat steps 1 to 3. The log is
   `journalctl -u 'anima-broker@*'`.
