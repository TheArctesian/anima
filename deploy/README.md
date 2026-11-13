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
(`ANIMA_HUB_IP`, default `172.30.0.10`) and listens on port 8443 there. guacd joins it at a fixed
address too (`ANIMA_GUACD_IP`, default `172.30.0.20`), so it can never take the hub's address,
which the VPS firewall and sshd admit (VP-13).

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
   | `ANIMA_HUB_IP` | no | `172.30.0.10` | Hub address inside `ANIMA_HUB_SUBNET`. The VPS firewall (`hub_source`) and sshd (`anima-probe@172.30.0.10`) use the same address (VP-13); change all three together. |
   | `ANIMA_GUACD_IP` | no | `172.30.0.20` | guacd address inside `ANIMA_HUB_SUBNET`; not `ANIMA_HUB_IP`. |
   | `ANIMA_HUB_IP_RANGE` | no | `172.30.0.128/25` | The part of `ANIMA_HUB_SUBNET` Docker hands out to containers without a fixed address. `ANIMA_HUB_IP` and `ANIMA_GUACD_IP` stay outside it (VP-13, `bin/preflight networks`). |
   | `ANIMA_HUB_GATEWAY` | no | `172.30.0.1` | The bridge address of the `gateway` network, inside `ANIMA_HUB_SUBNET`. |
   | `ANIMA_ACME_SUBNET` | no | `172.30.4.0/24` | Subnet of lego's `acme` network; in `ANIMA_PROBE_DENY`. Every `ANIMA_*_SUBNET` stays inside `172.30.0.0/16`, the range the VPS firewall drops from the VPN (VP-13). |
   | `ANIMA_PERMITTED_NETWORKS` | no | `100.64.0.0/10` | Client networks that may use the hub (NET-001): the VPN range and the site subnets, comma-separated. |
   | `ANIMA_LOG` | no | `info` | Hub log filter. |
   | `ANIMA_PUBLIC_ORIGIN` | no | `https://hub.anima.ing` | The origin clients use. The hub checks the browser cookie's `Origin` against it, and the web client's security headers use it. |
   | `APPRISE_ALLOW_SERVICES` | no | `discord,tgram,slack` | The services Apprise may send to. See "Apprise". |
   | `ANIMA_ACME_SERVER` | no | Let's Encrypt production | Use the staging URL to test. |
   | `ANIMA_RELEASES_ENDPOINT` | no | empty (publishing off) | The R2 S3 API origin, `https://<account>.r2.cloudflarestorage.com`. See "Release publishing". |
   | `ANIMA_UPDATER_PUBLIC_KEY` | no | empty (publishing off) | The updater's public key, the same text as `plugins.updater.pubkey` in `tauri.conf.json`. The hub's updater key must match it. |

   The compose file sets one more hub variable as a fixed value: `ANIMA_WEB_ROOT`
   (`/usr/share/anima/web`, the web client; empty turns it off). `ANIMA_PUBLIC_ORIGIN` and
   `ANIMA_WEB_ROOT` are defined in K7 of
   [phase-3-contracts.md](../docs/programme/phase-3-contracts.md). Until the web client
   (WP-43) is in the image, they do nothing.

3. Check the host:

   ```sh
   set -a; . ./.env; set +a
   sh bin/preflight
   ```

   It refuses an address outside 100.64.0.0/10 (so never `0.0.0.0`), a Docker Engine older
   than 28, a network subnet outside 172.30.0.0/16 or a fixed hub or guacd address that Docker
   could hand out (VP-13), and a missing secret file.

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

## No recordings volume

The hub records no session content (R-113, RA-008), so it has no recordings volume. A deploy from
before R-113 created an empty `deploy_recordings` volume that nothing mounts now. With the
operator's confirmation (VPS-005), remove it on the VPS with `sudo docker volume rm deploy_recordings`.

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

## Release publishing (WP-36, R-108, R-116)

The hub publishes the desktop releases its builders upload to the R2 bucket `anima-releases`
(spec Section 3.10, BLD-060 to BLD-070). It is off until `ANIMA_RELEASES_ENDPOINT` is set; while
it is off, a builder's upload is refused and nothing is published. Setting it up is an operator
step (VPS-005): agents do not create the R2 token or change `deploy/.env` on the VPS.

- **Two secret files** hold the R2 API token: its access key ID and its secret access key. Make the
  token in Cloudflare with Object Read and Write on `anima-releases` only. Put each value in its own
  file, `secrets/r2_access_key_id` and `secrets/r2_secret_access_key`, owned `root:10001`, mode
  `0440`. `compose.yaml` gives them to the hub as Docker secrets and `bin/preflight` checks them;
  `bin/make-secrets` cannot make them, because the values come from Cloudflare. The hub reads them once at start
  (`ANIMA_RELEASES_ACCESS_KEY_ID_FILE`, `ANIMA_RELEASES_SECRET_ACCESS_KEY_FILE`) and never logs them.
- **`ANIMA_RELEASES_ENDPOINT`** is the bucket's S3 API origin,
  `https://<account>.r2.cloudflarestorage.com`. `ANIMA_RELEASES_BUCKET` (default `anima-releases`)
  and `ANIMA_RELEASES_PUBLIC_BASE` (default `https://releases.anima.ing`) rarely change.
- **The updater key pair** is two more secret files, in their own folder `secrets/updater/`: the
  hub signs each release with the key (R-116, BLD-045). A hub that holds this key can sign updates
  every installed app trusts, so keep the VPS and the backup of the key as safe as the key itself.
  Agents never do these steps and never read these files (UPD-012, VPS-005). On your computer,
  copy the key file that `tauri signer generate` wrote to the VPS yourself, for example
  `scp ~/.config/anima/updater.key <you>@<vps>:updater.key`. Then, on the VPS, in the deploy folder
  (group 10001 is the hub's, `anima-hub` on the VPS, as for the R2 files):

  ```sh
  sudo install -d -m 0750 -o root -g 10001 secrets/updater
  sudo install -m 0440 -o root -g 10001 ~/updater.key secrets/updater/updater_key
  shred -u ~/updater.key
  # Type the key's password; it is not echoed. Press Enter alone only for a key made with no
  # password: an encrypted key with an empty password leaves publishing off.
  read -rs password && (umask 077 && printf '%s' "$password" > ~/updater_key_password)
  unset password
  sudo install -m 0440 -o root -g 10001 ~/updater_key_password secrets/updater/updater_key_password
  shred -u ~/updater_key_password
  ```

  `compose.yaml` mounts the folder read-only at `/run/secrets/updater` (`ANIMA_UPDATER_KEY_FILE`,
  `ANIMA_UPDATER_KEY_PASSWORD_FILE`). It is a folder, not two compose secrets, because compose
  refuses to create a container whose secret file is absent; with the folder, a hub without the
  key still starts, with publishing off (D-44). `bin/preflight` warns, and passes, when either file
  is absent; a file that is there must have mode `0440`, and `updater_key` must not be empty. The
  hub opens the key once at start and never logs it, its password or its seed. Install the pair
  before you deploy a hub that signs (WP-36 T6): until then that hub has publishing off. Unlike
  the updater pair, the R2 token files are compose secrets: while one is absent, compose does not
  start the hub at all, and `bin/preflight` fails.
- **`ANIMA_UPDATER_PUBLIC_KEY` must be the same text as `plugins.updater.pubkey` in
  `apps/desktop/src-tauri/tauri.conf.json`,** and the key in `secrets/updater/updater_key` must be
  its private half. If the variable is empty, a file is absent or unreadable, the key does not
  open with its password, or it is another key, the hub starts with publishing off and logs one
  warning that says which; it never prints the key. The public key is not secret, so it is a plain
  variable. The hub checks every signature it makes with it before it writes a feed.
- The hub reads every file back from the bucket with the R2 token, and checks that the public base
  answers 200 for it, before it writes `latest.json`, so the `releases.anima.ing` custom domain must
  serve the bucket first. It starts that check about 17 minutes after the upload: a builder's upload
  URLs live 15 minutes, and the hub waits 2 more for clock skew (D-28).
- **The hub host must keep time with NTP.** If the hub's clock runs more than 2 minutes ahead of
  R2's, a builder's upload URL still works while the hub publishes. The hub reads the checked files'
  `ETag`s again after it writes the feed and records a warning event ("... changed in the release
  bucket after the hub checked it") when one changed; check the clock and release the version again.
- **A hub with publishing off fails the jobs already handed to it.** If you unset
  `ANIMA_RELEASES_ENDPOINT` while a job waits to be published, that job fails with "Publishing is off
  on this hub"; release that version again once publishing is back.

## Deny list

`ANIMA_PROBE_DENY` lists the addresses that checks and notification targets must never reach
(NET-001, R-030). It holds the hub's own networks, lego's `acme` network, docker0
(`172.17.0.0/16`, whose gateway is the host) and the VPS public address. It also holds the
`notify` network, except the one address of Apprise. It does not hold the hub's VPN address
(`ANIMA_HUB_ADDRESS`): the VPS is a device at that address (VP-13), and the host firewall lets only
the hub container reach it, on 22/tcp and ping (see "The VPS as a device").

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
and WP-50 (client). Prometheus needs API keys (WP-43, WP-50) and `/metrics` (WP-46). Before that, the new
variables are harmless: the hub ignores them.

**1. Check the free disk space.**

```sh
# Run on the VPS.
df -h /var/lib/docker
```

Check: the free space is well above the size of the new image and the database dump of step 2.
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
`docker compose up -d hub`. It runs `migrate` first, and creates the `notify` network.

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
sudo docker inspect -f '{{range $name, $net := .NetworkSettings.Networks}}{{$name}} {{$net.IPAddress}}{{"\n"}}{{end}}' "$(sudo docker compose ps -q hub)"
```

The hub is on four networks: `deploy_database`, `deploy_gateway` (`172.30.0.10`), `deploy_localapi` and
`deploy_notify` (`172.30.3.2`). An earlier redeploy came back with only one network (operator-changes.md,
2026-11-06 05:19Z). If a network is missing, run `sudo docker compose up -d --force-recreate hub`.

```sh
# Run on the VPS, in /opt/anima/deploy.
sudo docker compose exec -T hub printenv ANIMA_PUBLIC_ORIGIN ANIMA_WEB_ROOT
```

The two lines are `https://hub.anima.ing` and `/usr/share/anima/web`.

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
  `169.254.0.0/16`, `172.16.0.0/12`, `192.168.0.0/16`, `224.0.0.0/4` and `240.0.0.0/4`, and everything
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

## The VPS as a device (VP-13)

The hub can check the VPS and open SSH to it, at the VPS's VPN address `100.64.0.1` (operator ruling
VP-13, 2026-11-11). Two changes make this possible, and both are needed:

- `ANIMA_PROBE_DENY` no longer holds `${ANIMA_HUB_ADDRESS}/32`. The Docker networks (with docker0,
  `172.17.0.0/16`, and lego's `acme` network), and the VPS public address stay denied.
- The host firewall has one exception to its DNS-005 drops: packets from the hub container's address
  `172.30.0.10` on a Docker bridge may reach `100.64.0.1` on 22/tcp, and ICMP echo with the usual rate
  limit. Every other container, every other source and every other port of `100.64.0.1` (443,
  Headscale's 9090, the broker, Apprise) stays dropped, and the hub opens no other connection to the
  host (not to docker0's `172.17.0.1`, a bridge gateway or the public address). The values are the
  `hub_source` and `hub_vpn_address` defines in `deploy/headscale/vps/nftables.conf`.

The source address is fixed by `compose.yaml`. The hub's packets to the host leave by its default
route. `database` and `localapi` are internal and have no gateway. Of `gateway` and `notify`, Docker
chooses the highest `gw_priority` (Docker 28 or newer), then the network name. The name already
chooses `gateway`, and `gw_priority: 1` keeps it so. The hub's fixed `gateway` address is
`172.30.0.10`. If you change `ANIMA_HUB_IP` or `ANIMA_HUB_SUBNET`, change `hub_source` to match.
A mismatch makes the VPS unreachable from the hub; it opens nothing. Only the hub reaches the VPS
this way: the SSH terminal and the checks run in the hub (R-062). guacd is a separate container,
so VNC or RDP to the VPS stays closed.

Nothing else can pose as the hub:

- guacd has its own fixed address (`ANIMA_GUACD_IP`, default `172.30.0.20`), and Docker gives any
  other container on `gateway` an address from `ANIMA_HUB_IP_RANGE` (`172.30.0.128/25`) only.
  `bin/preflight networks` refuses a hub or guacd address inside that range, and guacd on the hub's.
- The exception holds only when the host's route back to `172.30.0.10` is the bridge the packet came
  in on (`fib saddr . iif oif exists`), so a container on another bridge cannot forge the address.
- The host drops every packet from `tailscale0` whose source is in `172.30.0.0/16` (`hub_networks`),
  the range that holds every Anima network (`bin/preflight networks` keeps each `ANIMA_*_SUBNET`
  in it). The hub also refuses a router key or a route approval for a site whose subnet overlaps
  that range, and leaves such a site out of the VPN picture, as it does for the tailnet range.
- Invalid packets are dropped before the exception.

sshd admits the hub's user `anima-probe` only from that address, and the admin only over the VPN:
the drop-in that `harden.sh ssh` installs says
`AllowUsers <admin>@100.64.0.0/10 anima-probe@172.30.0.10`, and `harden.sh check` reports any other
`AllowUsers` entry, including `anima-probe` from another host or the admin from anywhere. The
admin logs in only over the VPN; the Hostinger browser console is the fallback. Password login stays
off for every user (VPS-001). The address in the drop-in, `hub_source` and `ANIMA_HUB_IP` must be
the same; change all three together.

`anima-probe` is a shell on the VPS, so it is confined. `harden.sh probe` makes it with uid and gid
`10002`, its own group only, no password and no sudo, and gives it one key, the hub's, with
`restrict,pty,from="172.30.0.10"`: no forwarding of any kind, and only from the hub. The firewall's
`output` chain drops everything uid 10002 sends to the host's own addresses (loopback, the VPN, the
public and the bridge-gateway addresses), to Docker's `172.16.0.0/12` and out of `tailscale0`; it may
still reach the internet. `harden.sh check` reports an `anima-probe` with another uid, another group,
a sudoers entry, a usable password or a key without those options. Optional, not done by
`harden.sh`: `sysctl user.max_user_namespaces=0` takes unprivileged user namespaces, a common route
to kernel bugs, away from every user; check first that nothing on the VPS needs them (rootless
containers do).

Each step needs the operator's confirmation in chat (VPS-005). If you applied an earlier version of
this section, use another suffix than `.before-vp13` for the copies below, so the first ones stay.

**1. Install the scripts and the firewall.** Copy every changed file of `deploy/headscale/vps/`
first; `harden.sh check` reads the rules it expects from `nftables.conf` beside it. Then apply the
firewall as in Phase 3 step 7: `harden.sh firewall` applies the rules with a 180 s rollback.
`harden.sh confirm`, run from a new SSH session over the VPN, keeps them and installs them for boot.

```sh
# Run on the Mac, in the repository root, at the commit that you deploy.
cd deploy/headscale/vps
scp nftables.conf harden.sh firewall.sh sshd.sh probe.sh sshd-anima.conf anima-admin@100.64.0.1:/tmp/
```

```sh
# Run on the VPS, in an SSH session over the VPN (ssh anima-admin@100.64.0.1).
cd /opt/anima/headscale/vps
for f in nftables.conf harden.sh firewall.sh sshd.sh sshd-anima.conf; do sudo cp -p "$f" "/root/$f.before-vp13"; done
sudo install -m 0755 -o root -g root /tmp/harden.sh harden.sh
sudo install -m 0644 -o root -g root /tmp/nftables.conf /tmp/firewall.sh /tmp/sshd.sh /tmp/probe.sh /tmp/sshd-anima.conf .
rm /tmp/nftables.conf /tmp/harden.sh /tmp/firewall.sh /tmp/sshd.sh /tmp/probe.sh /tmp/sshd-anima.conf
sudo --preserve-env=SSH_CONNECTION ./harden.sh --dry-run firewall
sudo --preserve-env=SSH_CONNECTION ./harden.sh firewall
```

Keep this session open. Within 180 s, open a **new** SSH session over the VPN and confirm:

```sh
# Run on the VPS, in a NEW SSH session over the VPN.
sudo --preserve-env=SSH_CONNECTION /opt/anima/headscale/vps/harden.sh confirm
```

If the new session cannot connect, do nothing. The previous rules come back after 180 s. Then put
the old files back (Undo) and ask the operator.

Check: `sudo grep -n '^define \(hub\|probe\)_' /etc/nftables/anima.nft` shows four defines, and
`sudo grep -c 'ip saddr \$hub_source' /etc/nftables/anima.nft` prints `5`. In
`sudo nft list table inet anima_filter`, chain `prerouting` starts with
`iifname "tailscale0" ip saddr 172.30.0.0/16 drop`, then two rules for
`ip saddr 172.30.0.10 ip daddr 100.64.0.1` that end in `fib saddr . iif oif exists accept`. In chain
`input`, `ct state invalid drop` follows the Apprise drop, then the same two rules. Chain `output`
holds three `meta skuid 10002` drops. `sudo /opt/anima/headscale/vps/harden.sh check` prints no
`VIOLATION` line about `table inet anima_filter`; it reports the sshd values until step 3.

Undo:

```sh
# Run on the VPS, in an SSH session over the VPN.
cd /opt/anima/headscale/vps
for f in nftables.conf harden.sh firewall.sh sshd.sh sshd-anima.conf; do sudo cp -p "/root/$f.before-vp13" "$f"; done
sudo rm probe.sh
sudo --preserve-env=SSH_CONNECTION ./harden.sh firewall
```

Then run `confirm` from a new session, as above.

**2. Redeploy the hub, guacd and lego.** The new compose file changes the hub's environment, the
`gateway` attachments of the hub and guacd, and the `gateway` network (`ip_range`, `gateway`) and the
`acme` network (a fixed subnet). `docker compose up -d` sees that both networks changed, removes and
recreates them, and recreates every container on them: the hub, guacd and certs (lego). The hub's
checks pause for that time, and any open RDP, VNC or SSH session ends. Checked with Docker Compose
5.7 on a throwaway project. If your Compose stops instead with an error that a network "needs to be
recreated" or "has incompatible configuration", run `sudo docker compose down` and then
`sudo docker compose up -d`; `down` keeps the volumes. Install the compose file and the preflight as
in Phase 3 step 4, keeping the current ones:

```sh
# Run on the Mac, in the repository root.
scp deploy/compose.yaml deploy/bin/preflight anima-admin@100.64.0.1:/tmp/
```

```sh
# Run on the VPS, in /opt/anima/deploy.
sudo cp -p compose.yaml compose.yaml.before-vp13
sudo cp -p bin/preflight /root/preflight.before-vp13
sudo install -m 0644 -o root -g root /tmp/compose.yaml compose.yaml
sudo install -m 0755 -o root -g root /tmp/preflight bin/preflight
rm /tmp/compose.yaml /tmp/preflight
set -a; . ./.env; set +a
sh bin/preflight networks
sudo docker compose config --quiet
sudo docker compose up -d
```

`preflight networks` reads the subnets and addresses from `.env`, as in "Check the host", and prints
nothing when they are right.

Check, from the hub's network namespace with the host's tools (the hub image has no shell):

```sh
# Run on the VPS, in /opt/anima/deploy.
pid=$(sudo docker inspect -f '{{.State.Pid}}' "$(sudo docker compose ps -q hub)")
sudo nsenter -t "$pid" -n ip -4 route get 100.64.0.1
sudo nsenter -t "$pid" -n ping -c 1 -W 2 100.64.0.1
sudo nsenter -t "$pid" -n timeout 5 bash -c 'exec 3<>/dev/tcp/100.64.0.1/22 && head -c 7 <&3'; echo " $?"
sudo nsenter -t "$pid" -n curl -sk -o /dev/null --connect-timeout 5 https://100.64.0.1/; echo "$?"
sudo nsenter -t "$pid" -n curl -sk -o /dev/null --connect-timeout 5 https://172.17.0.1/; echo "$?"
gpid=$(sudo docker inspect -f '{{.State.Pid}}' "$(sudo docker compose ps -q guacd)")
sudo nsenter -t "$gpid" -n ping -c 1 -W 2 100.64.0.1; echo "$?"
sudo docker network inspect deploy_gateway deploy_acme -f '{{.Name}} {{json .IPAM.Config}}'
```

The route line shows `via 172.30.0.1` and `src 172.30.0.10`. The ping gets one reply. The SSH line prints `SSH-2.0 0`. Both `curl` lines print `28`: the hub reaches
no other port of the host. The guacd ping prints `1` (no reply): no other container reaches the VPS.
The last command shows `deploy_gateway` with `"IPRange":"172.30.0.128/25","Gateway":"172.30.0.1"`
and `deploy_acme` with `"Subnet":"172.30.4.0/24"`. Also check that
`sudo docker compose ps hub guacd certs` shows all three `Up`, the hub and certs `(healthy)`, and that
`sudo docker inspect -f '{{.NetworkSettings.Networks.deploy_gateway.IPAddress}}' "$(sudo docker compose ps -q guacd)"`
prints `172.30.0.20`.

Undo: `sudo cp -p compose.yaml.before-vp13 compose.yaml`, `sudo cp -p /root/preflight.before-vp13 bin/preflight`
and `sudo docker compose up -d` (which recreates the two networks again).

**3. Make the hub's SSH user and let sshd admit it.** Do not give the hub `anima-admin`: it has
sudo. Make a key that only the hub holds; `harden.sh probe` makes the user and installs the key.

```sh
# Run on the Mac. Store the private key in Anima (step 4), then delete the local copy.
ssh-keygen -t ed25519 -N '' -C anima-probe -f /tmp/anima-probe
scp /tmp/anima-probe.pub anima-admin@100.64.0.1:/tmp/
```

```sh
# Run on the VPS, in an SSH session over the VPN.
cd /opt/anima/headscale/vps
sudo ./harden.sh --dry-run --key=/tmp/anima-probe.pub probe
sudo ./harden.sh --key=/tmp/anima-probe.pub probe
rm /tmp/anima-probe.pub
sudo ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub
```

`probe` refuses a uid or gid 10002 that another account holds, and an `anima-probe` with another uid,
group or home; it never prints the key. If `anima-probe` was made by hand before (an earlier version
of this section, with another uid), remove it first: `sudo userdel -r anima-probe`. Keep the host key
fingerprint from the last line for step 4. To change the key later, run `probe` again with the new one.

Then install the new sshd drop-in. `harden.sh ssh` writes it, checks it with `sshd -t`, `sshd -T`
and `sshd -T -C` (also for `anima-probe` from `172.30.0.10`), puts the old drop-in back if any
check fails, and reloads sshd. It keeps the old drop-in as `/var/lib/anima/00-anima.conf.previous`.
The new drop-in admits the admin only from the VPN range: run it from an SSH session over the VPN.

```sh
# Run on the VPS, in an SSH session over the VPN.
cd /opt/anima/headscale/vps
export ANIMA_ADMIN_USER=anima-admin
sudo --preserve-env=ANIMA_ADMIN_USER ./harden.sh --dry-run ssh
sudo --preserve-env=ANIMA_ADMIN_USER ./harden.sh ssh
```

**Keep this session open.** In a new session, check that `ssh anima-admin@100.64.0.1` still logs
in. Then check:

```sh
# Run on the VPS.
sudo sshd -T | grep -i '^allowusers'
id anima-probe
sudo cat /home/anima-probe/.ssh/authorized_keys | cut -c1-40
ANIMA_ADMIN_USER=anima-admin sudo --preserve-env=ANIMA_ADMIN_USER /opt/anima/headscale/vps/harden.sh check
```

`sshd -T` prints two lines, `allowusers anima-admin@100.64.0.0/10` and
`allowusers anima-probe@172.30.0.10`. `id` prints `uid=10002(anima-probe) gid=10002(anima-probe)
groups=10002(anima-probe)`. The key line starts `restrict,pty,from="172.30.0.10" ssh-ed25519`.
`check` ends with `ok: no violations`. A login as `anima-probe` from the Mac is refused: it may come
only from the hub.

Undo, in the session you kept open:

```sh
# Run on the VPS.
sudo install -m 0600 -o root -g root /var/lib/anima/00-anima.conf.previous /etc/ssh/sshd_config.d/00-anima.conf
sudo sshd -t && sudo systemctl reload sshd.service
sudo userdel -r anima-probe
```

**4. Add the VPS in the app.** Add a device with the address `100.64.0.1` and an SSH credential for
user `anima-probe` with the private key from step 3. At the first connection, compare the host key
fingerprint with the one from step 3 before you trust it. Then delete `/tmp/anima-probe` and
`/tmp/anima-probe.pub` on the Mac. Set **Operating system** to Linux and **Metrics source** to SSH, so the hub collects the VPS's CPU,
memory and disk over the same key (MET-002); it collects only from Linux and macOS devices. Add a ping check and a TCP check on port 22.

Undo: remove the device in the app.

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
