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
| `hub` | `anima-hub:local` | The hub. The only published port: `ANIMA_HUB_ADDRESS:443`. | `database`, `gateway` |

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
   | `ANIMA_HUB_IP` | no | `172.30.0.10` | Hub address inside `ANIMA_HUB_SUBNET`. |
   | `ANIMA_PERMITTED_NETWORKS` | no | `100.64.0.0/10` | Networks that devices may be in. |
   | `ANIMA_LOG` | no | `info` | Hub log filter. |
   | `ANIMA_ACME_SERVER` | no | Let's Encrypt production | Use the staging URL to test. |

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
7. Write `deploy/.env` (mode 0600) with `ANIMA_HUB_ADDRESS` and `ANIMA_ACME_EMAIL`.
8. Make `tailscaled` start before Docker: run `sudo systemctl edit docker.service` and add
   `[Unit]` and `After=tailscaled.service`. Then run `sudo systemctl daemon-reload`.
9. Check that WP-03's firewall is loaded: run `sudo nft list table inet anima_filter`. The
   chain `prerouting` must hold the rule
   `iifname != { "lo", "tailscale0" } fib daddr type local ip daddr 100.64.0.0/10 drop`.
   That rule drops hub-port traffic for the VPN address that does not arrive on
   `tailscale0`. If the table or the rule is missing, stop and apply WP-03's firewall first.
   WP-03's `daemon.json` makes Docker publish on `127.0.0.1` by default, so `compose.yaml`
   names `ANIMA_HUB_ADDRESS` in the hub's `ports` entry. Keep it. The compose file sets no
   bridge name on any network, because the forward chain relies on the default `br-*` names.
10. Run the first start (First start, steps 3 and 4) and check `docker compose ps`.
11. Copy `secrets/anima_key` to an offline store. Without it, the stored credentials cannot
    be opened. The database backup does not contain it.

## Checks

- `bun run --cwd deploy test` checks the compose file, the scripts (`bin/preflight`,
  `certs/install.sh`, `certs/run.sh` with a fake lego) and the database roles. It needs no Docker.
- `sh deploy/test/smoke.sh` is a manual test that needs Docker. It checks the Docker
  version, builds the gateway and hub images, checks the font, starts `postgres` and checks
  the roles, runs the certificate hook on a fixture certificate with the real container
  limits, and starts the hub without `migrate`. It uses a temporary copy of this folder and
  contacts neither Let's Encrypt nor Cloudflare.
