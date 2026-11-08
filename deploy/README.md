# Run the Anima hub

This folder holds the Docker Compose deployment of the hub (WP-04 stage A). It starts
PostgreSQL 17, the migration job, the certificate service, `guacd` and the hub.
Backups are described in [backup/README.md](backup/README.md).

**Status.** `anima-hub migrate` does not exist yet (WP-05 adds it, handoff H-1). Today the
`migrate` service starts the hub program as a server. It never completes, so `hub` waits
for it and does not start. The hub reads one PEM file for both its certificate and its key,
and reloads it when the file changes; WP-05 adds this too.

## What runs

| Service | Image | Purpose | Networks |
| --- | --- | --- | --- |
| `postgres` | `postgres:17.11-bookworm` | The database. No published port. | `database` |
| `migrate` | `anima-hub:local` | Runs `anima-hub migrate` as `anima_owner`, then exits. | `database` |
| `certs` | `goacme/lego:v4.25.2` | Gets and renews the certificate for `hub.anima.ing`. | `acme` |
| `guacd` | built from `gateway/` | The Guacamole daemon with the OpenDyslexic font. No published port. | `gateway` |
| `localapi` | built from `localapi/` | Two Tailscale LocalAPI calls for the hub. No published port. | `localapi` |
| `hub` | `anima-hub:local` | The hub and the web client. The only published port: `ANIMA_HUB_ADDRESS:443`. | `database`, `gateway`, `localapi`, `notify` |
| `apprise` | `caronc/apprise` (pinned, with digest) | Optional. Sends notifications to cloud services for the hub. Starts only with `--profile apprise`. No published port. | `notify` |

The `notify` network holds the hub (fixed address `172.30.3.2`) and Apprise (fixed address
`172.30.3.10`). It has a route out, because Apprise sends to the internet. The hub joins it
whether or not Apprise runs, so you can turn Apprise on later without a hub restart.

The `database` network has no route out. The hub joins `gateway` at a fixed address
(`ANIMA_HUB_IP`, default `172.30.0.10`) and listens on port 8443 there.

All containers have a read-only root file system, no Linux capability and
`no-new-privileges`. Two exceptions: `postgres` runs as user 999 (no capability needed), and
`certs` runs as root with the one capability `CHOWN`, because it gives the certificate
file to the hub user (10001).
Each service writes its log as JSON files, 10 MB each, 10 files.

## Prerequisites

- A Linux host with Docker Engine 28.0 or later and the Compose plugin (on AlmaLinux 10:
  the dnf packages `docker-ce` and `docker-compose-plugin`). Before version 28, a machine on
  the same routed network could reach the unpublished `guacd` port.
- `openssl` on the host (`dnf install -y openssl`; the minimal image lacks it).
- On a host with SELinux enforcing: `semanage` (package `policycoreutils-python-utils` on
  EL10). The install folder in this document is `/opt/anima`, so this folder is
  `/opt/anima/deploy`.
- The host firewall from WP-03 (`deploy/headscale/vps/nftables.conf`).
- The host has the VPN address that the hub will use (`ANIMA_HUB_ADDRESS`).
- A Cloudflare API token for the zone `anima.ing` (Operator changes, item 1).

## First start

1. Make the secret files. Run this as root, in this folder:

   ```sh
   read -rs CF_TOKEN
   printf '%s\n' "$CF_TOKEN" | sh bin/make-secrets --cloudflare-token-stdin
   unset CF_TOKEN
   ```

   The script makes each missing file in `secrets/` with a random value and the right owner
   and mode. It never overwrites a file and never prints a secret. It checks the characters
   of the Cloudflare token, which it reads from one line of standard input. In a terminal
   it asks for the token with no echo instead. It refuses a secrets folder that is
   writable by others, and it checks the files that exist. Run it again at any time: it
   only adds what is missing.

2. Set the environment in `deploy/.env` (mode 0600):

   | Variable | Required | Default | Meaning |
   | --- | --- | --- | --- |
   | `ANIMA_HUB_ADDRESS` | yes | none | The VPN address (100.64.0.0/10) that gets the published port 443. |
   | `ANIMA_ACME_EMAIL` | yes | none | Contact address for Let's Encrypt. |
   | `ANIMA_HUB_SUBNET` | no | `172.30.0.0/24` | Subnet of the `gateway` network. |
   | `ANIMA_DATABASE_SUBNET` | no | `172.30.1.0/24` | Subnet of the `database` network. |
   | `ANIMA_VPS_PUBLIC_ADDRESS` | yes | none | The VPS public IPv4 address (31.220.53.226 on this VPS). With both subnets it forms `ANIMA_PROBE_DENY`, so checks never probe the hub's own networks. |
   | `ANIMA_HUB_IP` | no | `172.30.0.10` | Hub address inside `ANIMA_HUB_SUBNET`. |
   | `ANIMA_PERMITTED_NETWORKS` | no | `100.64.0.0/10` | Client networks that may use the hub (NET-001): the VPN range and the site subnets, comma-separated. |
   | `ANIMA_LOG` | no | `info` | Hub log filter. |
   | `ANIMA_PUBLIC_ORIGIN` | no | `https://hub.anima.ing` | The origin clients use. The hub checks the browser cookie's `Origin` against it, and the web client's security headers use it. |
   | `ANIMA_RECORDING_MAX_BYTES` | no | `2147483648` | The most plaintext bytes one recording may hold (2 GiB). |
   | `APPRISE_ALLOW_SERVICES` | no | `discord,tgram,slack` | The services Apprise may send to. See "Apprise". |
   | `ANIMA_ACME_SERVER` | no | Let's Encrypt production | Use the staging URL to test. |

   The compose file sets two more hub variables as fixed values: `ANIMA_WEB_ROOT`
   (`/usr/share/anima/web`, the web client; empty turns it off) and `ANIMA_RECORDING_ROOT`
   (`/var/lib/anima/recordings`; empty turns recording off). The four variables
   `ANIMA_PUBLIC_ORIGIN`, `ANIMA_WEB_ROOT`, `ANIMA_RECORDING_ROOT` and
   `ANIMA_RECORDING_MAX_BYTES` are defined in K7 of
   [phase-3-contracts.md](../docs/programme/phase-3-contracts.md). Until the web client and
   recording code (WP-43, WP-48) is in the image, they do nothing.

3. Check the host:

   ```sh
   set -a; . ./.env; set +a
   sh bin/preflight
   ```

   It refuses an address outside 100.64.0.0/10 (so never `0.0.0.0`), a Docker Engine older
   than 28, and a missing secret file.

4. Start:

   ```sh
   docker compose up -d
   ```

**Let's Encrypt subscriber agreement.** Starting the `certs` service accepts the Let's
Encrypt subscriber agreement for you (`--accept-tos`). Read it first.

## Order of start

1. `postgres`, `certs` and `guacd` start at the same time. On an empty volume, `postgres`
   runs `postgres/init/10-anima-roles.sh` once. It is healthy when it accepts TCP
   connections.
2. `migrate` runs after `postgres` is healthy. It runs the migrations and exits with code 0.
3. `hub` starts when `postgres` is healthy, `migrate` has exited with code 0, `certs` is
   healthy and `guacd` has started.

`certs` is healthy when `/certs/hub/tls.pem` exists. The first order needs Cloudflare and
Let's Encrypt and can take a few minutes. If it fails, the service waits one hour before it
tries again, to stay below the Let's Encrypt limits. `docker compose up -d` waits about
15 minutes for `certs`. If the first order failed, fix the cause (see
`docker compose logs certs`) and run `docker compose restart certs` to order again at once.
Then run `docker compose up -d` again to start the services that waited.

## Certificate renewal

`certs` checks the certificate every 12 hours and renews it when fewer than 30 days remain.
It writes the key and the chain to one file, `/certs/hub/tls.pem` (owner 10001, mode 0400),
in one rename. The hub picks up a renewed file by itself once WP-05 has landed. No
restart is needed.

## Logs

```sh
docker compose logs -f hub
docker compose logs --tail 100 certs
docker compose ps
```

No log contains a secret (LOG-007).

## Rotate a password

These steps rotate the hub database password. Run them in `bash`, as root, in this folder.
The password never appears in a program argument: `printf` is a shell built-in, and the
SQL goes to `psql` on standard input. Keep the old files until the last step.

1. Make the new value, with files private from now on:

   ```sh
   umask 077
   NEW=$(openssl rand -hex 32)
   ```

2. Change the role. The first statement keeps the password out of the server log if the
   second one fails:

   ```sh
   printf "SET log_min_error_statement = panic;\nALTER ROLE anima_hub PASSWORD '%s';\n" "$NEW" |
       docker compose exec -T postgres psql -X -v ON_ERROR_STOP=1 -U postgres
   ```

3. Write the new files next to the old ones:

   ```sh
   printf '%s' "$NEW" >secrets/hub_password.new
   printf 'postgres://anima_hub:%s@postgres:5432/anima' "$NEW" >secrets/hub_database_url.new
   chown root:999 secrets/hub_password.new
   chown root:10001 secrets/hub_database_url.new
   chmod 0440 secrets/hub_password.new secrets/hub_database_url.new
   unset NEW
   ```

4. Move the old files aside and restart the hub on the new ones:

   ```sh
   mv secrets/hub_password secrets/hub_password.old
   mv secrets/hub_database_url secrets/hub_database_url.old
   mv secrets/hub_password.new secrets/hub_password
   mv secrets/hub_database_url.new secrets/hub_database_url
   restorecon -R secrets
   docker compose up -d --force-recreate hub
   docker compose logs --tail 20 hub
   ```

5. When the hub runs, delete `secrets/hub_password.old` and `secrets/hub_database_url.old`.

To rotate the owner password, use the role `anima_owner` and the files `owner_password` and
`owner_database_url` (same owners and modes), then run `docker compose run --rm migrate`.

## Gateway font

The gateway image holds the font family `OpenDyslexicM Nerd Font Mono`. Set guacd's
`font-name` to exactly this name. `fc-match OpenDyslexic` returns DejaVu Sans, not this font.
`test/smoke.sh` checks the name.

## Web client

The hub serves the web client at its own origin. See [../docs/setup/web-client.md](../docs/setup/web-client.md).
To read metrics with Prometheus, see [../docs/setup/prometheus.md](../docs/setup/prometheus.md).

## Recordings volume

Session recordings (D-14) go to the named volume `recordings`, mounted read-write in the hub only,
at `/var/lib/anima/recordings`.

- The hub image makes that folder with owner `10001:10001` and mode `0700`. A new volume copies
  both. The root file system of the hub stays read-only.
- Retention deletes a recording after 30 days by default (1 to 365, set in Settings).
- One recording holds at most `ANIMA_RECORDING_MAX_BYTES` plaintext bytes (2 GiB by default).
- Below 1 GiB of free space, a recording stops. The session goes on.
- The volume is not in the backups (D-15). See [backup/README.md](backup/README.md).
- Check the free disk space on the host before the first recording:

  ```sh
  # Run on the VPS.
  df -h /var/lib/docker
  ```

## Apprise (optional)

Apprise sends a notification to a cloud service for the hub (D-19). It is a Compose profile, so a
plain `docker compose up -d` does not start it.

```sh
# Run on the VPS, in /opt/anima/deploy.
sudo docker compose --profile apprise up -d apprise
```

Do this only after step 7 of "Phase 3 operator changes (WP-52)" has installed the firewall rules
through `harden.sh confirm`. Apprise has `restart: unless-stopped`, so once started it starts again
with Docker at every boot; the rules must already be in the boot copy of the firewall by then.

- It is stateless: it keeps no URL, no key and no result, and has no volume. Every request carries its
  own URLs. It holds no secret and publishes no port.
- The hub reaches it on the `notify` network. Enter `http://172.30.3.10:8000` as the Apprise address
  of the channel in Settings. The port is the one the image listens on.
- **Run it only with the host firewall rule.** Apprise can still fetch any URL through a service URL
  parameter such as `template`, so its onward traffic is not contained by the allow list. The rule in
  step 7 of "Phase 3 operator changes (WP-52)" contains it. Do not start Apprise without that rule.
- **Allow list.** `APPRISE_ALLOW_SERVICES` sets the only services it will send to. The default is
  `discord,tgram,slack` (Discord, Telegram, Slack). To change it, set the variable in `deploy/.env`
  and run `sudo docker compose --profile apprise up -d apprise`. Add only cloud services. Never add
  `json`, `xml`, `form`, `mailto`, `ntfy` or `apprise`: they take any host, so a channel could then
  reach networks that the hub's deny list protects.
- **Attachments.** `APPRISE_ATTACH_REJECT_URL` is `*`, so Apprise never fetches an `attach` URL. This
  does not stop a `template` parameter in a service URL.
- **Outside the hub's deny list.** Apprise's own onward traffic is not covered by `ANIMA_PROBE_DENY`.
  The allow list and the attachment reject do not contain it. The host firewall rule (step 7) does.

## Deny list

`ANIMA_PROBE_DENY` lists the addresses that checks and notification targets must never reach
(NET-001, R-030). It holds the hub's own networks, the VPS public address and the hub's VPN
address (`ANIMA_HUB_ADDRESS`, as `/32`). It also holds the `notify` network, except the one address
of Apprise.

The deny list cannot contain `172.30.3.0/24` as a whole: a notification target is checked against
the same list, so the hub would refuse its own Apprise (E-1). Leaving the subnet out would let a
check reach the host through the bridge gateway `172.30.3.1`, and the hub itself. So the list holds the
eight blocks that cover the subnet without `172.30.3.10`. The subnet and the address are fixed values
in `compose.yaml`; do not make them variables. The list does not cover Apprise's own onward traffic;
the host firewall rule does (step 7). A test proves that the blocks and the one address
together are exactly the subnet.

## Egress check

Some providers block outbound mail ports. This check shows what the hub can reach (NOT-002). Start
the hub first, then run it on the VPS:

```sh
# Run on the VPS, in /opt/anima/deploy, with the hub running.
sudo sh bin/egress-check [HOST:PORT ...]
```

- With no argument it tries `ntfy.sh:443`, `smtp.gmail.com:465` and `smtp.gmail.com:587`. Pass
  the SMTP host of your own mailbox to test that instead, for example
  `sudo sh bin/egress-check smtp.example.com:587`.
- It runs from inside the hub container and prints `open HOST:PORT` or `blocked HOST:PORT (reason)`
  for each target. `EGRESS_TIMEOUT` sets the seconds to wait (default 5).
- Exit code `0`: all targets open. `1`: some are blocked. `2`: bad arguments, or the hub container
  cannot be reached (is it up, and are you in `/opt/anima/deploy`?).
- It changes nothing.
- A TCP connect does not prove that SMTP works: a server can accept the connection and still refuse
  the login. After the check, send a test notification from the channel in Settings.

## Security notes

- The `postgres` image keeps `POSTGRES_PASSWORD` in the environment of process 1. Only user
  999 and users who can run `docker exec` can read it.
- Other containers can reach the hub's published port at its container address, so the hub
  must not trust a source address in `172.16.0.0/12`. Only the VPN range is trusted.

## Operator changes

Do each item once, on the real host (AlmaLinux 10, SELinux enforcing, as the sudo admin
user) or in Cloudflare. Nothing in this repository does them.

1. In Cloudflare, create an API token with the permission "Zone / DNS / Edit" on the zone
   `anima.ing` only.
2. Install the packages:
   `sudo dnf install -y docker-ce docker-compose-plugin openssl policycoreutils-python-utils`.
   Then run `sudo systemctl enable --now docker`.
3. Copy this `deploy` folder to `/opt/anima/deploy`.
4. Label the secrets folder, because file secrets cannot carry a label of their own:

   ```sh
   sudo semanage fcontext -a -t container_file_t '/opt/anima/deploy/secrets(/.*)?'
   sudo mkdir -p /opt/anima/deploy/secrets
   sudo restorecon -Rv /opt/anima/deploy/secrets
   ```

   The bind mounts in `compose.yaml` (`postgres/`, `certs/`) relabel themselves with the
   `z` option. Rerun `restorecon -Rv /opt/anima/deploy/secrets` after you add or replace a
   secret file by hand.
5. Run `getent group 999 10001`. On AlmaLinux 10, gid 999 is `systemd-oom` and has no
   members, and gid 10001 does not exist. If a user is listed in either group, remove that
   user from the group, or give the group another number. If gid 999 is `docker` on your
   host, its members are already root-equivalent; remove every other member.
6. Make the secrets (First start, step 1), with the Cloudflare token from item 1.
7. Write `deploy/.env` (mode 0600) with `ANIMA_HUB_ADDRESS`, `ANIMA_ACME_EMAIL` and `ANIMA_VPS_PUBLIC_ADDRESS`.
8. Make `tailscaled` start before Docker: run `sudo systemctl edit docker.service` and add
   `[Unit]` and `After=tailscaled.service`. Then run `sudo systemctl daemon-reload`.
9. Check that WP-03's firewall is loaded: run `sudo nft list table inet anima_filter`. The
   chain `prerouting` must hold the rule
   `iifname != { "lo", "tailscale0" } fib daddr type local ip daddr 100.64.0.0/10 drop`.
   That rule drops hub-port traffic for the VPN address that does not arrive on
   `tailscale0`. If the table or the rule is missing, stop and apply WP-03's firewall first.
   WP-03's `daemon.json` turns Docker's userland proxy off (`"userland-proxy": false`), so a
   VPN client reaches the hub by DNAT with its own 100.64 address; through the proxy the hub
   would see a 172.16.0.0/12 peer and refuse it (NET-002). Check with
   `sudo jq '."userland-proxy"' /etc/docker/daemon.json`, which must print `false`.
   That file also makes Docker publish on `127.0.0.1` by default, so `compose.yaml`
   names `ANIMA_HUB_ADDRESS` in the hub's `ports` entry. Keep it. The compose file sets no
   bridge name on any network, because the forward chain relies on the default `br-*` names.
10. Run the first start (First start, steps 3 and 4) and check `docker compose ps`.
11. Copy `secrets/anima_key` to an offline store. Without it, the stored credentials cannot
    be opened. The database backup does not contain it.

## Phase 3 operator changes (WP-52)

Do these once, in order, on the VPS in `/opt/anima/deploy` (AlmaLinux 10, as the sudo admin user).
Each change to the VPS needs the operator's confirmation in chat first (VPS-005). Nothing in this
repository does them. Log each one in `docs/programme/operator-changes.md`.

Each feature works once its package is in the image. Browser sign-in needs WP-43 and WP-44. The
Apprise channel and the test notification need WP-45 and WP-50. The retention settings need WP-46 (hub)
and WP-50 (client). Recordings need WP-47 (the device flag), WP-48 (recording), WP-49 (playback) and
WP-50 (the toggle). Prometheus needs API keys (WP-43, WP-50) and `/metrics` (WP-46). Before that, the new
variables are harmless: the hub ignores them.

**1. Check the free disk space (D-14).**

```sh
# Run on the VPS.
df -h /var/lib/docker
```

Check: the free space is well above 1 GiB (a recording stops below 1 GiB) and above the recordings you
expect to keep (30 days by default).
Undo: nothing changes.

**2. Keep the current image and compose file.**

```sh
# Run on the VPS, in /opt/anima/deploy.
sudo docker tag anima-hub:local anima-hub:previous
sudo cp -p compose.yaml compose.yaml.before-wp52
```

Then take a database backup. It is the way back from the Phase 3 migrations (see step 5, Undo). First
check that the backup unit is installed:

```sh
# Run on the VPS.
systemctl cat anima-backup.service
```

If it prints the unit, run it and wait for it to finish:

```sh
# Run on the VPS.
sudo systemctl start anima-backup.service
sudo journalctl -u anima-backup.service -n 50
```

The journal shows the dump path (`/var/backups/anima/anima-<UTC time>.dump`). The unit runs
`backup.sh` and then `offsite.sh`. If `offsite.sh` fails, the unit is marked failed although the
dump was written, so check the dump file itself. If `systemctl cat` says the unit does not exist, run
the backup script directly; it prints the dump path:

```sh
# Run on the VPS, in /opt/anima/deploy.
sudo /opt/anima/deploy/backup/backup.sh
```

Write down the dump path for step 5.

Check: `sudo docker image ls anima-hub` lists `previous` and `local` with the same image ID,
`ls compose.yaml.before-wp52` shows the file, and `sudo ls -l <dump path>` shows the new dump.
Undo: `sudo docker image rm anima-hub:previous` and `sudo rm compose.yaml.before-wp52`. This step
overwrites the earlier `previous` tag (the WP-18 image of 2026-11-06), and that image cannot be
restored from here.

**3. Build the hub image on the Mac for amd64, and load it on the VPS.** Use the commit that you
want to deploy (`<sha>` is its short hash).

```sh
# Run on the Mac, in the repository root.
docker buildx build --platform linux/amd64 -f deploy/hub/Dockerfile -t anima-hub:<sha> --load .
docker save anima-hub:<sha> | gzip > /tmp/anima-hub-<sha>.tar.gz
scp /tmp/anima-hub-<sha>.tar.gz anima-admin@100.64.0.1:/tmp/
```

```sh
# Run on the VPS.
gunzip -c /tmp/anima-hub-<sha>.tar.gz | sudo docker load
sudo docker tag anima-hub:<sha> anima-hub:local
rm /tmp/anima-hub-<sha>.tar.gz
```

```sh
# Run on the Mac.
rm /tmp/anima-hub-<sha>.tar.gz
```

Check: `sudo docker image ls anima-hub` shows `local` with the image ID of `<sha>`.
Undo: `sudo docker tag anima-hub:previous anima-hub:local`. The hub still runs the old image until
step 5.

**4. Install the new compose file and the egress check.**

```sh
# Run on the Mac, in the repository root.
scp deploy/compose.yaml deploy/bin/egress-check anima-admin@100.64.0.1:/tmp/
```

```sh
# Run on the VPS, in /opt/anima/deploy.
sudo install -m 0644 -o root -g root /tmp/compose.yaml compose.yaml
sudo install -m 0755 -o root -g root /tmp/egress-check bin/egress-check
sudo docker compose config --quiet
rm /tmp/compose.yaml /tmp/egress-check
```

Check: `docker compose config --quiet` prints nothing and exits 0. It needs `ANIMA_HUB_ADDRESS`,
`ANIMA_ACME_EMAIL` and `ANIMA_VPS_PUBLIC_ADDRESS` in `deploy/.env`, which are already there.
Undo: `sudo cp -p compose.yaml.before-wp52 compose.yaml` and `sudo rm bin/egress-check`.

**5. Start the new hub.** First check that `172.30.3.0/24` is free on the VPS, because the new `notify`
network uses it:

```sh
# Run on the VPS.
sudo docker network inspect $(sudo docker network ls -q) -f '{{.Name}} {{range .IPAM.Config}}{{.Subnet}}{{end}}'
```

No line may show `172.30.3.0/24`. If one does, stop and ask the operator. Then run
`docker compose up -d hub`. It runs `migrate` first, and creates the `recordings` volume and the
`notify` network.

```sh
# Run on the VPS, in /opt/anima/deploy.
sudo docker compose up -d hub
sudo docker compose ps
```

Check, in this order:

```sh
# Run on a VPN machine (an operator@ machine).
curl -sS -o /dev/null -w '%{http_code}\n' https://hub.anima.ing/health
```

The answer is `200`.

```sh
# Run on the VPS, in /opt/anima/deploy.
sudo docker compose exec -T hub stat -c '%u %a' /var/lib/anima/recordings
```

The answer is `10001 700`.

```sh
# Run on the VPS, in /opt/anima/deploy.
sudo docker inspect -f '{{range $name, $net := .NetworkSettings.Networks}}{{$name}} {{$net.IPAddress}}{{"\n"}}{{end}}' "$(sudo docker compose ps -q hub)"
```

The hub is on four networks: `deploy_database`, `deploy_gateway` (`172.30.0.10`), `deploy_localapi` and
`deploy_notify` (`172.30.3.2`). An earlier redeploy came back with only one network (operator-changes.md,
2026-11-06 05:19Z). If a network is missing, run `sudo docker compose up -d --force-recreate hub`.

```sh
# Run on the VPS, in /opt/anima/deploy.
sudo docker compose exec -T hub printenv ANIMA_PUBLIC_ORIGIN ANIMA_WEB_ROOT ANIMA_RECORDING_ROOT ANIMA_RECORDING_MAX_BYTES
```

The four lines are `https://hub.anima.ing`, `/usr/share/anima/web`, `/var/lib/anima/recordings` and
`2147483648`.

Undo: restore the database from the dump of step 2, then start the old hub. Everything the hub wrote
after the backup of step 2 is lost (devices, checks, audit entries, sessions). The order follows
[backup/README.md](backup/README.md), Restore. Remove Apprise first if you started it (step 8):
`sudo docker compose --profile apprise rm -sf apprise`.

```sh
# Run on the VPS, in /opt/anima/deploy.
sudo docker compose stop hub
sudo docker tag anima-hub:previous anima-hub:local
sudo cp -p compose.yaml.before-wp52 compose.yaml
sudo backup/restore.sh --yes <dump path from step 2>
sudo docker compose up -d migrate
sudo docker compose ps -a migrate
```

`restore.sh` refuses while `hub` or `migrate` runs. It restores into a new database and swaps it in
only when that worked, so a bad dump leaves `anima` unchanged. `migrate` now runs from the old image
against the restored database; wait until `ps -a` shows it exited with code 0. Then:

```sh
# Run on the VPS, in /opt/anima/deploy.
sudo docker compose up -d hub
sudo docker network rm deploy_notify
```

Check: `https://hub.anima.ing/health` answers `200` from a VPN machine, and the client shows the
devices and the audit log. The old database stays as `anima_previous_<time>`; drop it with the command
that `restore.sh` printed once you are sure.

Do not undo with only the old image and `docker compose up -d --no-deps hub`. The old `migrate` fails by
design on a database that holds migrations it does not know (sqlx `VersionMissing`), and the old hub does
not work on the new schema: K2 in [phase-3-contracts.md](../docs/programme/phase-3-contracts.md) moves
`sessions.host_keys` and `sessions.server_certificates` to the new schema `trust` (migration 0920),
and the old hub reads them in `sessions`.

The `recordings` volume stays. `sudo docker volume rm deploy_recordings` removes it and **destroys all
recordings**. Run it only if the operator wants a full revert.

**6. Run the egress check.**

```sh
# Run on the VPS, in /opt/anima/deploy, with the hub running.
sudo sh bin/egress-check
sudo EGRESS_TIMEOUT=10 sh bin/egress-check smtp.example.com:587
```

Replace `smtp.example.com:587` with the SMTP host and port of the operator's mailbox. Check: every
line starts with `open` and the exit code is 0. A `blocked` line names the reason; if a mail port
is blocked, use another port or a provider that allows it. Finish with a test notification from the
channel in Settings, because an open port does not prove that SMTP works.
Undo: nothing changes.

**7. Required before Apprise: install the firewall rules that contain Apprise.** Apprise can fetch
any URL through a service URL parameter such as `template`, and the allow list and attachment reject
do not stop that. The Anima host firewall contains it (NOT-002, D-19). The rules are in
`deploy/headscale/vps/nftables.conf`:

- `input` drops every packet from `172.30.3.10`, before any accept, so Apprise reaches no address of
  the VPS: not the bridge gateway `172.30.3.1`, not Headscale on the public address, not the hub's
  VPN address.
- `forward` drops Apprise's packets to `0.0.0.0/8`, `10.0.0.0/8`, `100.64.0.0/10`, `127.0.0.0/8`,
  `169.254.0.0/16`, `172.16.0.0/12`, `192.168.0.0/16` `224.0.0.0/4` and `240.0.0.0/4`, and everything
  it sends out of `tailscale0`. On the `notify` network it may only answer the hub. Public addresses
  stay open. That last claim holds only while the kernel's `br_netfilter` module is loaded, because
  without it bridged traffic skips the `forward` hook. It rests on deploy's `daemon.json`
  `"userland-proxy": false`, with which Docker loads `br_netfilter`.

They are part of the table `inet anima_filter`, so you apply them the way WP-03 applied that table:
`harden.sh firewall` with its 180 s rollback, then `harden.sh confirm` from a new SSH session over the
VPN. `confirm` installs the file as `/etc/nftables/anima.nft`, so the rules come back after a reboot;
nothing has to be repeated after a reboot. `harden.sh firewall` replaces the whole table from
`/opt/anima/headscale/vps/nftables.conf`, so the rules stay as long as the installed file has them.
A later `firewall` run with an older copy of that file removes them.
This needs the operator's confirmation (VPS-005). Do not start Apprise before this step is done.

Resolver check, first. Docker's embedded DNS forwards Apprise's lookups from the container to the
VPS's nameservers, so they pass the `forward` chain:

```sh
# Run on the VPS.
grep nameserver /etc/resolv.conf
```

If a `nameserver` is in one of the dropped ranges (for example `10.x.x.x`), Apprise cannot resolve
names. Stop and ask the programme; do not go on.

Copy the file to the folder `harden.sh` reads it from, and keep the current one:

```sh
# Run on the Mac, in the repository root, at the commit that you deploy.
scp deploy/headscale/vps/nftables.conf anima-admin@100.64.0.1:/tmp/nftables.conf
```

```sh
# Run on the VPS, in an SSH session over the VPN (ssh anima-admin@100.64.0.1).
sudo cp -p /opt/anima/headscale/vps/nftables.conf /root/nftables.conf.before-wp52
sudo install -m 0644 -o root -g root /tmp/nftables.conf /opt/anima/headscale/vps/nftables.conf
rm /tmp/nftables.conf
sudo --preserve-env=SSH_CONNECTION /opt/anima/headscale/vps/harden.sh --dry-run firewall
sudo --preserve-env=SSH_CONNECTION /opt/anima/headscale/vps/harden.sh firewall
```

The dry run checks the file with `nft -c` and changes nothing. If it or `firewall` refuses (for example
because an open public port would close), stop and ask the operator. `firewall` applies the rules and
arms the rollback. Keep this session open. Within 180 s, open a **new** SSH session over the VPN and
confirm:

```sh
# Run on the VPS, in a NEW SSH session over the VPN.
sudo --preserve-env=SSH_CONNECTION /opt/anima/headscale/vps/harden.sh confirm
```

If the new session cannot connect, do nothing: the previous rules come back after 180 s and the boot
copy is unchanged. Then put the old file back (first command of Undo) and ask the operator.

Check:

```sh
# Run on the VPS.
sudo nft list table inet anima_filter
sudo grep -n 'define apprise' /etc/nftables/anima.nft
sudo /opt/anima/headscale/vps/harden.sh check
lsmod | grep br_netfilter
sysctl net.bridge.bridge-nf-call-iptables
```

`lsmod` must print a `br_netfilter` line and `sysctl` must print `net.bridge.bridge-nf-call-iptables = 1`.
If either does not, the hub-only rule on `notify` does not apply: do not start Apprise, and ask the
programme. The first rule of chain `input` is `ip saddr 172.30.3.10 drop`, and chain `forward` has three rules
that start with `ip saddr 172.30.3.10`. The boot copy holds the three `define apprise` lines. `check`
ends with `ok: no violations`.

Then probe from Apprise's address. It works only while Apprise is not running, because the probe takes
the address `172.30.3.10`; once Apprise runs, stop it first (`sudo docker compose --profile apprise rm
-sf apprise`). The probe uses the Apprise image, which has `curl`:

```sh
# Run on the VPS, in /opt/anima/deploy.
sudo docker compose --profile apprise pull apprise
image=$(sudo docker compose --profile apprise config --images | grep apprise)
sudo docker run --rm --network deploy_notify --ip 172.30.3.10 --cap-drop ALL --user 1000:1000 \
  --entrypoint sh "$image" -c 'for url in https://172.30.3.1/ https://hs.anima.ing/health https://100.64.0.1/ http://172.30.0.10:8443/ http://172.30.3.2:8443/ http://169.254.169.254/ https://ntfy.sh/; do curl -s -o /dev/null --connect-timeout 5 "$url"; echo "$? $url"; done'
```

Each line is the curl exit code and the URL. The first six must print `28` (the connection timed
out: the firewall dropped it). The last must print `0`: Apprise resolves names and reaches the
internet. Run it with the hub up. On the first six, any other code means the rules do not hold: do not
start Apprise, and ask the programme. `7` (connection refused) or `35` or `60` (a TLS answer) means
the packet got through; `7` on `http://172.30.3.2:8443/` means the bridge is unfiltered
(`br_netfilter` is not loaded). `6` means a resolver problem, not a packet that got through: fix the
resolver check above first. After step 8, the test notification still arrives.

Undo: stop Apprise first if it runs (`sudo docker compose --profile apprise rm -sf apprise`); it must
not run without these rules. Then apply the previous file the same way:

```sh
# Run on the VPS, in an SSH session over the VPN.
sudo install -m 0644 -o root -g root /root/nftables.conf.before-wp52 /opt/anima/headscale/vps/nftables.conf
sudo --preserve-env=SSH_CONNECTION /opt/anima/headscale/vps/harden.sh firewall
```

```sh
# Run on the VPS, in a NEW SSH session over the VPN.
sudo --preserve-env=SSH_CONNECTION /opt/anima/headscale/vps/harden.sh confirm
```

If `/root/nftables.conf.before-wp52` is lost, take the file from git as it was before the Apprise
rules: `git show <commit>~1:deploy/headscale/vps/nftables.conf`, where `<commit>` is the commit that
added them (`git log -1 --format=%h -S 'define apprise' -- deploy/headscale/vps/nftables.conf`).

**8. Optional: start Apprise, only after step 7.** Run it only once `harden.sh confirm` in step 7 has
installed the rules for boot. Apprise has `restart: unless-stopped`: from now on Docker starts it at
every boot, so it must never run on a host whose firewall lacks the rules.

```sh
# Run on the VPS, in /opt/anima/deploy.
sudo docker compose --profile apprise up -d apprise
sudo docker compose --profile apprise ps apprise
sudo docker compose --profile apprise exec -T apprise curl -fsS http://127.0.0.1:8000/status
```

Check: `apprise` is `Up (healthy)` and the status call prints `OK`. Then, in Settings, make an
Apprise channel with the address `http://172.30.3.10:8000` and a `discord://`, `tgram://` or
`slack://` URL, and send a test notification.
Undo: `sudo docker compose --profile apprise rm -sf apprise`.

**No other change.** Phase 3 makes no change to Cloudflare, DNS, CORS or Headscale (plan Section 6).
`hub.anima.ing` stays an unproxied record to the VPN address. The web client is same-origin, so
`CLIENT_ORIGINS` stays as it is. `operator@` already reaches `tag:hub` on tcp:443. Only a Prometheus
scraper that is not an `operator@` machine needs a grant; see
[../docs/setup/prometheus.md](../docs/setup/prometheus.md).

## Checks

- `bun run --cwd deploy test` checks the compose file, the scripts (`bin/preflight`,
  `bin/egress-check`, `certs/install.sh`, `certs/run.sh` with a fake lego) and the database roles. It needs no Docker.
- `sh deploy/test/apprise-smoke.sh` is a manual test that needs Docker. It starts only Apprise, on a
  copy of the `notify` network with no route out, and checks the hardening, the allow list and the
  attachment reject. Nothing leaves the machine.
- `sh deploy/test/smoke.sh` is a manual test that needs Docker. It checks the Docker
  version, builds the gateway and hub images, checks the font, starts `postgres` and checks
  the roles, runs the certificate hook on a fixture certificate with the real container
  limits, and starts the hub without `migrate`. It uses a temporary copy of this folder and
  contacts neither Let's Encrypt nor Cloudflare.
