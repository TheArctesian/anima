# VPN and network setup guide

This guide builds the Anima VPN: a Headscale control server on the Hostinger VPS, a subnet router
at each site that has devices, and the Tailscale client on every computer that runs Anima. You do
every step yourself, by hand, in order. Spec: Sections 4.1, 4.2 and 4.2.1 (VPN-001 to VPN-007,
VPS-001 to VPS-006, NET-005, DNS-001, DNS-003 to DNS-006, DNS-008, OD-10).

Rules for this guide:

- The operator runs every command. An agent must not run any command in this guide against the
  VPS, Hostinger, Cloudflare or the Headscale server (VPS-005, DNS-008).
- Each code block starts with a comment that says where it runs.
- Words in `<ANGLE_BRACKETS>` are values you fill in. A pre-auth key, a token or a password is
  shown once. Never paste one into chat, a note, a ticket or a commit.

## 0. Overview

### Topology

```text
                      Hostinger VPS  (hs.anima.ing, hub.anima.ing)
                      Headscale + DERP relay + hub (later, WP-04)
                                   |
        +--------------------------+--------------------------+
        |                          |                          |
  Home 10.10.0.0/24         Office 1 10.20.0.0/24      Office 2 10.30.0.0/24
  Linux PC: subnet router   Mac: client                Linux PC: subnet router
  Pi-hole (no VPN software) (no router in Phase 1)     Mac: client
```

| Site | Subnet (VPN-001) | Router | Client computers |
| --- | --- | --- | --- |
| Home | `10.10.0.0/24` | Linux PC (`home-router`) | none |
| Office 1 | `10.20.0.0/24` | none in Phase 1 | Mac |
| Office 2 | `10.30.0.0/24` | Linux PC (`office2-router`) | Mac |

What runs where:

| Machine | Runs |
| --- | --- |
| VPS | Headscale (Docker Compose, `/opt/anima/headscale`), Tailscale (node `hub`, tag `tag:hub`), later the Anima hub on the VPN address only, the daily backup unit |
| Home Linux PC | Tailscale subnet router, restic `rest-server` (backup target), weekly prune |
| Office 2 Linux PC | Tailscale subnet router |
| Each Mac | Tailscale client, personal node of Headscale user `operator` |
| Pi-hole and other devices | No VPN software; they are reached through their site router |

Names:

| Name | Meaning |
| --- | --- |
| `hs.anima.ing` | Public A record to the VPS. Headscale control server and DERP relay. |
| `hub.anima.ing` | Public A record to the hub's VPN address (`100.64.0.0/10`). Only VPN members can connect (DNS-005). |
| `<host>.net.anima.ing` | VPN-only names from Headscale (MagicDNS), for example `hub.net.anima.ing`. No public records (DNS-006). |
| `updates.anima.ing` | Update redirect, set up with WP-04. Not part of this guide. |

### Operator changes, in order

Every row changes the VPS, the Hostinger account or the Cloudflare account. **Confirm each one in
chat first (VPS-005, DNS-008)** before you do it.

| # | Change | Where | Step |
| --- | --- | --- | --- |
| 1 | Create the operator Cloudflare token; add `hs` A and CAA records | Cloudflare | 1 |
| 2 | Hostinger panel firewall: 443/tcp, 3478/udp, 41641/udp, and 22/tcp temporarily | Hostinger | 1 |
| 3 | Install packages, copy files, render the config, start Headscale | VPS | 2 |
| 4 | Create user `operator`; create the `hub` pre-auth key; join the VPS to the VPN | VPS | 2, 3 |
| 5 | Add the `hub` A record | Cloudflare | 3 |
| 6 | Harden SSH, updates, Docker, firewall; confirm the firewall | VPS | 4 |
| 7 | Remove 22/tcp from the Hostinger panel firewall | Hostinger | 4 |
| 8 | Approve routes; register clients; set an expiry; reload the policy | VPS | 5, 6, 8 |
| 9 | Install and start the backup units | VPS, home PC | 9 |
| 10 | Revoke the operator Cloudflare token | Cloudflare | 1, 3 |

Order matters. The Mac you use to administer the VPS must be on the VPN before you apply the VPS
firewall (step 4), because the firewall closes SSH on the public interface.

### Prerequisites

- A VPS with Ubuntu 24.04 LTS. Check it in step 2.
- The Hostinger **browser console**: your way back in if SSH over the VPN fails.
- A non-root user on the VPS in group `sudo`, with your SSH public key in
  `~/.ssh/authorized_keys`. Test `ssh <ADMIN_USER>@<VPS_PUBLIC_IPV4>` and `sudo -v` before you start.
- On the VPS: Docker Engine with the compose plugin from Docker's repository, Tailscale from
  Tailscale's repository, `sqlite3`, `restic`, `jq`, and `gettext-base` (provides `envsubst`).
  Step 2 installs them.
- On your Mac: this repository, `bun`, `curl`, `jq`, `dig`, `openssl`.
- A Cloudflare account that holds `anima.ing`.

## 1. DNS first

**Operator change 1, 10 (confirm in chat first: VPS-005, DNS-008).** Create the `hs.anima.ing`
A record and the CAA record before Headscale first starts, because Headscale asks Let's Encrypt for
its certificate at first start.

Follow [`deploy/dns/README.md`](../../deploy/dns/README.md): the token rules (DNS-003), the
`fill.ts` check, and the `hs` and CAA rows of the apply table. Do not copy that procedure here.
Skip the `hub` row for now; step 3 does it. Revoke the operator token when you finish all the DNS
steps you need now.

Check from your Mac:

```bash
# Runs on: operator's Mac
dig +short hs.anima.ing @1.1.1.1     # must print the VPS public IPv4
dig +short CAA anima.ing @1.1.1.1
```

**Operator change 2 (confirm in chat first: VPS-005).** Hostinger panel firewall for the VPS.
This is a second layer; the host firewall in step 4 is the one that counts. In the Hostinger panel
allow only these inbound rules:

| Port | Why |
| --- | --- |
| 443/tcp | Headscale, DERP, certificate challenge (VPS-002) |
| 3478/udp | STUN (VPS-002) |
| 41641/udp | WireGuard of the VPS node (VPS-002) |
| 22/tcp | **Temporary.** Remove it at the end of step 4. |

If you cannot use a panel firewall, skip this; the host firewall still protects the VPS.

## 2. The VPS: packages and Headscale

Everything in this step runs on the VPS over SSH. Connect over the public address for now:

```bash
# Runs on: operator's Mac
ssh <ADMIN_USER>@<VPS_PUBLIC_IPV4>
```

**Operator change 3, 4 (confirm in chat first).**

1. Check the system:

   ```bash
   # Runs on: VPS (SSH)
   . /etc/os-release && echo "$PRETTY_NAME"     # must say Ubuntu 24.04
   ```

2. Install Docker Engine and the compose plugin from Docker's repository:

   ```bash
   # Runs on: VPS (SSH)
   sudo apt-get update
   sudo apt-get install -y ca-certificates curl
   sudo install -m 0755 -d /etc/apt/keyrings
   sudo curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc
   sudo chmod a+r /etc/apt/keyrings/docker.asc
   echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu $(. /etc/os-release && echo "${UBUNTU_CODENAME:-$VERSION_CODENAME}") stable" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
   sudo apt-get update
   sudo apt-get install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin
   ```

3. Install Tailscale from Tailscale's repository, and the other tools:

   ```bash
   # Runs on: VPS (SSH)
   curl -fsSL https://pkgs.tailscale.com/stable/ubuntu/noble.noarmor.gpg | sudo tee /usr/share/keyrings/tailscale-archive-keyring.gpg > /dev/null
   curl -fsSL https://pkgs.tailscale.com/stable/ubuntu/noble.tailscale-keyring.list | sudo tee /etc/apt/sources.list.d/tailscale.list > /dev/null
   sudo apt-get update
   sudo apt-get install -y tailscale sqlite3 restic jq gettext-base
   ```

4. Copy the files to the VPS. `render.sh` reads the template and writes the finished config
   somewhere else, so keep the repository copy in your home folder and install from it:

   ```bash
   # Runs on: operator's Mac, from the repository root
   scp -r deploy/headscale <ADMIN_USER>@<VPS_PUBLIC_IPV4>:anima-headscale
   ```

5. Create the folders and render the config. `sudo` is required: the rendered config is `0600`
   and owned by root, and the container has no capability to read files of other owners.
   `render.sh` stops, and writes nothing, if the public address is not a public IPv4 address on
   this machine or the email looks wrong.

   ```bash
   # Runs on: VPS (SSH)
   sudo install -d -m 0755 /opt/anima /opt/anima/headscale /opt/anima/headscale/config
   sudo install -d -m 0700 /opt/anima/headscale/lib
   cd ~/anima-headscale
   sudo env VPS_PUBLIC_IPV4=<VPS_PUBLIC_IPV4> ACME_EMAIL=<YOUR_EMAIL> \
     bash ./config/render.sh ./config/config.yaml /opt/anima/headscale/config/config.yaml
   sudo install -m 0644 ./compose.yaml /opt/anima/headscale/compose.yaml
   sudo install -m 0644 ./config/policy.hujson /opt/anima/headscale/config/policy.hujson
   sudo ls -l /opt/anima/headscale/config      # config.yaml is -rw------- root root
   ```

6. Check the image digest before you start it. `compose.yaml` pins the image by tag and digest.
   The two digests after `@` must be identical:

   ```bash
   # Runs on: VPS (SSH)
   sudo docker pull docker.io/headscale/headscale:0.29.7
   sudo docker image inspect --format '{{index .RepoDigests 0}}' docker.io/headscale/headscale:0.29.7
   grep 'image:' /opt/anima/headscale/compose.yaml
   ```

7. Start Headscale and define a shorthand for its CLI. The CLI talks to Headscale over a unix
   socket inside the container; there is no other admin interface (VPS-003). The shorthand lasts
   only in this SSH session; define it again in each new session.

   ```bash
   # Runs on: VPS (SSH)
   sudo docker compose -f /opt/anima/headscale/compose.yaml up -d
   hsctl() { sudo docker compose -f /opt/anima/headscale/compose.yaml exec headscale headscale "$@"; }
   sudo docker compose -f /opt/anima/headscale/compose.yaml ps        # headscale: healthy
   sudo docker compose -f /opt/anima/headscale/compose.yaml logs --tail 30 headscale
   ```

   The log shows a **warning** that the policy `tests` failed and that the server starts anyway.
   This is expected: the `tests` rows name the hub, a client and both routers, and none are
   enrolled yet. Headscale applies the policy anyway at boot. Until all four kinds of node
   are enrolled, do not reload the policy (a reload with failing rows is rejected). Approving
   routes needs no reload. If you must change the policy earlier, delete the router row groups
   (the file marks them) from the copy you install.

   If the state is not `healthy` after a minute, read the log. The first start writes the
   database and two private keys into `lib/`; the compose file assumes the image runs as root and
   needs no extra capability for that.

8. Check the certificate. Run this from your Mac. The first request can take a few seconds while
   Let's Encrypt issues the certificate with TLS-ALPN-01:

   ```bash
   # Runs on: operator's Mac
   openssl s_client -connect hs.anima.ing:443 -servername hs.anima.ing </dev/null 2>/dev/null \
     | openssl x509 -noout -dates -issuer
   ```

   The issuer must be Let's Encrypt and `notAfter` about 90 days away.

9. Check there are **no API keys**. The `/api/v1` interface is on the public port and only an API
   key opens it, so the list must be empty. Check again after every Headscale upgrade:

   ```bash
   # Runs on: VPS (SSH)
   hsctl apikeys list          # must list no keys
   ```

   If any key is listed, expire it (`hsctl apikeys expire --prefix <PREFIX>`) and tell the
   operator in chat before you go on.

10. Create the user. Client computers belong to this user:

    ```bash
    # Runs on: VPS (SSH)
    hsctl users create operator
    hsctl users list            # operator is listed
    ```

## 3. The VPS joins the VPN as the hub

**Operator change 4, 5 (confirm in chat first).** The VPS joins its own VPN with the name `hub`
and the tag `tag:hub`. A tagged node never expires (VPN-003), so no renewal is needed.

1. Create a tagged, single-use pre-auth key that is valid for one hour. It is shown once. Do not
   copy it anywhere except into the next command:

   ```bash
   # Runs on: VPS (SSH)
   hsctl preauthkeys create --tags tag:hub --expiration 1h
   ```

2. Join. Type the key at the prompt; it is not echoed and not stored in shell history:

   ```bash
   # Runs on: VPS (SSH)
   read -rs -p "Pre-auth key: " PREAUTH_KEY; echo
   sudo systemctl enable --now tailscaled
   sudo tailscale up --login-server https://hs.anima.ing --authkey "$PREAUTH_KEY" --hostname hub --accept-routes
   unset PREAUTH_KEY
   ```

   `--accept-routes` lets the hub reach the site subnets once the routers are approved (step 5).
   The key is visible in the process list for a moment. It is single use and valid for one hour,
   and Headscale consumes it at once when the node joins; `unset` removes it from the shell.

3. Read the VPN address Headscale gave the hub, and check the node:

   ```bash
   # Runs on: VPS (SSH)
   tailscale ip -4                 # for example 100.64.0.1: this is <HUB_VPN_IPV4>
   hsctl nodes list                # hub, tag:hub, no expiry
   ```

4. Add the `hub` DNS record. Follow the `hub` row of the apply table in
   [`deploy/dns/README.md`](../../deploy/dns/README.md), with `HUB_VPN_IPV4` set to the address
   above. Revoke the operator Cloudflare token afterwards (operator change 10). Check:

   ```bash
   # Runs on: operator's Mac
   dig +short hub.anima.ing @1.1.1.1     # must print <HUB_VPN_IPV4>
   ```

5. Check the MagicDNS name inside the VPN. This works from any machine that is on the VPN. On
   the VPS:

   ```bash
   # Runs on: VPS (SSH)
   getent hosts hub.net.anima.ing        # must print <HUB_VPN_IPV4>
   ```

   Repeat it on each client in step 6.

The hub program itself is installed by the hub guide (WP-04). Until then nothing listens on
`hub.anima.ing`.

## 4. Harden the VPS

The script `deploy/headscale/vps/harden.sh` changes the VPS in small steps, each with a guard
against locking you out. Run it from a copy of the repository folder on the VPS (the script reads
the config files beside it). Do these in order, one command at a time.

**Operator change 6, 7 (confirm in chat first).**

0. Get the script to a root-owned place and set the admin user. `ANIMA_ADMIN_USER` is the account
   you log in with; it must be non-root, in group `sudo`, with a key in `authorized_keys`.

   ```bash
   # Runs on: VPS (SSH)
   sudo rm -rf /opt/anima/headscale/vps
   sudo cp -r ~/anima-headscale/vps /opt/anima/headscale/vps
   sudo chown -R root:root /opt/anima/headscale/vps
   export ANIMA_ADMIN_USER=<ADMIN_USER>
   cd /opt/anima/headscale/vps
   ```

1. `check` reports and changes nothing. Read every line. Before the hardening it lists
   violations (SSH password login, no firewall table, no `nftables` drop-in, listeners). That is
   expected. It also warns if Docker is older than version 28 and fails if `tailscale0` is missing.

   ```bash
   # Runs on: VPS (SSH)
   sudo --preserve-env=ANIMA_ADMIN_USER ./harden.sh check
   ```

2. `ssh` installs the SSH rules: keys only, no root login, only `ANIMA_ADMIN_USER` may log in
   (VPS-001). Guards: it refuses unless that user is a non-root sudo user with a key in
   `authorized_keys`; it checks the result with `sshd -t` and `sshd -T`, and puts the old file back
   if either check fails. **Keep your current session open.** Open a second session and log in
   before you close the first.

   ```bash
   # Runs on: VPS (SSH)
   sudo --preserve-env=ANIMA_ADMIN_USER ./harden.sh ssh
   ```

3. `updates` turns on automatic security updates, with a reboot at 04:30 UTC when needed
   (VPS-004). The rules cover Ubuntu's security pockets and, through `origin=Docker` and
   `origin=Tailscale`, the two extra repositories. First confirm that those two names match
   what your VPS reports. Docker and Tailscale are installed already:

   ```bash
   # Runs on: VPS (SSH)
   apt-cache policy docker-ce tailscale | grep -E 'o=|^[a-z]'     # expect o=Docker and o=Tailscale
   grep -n 'origin=' /opt/anima/headscale/vps/52anima-unattended-upgrades
   ```

   If an `o=` value differs (for example in capitalisation), edit the `origin=` line in
   `/opt/anima/headscale/vps/52anima-unattended-upgrades` to match, then install:

   ```bash
   # Runs on: VPS (SSH)
   sudo ./harden.sh updates
   sudo unattended-upgrade --dry-run --debug 2>&1 | grep -i 'allowed origins'
   ```

   A Docker or Tailscale upgrade restarts its daemon: containers keep running (`live-restore`),
   and the VPN drops for a few seconds.

4. `docker` merges `daemon.json` into `/etc/docker/daemon.json`: published ports bind to
   `127.0.0.1` by default (Docker would otherwise bypass the firewall), logs are rotated, and
   `live-restore` keeps containers running while Docker restarts. Restart Docker to apply it; the
   Headscale container restarts once, and clients stay connected meanwhile.

   ```bash
   # Runs on: VPS (SSH)
   sudo ./harden.sh --restart-docker docker
   sudo docker compose -f /opt/anima/headscale/compose.yaml ps       # headscale: healthy again
   ```

5. Before the firewall: the Mac you administer from must be on the VPN. Do step 6 for that Mac
   now (sections A and B), then return here. Test:

   ```bash
   # Runs on: operator's Mac
   ssh <ADMIN_USER>@<HUB_VPN_IPV4>      # must log in over the VPN (the policy allows tcp 22 to the hub)
   ```

6. `firewall` applies the nftables rules (VPS-002): drop everything inbound except 443/tcp,
   3478/udp, 41641/udp and, on the VPN interface only, 22/tcp. Guards: it refuses if `tailscale0`
   is missing; it refuses unless your SSH session itself comes over the VPN (or you pass
   `--console`); it validates the rules before it applies them; and it **arms a rollback before
   applying**. Run it inside an SSH session you opened to `<HUB_VPN_IPV4>`, not to the public address:

   ```bash
   # Runs on: VPS (SSH over the VPN: ssh <ADMIN_USER>@<HUB_VPN_IPV4>)
   export ANIMA_ADMIN_USER=<ADMIN_USER>
   cd /opt/anima/headscale/vps
   sudo --preserve-env=ANIMA_ADMIN_USER,SSH_CONNECTION ./harden.sh firewall
   ```

7. `confirm`. You have 180 seconds. Open a NEW SSH session over the VPN and run:

   ```bash
   # Runs on: VPS (a NEW SSH session over the VPN)
   export ANIMA_ADMIN_USER=<ADMIN_USER>
   cd /opt/anima/headscale/vps
   sudo --preserve-env=ANIMA_ADMIN_USER,SSH_CONNECTION ./harden.sh confirm
   ```

   `confirm` refuses to run in the same session that ran `firewall`, or in one that does not come
   over the VPN (from the Hostinger console use `--console`). It cancels the rollback, installs
   the rules for boot, and installs `/etc/systemd/system/nftables.service.d/anima.conf`. That
   drop-in makes a stop of `nftables` remove only the Anima table. If you do nothing, or the new
   session cannot connect, after 180 seconds the previous rules come back by themselves (the
   VPS is as open as before: keep 22/tcp in the Hostinger firewall until `confirm` succeeds).
   Then find the cause and run `firewall` again. Without
   that drop-in, never `systemctl stop nftables` on this host: it flushes Docker's and Tailscale's
   rules too.

8. If SSH over the VPN does not work and you are locked out, open the Hostinger browser console,
   log in there, and run the firewall step from it:

   ```bash
   # Runs on: VPS (Hostinger browser console)
   export ANIMA_ADMIN_USER=<ADMIN_USER>
   cd /opt/anima/headscale/vps
   sudo --preserve-env=ANIMA_ADMIN_USER ./harden.sh --console firewall
   ```

   To remove the firewall table completely from the console: `sudo nft delete table inet anima_filter`.

9. Run `check` again from the VPN session. It must say `ok: no violations`. It now also needs the
   Anima table, the `nftables` unit enabled, the drop-in and `tailscale0`:

   ```bash
   # Runs on: VPS (SSH over the VPN)
   sudo --preserve-env=ANIMA_ADMIN_USER ./harden.sh check
   ```

10. Note for the hub guide (WP-04): the firewall drops packets for `100.64.0.0/10` that do not
    arrive on `tailscale0`, so a container on a Docker bridge network cannot reach any VPN address.
    Treat this as a WP-04 note for the hub's network setup.

11. **Operator change 7.** In the Hostinger panel, remove the 22/tcp rule that you added in step 1 of this guide.
    SSH now works only over the VPN.

## 5. Subnet routers

A subnet router makes the devices of one site reachable over the VPN. The devices need no VPN
software. Do this on the home Linux PC (`home-router`, `10.10.0.0/24`, tag `tag:site-home`) and on
the office 2 Linux PC (`office2-router`, `10.30.0.0/24`, tag `tag:site-office2`). Office 1 has no
router in Phase 1: its Mac is a client, and its devices are not reachable until a router is added.

**Operator change 8 (confirm in chat first).**

1. Install Tailscale from Tailscale's repository for your distribution
   (<https://tailscale.com/kb/1031/install-linux>) and turn on IP forwarding:

   ```bash
   # Runs on: home Linux PC (and again on: office 2 Linux PC)
   echo 'net.ipv4.ip_forward = 1' | sudo tee /etc/sysctl.d/99-tailscale.conf
   echo 'net.ipv6.conf.all.forwarding = 1' | sudo tee -a /etc/sysctl.d/99-tailscale.conf
   sudo sysctl -p /etc/sysctl.d/99-tailscale.conf
   sudo systemctl enable --now tailscaled
   ```

2. On the VPS, create a pre-auth key for the router. It carries two tags: `tag:router` and the
   tag of its own site. Use `tag:site-office2` for the office 2 router. The key is single use,
   valid for one hour, and shown once:

   ```bash
   # Runs on: VPS (SSH over the VPN)
   hsctl preauthkeys create --tags tag:router,tag:site-home --expiration 1h
   ```

3. Join and advertise the site subnet. Keep the default `--snat-subnet-routes`: devices at the
   site then see the router as the source and answer to it, so they need no extra route. Do not
   add `--accept-routes` on a router.

   ```bash
   # Runs on: home Linux PC
   read -rs -p "Pre-auth key: " PREAUTH_KEY; echo
   sudo tailscale up --login-server https://hs.anima.ing --authkey "$PREAUTH_KEY" \
     --advertise-routes=10.10.0.0/24 --hostname home-router
   unset PREAUTH_KEY
   ```

   ```bash
   # Runs on: office 2 Linux PC (key made with --tags tag:router,tag:site-office2)
   read -rs -p "Pre-auth key: " PREAUTH_KEY; echo
   sudo tailscale up --login-server https://hs.anima.ing --authkey "$PREAUTH_KEY" \
     --advertise-routes=10.30.0.0/24 --hostname office2-router
   unset PREAUTH_KEY
   ```

4. Approve the route on the VPS. The policy has no `autoApprovers` on purpose: a node that is
   later taken over could otherwise advertise any subnet, for example another site's, and pull
   that traffic to itself. A route works only after you approve it, one by one:

   ```bash
   # Runs on: VPS (SSH over the VPN)
   hsctl nodes list-routes                 # note the node ID of home-router
   hsctl nodes approve-routes --identifier <NODE_ID> --routes 10.10.0.0/24
   hsctl nodes list-routes                 # the route is now approved and served
   hsctl nodes list                        # home-router: tag:router and tag:site-home only, no expiry
   ```

   Do the same for the office 2 router with `10.30.0.0/24`. Approve only the one subnet that
   belongs to the router.

5. Optional, for faster forwarding on Linux with kernel 6.2 or newer. It is not kept after a
   reboot:

   ```bash
   # Runs on: home Linux PC (and office 2 Linux PC)
   NETDEV=$(ip -o route get 8.8.8.8 | cut -f 5 -d " ")
   sudo ethtool -K "$NETDEV" rx-udp-gro-forwarding on rx-gro-list off
   ```

Check from the VPS that the hub reaches the home subnet (the backup in step 9 needs it):

```bash
# Runs on: VPS (SSH over the VPN)
ping -c 3 <A_HOME_DEVICE_ADDRESS>        # for example the Pi-hole, 10.10.0.x
```

## 6. Client computers

Each Mac (and any Linux client) is a **personal node** of the user `operator`, not a tagged node.
Tagged nodes never expire, and a client key must expire after 180 days (VPN-003). Do not add
`--advertise-tags` on a client, ever.

### A. Install Tailscale

```bash
# Runs on: Mac at office 1 (and again on: Mac at office 2)
brew search --cask tailscale        # pick the Tailscale app: tailscale-app (older name: tailscale)
brew install --cask tailscale-app
```

You can also install the standalone package from <https://tailscale.com/download/mac>. Open the
app once and, in its settings, turn on "Launch at login" so that the client starts when you log
in. The command-line tool is inside the app; make a shorthand for the commands below:

```bash
# Runs on: Mac at office 1 (and Mac at office 2)
TS=/Applications/Tailscale.app/Contents/MacOS/Tailscale
"$TS" version
```

On a Linux client: install Tailscale from the repository, then `sudo systemctl enable --now tailscaled`.

### B. Register the client (interactive, with approval)

1. Start the login against your server. `--accept-routes` lets this computer use the site routes:

   ```bash
   # Runs on: Mac at office 1 (and Mac at office 2)
   "$TS" login --login-server https://hs.anima.ing --accept-routes
   ```

   On a Linux client use `sudo tailscale login --login-server https://hs.anima.ing --accept-routes`.
   If you use the Mac app instead, choose its alternate (custom) coordination server option and
   enter `https://hs.anima.ing`; where that option sits depends on the app version.

2. The command prints a web address that ends in an **auth ID**. Open it; the page shows a
   `headscale auth register` command with the same ID. Compare the ID on the device with the ID
   in the command. If they differ, stop: someone else started that login.

3. Approve on the VPS:

   ```bash
   # Runs on: VPS (SSH over the VPN)
   hsctl auth register --user operator --auth-id <AUTH_ID>
   ```

4. Check the node. **It must show no tags**, and the expiry must be no later than today plus 180
   days. A client may ask for its own expiry, so an empty or later expiry is possible and must be
   fixed:

   ```bash
   # Runs on: VPS (SSH over the VPN)
   hsctl nodes list
   date -u -d '+180 days' +%Y-%m-%dT%H:%M:%SZ        # the latest allowed expiry
   hsctl nodes expire --identifier <NODE_ID> --expiry <THAT_TIMESTAMP>
   ```

   If the node shows a tag, delete it and register it again after you have removed
   `--advertise-tags` from the client's command: `hsctl nodes delete --identifier <NODE_ID>`.

### C. Use the site routes and stay connected

```bash
# Runs on: Mac at office 1 (and Mac at office 2)
"$TS" set --accept-routes
"$TS" status
```

In the Mac app, the same setting is "Use Tailscale subnets". Keep "Launch at login" on. On Linux:

```bash
# Runs on: Linux client
sudo tailscale set --accept-routes
sudo systemctl enable --now tailscaled
```

Check MagicDNS on the client:

```bash
# Runs on: each client computer
"$TS" ping hub                     # on Linux: tailscale ping hub
dig +short hub.net.anima.ing       # must print the hub VPN address
```

### The Mac at office 2

This Mac is inside `10.30.0.0/24`, which its own site router also advertises. A device on the
same LAN is reached directly on the local network; the Mac should not send that traffic through
the VPN. Check it:

```bash
# Runs on: Mac at office 2
route -n get 10.30.0.5 | grep interface     # use a real device address; must name your LAN interface (en0), not utunN
```

Traffic to office 1 and home still goes through the VPN. If the output names a `utun` interface,
local traffic is going through the VPN: report it before you rely on this Mac.

## 7. Key expiry and renewal (VPN-003)

| Node | Tag | Expires |
| --- | --- | --- |
| Hub (VPS) | `tag:hub` | never |
| Routers | `tag:router` plus a site tag | never |
| Client computers | none (personal nodes) | 180 days after registration |

See the dates:

```bash
# Runs on: VPS (SSH over the VPN)
hsctl nodes list          # read the Expiry column; put a reminder in your calendar 150 days after each registration
```

Renew a client before its date, or as soon as it shows "needs login":

1. On the client, log in again (same command as in step 6B):

   ```bash
   # Runs on: Mac at office 1 (or: Mac at office 2)
   "$TS" login --login-server https://hs.anima.ing --accept-routes
   ```

2. Compare the auth ID on the device with the one in the command, then approve it on the VPS:
   `hsctl auth register --user operator --auth-id <AUTH_ID>`.
3. Run the checks of step 6B.4 again: **no tags** and an expiry within 180 days. A renewal is the
   moment a taken-over client could ask for a tag (and with it a key that never expires), so
   check the tags every time.

## 8. Access rules (VPN-004)

The policy is `deploy/headscale/config/policy.hujson`. Anything it does not allow is denied.

| From | To | Ports |
| --- | --- | --- |
| Client computers (`operator@`) | the hub (`tag:hub`) | tcp 443 (hub), tcp 22 (VPS administration) |
| Client computers | all three site subnets | all |
| The hub | all three site subnets | all |
| Each site router | its own site subnet only | all |
| Anything else | | denied |

Nothing allows a device, a router or the hub to start a connection to a client computer. Replies
to connections that clients start still flow.

Tag owners are empty: no node can claim a tag when it registers. Only you apply tags, with a
tagged pre-auth key.

**To change the rules:**

1. Edit the policy and its `tests` block on your Mac, in the full form. Run the tests there. They are the
   authoritative check:

   ```bash
   # Runs on: operator's Mac, from the repository root
   bun test deploy/headscale
   ```

2. Copy it to the VPS as a candidate and check it with Headscale as a smoke test (Headscale's own
   test run is less strict than the bun tests):

   ```bash
   # Runs on: operator's Mac, from the repository root
   scp deploy/headscale/config/policy.hujson <ADMIN_USER>@<HUB_VPN_IPV4>:policy.hujson
   ```

   ```bash
   # Runs on: VPS (SSH over the VPN)
   sudo install -m 0644 ~/policy.hujson /opt/anima/headscale/config/policy.hujson.new
   hsctl policy check --file /etc/headscale/policy.hujson.new
   ```

3. Install it and reload (no restart; connections stay up):

   ```bash
   # Runs on: VPS (SSH over the VPN)
   sudo mv /opt/anima/headscale/config/policy.hujson.new /opt/anima/headscale/config/policy.hujson
   sudo docker compose -f /opt/anima/headscale/compose.yaml kill -s HUP headscale
   sudo docker compose -f /opt/anima/headscale/compose.yaml logs --tail 20 headscale
   ```

**The `tests` block and the enrolment order.** Headscale refuses a test row whose source or
destination has no node. At boot it only warns and starts anyway (step 2.7), but a **reload with
a failing row is rejected**. So do not reload the policy until the hub, a client and both routers
are enrolled. The repository file groups its rows by the nodes they need: first the hub and a
client, then the home router, then the office 2 router. If you must change the policy earlier,
delete the router row groups from the copy you install and add them back when the routers join.
Run `bun test deploy/headscale` on the full file every time.

## 9. Backups (VPS-006)

Every day the VPS saves a consistent copy of the Headscale database, its two private keys, the
config and the policy, and sends it, encrypted with restic, to a `rest-server` on the home Linux
PC. The `rest-server` runs append-only: a taken-over VPS can add snapshots but cannot delete old
ones. Pruning runs on the home PC. The full file map, the modes and the restore commands are in
[`deploy/headscale/backup/README.md`](../../deploy/headscale/backup/README.md). This step lists
the order.

The backup uses the home router (step 5) and the hub's `--accept-routes` (step 3). Do those first.

**Operator change 9 (confirm in chat first).**

1. Home PC: install `restic`, `jq` (the prune script needs it), `apache2-utils` (for `htpasswd`) and the `rest-server` binary
   (download a release from <https://github.com/restic/rest-server/releases>, check its checksum,
   and install it as `/usr/local/bin/rest-server`). Create the user and the folders:

   ```bash
   # Runs on: home Linux PC
   sudo useradd --system --user-group --no-create-home --shell /usr/sbin/nologin restic
   sudo install -d -o restic -g restic -m 0700 /srv/restic
   sudo install -d -m 0755 /etc/restic
   ```

2. Home PC: the login of the VPS, and the files the units read. These hold secrets. Make the
   rest-server password hex, so it needs no URL encoding: run `openssl rand -hex 32` on the home PC,
   read it from the screen and type it at the two prompts below (here and on the VPS). Choose the
   restic encryption password in your password manager and keep it offline too: without it the
   backups cannot be read. Never paste either password into chat or a note.

   ```bash
   # Runs on: home Linux PC
   openssl rand -hex 32                                         # the rest-server password: read it, then type it at the next prompt
   sudo htpasswd -B -c /etc/restic/htpasswd headscale-vps       # asks for the rest-server password
   sudo chown root:restic /etc/restic/htpasswd && sudo chmod 0640 /etc/restic/htpasswd
   sudo bash -c 'umask 077; read -rsp "restic encryption password: " P; echo; printf "%s" "$P" > /etc/restic/headscale-password'
   echo 'HOME_PC_LAN_IP=<HOME_PC_LAN_IP>' | sudo tee /etc/restic/rest-server.env     # 10.10.0.x; required, not a secret
   sudo chmod 0644 /etc/restic/rest-server.env
   ```

3. Home PC: install the units (the repository names end in `.unit`; systemd needs `.service`) and
   start the receiver and the weekly prune. `rest-server-allow.conf` is a template: the receiver
   accepts only the hub's VPN address, and you write the real address (`tailscale ip -4` on the
   VPS, <HUB_VPN_IPV4>) in place of the placeholder. The receiver also caps uploads (`--max-size`).
   Put `/srv/restic` on its own filesystem or under a quota. Run from your copy of the repository
   files:

   ```bash
   # Runs on: home Linux PC, in a copy of deploy/headscale/backup
   sudo install -m 0644 rest-server.unit /etc/systemd/system/rest-server.service
   sudo install -d -m 0755 /etc/systemd/system/rest-server.service.d
   sed 's/\${HUB_VPN_IPV4}/<HUB_VPN_IPV4>/' rest-server-allow.conf | sudo tee /etc/systemd/system/rest-server.service.d/allow.conf > /dev/null
   sudo install -m 0755 prune.sh /usr/local/bin/restic-prune
   sudo install -m 0644 restic-prune.unit /etc/systemd/system/restic-prune.service
   sudo install -m 0644 restic-prune.timer /etc/systemd/system/restic-prune.timer
   sudo systemctl daemon-reload
   sudo systemctl enable --now rest-server.service restic-prune.timer
   ```

4. VPS: the secret files. Both are `0600` and owned by root. The repository file is one line:
   `rest:http://headscale-vps:<rest-server password>@<HOME_PC_LAN_IP>:8000/headscale-vps/headscale`.
   Type the hex rest-server password from the home PC.

   ```bash
   # Runs on: VPS (SSH over the VPN)
   sudo install -d -m 0700 /etc/anima/backup
   sudo bash -c 'umask 077; read -rsp "restic encryption password (same as on the home PC): " P; echo; printf "%s" "$P" > /etc/anima/backup/restic-password'
   sudo bash -c 'umask 077; read -rp "Home PC LAN IP: " H; read -rsp "rest-server password: " P; echo; printf "rest:http://headscale-vps:%s@%s:8000/headscale-vps/headscale\n" "$P" "$H" > /etc/anima/backup/restic-repository'
   sudo ls -l /etc/anima/backup        # both -rw------- root root
   ```

5. VPS: create the repository once, install the script and units, and run the first backup by
   hand:

   ```bash
   # Runs on: VPS (SSH over the VPN)
   sudo restic --repository-file /etc/anima/backup/restic-repository --password-file /etc/anima/backup/restic-password init
   cd ~/anima-headscale/backup
   sudo install -m 0700 backup.sh /usr/local/sbin/headscale-backup
   sudo install -m 0644 headscale-backup.unit /etc/systemd/system/headscale-backup.service
   sudo install -m 0644 headscale-backup.timer /etc/systemd/system/headscale-backup.timer
   sudo systemctl daemon-reload
   sudo systemctl enable --now headscale-backup.timer
   sudo systemctl start headscale-backup.service
   sudo journalctl -u headscale-backup.service -n 50
   ```

   First-run checks on the VPS and the home PC (the unit runs without capabilities and with
   `/opt/anima/headscale` read-only):

   - The journal must not show `unable to open database file`. If it does while Headscale is
     stopped, start Headscale once and run the backup again. If it persists, follow the backup
     README (a `ReadWritePaths=` line for the `db.sqlite-shm` file only).
   - `sudo ls -ln /opt/anima/headscale/lib` must show owner `0` for every file. If not, see the
     backup README.
   - On the home PC, `sudo journalctl -u rest-server.service` must show the request coming from
     the hub's VPN address you put in `allow.conf`.

6. Check from the home PC that the snapshot arrived:

   ```bash
   # Runs on: home Linux PC
   sudo restic --repo /srv/restic/headscale-vps/headscale --password-file /etc/restic/headscale-password snapshots
   ```

   The prune runs every Sunday on this PC. It keeps snapshots by age: every snapshot of the last
   14 days, then weekly ones for two months, then monthly ones for a year. Check it once with
   `systemctl list-timers restic-prune.timer`.

   **If the prune refuses.** The prune script stops, and deletes nothing, when a snapshot is dated
   more than an hour in the future or when one UTC day has more than two snapshots (a running
   VPS could otherwise flood the repository to push good snapshots out). Running the backup by
   hand more than twice in one day also triggers it. Look at the list:

   ```bash
   # Runs on: home Linux PC
   sudo restic --repo /srv/restic/headscale-vps/headscale --password-file /etc/restic/headscale-password --no-cache --no-lock snapshots --host anima-vps --tag headscale
   ```

   If the extra snapshots came from your own manual runs, forget those snapshot IDs by hand
   (`restic forget <ID>`). If you did not make them, **suspect the VPS**: treat it as taken over,
   check `hsctl nodes list`, the policy and the SSH logs before you prune.

   The alert for a backup that has stopped arriving is deferred work (GitHub issue #1). Until then
   look at `systemctl list-timers headscale-backup.timer` on the VPS and at the snapshot list
   once a week.

**Restore drill, every quarter.** A backup you have not restored is not a backup. On the home PC,
restore the latest snapshot into a scratch folder and check it. This does not touch the live
server:

```bash
# Runs on: home Linux PC
sudo restic --repo /srv/restic/headscale-vps/headscale --password-file /etc/restic/headscale-password \
  --no-cache --no-lock restore latest --host anima-vps --tag headscale --target /root/headscale-restore
sudo find /root/headscale-restore -type f                     # db.sqlite, two private keys, config.yaml, policy.hujson
sudo sqlite3 "$(sudo find /root/headscale-restore -name db.sqlite)" "PRAGMA integrity_check"      # must print ok
sudo rm -rf /root/headscale-restore                           # it holds private keys
```

`--no-cache --no-lock` keep the drill from writing into the repository, which belongs to the
`restic` user. To rebuild a lost VPS, use the restore commands in the backup README: restore into
`/root/headscale-restore`, check the database, stop Headscale, copy the files into `lib/` and
`config/`, start it, confirm that `hsctl nodes list` shows every node, and `rm -rf` the restore
folder at the end (it holds private keys).

## 10. Warnings

**A full-tunnel VPN on the same computer (VPN-007).** A company or other VPN in full-tunnel mode
sends all traffic into its own tunnel. It can take over the Anima ranges (`100.64.0.0/10` and the
site subnets `10.10.0.0/24`, `10.20.0.0/24`, `10.30.0.0/24`), or block the Tailscale ports
(41641/udp and 3478/udp). The symptom in Anima is "Not on the network" and `tailscale status`
reporting a health problem or no peers. Fix it in one of two ways:

- Ask the other VPN's owner for split-tunnel exceptions for `100.64.0.0/10` and the three site
  subnets (and let UDP 41641 and 3478 out), or
- Disconnect the other VPN while you use Anima.

**Address overlap.** Some internet providers and phone hotspots use `100.64.0.0/10` (carrier-grade
NAT) for their own customers. If a hotspot assigns you an address in that range, the VPN and the
hotspot can clash: try another network. Site subnets must stay different from each other and from
any LAN you visit (VPN-001).

## 11. The risk of controlling Headscale (OD-10)

Headscale has no "Tailnet Lock". With Tailscale's hosted service, a device joins only after other
devices sign it. With Headscale, whoever controls the Headscale server, that is the VPS, can add a
node or change the policy, and so reach every site. This risk is accepted; the design reduces it
as follows.

| Measure | Where |
| --- | --- |
| SSH keys only; no root login; SSH only over the VPN; firewall allows only 443/tcp, 3478/udp, 41641/udp | Step 4 |
| Automatic security updates | Step 4 |
| Admin only on the VPN: the CLI over a unix socket; remote gRPC and metrics on loopback | Step 2 |
| No API keys (checked, and checked again after upgrades); no OIDC | Step 2 |
| Pre-auth keys are single use and last one hour | Steps 3, 5 |
| Routes need your manual approval; no `autoApprovers` | Step 5 |
| Tag owners are empty: a node cannot claim a tag | Step 8 |
| Tags and expiry checked at every client registration and renewal | Steps 6, 7 |
| Append-only backups off the VPS | Step 9 |
| Devices still need their own login: SSH keys, passwords from the hub vault | Hub guide |

**Detection.** Review `hsctl nodes list` regularly against the nodes you expect: `hub`,
`home-router`, `office2-router` and two Macs. Any other node, any extra tag, any route you did not
approve, or any expiry you did not set is a reason to act. Also look at the policy file on the
server and compare it with the repository. Automatic alerting for this is deferred work, tracked
on [GitHub issue #1](https://github.com/TheArctesian/anima/issues/1) (R-007).

**Residual risk.** If an attacker gets root on the VPS, they can still add a node and widen the
policy, and reach every site, until you notice. The measures make that harder and make you more
likely to notice; they do not prevent it. Devices keep their own authentication, so an attacker on
the VPN still needs a device password or key to open a session.

## 12. Verification checklist

From a client computer (a Mac; on Linux omit `"$TS"` and use `tailscale`):

```bash
# Runs on: each Mac
"$TS" status                 # hub and routers listed; the other VPN peers visible
"$TS" ping hub               # replies; shows "direct" or "via DERP(anima)" (VPN-005)
ping -c 3 <A_HOME_DEVICE_ADDRESS>          # reach a device at home
ping -c 3 <AN_OFFICE_2_DEVICE_ADDRESS>     # reach a device at office 2
"$TS" netcheck               # the DERP list names region "anima"
curl -sv https://hub.anima.ing             # only after the hub is installed (WP-04): a valid certificate
```

`tailscale status` shows, for each peer, "direct" with an address or "relay" with the region
`anima`. A relayed path uses the bandwidth of the VPS (VPN-005).

From outside the VPN, with Tailscale off (for example on a phone hotspot):

```bash
# Runs on: operator's Mac, Tailscale off
curl -q --max-time 10 https://hub.anima.ing        # must fail (timeout or connection error)
nc -vz -w 5 <VPS_PUBLIC_IPV4> 22                   # must fail: SSH is closed to the internet
```

On the VPS:

```bash
# Runs on: VPS (SSH over the VPN)
cd /opt/anima/headscale/vps && sudo --preserve-env=ANIMA_ADMIN_USER ./harden.sh check     # ok: no violations
sudo ss -tulpn          # public listeners: headscale on <VPS_PUBLIC_IPV4>:443 and :3478, tailscaled on 41641 only
hsctl apikeys list      # no keys
hsctl nodes list        # exactly the nodes you expect; clients with no tags and an expiry within 180 days
```

Other listeners must be on `127.0.0.1`, `::1`, the VPN address, or `tailscale0` (for example
`sshd` on port 22 is reachable only on the VPN interface; Headscale's `127.0.0.1:9090` and
`127.0.0.1:50443`; the local DNS resolver).

If `dig +short hub.anima.ing` is empty at a site while `@1.1.1.1` answers, the site resolver (for
example the Pi-hole) drops answers with private addresses. See the end of
[`deploy/dns/README.md`](../../deploy/dns/README.md).
