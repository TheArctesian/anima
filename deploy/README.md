# Run the Anima hub

This folder holds the Docker Compose deployment of the hub (WP-04 stage A). It starts
PostgreSQL 17, the migration job, the certificate service, `guacd` and the hub.
Backups are described in [backup/README.md](backup/README.md).

## What runs

| Service | Image | Purpose | Networks |
| --- | --- | --- | --- |
| `postgres` | `postgres:17.6-bookworm` | The database. No published port. | `database` |
| `migrate` | `anima-hub:local` | Runs `anima-hub migrate` as `anima_owner`, then exits. | `database` |
| `certs` | `goacme/lego` | Gets and renews the certificate for `hub.anima.ing`. | `acme` |
| `guacd` | built from `gateway/` | The Guacamole daemon with the OpenDyslexic font. No published port. | `gateway` |
| `hub` | `anima-hub:local` | The hub. The only published port: `ANIMA_HUB_ADDRESS:443`. | `database`, `gateway` |

The `database` network has no route out. The hub joins `gateway` at a fixed address
(`ANIMA_HUB_IP`, default `172.30.0.10`) and listens on port 8443 there.

All containers have a read-only root file system, no Linux capability and
`no-new-privileges`. Two exceptions: `postgres` runs as user 999 (no capability needed), and
`certs` runs as root with the one capability `CHOWN`, because it gives the certificate
files to the hub user (10001).
Each service writes its log as JSON files, 10 MB each, 10 files.

## Prerequisites

- A Linux host with Docker Engine and the Compose plugin.
- The host has the VPN address that the hub will use (`ANIMA_HUB_ADDRESS`).
- A Cloudflare API token for the zone `anima.ing` (see Operator changes).

## First start

1. Make the secret files. Run this as root, in this folder:

   ```sh
   sh bin/make-secrets
   ```

   The script makes each missing file in `secrets/` with a random value and the right owner
   and mode. It never overwrites a file and never prints a secret. It asks for the
   Cloudflare token if you run it in a terminal. Run it again at any time: it only adds
   what is missing.

2. Set the environment. Put these in `deploy/.env` (mode 0600) or export them:

   | Variable | Required | Default | Meaning |
   | --- | --- | --- | --- |
   | `ANIMA_HUB_ADDRESS` | yes | none | The VPN address that gets the published port 443. |
   | `ANIMA_ACME_EMAIL` | yes | none | Contact address for Let's Encrypt. |
   | `ANIMA_HUB_SUBNET` | no | `172.30.0.0/24` | Subnet of the `gateway` network. |
   | `ANIMA_HUB_IP` | no | `172.30.0.10` | Hub address inside `ANIMA_HUB_SUBNET`. |
   | `ANIMA_PERMITTED_NETWORKS` | no | `100.64.0.0/10` | Networks that devices may be in. |
   | `ANIMA_LOG` | no | `info` | Hub log filter. |
   | `ANIMA_ACME_SERVER` | no | Let's Encrypt production | Use the staging URL to test. |

   Compose refuses to start if a required variable is missing.

3. Start:

   ```sh
   docker compose up -d
   ```

**Let's Encrypt subscriber agreement.** Starting the `certs` service accepts the Let's
Encrypt subscriber agreement for you (`--accept-tos`). Read it first.

## Order of start

1. `postgres` starts. On an empty volume it runs `postgres/init/10-anima-roles.sh` once.
   It is healthy when it accepts connections on TCP.
2. `migrate` runs the migrations and exits with code 0.
3. `certs` gets the certificate. It is healthy when the key and the chain are installed.
   This step needs Cloudflare and Let's Encrypt. If the order fails, the service waits one
   hour before it tries again, so it stays below the Let's Encrypt limits.
4. `guacd` starts.
5. `hub` starts when all of the above are ready.

The hub needs `anima-hub migrate` and the certificate reload from WP-05. Until WP-05 is
merged, `migrate` fails and the hub does not start.

## Certificate renewal

`certs` checks the certificate every 12 hours and renews it when fewer than 30 days remain.
The hub loads a renewed certificate without a restart once WP-05 has landed. Until then,
restart the hub after a renewal: `docker compose restart hub`.

## Logs

```sh
docker compose logs -f hub          # follow the hub
docker compose logs --tail 100 certs
docker compose ps                   # state and health
```

The `migrate` log shows the last migration run. No log contains a secret (LOG-007).

## Rotate a password

Do this for `hub_password` (the same steps work for `owner_password`).

1. Stop the hub: `docker compose stop hub`.
2. Make a new value and the matching URL. Keep the old files until step 5:
   `NEW=$(openssl rand -hex 32)`.
3. Change the role in the database:
   `docker compose exec postgres psql -U postgres -c "ALTER ROLE anima_hub PASSWORD '$NEW'"`.
4. Write the same value to `secrets/hub_password` (no trailing newline) and write
   `postgres://anima_hub:<NEW>@postgres:5432/anima` to `secrets/hub_database_url`. Keep the
   owner (`root:10001` for the URL, `root:999` for the password) and mode `0440`.
5. Start the hub: `docker compose up -d hub`. Check `docker compose logs hub`.

For `owner_password`, use `anima_owner` and `secrets/owner_database_url`, and rerun
`docker compose up -d migrate` to check the new URL.
`postgres_password` only matters for the first start; to change it, use `ALTER ROLE postgres`
and update the file.

## Operator changes

Do each item once, on the real host or in Cloudflare. Nothing in this repository does them.

1. Create a Cloudflare API token with the permission "Zone / DNS / Edit" on the zone
   `anima.ing` only. Give it no other permission and no other zone.
2. Place the token on the host with `sh bin/make-secrets` (run it in a terminal and paste
   the token) or write it to `secrets/cloudflare_dns_token` with owner `root:root`, mode 0400.
3. Check that the host groups 999 and 10001 hold no user who should not read the secrets:
   `getent group 999 10001`. Both gids should be empty or hold no one.
4. Create `deploy/.env` (mode 0600) with `ANIMA_HUB_ADDRESS` and `ANIMA_ACME_EMAIL`.
5. Make sure `tailscaled` starts before Docker, so the VPN address exists when Docker
   binds the port (for example a systemd drop-in for `docker.service` with
   `After=tailscaled.service`).
6. Firewall: publish nothing else. The compose file publishes only `ANIMA_HUB_ADDRESS:443`.
   Do not open 443 on the public interface.
7. Run the first start (`docker compose up -d`) and check `docker compose ps`.
8. Back up `secrets/anima_key` offline (for example in a password manager). Without it, the
   stored credentials cannot be opened. The database backup does not contain it.

## Checks

- `bun run --cwd deploy test` checks the compose file, the secret script and the
  database roles. It needs no Docker.
- `sh deploy/test/smoke.sh` is a manual test that needs Docker. It builds the gateway
  image, checks the font, starts `postgres` and checks the roles. It uses a temporary
  copy of this folder and contacts neither Let's Encrypt nor Cloudflare.
