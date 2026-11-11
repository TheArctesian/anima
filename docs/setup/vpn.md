# VPN and network setup guide

This guide builds the Anima VPN: a Headscale control server on the Hostinger VPS, a subnet router
at each site that has devices, and the Tailscale client on every computer that runs Anima. You do
every step yourself, by hand, in order. Spec: Sections 4.1, 4.2 and 4.2.1 (VPN-001 to VPN-007,
VPS-001 to VPS-006, NET-005, DNS-001, DNS-003 to DNS-006, DNS-008, OD-10).

Rules for this guide:

- You, the operator, run every command, by hand. Each change to the VPS, Hostinger or Cloudflare
  needs your confirmation in chat first (VPS-005, DNS-008); an assistant prepares and explains,
  and never applies one.
- Each code block starts with a comment that says where it runs.
- Words in `<ANGLE_BRACKETS>` are values you fill in. A pre-auth key, a token or a password is
  shown once. Never paste one into chat, a note, a ticket or a commit, and do not copy one with
  the clipboard (Universal Clipboard sends it to your other Apple devices). Type it.
- `hsctl` is a shell function defined in step 2.7. It exists only in the SSH session where you
  defined it; define it again in every new SSH session.

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
| 0 | Read-only inventory (send it to the programme first); create the admin user in group `wheel` with your key | VPS (root) | 2.0 |
| 0a | Existing workloads: read-only checks; record the agreed ones in `/etc/anima/known-workloads` (R-050) | VPS | 2.0a |
| 1 | Create the first operator Cloudflare token; add `hs` A and CAA records; revoke the token | Cloudflare | 1 |
| 2 | Hostinger panel firewall: 443/tcp, 3478/udp, 41641/udp, and 22/tcp temporarily | Hostinger | 1 |
| 2a | SELinux disabled: record the deviation (R-050), or turn SELinux on (relabel and reboots) | VPS | 2.0b |
| 3 | Install packages, copy files, render the config, start Headscale | VPS | 2 |
| 4 | Create user `operator`; create the `hub` pre-auth key; join the VPS to the VPN | VPS | 2, 3 |
| 5 | Create a second, short-lived operator token; add the `hub` A record; revoke the token | Cloudflare | 3 |
| 6 | Harden SSH, updates and Docker | VPS | 4.0 to 4.4 |
| 6a | **Only when SELinux was turned on:** recreate the other containers; 24 h in Permissive; read the denials | VPS | 4.4a |
| 6b | **Only when SELinux was turned on:** SELinux to Enforcing | VPS | 4.4b |
| 7 | Register the admin Mac and approve it (over public SSH) | VPS | 4.5, 6 |
| 8 | Apply the firewall and confirm it | VPS | 4.6, 4.7 |
| 9 | Remove 22/tcp from the Hostinger panel firewall | Hostinger | 4.11 |
| 10 | Create router pre-auth keys; approve router routes | VPS | 5 |
| 11 | Register the other clients; set expiry | VPS | 6 |
| 12 | Install the full policy and reload it (only when all nodes are enrolled) | VPS | 8 |
| 13 | Install and start the backup units | VPS, home PC | 9 |
| 14 | **Recurring:** renew each client before its 180 days end | VPS | 7 |
| 15 | **When needed:** rebuild or restore the VPS from backup (snapshot by ID, `hub` node replaced, records updated) | VPS, home PC, Cloudflare, Hostinger | 9 |
| 16 | **When needed:** upgrade the Headscale image | VPS | 13.1 |
| 17 | **When needed:** revoke a lost or stolen client | VPS | 13.2 |
| 18 | **When needed:** follow a change of the hub's VPN IPv4 | home PC, Cloudflare | 13.3 |

Every row has the marker **confirm in chat first (VPS-005, DNS-008)**, including the recurring,
rebuild and maintenance rows.

Order matters. The Mac you use to administer the VPS must be on the VPN before you apply the VPS
firewall (row 8), because the firewall closes SSH on the public interface. Rows 7 and 8 are in
step 4 of this guide.

### Prerequisites

- A VPS with **AlmaLinux 10**, SELinux in enforcing mode, or SELinux disabled with the deviation
  recorded (step 2.0b). Hostinger's image boots with SELinux disabled (`selinux=0` on the kernel
  command line). Check it in step 2.0.
- The Hostinger **browser console**: your way back in if SSH over the VPN fails.
- SSH as `root` with your key, for step 2.0 only. Step 2.0 creates the non-root admin user in
  group `wheel` (AlmaLinux has no group `sudo`), with your SSH public key in
  `~/.ssh/authorized_keys`; every later step uses that user.
- On the VPS, from step 2: Docker Engine with the compose plugin from Docker's RHEL repository,
  Tailscale from Tailscale's RHEL 10 repository, `restic` from EPEL, and `sqlite` (the `sqlite3`
  command), `jq`, `gettext-envsubst` (provides `envsubst`) and `policycoreutils-python-utils`
  (provides `semanage`) from AlmaLinux. Step 2 installs them.
- On your Mac: this repository, `bun`, `curl`, `jq`, `dig`, `openssl`.
- A Cloudflare account that holds `anima.ing`.

## 1. DNS first

**Operator change 1 (confirm in chat first: VPS-005, DNS-008).** Create the `hs.anima.ing`
A record and the CAA record before Headscale first starts, because Headscale asks Let's Encrypt for
its certificate at first start.

Follow [`deploy/dns/README.md`](../../deploy/dns/README.md): the token rules (DNS-003), the
`fill.ts` check, and the `hs` and CAA rows of the apply table. Do not copy that procedure here.
Skip the `hub` row for now; step 3 does it. Create this token for the `hs` and CAA rows only, and
revoke it when those two are done. Step 3 uses a second, new token.

Check from your Mac:

```bash
# Runs on: operator's Mac
dig +short hs.anima.ing @1.1.1.1     # must print the VPS public IPv4
dig +short CAA anima.ing @1.1.1.1
```

**Operator change 2 (confirm in chat first: VPS-005, DNS-008).** Hostinger panel firewall for the VPS.
This is a second layer; the host firewall in step 4 is the one that counts. In the Hostinger panel
allow only these inbound rules:

| Port | Why |
| --- | --- |
| 443/tcp | Headscale, DERP, certificate challenge (VPS-002) |
| 3478/udp | STUN (VPS-002) |
| 41641/udp | WireGuard of the VPS node (VPS-002) |
| 22/tcp | **Temporary.** Remove it at the end of step 4. |

**Sync the firewall after every change.** Rules you add or change after the firewall is
activated do not apply until it is synced. Sync it in the panel, or with
`hostinger vps firewall sync <FIREWALL_ID> <VM_ID>`, then check that the firewall reports
`is_synced: true`. Until then the old rules still apply, and the panel looks right while it is
not. During a sync, existing connections can drop and 443/tcp can refuse for a few seconds.

If you cannot use a panel firewall, skip this; the host firewall still protects the VPS.
Your Hostinger and Cloudflare accounts are as powerful as the VPS itself (see step 11): turn on
two-factor authentication on both.

## 2. The VPS: packages and Headscale

Everything in this step runs on the VPS over SSH, over the public address for now.

**Operator change 0 (confirm in chat first: VPS-005).**

0. **Inventory, then the admin user.** The VPS may already run other services (and Hostinger
   keeps its own daily backups of it). Look before you change anything. Copy the repository
   folder to root's home and log in as root (the only time you use root over SSH):

   ```bash
   # Runs on: operator's Mac, from the repository root
   scp -r deploy/headscale root@<VPS_PUBLIC_IPV4>:anima-headscale
   scp ~/.ssh/id_ed25519.pub root@<VPS_PUBLIC_IPV4>:admin-key.pub       # your PUBLIC key (not a secret)
   ssh root@<VPS_PUBLIC_IPV4>
   ```

   Check the system and take the read-only inventory. `check` changes nothing:

   ```bash
   # Runs on: VPS (SSH as root)
   . /etc/os-release && echo "$PRETTY_NAME"        # must say AlmaLinux 10.x
   getenforce                                       # Enforcing, Permissive, or Disabled (then step 2.0b)
   bash ~/anima-headscale/vps/harden.sh check > ~/anima-inventory.txt 2>&1; less ~/anima-inventory.txt
   ```

   Send the part from `== Listening sockets` to `== End of inventory` to the programme. It
   holds program, unit, container, network and port names, and no secrets. Most `VIOLATION`
   lines after it are expected now; send those that name a `Match` block or
   `/etc/sysconfig/nftables.conf` too, because steps 4.2 and 4.6 refuse until they are gone.
   **Stop and wait for the programme if the inventory lists anything unexpected:** a line
   marked `!` (a container or service in `/etc/anima/known-workloads` is marked `=`, step 2.0a), an existing Docker network other than `bridge`, `host` and
   `none`, a firewalld zone with anything set beyond its interfaces and the services `ssh`,
   `cockpit` and `dhcpv6-client` (ports, sources, rich rules, forward ports, masquerade, a
   target other than `default`), a firewalld policy other than `allow-host-ipv6`, a direct
   rule, or an nftables table other than `inet firewalld`. Each of those is a service that the firewall (step 4.6) or the Docker
   settings (step 4.4) could break.

   Then create the admin user. On AlmaLinux the admin group is `wheel`. **Secret:** the
   password `passwd` asks for, from your password manager; you need it for `sudo` and for the
   Hostinger console, never for SSH:

   ```bash
   # Runs on: VPS (SSH as root)
   useradd --create-home --groups wheel <ADMIN_USER>
   passwd <ADMIN_USER>
   install -d -m 0700 -o <ADMIN_USER> -g <ADMIN_USER> /home/<ADMIN_USER>/.ssh
   install -m 0600 -o <ADMIN_USER> -g <ADMIN_USER> ~/admin-key.pub /home/<ADMIN_USER>/.ssh/authorized_keys
   restorecon -R /home/<ADMIN_USER>/.ssh          # SELinux label ssh_home_t; sshd ignores keys without it
   mv ~/anima-headscale /home/<ADMIN_USER>/anima-headscale
   chown -R <ADMIN_USER>:<ADMIN_USER> /home/<ADMIN_USER>/anima-headscale
   rm ~/admin-key.pub
   ```

   Keep the root session open and test a second login from a new terminal on your Mac:

   ```bash
   # Runs on: operator's Mac (a new terminal)
   ssh <ADMIN_USER>@<VPS_PUBLIC_IPV4>
   ```

   ```bash
   # Runs on: VPS (SSH as <ADMIN_USER>)
   sudo -v                 # asks for the password above and succeeds
   id -nG                  # lists wheel
   ```

   Only when both work, close the root session. Root login over SSH stays possible until
   `harden.sh ssh` (step 4.2) turns it off. Every step from here on runs as `<ADMIN_USER>`.

**Operator change 0a, 2a (confirm in chat first: VPS-005).**

0a. **Existing workloads.** If step 0 listed containers or `!` services, collect these (all
   read-only) and send them to the programme. Agree with the programme how each one is treated
   before going on:

   ```bash
   # Runs on: VPS (SSH as <ADMIN_USER>)
   cat /proc/cmdline                                        # selinux=0 means step 2.0b is needed
   rpm -q selinux-policy-targeted container-selinux policycoreutils
   sudo docker version --format '{{.Server.Version}}'       # 28 or newer
   sudo docker inspect -f '{{.Name}} restart={{.HostConfig.RestartPolicy.Name}} {{range .Mounts}}{{.Type}}:{{.Source}}->{{.Destination}} {{end}}' $(sudo docker ps -q)
   sudo sshd -T | grep -i authorizedkeyscommand            # none (sssd is not in the SSH login path)
   ```

   Also note where each project's compose file is, and where a tunnel such as cloudflared sends
   its traffic: a container name works through the firewall; the host's own address does not
   (the firewall closes host ports to containers). **Stop** if Docker is older than 28. Report a
   container with restart policy `no`: the Docker restart in step 4.4 keeps it running, but a
   reboot (an automatic update at 04:30 UTC that needs one, step 4.3) leaves it stopped. Only its
   owner changes that.

   Record the services the programme agreed to keep, so the inventory marks them `=` instead of
   `!` (R-050). The file is a record only: it changes no check and no service.

   ```bash
   # Runs on: VPS (SSH as <ADMIN_USER>); one line per agreed container or service
   sudo install -d -m 0755 /etc/anima
   printf '%s\n' '# R-050: kept running by the operator' \
     'container <CONTAINER_NAME>' 'service <UNIT>.service' | sudo tee /etc/anima/known-workloads
   sudo bash ~/anima-headscale/vps/harden.sh check | sed -n '/== Listening/,/== firewalld/p'   # no ! lines
   ```

   Undo: `sudo rm /etc/anima/known-workloads`.

   What the Anima changes do to those containers: the firewall (step 4.6) replaces only its own
   table and keeps Docker's; forwarding stays Docker's to decide; containers keep reaching the
   internet and each other. The Docker step (4.4) reloads Docker to turn on `live-restore` before
   it restarts Docker, so no container stops, and the new Docker defaults apply only to containers
   created later. A port published as `0.0.0.0:PORT` would bypass the host firewall
   (Docker's DNAT does not pass the input chain), so publish nothing on a shared host; reach the
   service through its tunnel.

0b. **SELinux (only if `getenforce` said Disabled).** Turning SELinux on means a relabel of every
   file, including the volumes of other containers, two reboots that stop every service for some
   minutes, and, once on, other projects' containers run confined the next time they are created
   (step 4.4a). **Where other services must keep running (R-050), record the deviation instead**,
   and skip steps 4.4a and 4.4b. Headscale then runs unconfined, like every other container; the
   other hardening is unchanged.

   ```bash
   # Runs on: VPS (SSH as <ADMIN_USER>)
   sudo install -d -m 0755 /etc/anima
   echo 'R-050 (<DATE>): SELinux stays disabled; the operator keeps <SERVICES> running' | sudo tee /etc/anima/selinux-disabled
   ```

   Check: `harden.sh check` prints `note: SELinux stays disabled, a recorded deviation` and no
   SELinux `VIOLATION`. Undo: `sudo rm /etc/anima/selinux-disabled`.

   **To turn SELinux on instead** (now, or later in a window the operator agrees). On AlmaLinux 10
   the config file alone cannot disable SELinux, so Disabled means `selinux=0` on the kernel
   command line. Remove it, set Permissive (log denials, enforce nothing) and relabel the whole
   file system at the next boot. Keep the Hostinger browser console open: the relabel reboots
   twice and takes some minutes. Check `rpm -q container-selinux` first.

   ```bash
   # Runs on: VPS (SSH as <ADMIN_USER>)
   sudo grubby --update-kernel ALL --remove-args selinux
   sudo sed -i 's/^SELINUX=.*/SELINUX=permissive/' /etc/selinux/config
   sudo touch /.autorelabel
   sudo systemctl reboot
   ```

   Check, after you log in again:

   ```bash
   # Runs on: VPS (SSH as <ADMIN_USER>)
   getenforce                                  # Permissive
   grep -c selinux=0 /proc/cmdline             # 0
   ls -Zd /home/<ADMIN_USER>/.ssh              # ...:ssh_home_t:...
   sudo docker ps                              # the containers from step 0 are up again
   ```

   Check each existing service as its owner would (for a tunnelled web app: open it). Undo:
   `sudo sed -i 's/^SELINUX=.*/SELINUX=disabled/' /etc/selinux/config`,
   `sudo grubby --update-kernel ALL --args selinux=0`, `sudo systemctl reboot`. Remove
   `/etc/anima/selinux-disabled` if it exists; `check` reports it once SELinux is on.

**Operator change 3, 4 (confirm in chat first: VPS-005).**

1. Check the system again as `<ADMIN_USER>`:

   ```bash
   # Runs on: VPS (SSH)
   . /etc/os-release && echo "$PRETTY_NAME"     # must say AlmaLinux 10.x
   getenforce                                   # Enforcing or Permissive; Disabled only with the deviation (2.0b)
   ```

2. Install Docker Engine and the compose plugin from Docker's RHEL repository. **If the
   inventory showed Docker already installed** (a `dockerd` listener, a container, or
   `/etc/docker/daemon.json`), skip this step: do not reinstall over it, and tell the programme.
   Step 4.4 then needs a decision first (see there).

   ```bash
   # Runs on: VPS (SSH)
   sudo dnf -y install dnf-plugins-core
   sudo dnf config-manager --add-repo https://download.docker.com/linux/rhel/docker-ce.repo
   sudo dnf -y install docker-ce docker-ce-cli containerd.io docker-compose-plugin
   sudo systemctl enable --now docker
   ```

   `dnf` imports the repository's signing key on first use. Check that it is Docker's (key ID
   `621e9f35`, the end of Docker's fingerprint `060A 61C5 1B55 8A7F 742B 77AA C52F EB6B 621E 9F35`):

   ```bash
   # Runs on: VPS (SSH)
   rpm -q gpg-pubkey --qf '%{version} %{summary}\n'      # 621e9f35 Docker Release (CE rpm)
   sudo docker version --format '{{.Server.Version}}'     # must be 28 or newer
   ```

   **Preflight: Docker 28 or newer.** Before 28, Docker turns on `route_localnet`, so a port
   published on `127.0.0.1` is reachable from the network. If Docker was already installed
   and is older, stop and tell the programme; `harden.sh check` reports it as a violation, and
   `docker` and `firewall` refuse to run.

3. Install Tailscale from Tailscale's RHEL 10 repository, EPEL (for `restic`) and the other
   tools. AlmaLinux provides `sqlite`, `jq`, `gettext-envsubst` and `policycoreutils-python-utils`;
   `restic` is only in EPEL:

   ```bash
   # Runs on: VPS (SSH)
   sudo dnf config-manager --add-repo https://pkgs.tailscale.com/stable/rhel/10/tailscale.repo
   sudo dnf -y install epel-release
   sudo dnf -y install tailscale sqlite restic jq gettext-envsubst policycoreutils-python-utils
   rpm -q gpg-pubkey --qf '%{version} %{summary}\n'      # adds 957f5868 Tailscale and e37ed158 EPEL 10
   ```

4. The files are in `~/anima-headscale` since step 0. `render.sh` reads the template and
   writes the finished config somewhere else, so keep that copy in your home folder and install
   from it. If you change the repository later, copy it again from your Mac:

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

   Label Headscale's two folders for containers (SELinux, D-9). `harden.sh selinux` records the
   rule `/opt/anima/headscale/(config|lib)(/.*)?` → `container_file_t` with `semanage fcontext`
   and applies it with `restorecon`. The rule is permanent, so a relabel keeps it, and every file
   you later create in those folders takes the label. It must be in place before step 4.4 turns
   on SELinux labels in Docker:

   ```bash
   # Runs on: VPS (SSH)
   sudo bash ./vps/harden.sh selinux
   ls -Zd /opt/anima/headscale/config /opt/anima/headscale/lib     # both ...:container_file_t:s0
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
   only in this SSH session; define it again in each new session. `-T` lets you pipe its output;
   `</dev/null` stops it from reading the rest of a script you feed to `bash -s`.

   ```bash
   # Runs on: VPS (SSH)
   sudo docker compose -f /opt/anima/headscale/compose.yaml up -d
   hsctl() { sudo docker compose -f /opt/anima/headscale/compose.yaml exec -T headscale headscale "$@" </dev/null; }
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

   If any key is listed, expire it (`hsctl apikeys expire --prefix <PREFIX>`), find out who made
   it, and treat the server as suspect until you know.

10. Create the user. Client computers belong to this user:

    ```bash
    # Runs on: VPS (SSH)
    hsctl users create operator
    hsctl users list            # operator is listed
    ```

## 3. The VPS joins the VPN as the hub

**Operator change 4 (confirm in chat first: VPS-005).** The VPS joins its own VPN with the name `hub`
and the tag `tag:hub`. A tagged node never expires (VPN-003), so no renewal is needed.

1. Create a tagged, single-use pre-auth key that is valid for one hour, straight into a file that only
   root can read. The key is never printed. (`sh -c` is used so that the shell, not `sudo`, makes the
   file; `sudo` writes every command line to `/var/log/secure` and the journal, so a key on a
   command line would be logged.)

   ```bash
   # Runs on: VPS (SSH)
   sudo -v                              # ask for the sudo password first, so the pipeline below does not stall
   hsctl preauthkeys create --tags tag:hub --expiration 1h | sudo sh -c 'umask 077; cat > /run/anima-preauth'
   sudo wc -c /run/anima-preauth        # a few dozen characters: the file is not empty
   ```

2. Join with the key file, delete it, and check that Headscale marks the key as used. The key is
   single use and valid for one hour:

   ```bash
   # Runs on: VPS (SSH)
   sudo systemctl enable --now tailscaled
   sudo tailscale up --login-server https://hs.anima.ing --auth-key=file:/run/anima-preauth --hostname hub --accept-routes --accept-dns=false --snat-subnet-routes=false
   sudo rm -f /run/anima-preauth
   hsctl preauthkeys list               # the key shows as used
   ```

   `--accept-routes` lets the hub reach the site subnets once the routers are approved (step 5).
   `--accept-dns=false` leaves the VPS's resolver alone (R-050): without systemd-resolved,
   tailscaled would rewrite `/etc/resolv.conf` to `100.100.100.100`, and every container started
   later would depend on it for all DNS. Nothing on the VPS needs MagicDNS names.
   `--snat-subnet-routes=false` keeps a client's VPN address on its way to the hub: otherwise
   tailscaled, in iptables mode, masquerades everything forwarded from `tailscale0`, so Docker
   hands the hub `172.30.0.1` and the hub refuses every client (NET-002). The VPS advertises no
   routes, so nothing needs that masquerade.
   If the key is not marked used (the join failed), expire it:
   `hsctl preauthkeys expire --id <KEY_ID>`.

3. Read the VPN address Headscale gave the hub, and check the node:

   ```bash
   # Runs on: VPS (SSH)
   tailscale ip -4                 # for example 100.64.0.1: this is <HUB_VPN_IPV4>
   hsctl nodes list                # hub, tag:hub, no expiry
   ```

4. **Operator change 5 (confirm in chat first: DNS-008).** Add the `hub` DNS record. The token from
   step 1 is gone, so create a second, short-lived operator token (end date: today) and repeat the
   apply steps of [`deploy/dns/README.md`](../../deploy/dns/README.md): the child shell, the
   header file, `ZONE_ID`, the helpers, then the `hub` row with `HUB_VPN_IPV4` set to the address
   above. Revoke the token when you finish. Check:

   ```bash
   # Runs on: operator's Mac
   dig +short hub.anima.ing @1.1.1.1     # must print <HUB_VPN_IPV4>
   ```

5. Check the MagicDNS name inside the VPN, from a client (the VPS joined with
   `--accept-dns=false`, so it does not resolve `net.anima.ing` names itself):

   ```bash
   # Runs on: a client on the VPN (step 6)
   getent hosts hub.net.anima.ing        # must print <HUB_VPN_IPV4> (macOS: dscacheutil -q host -a name hub.net.anima.ing)
   ```

The hub program itself, and the separate Cloudflare token it uses for its certificate (DNS-002,
DNS-003), come with the hub guide (WP-04). Until then nothing listens on `hub.anima.ing`. If the
name does not resolve at a site, the client can also take the hub address typed by the operator
(NET-005, DNS-001).

## 4. Harden the VPS

The script `deploy/headscale/vps/harden.sh` changes the VPS in small steps, each with a guard
against locking you out. Run it from a copy of the repository folder on the VPS (the script reads
the config files beside it). Do these in order, one command at a time.

**Operator change 6, 6a, 6b, 7, 8, 9 (confirm in chat first: VPS-005).** Every `harden.sh` command has
`--dry-run` (for example `./harden.sh --dry-run firewall`), which prints what it would do and
changes nothing.

0. Get the script to a root-owned place and set the admin user. `ANIMA_ADMIN_USER` is the account
   you log in with (step 2.0); it must be non-root, in group `wheel`, with a key in `authorized_keys`.

   ```bash
   # Runs on: VPS (SSH)
   sudo rm -rf /opt/anima/headscale/vps
   sudo cp -r ~/anima-headscale/vps /opt/anima/headscale/vps
   sudo chown -R root:root /opt/anima/headscale/vps
   export ANIMA_ADMIN_USER=<ADMIN_USER>
   cd /opt/anima/headscale/vps
   ```

1. `check` reports and changes nothing. It prints the inventory of step 2.0 again, then the
   violations. Compare the inventory with the one you sent: anything new that is marked `!` is a
   reason to stop and ask. Before the hardening it lists violations (SSH password login, no
   automatic updates, Docker without SELinux labels, no firewall table, no `nftables` drop-in,
   firewalld not masked, listeners). That is expected. It also reports Docker older than
   version 28 (or a version it cannot read), any interface other than `lo`, `tailscale0`,
   `docker0` and `br-*` with an address in `172.16.0.0/12`, and fails if `tailscale0` is
   missing. Run it as the same admin user as in the next
   steps, with `ANIMA_ADMIN_USER` exported.

   ```bash
   # Runs on: VPS (SSH)
   sudo --preserve-env=ANIMA_ADMIN_USER ./harden.sh check
   ```

2. `ssh` installs the SSH rules: keys only, no root login, only `ANIMA_ADMIN_USER` may log in
   (VPS-001). It installs `/etc/ssh/sshd_config.d/00-anima.conf`; AlmaLinux reads the drop-ins
   before the main file and keeps the first value, so `00-` wins over AlmaLinux's
   `01-permitrootlogin.conf` (or `25-`), `50-redhat.conf` and cloud-init's file. Guards: it refuses
   unless that user is a non-root member of `wheel` with a key in `authorized_keys`; it refuses
   while `/etc/ssh/sshd_config` or any file it includes has an active `Match` line in any form
   (`Match`, `match=`, `"Match"`; a provider image can carry one that turns passwords or root
   login back on for some addresses, and `sshd -T` alone does not show it: remove the block,
   then run `ssh` again), and while any `Include` is written in a form it cannot follow
   literally (`Include=`, quotes, a path with spaces: rewrite it as `Include /path/pattern`); it checks the result
   with `sshd -t`, `sshd -T` and `sshd -T -C` for root and the admin user from a public and a VPN
   address, and puts the old file back if any check fails; then it reloads `sshd.service`. **Keep your current session open.** Open a second session and log in
   before you close the first.

   Before you run it, prove your way back in: log in at the Hostinger browser console as
   `<ADMIN_USER>` with the password from step 2.0 and run `sudo -v`. SSH password login stays
   off; the password is only for the console and `sudo`. Do not go on until `sudo -v` works at the
   console.

   ```bash
   # Runs on: VPS (SSH)
   sudo --preserve-env=ANIMA_ADMIN_USER ./harden.sh ssh
   ```

3. **If other containers run and Docker's `live-restore` is off** (`docker info --format
   '{{.LiveRestoreEnabled}}'` says `false`), do step 4 before this one: the first automatic
   update of `docker-ce` or `containerd.io` would otherwise restart Docker and stop every
   container (R-050).

   `updates` turns on automatic updates (VPS-004) with `dnf-automatic` (AlmaLinux 10 uses dnf 4;
   there is no dnf5 plugin). It installs **all** updates, not only those marked as security
   fixes: dnf finds security fixes through advisories, and Tailscale's repository publishes none,
   so a security-only setting would never update `tailscaled`. Every enabled repository is
   included: AlmaLinux, EPEL, Docker and Tailscale. The timer runs at 03:45 UTC, after the
   backup; when an update needs a reboot (kernel, glibc, systemd and the like), the VPS reboots at
   04:30 UTC.

   ```bash
   # Runs on: VPS (SSH)
   sudo ./harden.sh updates
   systemctl list-timers dnf-automatic.timer --no-pager    # next run 03:45 UTC (plus up to 10 min)
   dnf repolist                                            # docker-ce-stable and tailscale-stable are listed
   timedatectl | grep 'Time zone'                          # UTC; the 04:30 reboot uses local time
   ```

   A Docker or Tailscale upgrade restarts its daemon: containers keep running (`live-restore`),
   and the VPN drops for a few seconds.

   An update can also bring in a new package that enables its own service. For example, `bluez`
   arrived as a new dependency and enabled `bluetooth.service`. `check` then marks the service
   `!` under `== Enabled services`. After an update run, and after its reboot, run `check`
   again. Disable a new service the VPS does not need (`sudo systemctl disable --now NAME`). If
   the operator keeps one, add it as a `service NAME` line to `/etc/anima/known-workloads`
   (step 2.0a). Do not add it to `stock-units.txt`: that file lists AlmaLinux's stock units
   only. The old `/etc/dnf/automatic.conf` is kept as
   `/var/lib/anima/automatic.conf.previous`.

4. `docker` merges `daemon.json` into `/etc/docker/daemon.json`: published ports bind to
   `127.0.0.1` by default (Docker would otherwise bypass the firewall), `selinux-enabled` makes
   Docker run containers confined (`container_t`; Docker CE on AlmaLinux does not do this by
   default, so Headscale would run unconfined as `spc_t`), logs are rotated (unless the existing
   file sets its own `log-driver` or `log-opts`: those win, R-050), `live-restore`
   keeps containers running while Docker restarts, and `userland-proxy: false` makes Docker
   forward published ports by DNAT, so a container sees the client's own address instead of
   Docker's proxy (the hub refuses 172.16.0.0/12 peers, NET-002). It refuses while Headscale's folders are not
   labelled `container_file_t` (step 2.5). With `--restart-docker` it first reloads Docker, which
   turns `live-restore` on from the new file, refuses to go on unless it is on, then restarts
   Docker: every running container keeps running, and it fails naming any container that did not.
   Then recreate Headscale: a container keeps the settings and label it was created with, so
   Headscale stays unconfined (`spc_t`) until it is created again. Clients reconnect within
   seconds.

   **If Docker was installed before this guide (step 2.2), decide with the programme first**
   (step 2.0a). This step changes Docker's defaults for every container created later; running
   containers keep theirs, and only their owner recreates them. On the next creation, a container
   that publishes a port without an address (`-p 8080:80`) moves to `127.0.0.1:8080`, so a
   service reached from outside stops working; with SELinux on, a container with a bind mount
   that is not labelled for containers loses access to it. The previous file is kept as
   `/var/lib/anima/daemon.json.previous`.

   ```bash
   # Runs on: VPS (SSH)
   sudo ./harden.sh --restart-docker docker
   sudo docker compose -f /opt/anima/headscale/compose.yaml up -d --force-recreate
   sudo docker compose -f /opt/anima/headscale/compose.yaml ps       # headscale: healthy again
   sudo docker info --format '{{.SecurityOptions}}'                  # lists name=selinux
   sudo docker inspect -f '{{.ProcessLabel}}' \
     "$(sudo docker compose -f /opt/anima/headscale/compose.yaml ps -q headscale)"   # ...:container_t:s0:c...
   sudo ausearch -m avc -ts recent                                   # <no matches>
   sudo docker ps                                                    # the containers of step 0 are still up
   ```

   With the SELinux deviation (2.0b), skip the three SELinux lines: `docker info` does not list
   `name=selinux` and the label is empty. `harden.sh check` reports a Headscale container that is not `container_t`. If `ausearch`
   shows denials for Headscale, or it is not healthy, undo, and report the denial lines. The undo
   keeps `live-restore` on (R-050): a Docker daemon that starts without it stops every container
   it finds running.

   ```bash
   # Runs on: VPS (SSH)
   sudo docker info --format '{{.LiveRestoreEnabled}}'     # must say true; otherwise stop and report
   if [ -f /var/lib/anima/daemon.json.previous ]; then
     sudo jq '. + {"live-restore": true}' /var/lib/anima/daemon.json.previous | sudo tee /etc/docker/daemon.json
   else
     echo '{"live-restore": true}' | sudo tee /etc/docker/daemon.json
   fi
   sudo systemctl restart docker
   sudo docker ps                                           # the containers of step 0 are still up
   sudo docker compose -f /opt/anima/headscale/compose.yaml up -d --force-recreate
   ```

4a. **Only when SELinux was turned on (2.0b): other containers, then 24 hours in Permissive.**
   Only containers created after this step run confined. The owner of each other project, when
   they agree, recreates it with its own compose file
   (`sudo docker compose up -d --force-recreate` in its folder) and checks the service works.
   Then wait at least 24 hours, so every scheduled job has run once, and read the denials since
   then:

   ```bash
   # Runs on: VPS (SSH)
   sudo docker ps -q | xargs sudo docker inspect -f '{{.Name}} {{.ProcessLabel}}'   # all container_t
   sudo ausearch -m avc -ts <TIME_OF_STEP_4A> | audit2why
   ```

   A denial for a bind mount of another project is fixed in that project's compose file (`:z`
   on that mount), or with `sudo semanage fcontext -a -t container_file_t '<DIR>(/.*)?'` and
   `sudo restorecon -R <DIR>`. Docker labels named volumes itself. Never label
   `/opt/anima/headscale` for another container (D-9). **Stop and report** any denial you cannot
   explain, including those for host agents such as `monarx-agent` or `sssd`; step 4b would block
   it.

4b. **Only when SELinux was turned on: Enforcing.** Turn enforcement on now, before the firewall: its rollback probe (step 4.6)
   must run under the SELinux mode a real rollback would meet.

   ```bash
   # Runs on: VPS (SSH)
   sudo setenforce 1
   sudo sed -i 's/^SELINUX=.*/SELINUX=enforcing/' /etc/selinux/config
   getenforce                                  # Enforcing
   sudo ausearch -m avc -ts recent             # <no matches>
   sudo ./harden.sh check                      # no SELinux line
   ```

   Check Headscale and each other service again. Undo: `sudo setenforce 0` and
   `sudo sed -i 's/^SELINUX=.*/SELINUX=permissive/' /etc/selinux/config`.

5. Before the firewall: the Mac you administer from must be on the VPN. Do section 6, A to C,
   for that Mac now. The VPN is not closed yet, so you approve it from an SSH session over the
   **public** address, which still works. Then return here. Test:

   ```bash
   # Runs on: operator's Mac
   ssh <ADMIN_USER>@<HUB_VPN_IPV4>      # must log in over the VPN (the policy allows tcp 22 to the hub)
   ```

6. `firewall` applies the nftables rules (VPS-002): drop everything inbound except 443/tcp,
   3478/udp, 41641/udp and, on the VPN interface only, 22/tcp. Guards: it refuses if `tailscale0`
   is missing; it refuses unless your SSH session itself comes over the VPN (or you pass
   `--console`); it validates the rules before it applies them; it proves that the rollback can
   run as a systemd unit under SELinux; and it **arms a rollback before applying**.

   It also **refuses while a port that is open now would close**: every public listener and every
   port firewalld allows (ports and services of the active zones), except 22/tcp (SSH moves to the
   VPN on purpose) and the Anima ports. It prints each one as `would close:`. A port that Docker
   publishes (`docker-proxy`) is printed as a `note:` instead: Docker's rules run before this
   firewall, so it stays reachable until step 4 moves it to `127.0.0.1` when the container is
   recreated. Anything firewalld does that is not a plain port shows as a token in place of the
   port: `public:rich-rules`, `public:sources`, `public:forward-ports`, `public:source-ports`,
   `public:protocols`, `public:masquerade`, `public:target` (the zone name first),
   `service:NAME` (a service that includes others or opens protocols or source ports),
   `policy:NAME` (a firewalld policy; `allow-host-ipv6` and Docker's `docker-forwarding` pass
   only in their stock form, as `sudo firewall-cmd --info-policy=NAME` prints it, `(active)`
   after the name included) or `direct-rules`. Read it with
   `sudo firewall-cmd --zone=public --list-all`, `--info-service=NAME`, `--info-policy=NAME` or
   `--direct --get-all-rules`, and send it to the programme. For each, decide:

   - **Keep it open** (another service on the VPS needs it from the internet): add one line
     `PORT/PROTO` (for example `8080/tcp`) to `/etc/anima/firewall-allow`. The file may hold only
     single tcp or udp ports; it never opens 22/tcp and never touches the VPN rules.
   - **Close it on purpose** (for example firewalld's default `cockpit` 9090/tcp and
     `dhcpv6-client` 546/udp; keep 546/udp only if the VPS gets IPv6 by DHCPv6): pass
     `--allow-closing 9090/tcp,546/udp` to both `firewall` and `confirm`. A token closes the same
     way (for example `--allow-closing 9090/tcp,public:rich-rules`); keep the ports it opened in
     the allow file if they must stay open.

   It refuses while Docker is older than 28, and while an interface other than `lo`,
   `tailscale0`, `docker0` and `br-*` holds an address in `172.16.0.0/12` (a provider private
   network, for example): the rules drop that range unless it comes from those interfaces, so
   a neighbour with a static route cannot reach a container's own address past Docker's
   published ports (NET-002). Send that interface to the programme.

   It refuses while `/etc/sysconfig/nftables.conf` holds anything but comments and the Anima
   include: AlmaLinux ships it as comments only, and a rule or an include there could flush
   Docker's and Tailscale's tables at every boot. Send those lines to the programme. `confirm`
   also refuses while `nftables.service` has a drop-in other than Anima's, a unit file in `/etc`
   or `/run` that overrides the package's, or while `iptables.service` or `ip6tables.service` is
   enabled: any of them could flush those tables. After it installs the drop-in it reads the
   unit as systemd loaded it (`systemctl show`), and stops before masking firewalld if the unit
   still runs anything but `nft -f /etc/sysconfig/nftables.conf`.

   Run it inside an SSH session you opened to `<HUB_VPN_IPV4>`, not to the public address:

   ```bash
   # Runs on: VPS (SSH over the VPN: ssh <ADMIN_USER>@<HUB_VPN_IPV4>)
   export ANIMA_ADMIN_USER=<ADMIN_USER>
   cd /opt/anima/headscale/vps
   sudo --preserve-env=ANIMA_ADMIN_USER,SSH_CONNECTION ./harden.sh --allow-closing <PORTS> firewall
   ```

   (Leave out `--allow-closing <PORTS>` if nothing would close.)

   Before it changes anything, `firewall` saves the whole ruleset for the record. On a host where
   Docker runs, `nft list ruleset` can print warnings about Docker's tables, such as
   `table ip nat is managed by iptables-nft, do not touch!` and `XT target MASQUERADE not found`.
   They are harmless: the rollback restores only `inet anima_filter` and never loads that saved
   file, and Docker's tables are not changed.

7. `confirm`. You have 180 seconds. Open a NEW SSH session over the VPN (a different session from
   the one that ran `firewall`) and run:

   ```bash
   # Runs on: VPS (a NEW SSH session over the VPN)
   export ANIMA_ADMIN_USER=<ADMIN_USER>
   cd /opt/anima/headscale/vps
   sudo --preserve-env=ANIMA_ADMIN_USER,SSH_CONNECTION ./harden.sh --allow-closing <PORTS> confirm
   ```

   `confirm` refuses to run in the same session that ran `firewall`, in any SSH connection that
   was already open when `firewall` ran (only a connection made through the new rules proves that
   they let you in), or in one that does not come over the VPN. It checks the open ports and the
   boot file again, and has `nft -c` check the boot file with the new rules before it changes
   anything and again before it masks firewalld. It cancels the rollback, installs the rules as
   `/etc/nftables/anima.nft`, adds `include "/etc/nftables/anima.nft"` to the end of
   `/etc/sysconfig/nftables.conf` (the file AlmaLinux's `nftables.service` loads at boot; the
   rest of the file is kept, and its first version is saved as
   `/var/lib/anima/nftables.conf.previous`), installs
   `/etc/systemd/system/nftables.service.d/anima.conf`, enables `nftables.service`, and masks
   firewalld (`systemctl mask`, plus `stop` if it is running). The Anima table is then the only host firewall:
   firewalld's rules are not covered by the rollback, and Docker keeps its own rules without it.

   AlmaLinux's `nftables.service` flushes the whole ruleset on stop **and on reload**. The drop-in
   clears the stop action and replaces the reload with a plain load: `systemctl stop nftables`,
   a reload, an update of the nftables package and a shutdown keep the live rules instead of
   flushing every table, including Docker's and Tailscale's, so the host is never left
   unfiltered. To remove the firewall you must do it on purpose, with
   `sudo nft delete table inet anima_filter` (step 8 below).

   The rules also drop packets for this host's own VPN addresses that did not arrive on `lo` or
   `tailscale0` (DNS-005), and a forward rule drops traffic into `tailscale0` unless it comes from
   `lo`, `tailscale0`, `docker0` or a `br-*` bridge. Four more rules contain the optional Apprise
   container (`172.30.3.10`, NOT-002): it reaches no address of this host, no private, VPN, loopback,
   link-local, Docker or multicast range and nothing through `tailscale0`, only the internet and its
   replies to the hub (`deploy/README.md`, Phase 3 operator changes, step 7).

   If you do nothing, or the new session cannot connect, after 180 seconds the previous rules come
   back by themselves. The VPS is then as open as before, so keep 22/tcp in the Hostinger firewall
   until `confirm` succeeds. Find the cause and run `firewall` again.

8. **Recovery when SSH over the VPN does not work.** Open the Hostinger browser console, log in
   (you proved this in step 4.2), and take the firewall off:

   ```bash
   # Runs on: VPS (Hostinger browser console)
   sudo systemctl stop anima-firewall-rollback.timer      # only if the rollback is still armed
   sudo nft delete table inet anima_filter                # removes the Anima firewall now
   sudo systemctl disable nftables.service                # the rules must not come back at boot
   sudo rm -f /etc/nftables/anima.nft
   sudo ls /var/lib/anima/nftables.conf.previous          # the old /etc/sysconfig/nftables.conf
   sudo cp /var/lib/anima/nftables.conf.previous /etc/sysconfig/nftables.conf
   sudo ls /var/lib/anima/firewalld-was-active            # present if firewalld ran before confirm
   sudo systemctl unmask firewalld && sudo systemctl enable --now firewalld     # only if it was
   sudo systemctl status tailscaled --no-pager            # fix the cause: restart it, read the log
   sudo journalctl -u tailscaled -n 50 --no-pager
   tailscale status
   ```

   (`/var/lib/anima` is readable by root only, so use `sudo`.) Skip the `cp` line if the `ls` finds
   no file, and the firewalld line if its `ls` finds none. Then repeat 4.5 to 4.7 over the VPN. The VPS is
   open on the public interface while the firewall is off; keep 22/tcp in the Hostinger firewall
   until you are done. Do not apply the firewall with `--console` unless you have proved SSH over the
   VPN works from another machine; with `--console` the script cannot check your session.
   `confirm --console` warns that SSH over the VPN was not proven: before you close the console,
   open a NEW SSH login over the VPN.

9. Run `check` again from the VPN session. It must say `ok: no violations`. It now also needs the
   Anima table, the `nftables` unit enabled with the boot include and the drop-in, firewalld
   masked, SELinux enforcing with Headscale's labels, Docker with SELinux labels, `dnf-automatic`
   and `tailscale0`:

   ```bash
   # Runs on: VPS (SSH over the VPN)
   sudo --preserve-env=ANIMA_ADMIN_USER ./harden.sh check
   ```

10. Note for the hub guide (WP-04): the firewall drops packets addressed to this host's own VPN
    address that did not arrive on `lo` or `tailscale0`. A container on a Docker bridge network
    therefore cannot connect to the host's own VPN address, but it can reach other VPN peers and
    the site subnets. Treat this as a WP-04 note for the hub's network setup.

11. **Operator change 9 (confirm in chat first: VPS-005).** In the Hostinger panel, remove the 22/tcp rule that you added in step 1
    of this guide, then sync the firewall and check `is_synced: true` (step 1). SSH now works
    only over the VPN. (`sshd` itself still listens on all addresses;
    the `nftables` rules are what restrict port 22 to the VPN interface.)

## 5. Subnet routers

A subnet router makes the devices of one site reachable over the VPN. The devices need no VPN
software. Do this on the home Linux PC (`home-router`, `10.10.0.0/24`, tag `tag:site-home`) and on
the office 2 Linux PC (`office2-router`, `10.30.0.0/24`, tag `tag:site-office2`). Office 1 has no
router in Phase 1: its Mac is a client, and its devices are not reachable until a router is added.

**Operator change 10 (confirm in chat first: VPS-005).**

All Tailscale clients in this guide must be **version 1.80 or later**: Headscale 0.29.7 refuses
older clients.

Copy tag names exactly from `policy.hujson`. Headscale 0.29.7 also accepts a pre-auth key with a
tag that is not in the policy (for example a typo), so after each join check that `hsctl nodes
list` shows exactly the tags you expect.

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
   valid for one hour, and shown once on your screen. Read it there; do not copy it:

   ```bash
   # Runs on: VPS (SSH over the VPN)
   hsctl preauthkeys create --tags tag:router,tag:site-home --expiration 1h
   ```

3. On the router, type the key into a file that only root can read (`sudo` logs command lines, so
   the key must not be on one), join from that file, and delete it. Keep the default
   `--snat-subnet-routes`: devices at the site then see the router as the source and answer to
   it, so they need no extra route. Do not add `--accept-routes` on a router.

   ```bash
   # Runs on: home Linux PC
   sudo bash -c 'umask 077; read -rsp "Pre-auth key: " K; echo; printf "%s" "$K" > /run/anima-preauth'
   sudo tailscale up --login-server https://hs.anima.ing --auth-key=file:/run/anima-preauth \
     --advertise-routes=10.10.0.0/24 --hostname home-router
   sudo rm -f /run/anima-preauth
   ```

   ```bash
   # Runs on: office 2 Linux PC (key made with --tags tag:router,tag:site-office2)
   sudo bash -c 'umask 077; read -rsp "Pre-auth key: " K; echo; printf "%s" "$K" > /run/anima-preauth'
   sudo tailscale up --login-server https://hs.anima.ing --auth-key=file:/run/anima-preauth \
     --advertise-routes=10.30.0.0/24 --hostname office2-router
   sudo rm -f /run/anima-preauth
   ```

   Clear the VPS terminal and its scrollback (on macOS Terminal press Cmd-K, or run `clear && printf '\e[3J'`), then check that each key is marked used. Expire any key that
   was not used:

   ```bash
   # Runs on: VPS (SSH over the VPN)
   hsctl preauthkeys list
   hsctl preauthkeys expire --id <KEY_ID>        # only for a key that shows as unused
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

**Operator change 7 and 11 (confirm in chat first: VPS-005).** Each Mac (and any Linux client) is a
**personal node** of the user `operator`, not a tagged node.
Tagged nodes never expire, and a client key must expire after 180 days (VPN-003). Do not add
`--advertise-tags` on a client, ever.

### A. Install Tailscale

Use Tailscale 1.80 or later (Headscale 0.29.7 refuses older clients).

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

2. The command prints a web address that ends in an **auth ID**. Read the ID from the screen of the
   computer you are sitting at. Type that ID into the `hsctl auth register` command yourself.
   **Never approve an ID that reached you by chat, mail or a link**: an attacker who starts a login
   can send you their ID.

3. Approve on the VPS. For the first Mac, before the firewall is applied, you reach the VPS over
   SSH to the **public** address; afterwards, over the VPN address:

   ```bash
   # Runs on: VPS (SSH)
   hsctl auth register --user operator --auth-id <AUTH_ID_FROM_THE_DEVICE>
   hsctl nodes list          # this hostname is new, registered just now
   ```

   Then check on the client that it reports being logged in (`"$TS" status` shows its VPN
   address). If the node is not the one you expect, or the client does not report it, delete the
   node: `hsctl nodes delete --identifier <NODE_ID>`.

4. Check the node. **It must show no tags**, and the expiry must be no later than today plus 180
   days. A client may ask for its own expiry, so an empty or later expiry is possible and must be
   fixed. The `[ -n "$EXP" ]` guard matters: an empty `--expiry` expires the node immediately.

   ```bash
   # Runs on: VPS (SSH)
   hsctl nodes list
   EXP=$(date -u -d '+180 days' +%Y-%m-%dT%H:%M:%SZ) && [ -n "$EXP" ] && hsctl nodes expire --identifier <NODE_ID> --expiry "$EXP"
   ```

   If the node shows a tag, delete it and register it again after you have removed
   `--advertise-tags` from the client's command: `hsctl nodes delete --identifier <NODE_ID>`.

5. **Stagger the two Macs.** Two Macs registered on the same day expire on the same day. Give the
   second Mac an expiry of `+150 days` instead of `+180 days` in the command above, so that the
   expiry dates are about 30 days apart and one Mac with a valid key is always there to approve the
   other (step 7).

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

**Operator change 14, recurring (confirm in chat first: VPS-005).**

| Node | Tag | Expires |
| --- | --- | --- |
| Hub (VPS) | `tag:hub` | never |
| Routers | `tag:router` plus a site tag | never |
| Client computers | none (personal nodes) | 180 days after registration |

See the dates:

```bash
# Runs on: VPS (SSH over the VPN)
hsctl nodes list          # read the Expiry column; put a reminder in your calendar 30 days before each one
```

**Why you must renew early.** You administer the VPS only over the VPN. A Mac whose key has
expired is off the VPN, so it cannot reach the VPS to approve its own renewal. Renewing also drops
that Mac's connection until the login is approved. So:

- Renew each Mac about 30 days **before** its expiry. Never wait for "needs login".
- Approve the renewal from an SSH session on the **other** Mac, which still has a valid key. That is
  why the two expiries are staggered (step 6B.5).
- If both Macs have expired, use the Hostinger browser console (step 4.2): log in there, define
  `hsctl` again, and approve the login with the auth ID read from the Mac's screen.

Renew:

1. On the Mac to renew, start the login again with the same profile. Its connection drops until
   the login is approved:

   ```bash
   # Runs on: Mac at office 1 (or: Mac at office 2)
   "$TS" up --force-reauth --login-server https://hs.anima.ing --accept-routes
   ```

   If `tailscale up` refuses because other non-default flags are set, run it again with the flags
   that its error message lists.

2. On the other Mac, open an SSH session to the VPS over the VPN and approve it, typing the ID
   shown on the Mac being renewed (never one from chat, mail or a link):

   ```bash
   # Runs on: VPS (SSH over the VPN, opened from the other Mac)
   hsctl auth register --user operator --auth-id <AUTH_ID_FROM_THE_DEVICE>
   ```

3. Run the checks of step 6B.3 and 6B.4 again: the node is the one you expect, it reports logged
   in, it has **no tags**, and its expiry is within 180 days. After a renewal the expiry is 180 days
   from the approval; do not apply the `+150` days again. The stagger holds because you renew each
   Mac about 30 days early, on different days. A
   renewal is the moment a taken-over client could ask for a tag (and with it a key that never
   expires), so check the tags every time.

## 8. Access rules (VPN-004)

**Operator change 12 (confirm in chat first: VPS-005).** The policy is
`deploy/headscale/config/policy.hujson`. Anything it does not allow is denied.

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

2. Copy it to the VPS as a candidate and check it against the running server as a smoke test
   (Headscale's own test run is less strict than the bun tests):

   ```bash
   # Runs on: operator's Mac, from the repository root
   scp deploy/headscale/config/policy.hujson <ADMIN_USER>@<HUB_VPN_IPV4>:policy.hujson
   ```

   ```bash
   # Runs on: VPS (SSH over the VPN)
   sudo install -m 0644 ~/policy.hujson /opt/anima/headscale/config/policy.hujson.new
   hsctl policy check --file /etc/headscale/policy.hujson.new
   ```

   `--bypass-grpc-and-access-database-directly` is only for when the server is not running.
   If the check fails, delete the candidate and stop:
   `sudo rm -f /opt/anima/headscale/config/policy.hujson.new`.

3. Keep the old file, install the new one and reload (no restart; connections stay up):

   ```bash
   # Runs on: VPS (SSH over the VPN)
   sudo cp -p /opt/anima/headscale/config/policy.hujson /opt/anima/headscale/config/policy.hujson.old
   sudo mv /opt/anima/headscale/config/policy.hujson.new /opt/anima/headscale/config/policy.hujson
   sudo kill -HUP "$(sudo docker inspect -f '{{.State.Pid}}' headscale-headscale-1)"
   sudo docker compose -f /opt/anima/headscale/compose.yaml logs --tail 20 headscale
   ```

   Send the signal to the process, not with `docker kill -s HUP`: Docker counts any
   `docker kill` as a manual stop, and before the restart policy became `always` that left
   Headscale, and with it the VPN and the hub, down after the next reboot.

   If the log shows the reload was rejected, put the old file back at once, because the VPS
   loads the file on its 04:30 reboot or at the next restart:
   `sudo mv /opt/anima/headscale/config/policy.hujson.old /opt/anima/headscale/config/policy.hujson`.

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

The backup of the Anima hub database (VPS-006) comes with the hub guide (WP-04) and goes to the
same `rest-server`, in its own repository.

The backup uses the home router (step 5) and the hub's `--accept-routes` (step 3). Do those first.

**Operator change 13 and 15 (confirm in chat first: VPS-005).**

1. Home PC: install `restic`, `jq` (the prune script needs it), `sqlite3` (the restore drill needs it), `apache2-utils` (for `htpasswd`) and the `rest-server` binary
   (download a release from <https://github.com/restic/rest-server/releases>, check its checksum,
   and install it as `/usr/local/bin/rest-server`). Create the user and the folders:

   ```bash
   # Runs on: home Linux PC
   sudo useradd --system --user-group --no-create-home --shell /usr/sbin/nologin restic
   sudo install -d -o restic -g restic -m 0700 /srv/restic
   sudo install -d -m 0755 /etc/restic
   ```

2. Home PC: the login of the VPS, and the files the units read. These hold secrets. Generate
   both passwords in your password manager. Make the rest-server password letters and digits only
   (at least 32 characters), so it needs no URL encoding. Type each one at the prompts (here and on
   the VPS); do not print them with `openssl rand`. Keep the restic encryption password offline
   too: without it the backups cannot be read. Never paste either password into chat or a note.

   ```bash
   # Runs on: home Linux PC
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
   Type the rest-server password from your password manager.

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
   `/opt/anima/headscale` read-only). On AlmaLinux, `restic` 0.19 comes from EPEL and `sqlite3`
   3.46 from AppStream, both in `/usr/bin`. Under SELinux the unit runs as
   `unconfined_service_t` (systemd starts `/usr/local/sbin/headscale-backup`, labelled `bin_t`,
   in that domain), which may read Headscale's `container_file_t` files; the systemd sandbox, not
   SELinux, confines it:

   ```bash
   # Runs on: VPS (SSH over the VPN)
   ls -Z /usr/local/sbin/headscale-backup                 # ...:bin_t:s0
   sudo ausearch -m avc -ts today -c restic               # <no matches>
   sudo ausearch -m avc -ts today -c sqlite3              # <no matches>
   ```

   - The journal must not show `unable to open database file`. If it does while Headscale is
     stopped, start Headscale once and run the backup again. If it persists, follow the backup
     README (a `ReadWritePaths=-…/db.sqlite-shm` line for the `-shm` file only).
   - `sudo ls -ln /opt/anima/headscale/lib` must show owner `0` for every file. If not, see the
     backup README.
   - On the home PC, `sudo journalctl -u rest-server.service` must show the request coming from
     the hub's VPN address you put in `allow.conf`.

6. Check from the home PC that the snapshot arrived:

   ```bash
   # Runs on: home Linux PC
   sudo restic --repo /srv/restic/headscale-vps/headscale --password-file /etc/restic/headscale-password --no-cache --no-lock snapshots
   ```

   The prune runs every Sunday on this PC. It keeps every snapshot that is less than one year
   older than the newest (`--keep-within 1y`) and removes older ones. Check it once with
   `systemctl list-timers restic-prune.timer`. The units run with core dumps off, and
   `rest-server` has a start limit, so a crash loop stops instead of spinning.

   **If the prune refuses.** The prune script stops, and deletes nothing, when a snapshot is dated
   more than an hour in the future (that would move the one-year window) or when one UTC day has
   more than two snapshots (a growth bound). Running the backup by hand more than twice in one
   day also triggers it. Look at the list:

   ```bash
   # Runs on: home Linux PC
   sudo restic --repo /srv/restic/headscale-vps/headscale --password-file /etc/restic/headscale-password --no-cache --no-lock snapshots --host anima-vps --tag headscale
   ```

   If the extra snapshots came from your own manual runs, forget those snapshot IDs by hand on the
   home PC (not from the VPS, whose access is append-only). Run it as the `restic` user, not as
   root: a root process that is killed while it holds the repository lock leaves a root-owned lock
   file, and then the prune and the rest-server fail without anyone noticing. The password file is
   readable by root only, so a transient service as `restic` receives it as a credential. The weekly
   prune frees the space:

   ```bash
   # Runs on: home Linux PC
   sudo systemd-run --pipe --wait --uid=restic -p LoadCredential=restic-password:/etc/restic/headscale-password \
     sh -c 'restic --repo /srv/restic/headscale-vps/headscale --password-file "$CREDENTIALS_DIRECTORY/restic-password" --no-cache forget <SNAPSHOT_ID>'
   sudo find /srv/restic ! -user restic          # must print nothing
   ```

   The wording depends on the restic version: restic 0.16.4 prints `1 / 1 files deleted`, not
   "removed snapshot". A mistyped ID prints "Ignoring ... no matching ID found" and still exits 0.
   So run the `--no-lock snapshots` command above again and confirm the ID is gone. If `find`
   prints anything, repair the owner: `sudo chown -R restic:restic /srv/restic/headscale-vps`.

   The `snapshots` and `restore` commands in this guide run as root with `--no-lock`, which writes
   no lock file into the repository, so they are safe.

   If you did not make them, **suspect the VPS**: treat it as taken over, check `hsctl nodes list`,
   `hsctl preauthkeys list`, the policy and the SSH logs before you prune.

   The alert for a backup that has stopped arriving is deferred work (GitHub issue #1). Until then
   look at `systemctl list-timers headscale-backup.timer` on the VPS and at the snapshot list
   once a week.

**Restore drill, every quarter.** A backup you have not restored is not a backup. On the home PC,
restore the latest snapshot into a scratch folder and check it. This does not touch the live
server. `latest` is acceptable here, and only here: the drill proves that the newest backup can be
read and decrypted, and it puts nothing into service, so a snapshot a taken-over VPS wrote does no
harm. A real restore never uses `latest`:

```bash
# Runs on: home Linux PC
sudo restic --repo /srv/restic/headscale-vps/headscale --password-file /etc/restic/headscale-password \
  --no-cache --no-lock restore latest --host anima-vps --tag headscale --target /root/headscale-restore \
  --include '/tmp/headscale-backup.*/db.sqlite' --include '/tmp/headscale-backup.*/noise_private.key' \
  --include '/tmp/headscale-backup.*/derp_server_private.key'
sudo find /root/headscale-restore -type f                     # db.sqlite and the two private keys
sudo find /root/headscale-restore -type l                     # must print nothing
sudo sqlite3 "$(sudo find /root/headscale-restore -type f -name db.sqlite)" "PRAGMA integrity_check"      # must print ok
sudo rm -rf /root/headscale-restore                           # it holds private keys
```

`--no-cache --no-lock` keep the drill from writing into the repository, which belongs to the
`restic` user. The drill restores only the three files a real restore uses, and `-type f` never
follows a link a crafted snapshot could hold.

**Rebuild or restore the VPS (operator change 15, confirm in chat first: VPS-005, DNS-008).** Use
the restore procedure in the [backup README](../../deploy/headscale/backup/README.md#restore); in
short:

1. Choose a snapshot **by ID**, dated by its arrival on the home PC before the suspected
   compromise. The VPS sets snapshot times itself, so after an incident never use `latest`.
2. Restore it on the home PC and take only `db.sqlite` and the two private keys. Render
   `config.yaml` again with `render.sh` for the VPS's public IPv4 (mode 0600, root), and install
   `policy.hujson` from the repository, not from the snapshot.
3. After Headscale starts, delete the old `hub` node (`hsctl nodes delete`) and join again with a
   new tagged pre-auth key (step 3). Then review every node, route, pre-auth key and API key
   (step 11, Detection).
4. If the hub's VPN IPv4 changed, update `allow.conf` on the home PC and the `hub` record
   ([Hub VPN address changes](#133-hub-vpn-address-changes)). On a new VPS, update the `hs`
   record and the Hostinger panel firewall too.

## 10. Warnings

**A full-tunnel VPN on the same computer (VPN-007).** A company or other VPN in full-tunnel mode
sends all traffic into its own tunnel. It can take over the Anima ranges (`100.64.0.0/10` and the
site subnets `10.10.0.0/24`, `10.20.0.0/24`, `10.30.0.0/24`), or block the Tailscale ports
(41641/udp and 3478/udp) or 443/tcp to `hs.anima.ing` (control server and relay). The symptom in Anima is "Not on the network" and `tailscale status`
reporting a health problem or no peers. Fix it in one of two ways:

- Ask the other VPN's owner for split-tunnel exceptions for `100.64.0.0/10` and the three site
  subnets (and let UDP 41641 and 3478, and 443/tcp to `hs.anima.ing`, out), or
- Disconnect the other VPN while you use Anima.

**Address overlap.** Some internet providers and phone hotspots use `100.64.0.0/10` (carrier-grade
NAT) for their own customers. If a hotspot assigns you an address in that range, the VPN and the
hotspot can clash: try another network. Site subnets must stay different from each other and from
any LAN you visit (VPN-001).

## 11. The risk of controlling Headscale (OD-10)

Tailnet Lock is an opt-in feature of Tailscale's hosted service: devices must sign a new node
before it joins. Headscale does not have it. So whoever controls the Headscale server can add a
node or change the policy, and so reach every site. Controlling the server means root on the VPS,
and also the Hostinger account (its browser console gives root without SSH), and the Cloudflare
account (it can move `hs.anima.ing` to another server). The spec asks to verify the Tailnet Lock
point before Phase 1 (OD-10); that item is recorded as open in the WP-03 report. This risk is
accepted; the design reduces it as follows.

| Measure | Where |
| --- | --- |
| SSH keys only; no root login; SSH only over the VPN; firewall allows only 443/tcp, 3478/udp, 41641/udp | Step 4 |
| Automatic security updates | Step 4 |
| Admin only on the VPN: the CLI over a unix socket; remote gRPC and metrics on loopback | Step 2 |
| No API keys (checked, and checked again after upgrades); no OIDC | Step 2 |
| Pre-auth keys are single use and last one hour; they are kept out of command lines and logs | Steps 3, 5 |
| Routes need your manual approval; no `autoApprovers` | Step 5 |
| Tag owners are empty: a node cannot claim a tag | Step 8 |
| Tags and expiry checked at every client registration and renewal | Steps 6, 7 |
| Append-only backups off the VPS | Step 9 |
| Two-factor authentication on the Hostinger and Cloudflare accounts (you turn it on) | Step 1 |

**Detection.** Review `hsctl nodes list`, `hsctl preauthkeys list` and `hsctl apikeys list`
regularly against what you expect: the nodes `hub`, `home-router`, `office2-router` and two Macs, no
unused pre-auth keys, no API keys. Any other node, any extra tag, any route you did not approve, or
any expiry you did not set is a reason to act. Also look at the policy file on the
server and compare it with the repository. Automatic alerting for this is deferred work, tracked
on [GitHub issue #1](https://github.com/TheArctesian/anima/issues/1) (R-007).

**The broker (Tier B).** The VPN build adds a small program, the broker, on the VPS. It is the only path from the hub to Headscale: the hub has no Headscale API key and no access to the Headscale socket. The broker runs one request for each connection on a unix socket. It has six verbs: `nodes`, `personal-key`, `router-key`, `approve-route`, `register` and `renew`. It refuses a tag that is not in the policy, strips every key from what it returns, limits the number of keys for each day and the number of nodes, and writes one log line for each request. It does not change the policy. The broker mount works only while SELinux is disabled (R-054); turning SELinux on needs a policy module first.

**Residual risk.** If an attacker gets root on the VPS (by SSH, by the Hostinger console, or by a
takeover of the Hostinger account), they can add a node and widen the policy, and reach every site,
until you notice. The hub and its credential vault run on the same VPS (OD-9): once WP-04 installs
the hub, root on the VPS also exposes the stored device credentials. The measures make a takeover
harder and make you more likely to notice; they do not prevent it. Devices keep their own
authentication, but that does not help if their credentials are read from the hub.

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
export ANIMA_ADMIN_USER=<ADMIN_USER>
cd /opt/anima/headscale/vps && sudo --preserve-env=ANIMA_ADMIN_USER ./harden.sh check     # ok: no violations
sudo ss -tulpn          # public listeners: headscale on <VPS_PUBLIC_IPV4>:443 and :3478, tailscaled on 41641 only
hsctl apikeys list      # no keys
hsctl nodes list        # exactly the nodes you expect; clients with no tags and an expiry within 180 days
```

On the home PC, the receiver must accept the hub's current VPN address:

```bash
# Runs on: home Linux PC
grep IPAddressAllow /etc/systemd/system/rest-server.service.d/allow.conf    # must equal tailscale ip -4 on the VPS
```

Other listeners must be on `127.0.0.1`, `::1`, or the VPN address. `sshd` listens on `0.0.0.0:22`
and `[::]:22`; the `nftables` rules, not `sshd`, restrict it to the VPN interface. Also expect
Headscale's `127.0.0.1:9090` and
`127.0.0.1:50443` and the local DNS resolver.

If `dig +short hub.anima.ing` is empty at a site while `@1.1.1.1` answers, the site resolver (for
example the Pi-hole) drops answers with private addresses. See the end of
[`deploy/dns/README.md`](../../deploy/dns/README.md).

## 13. Maintenance

### 13.1 Upgrade the Headscale image

**Operator change 16 (confirm in chat first: VPS-005).** Headscale is pinned by tag and digest
in `compose.yaml`; an upgrade is a reviewed change to that line.

1. On your Mac, read the release notes for config or CLI changes, update the `image:` line (tag
   and digest) and run `bun test deploy/headscale`.
2. On the VPS, take a backup first (`sudo systemctl start headscale-backup.service`, then check
   the journal), and note its snapshot ID on the home PC: a database migration cannot be undone
   except by a restore.
3. Pull the new image and check that its digest equals the one in the new `compose.yaml`
   (step 2.6). Install the new file and start it:

   ```bash
   # Runs on: VPS (SSH over the VPN)
   sudo docker pull docker.io/headscale/headscale:<NEW_VERSION>
   sudo docker image inspect --format '{{index .RepoDigests 0}}' docker.io/headscale/headscale:<NEW_VERSION>
   sudo cp -p /opt/anima/headscale/compose.yaml /opt/anima/headscale/compose.yaml.old
   sudo install -m 0644 ~/anima-headscale/compose.yaml /opt/anima/headscale/compose.yaml
   sudo docker compose -f /opt/anima/headscale/compose.yaml up -d
   sudo docker compose -f /opt/anima/headscale/compose.yaml ps        # healthy
   ```

4. Check again what an upgrade could change: `hsctl apikeys list` is empty,
   `hsctl policy check --file /etc/headscale/policy.hujson` exits 0, and `hsctl nodes list`
   shows the same nodes, tags and expiries as before.

Undo: put `compose.yaml.old` back and `up -d`. If the new version migrated the database, restore
the snapshot from step 2 by ID (the backup README, Restore).

### 13.2 Revoke a lost or stolen client

**Operator change 17 (confirm in chat first: VPS-005).** Do it from the other Mac, over the VPN,
as soon as you know.

```bash
# Runs on: VPS (SSH over the VPN, from the other Mac)
hsctl nodes list                                   # find the lost client's ID
hsctl nodes expire --identifier <NODE_ID>          # cuts it off now
hsctl nodes delete --identifier <NODE_ID>
hsctl nodes list-routes                            # only the routes you approved remain
hsctl preauthkeys list                             # expire any key you did not use
```

If it was a Mac with your SSH key, remove that key from `~<ADMIN_USER>/.ssh/authorized_keys` on
the VPS (and on the home and office PCs), and sign out of the Hostinger and Cloudflare accounts
on it. If both Macs are lost, use the Hostinger browser console (step 4.8).

### 13.3 Hub VPN address changes

**Operator change 18 (confirm in chat first: VPS-005, DNS-008).** The hub's VPN IPv4 changes only
when the `hub` node is registered again (for example after a restore, step 9). After
`tailscale ip -4` on the VPS shows the new `<HUB_VPN_IPV4>`:

1. On the home PC, write the new address into the receiver's allow list and restart it:

   ```bash
   # Runs on: home Linux PC, in a copy of deploy/headscale/backup
   sed 's/\${HUB_VPN_IPV4}/<HUB_VPN_IPV4>/' rest-server-allow.conf | sudo tee /etc/systemd/system/rest-server.service.d/allow.conf > /dev/null
   sudo systemctl daemon-reload && sudo systemctl restart rest-server.service
   ```

2. Change the `hub` record: with a new short-lived operator token, delete the old `hub` record
   and apply the `hub` row of [`deploy/dns/README.md`](../../deploy/dns/README.md) again with
   `HUB_VPN_IPV4` set to the new address. Revoke the token. Check with
   `dig +short hub.anima.ing @1.1.1.1`.
3. Run the backup once (`sudo systemctl start headscale-backup.service`) and check that it
   arrives (step 9.6). Tell WP-04: the hub listens on this address (`ANIMA_LISTEN`).
