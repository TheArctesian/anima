# WP-04 stage A report (package orchestrator)

Status: **done**. Tasks T1, T2 and T3 are accepted, and the final package review (Opus, security) has two rounds. `bun run gates` passes 12/12 on the branch tip, which includes the latest `phase-1`. Nothing is pushed or merged into `phase-1`. Stage B (releases, R-006) is Wave 3 and is not in this report.

| Item | Value |
| --- | --- |
| Branch | `claude/wp-04a-hub-deploy-88e451` |
| Tip | see the final commit of this report (includes `phase-1` at `bf94907`) |
| Plan and review log | [WP-04.md](WP-04.md) |
| Spec | GW-001–004, DB-001–003, DB-006, DB-007, RA-011, DNS-002, DNS-003, SEC-001, SEC-004, NET-001, NET-003, VPS-006, LOG-007 |
| Rulings applied | R-016, R-019, R-021, R-023, R-025, R-030, R-031 |

## Gates (`bun run gates`, tip)

All 12 pass:
- `cargo fmt`
- `cargo clippy`
- `cargo test`
- `bun install --frozen-lockfile`
- `lint`
- `check`
- `test` (the `@anima/deploy` workspace has 243 tests; 30 s per test, because each backup test migrates a real database)
- the client build
- `limits`
- `names`
- `imports`
- `baseline --base phase-1`

`deploy/test/smoke.sh` is a manual Docker check and is not a gate. It passes on Docker 29.9.2 with `phase-1` at `bf94907`. It:
- builds the gateway image and checks the font;
- starts PostgreSQL and checks the roles;
- runs the certificate hook under the real container limits;
- builds the hub image from the checkout (`--bin anima-hub`);
- runs `anima-hub migrate` as the owner role twice, and both runs exit 0;
- starts the hub with the runtime login only. The hub passes its start check (R-023) and becomes healthy.

**TLS only.** With WP-05 T4 the smoke test waits until `anima-hub health` reports the hub healthy, fails if plain HTTP gets any HTTP answer there, and needs Docker's proxied `/graphql` request closed before TLS, with a `Refused a connection from 172.…` event (NET-002).

## What the package delivers (`deploy/`)

- **`compose.yaml`.** Five services: `postgres` (17.11-bookworm), `migrate`, `certs`, `guacd`, `hub`.
  - **Networks:**
    - `database` is internal, with a fixed subnet (default 172.30.1.0/24). It holds postgres, migrate and hub.
    - `gateway` has a fixed subnet (172.30.0.0/24). It holds the hub at 172.30.0.10 and guacd.
    - `acme` holds certs only.
  - **Ports:** the only published port is `${ANIMA_HUB_ADDRESS}:443 → 8443`. PostgreSQL and guacd publish nothing (DB-001, GW-002, NET-003).
  - **Start order (R-016):** postgres must be healthy first. Then `migrate` (`anima-hub migrate`, with the owner URL only) must complete successfully, and certs must be healthy. Only then does the hub start, holding only the runtime URL and the vault key.
  - **Hardening:** every service has a read-only root, `cap_drop: ALL` (certs adds CHOWN only), `no-new-privileges` and `json-file` logs at 10 MB × 10.
  - **SELinux:** bind mounts use the `z` option.
  - **R-030:** `ANIMA_PROBE_DENY` covers both hub subnets and the VPS public /32. A test checks that it names the subnet of every network the hub is on. `preflight public` refuses a private, VPN, link-local or documentation address for `ANIMA_VPS_PUBLIC_ADDRESS`.
- **`hub/Dockerfile`.** A multi-stage build with an allow-list context. It builds only `cargo build --release --locked --bin anima-hub`; a test refuses `--all-targets`, `--tests` and features, so no test-support code ships (R-031). The runtime image is `debian:bookworm-20261027-slim` and runs as user `10001:10001`.
- **`gateway/Dockerfile`.** Built from `guacamole/guacd:1.6.0`. It adds only `OpenDyslexicMNerdFontMono-Regular.otf` and its licence (RA-011).
- **`certs/`.** lego 4.25.2 obtains the certificate by DNS-01 against Cloudflare, reading the token from a file secret (DNS-002, DNS-003).
  - The install hook writes one `tls.pem` (key plus chain, 10001, 0400) with one rename.
  - A stamp makes the hook a no-op when nothing changed. The hook runs at start and after every renew check.
  - There is one renew check at start and one every 12 hours.
  - lego's own state is in a volume that the hub never mounts.
- **`bin/make-secrets`.** Makes the seven file secrets with random values and the D-1 owners and modes.
  - It never prints or overwrites a value.
  - It checks the folder it writes into and the files already there.
  - It reads the Cloudflare token with no echo, or from standard input.
  - It is tested with shims for openssl, chown, chmod, stty and find.
- **`bin/preflight`.** Checks:
  - the hub address is IPv4 in 100.64.0.0/10;
  - the VPS public address is a public IPv4;
  - the Docker Engine is version 28 or later;
  - each secret file is present, not empty, and has the right mode.
- **`postgres/roles.sql` and `init/10-anima-roles.sh`.** These run on the first start of an empty volume (R-016, R-021).
  - They create `anima_runtime` (NOLOGIN), `anima_owner` (LOGIN; no superuser, CREATEROLE or CREATEDB) and `anima_hub` (LOGIN, a member of `anima_runtime`, owning nothing).
  - They create the database `anima`, owned by `anima_owner`.
  - They revoke CONNECT from PUBLIC on all databases.
  - Passwords are never put in argv or the server log; this is tested under every log setting.
  - The R-023 start-check properties are tested.
- **`backup/`** (DB-007, VPS-006):
  - **`backup.sh`** writes `pg_dump -Fc` and a row-count manifest from one exported snapshot, under a lock. Pruning is by strict file name.
  - **`restore.sh --yes`** checks the dump, restores into a temporary database, then swaps it in by rename. It keeps the previous database and refuses while the hub or migrate is running.
  - **`restore-check.sh`** is a drill that compares a restored copy with the manifest and checks the log-table privileges. It can run at any time.
  - **`offsite.sh`** sends the dump and manifest with restic to WP-03's append-only rest-server on the home PC.
  - The units and timers ship as files, including a prune unit for the home PC.

## Review findings (summary; detail in the review log)

- **T1:**
  - must: empty secrets on an openssl failure (fixed).
  - shoulds: CREATE ROLE passwords in duration/sample logs, NULL passwords from a missing file, an untrusted secrets folder (all fixed and locked by mutation tests).
- **T2:**
  - musts: the lego state volume mounted over the lego binary, and the hook's chmod after chown under the dropped capabilities. Both were proved in Docker and fixed.
  - shoulds:
    - key/chain mismatch: one `tls.pem` now;
    - Docker 28 or later: guacd was reachable from a routed neighbour on 27, proved in dind;
    - DNAT without an interface match: replaced by WP-03's nftables rule plus a runbook check;
    - rotation put the password in argv: fixed;
    - a renewal whose hook failed was never installed: fixed.
- **T3:**
  - musts: the drill compared with live counts, and `--yes` dropped the live database before it checked the dump (both fixed).
  - shoulds: malformed names pruned, the restic README (https, no init, no install), mutants (all fixed).
- **Final review:**
  - shoulds: the home-PC `restic forget --no-lock` always failed (fixed with a `User=restic` unit; the command was proved in restic 0.18.1), and plan IDs in code comments and test names (COM-002; fixed).
  - coulds: fixed, except C-3 and C-4 below.
  - The reviewer ran the hub's R-023 start-check SQL against the deployed and the restored database: no refusal.

## Operator changes runbook (R-025)

Do the steps in this order.
- **Hosts:**
  - **VPS** is AlmaLinux 10, 31.220.53.226, reached as WP-03's sudo admin user over SSH on the VPN.
  - **HOME** is the home Linux PC that runs WP-03's rest-server.
  - **CF** is the Cloudflare account.
  - **MAC** is the operator's machine.
- **Paths:** `D=/opt/anima/deploy` on the VPS.
- **Secrets:** 🔒 marks a step that handles a secret value. The file is named; never print it, `cat` it or pass it in argv.
- **Prerequisites:**
  - WP-03 is applied: Tailscale on the VPS with `tag:hub`, the nftables table `inet anima_filter`, `daemon.json`, and the rest-server on HOME.
  - The hub's VPN address is known: `tailscale ip -4` on the VPS.
  - WP-05's `migrate` subcommand is on `phase-1` (done at `2f28917`).
  - **WP-05's TLS edge (H-2) is on `phase-1`** (WP-05 T4). Before it, the hub answers plain HTTP on 443; the smoke test now fails on plain HTTP, so a passing smoke run on the merged branch proves it.
  - **The userland proxy is off** (`"userland-proxy": false` in WP-03's `daemon.json`). Docker then forwards VPN clients by DNAT, and the hub sees their 100.64 address. The hub refuses every 172.16.0.0/12 peer, so a proxied or SNAT'd client is locked out (step 10b).

| # | Host | Command | Check | Undo |
| --- | --- | --- | --- | --- |
| 1 | CF | 🔒 Create an API token: permission Zone / DNS / Edit on zone `anima.ing` only; IP filter to 31.220.53.226 (optional, as WP-03 did). Keep it only in the clipboard for step 7. | The token list shows one token with that single permission | Delete the token in CF |
| 2 | VPS | `sudo dnf install -y docker-ce docker-compose-plugin openssl policycoreutils-python-utils && sudo systemctl enable --now docker` (skip packages WP-03 already installed) | `docker version --format '{{.Server.Version}}'` ≥ 28.0; `semanage --help` runs | `sudo dnf remove openssl policycoreutils-python-utils` (keep Docker if WP-03 uses it) |
| 3 | VPS | `sudo systemctl edit docker.service` → add `[Unit]` / `After=tailscaled.service` / `Wants=tailscaled.service`; `sudo systemctl daemon-reload` | `systemctl show docker -p After` lists `tailscaled.service` | `sudo rm -r /etc/systemd/system/docker.service.d/override.conf && sudo systemctl daemon-reload` |
| 4 | VPS | `sudo nft list table inet anima_filter` | Chain `prerouting` has `iifname != { "lo", "tailscale0" } fib daddr type local ip daddr 100.64.0.0/10 drop`. **Stop** if it is missing (WP-03 first) | — (read only) |
| 5 | MAC→VPS | Copy the `deploy/` folder of the merged branch: `rsync -a --exclude secrets --exclude node_modules deploy/ <admin>@<hub VPN IP>:/tmp/anima-deploy/`, then on VPS `sudo mkdir -p /opt/anima && sudo rsync -a /tmp/anima-deploy/ /opt/anima/deploy/ && sudo chown -R root:root /opt/anima` | `ls /opt/anima/deploy/compose.yaml` | `sudo rm -rf /opt/anima` |
| 6 | VPS | `sudo semanage fcontext -a -t container_file_t '/opt/anima/deploy/secrets(/.*)?' && sudo install -d -m 0700 /opt/anima/deploy/secrets && sudo restorecon -Rv /opt/anima/deploy/secrets` | `ls -Zd /opt/anima/deploy/secrets` shows `container_file_t` | `sudo semanage fcontext -d '/opt/anima/deploy/secrets(/.*)?'` |
| 7 | VPS | 🔒 As root in bash, in `$D`: `read -rs CF_TOKEN; printf '%s\n' "$CF_TOKEN" \| sh bin/make-secrets --cloudflare-token-stdin; unset CF_TOKEN; restorecon -Rv secrets`. This makes `postgres_password`, `owner_password`, `hub_password`, `owner_database_url`, `hub_database_url`, `anima_key` and `cloudflare_dns_token`. | `sudo ls -ln $D/secrets` shows 7 files: `0 999` (the three passwords, 0440), `0 10001` (the two URLs and `anima_key`, 0440), `0 0` (the token, 0400). Run `getent group 999 10001`: 999 is `systemd-oom` with no members. | `sudo rm $D/secrets/*` (only before step 10; after it, the volume holds these passwords) |
| 8 | VPS | Write `$D/.env` (mode 0600, root) with `ANIMA_HUB_ADDRESS=<hub VPN IPv4>`, `ANIMA_ACME_EMAIL=<operator e-mail>` and `ANIMA_VPS_PUBLIC_ADDRESS=31.220.53.226`. Add `ANIMA_PERMITTED_NETWORKS=100.64.0.0/10,<site subnets>` (NET-001). | `sudo sh -c 'cd /opt/anima/deploy && set -a && . ./.env && set +a && sh bin/preflight'` exits 0 | `sudo rm $D/.env` |
| 9 | VPS | `cd $D && sudo docker compose build` | `sudo docker images` lists `anima-hub:local` and the gateway image | `sudo docker image rm anima-hub:local <gateway image>` |
| 10 | VPS | `cd $D && sudo docker compose up -d` (this accepts the Let's Encrypt subscriber agreement for `ANIMA_ACME_EMAIL`) | `sudo docker compose ps`: postgres healthy, migrate exited 0, certs healthy, hub healthy (`anima-hub health`). `sudo docker compose logs certs` shows `installed /certs/hub/tls.pem`. From a VPN client: `curl -sf -o /dev/null -w '%{http_code}\n' -X POST -H 'content-type: application/json' -d '{"query":"{__typename}"}' https://hub.anima.ing/graphql` prints `200` with a valid certificate (a closed connection with no HTTP status means the hub saw a 172.16/12 peer: go to step 10b), and `curl -s http://hub.anima.ing:443/health` gets no HTTP answer. From outside the VPN, the connection fails. If certs is unhealthy after the first order: `sudo docker compose restart certs` | `sudo docker compose down` (keeps volumes); `down -v` deletes the database, so do that only before real data exists |
| 10b | VPS, then MAC (on the VPN), then VPS | **Client address check.** On VPS: `pgrep -a docker-proxy`. On MAC: `tailscale ip -4`; then send one sign-in that must fail, with a made-up name and no real password: `curl -sS https://hub.anima.ing/graphql -H 'content-type: application/json' -d '{"query":"mutation { login(input: {username: \"source-check\", password: \"not-a-password\"}) { token } }"}'`. On VPS: `sudo docker compose -f $D/compose.yaml exec -T postgres psql -X -U postgres -d anima -Atc "select at, message from events.event_log where source = 'access' order by at desc limit 3"` | `pgrep` prints nothing. The MAC's curl gets a GraphQL error, not a closed connection. The newest event is `Sign-in failed for an unknown username from 100.x.y.z.`, with the MAC's own `tailscale ip -4` address. **Stop** if the event says `Refused a connection from 172.…`: Docker is proxying or SNATing VPN clients. Check `/etc/docker/daemon.json` has `"userland-proxy": false`, then `sudo systemctl restart docker` and repeat. | — (read only; the failed attempt only counts toward that address's sign-in throttle) |
| 11 | VPS→offline | 🔒 Copy `$D/secrets/anima_key` to an offline store (for example an encrypted USB key) with `sudo install -m 0400 $D/secrets/anima_key <mount>/anima_key` | `sudo cmp $D/secrets/anima_key <mount>/anima_key` exits 0 | Delete the copy |
| 12 | VPS | `sudo dnf install -y epel-release && sudo dnf install -y restic` | `restic version` | `sudo dnf remove restic` |
| 13 | HOME | 🔒 `sudo htpasswd -B /etc/restic/htpasswd anima-db` (it prompts; type a new rest-server password). Then create `/etc/restic/anima-db-password` (the restic repository password, mode 0400, owner `restic`) with `umask 077; read -rs P; printf '%s' "$P" > /etc/restic/anima-db-password; unset P`. Check `--max-size` per `deploy/backup/README.md`. | `sudo grep -c '^anima-db:' /etc/restic/htpasswd` prints 1 | `sudo htpasswd -D /etc/restic/htpasswd anima-db` |
| 14 | VPS | 🔒 In bash as root: `umask 077; read -rs U; printf '%s' "rest:http://anima-db:$U@<home PC LAN IP>:8000/anima-db/" > $D/secrets/restic_repository; read -rs P; printf '%s' "$P" > $D/secrets/restic_password; unset U P; restorecon -Rv $D/secrets` (the same password as step 13) | `sudo ls -l $D/secrets/restic_*` shows two files, mode 0600 | `sudo rm $D/secrets/restic_repository $D/secrets/restic_password` |
| 15 | VPS | `sudo restic --repository-file $D/secrets/restic_repository --password-file $D/secrets/restic_password init` | The command prints `created restic repository` | On HOME: `sudo rm -r /srv/restic/anima-db` |
| 16 | VPS | `sudo install -m 0644 $D/backup/anima-backup.unit /etc/systemd/system/anima-backup.service && sudo install -m 0644 $D/backup/anima-backup.timer /etc/systemd/system/anima-backup.timer && sudo systemctl daemon-reload` | `systemctl cat anima-backup.service` | `sudo rm /etc/systemd/system/anima-backup.{service,timer} && sudo systemctl daemon-reload` |
| 17 | VPS | `sudo systemctl start anima-backup.service` | `journalctl -u anima-backup.service -n 20` shows the dump path and a restic snapshot. `sudo ls -l /var/backups/anima` shows `anima-<UTC>.dump` and `.counts`, mode 0600 | `sudo rm /var/backups/anima/anima-*` |
| 18 | VPS | `sudo $D/backup/restore-check.sh /var/backups/anima/<dump from step 17>` | Exit 0, "match the backup manifest" | — (it drops its own check database) |
| 19 | VPS | `sudo systemctl enable --now anima-backup.timer` | `systemctl list-timers anima-backup.timer` | `sudo systemctl disable --now anima-backup.timer` |
| 20 | HOME | `sudo install -m 0644 <repo>/deploy/backup/home-pc/anima-db-prune.unit /etc/systemd/system/anima-db-prune.service && sudo install -m 0644 <repo>/deploy/backup/home-pc/anima-db-prune.timer /etc/systemd/system/anima-db-prune.timer && sudo systemctl daemon-reload && sudo systemctl enable --now anima-db-prune.timer` | `systemctl list-timers anima-db-prune.timer`; `sudo systemctl start anima-db-prune.service` exits 0 | `sudo systemctl disable --now anima-db-prune.timer && sudo rm /etc/systemd/system/anima-db-prune.*` |
| 21 | VPS→offline | 🔒 Copy `$D/secrets/restic_password` offline, as in step 11 | `cmp` exits 0 | Delete the copy |
| 22 | VPS | Reboot test: `sudo systemctl reboot`. After it, run `sudo docker compose -f $D/compose.yaml ps` | The hub is running. If its port bind failed (the VPN address was not ready, C-4), run `sudo docker compose -f $D/compose.yaml up -d hub` and report it, so a wait unit can be added | — |

Not a step: password rotation is in `deploy/README.md` "Rotate a password". It handles 🔒 `hub_password` and `hub_database_url`.

## Handoffs

1. **WP-05, taken under R-021 (check at its merge):**
   - `anima-hub migrate` reads `ANIMA_DATABASE_URL_FILE` and exits 0. **Done:** the smoke test proves it at `bf94907`.
   - The bind check accepts the private bridge address 172.30.0.10. **Done:** the hub starts on it in the smoke test.
   - **Done (H-2, WP-05 T4):** the hub serves TLS; the smoke test fails on plain HTTP and waits for `anima-hub health`. The hub reads the chain and the key from the one file `tls.pem` (both TLS variables name it), reloads it when it changes, checks the pair with `keys_match` before swapping, and does not trust 172.x client sources.
   - `config.rs` reads `ANIMA_PROBE_DENY` (R-030). The compose test accepts the name from `contracts/conventions.md` until then.
2. **Done (WP-05 T4), the hub health probe:** compose's hub healthcheck runs `anima-hub health`.
3. **WP-10, the gateway font:** the family name in the image is `OpenDyslexicM Nerd Font Mono`, while `fc-match OpenDyslexic` returns DejaVu. The guacd `font-name` argument must use the exact name.
4. **WP-03:**
   - (C-3, not proven) A neighbour on the provider network with a static route to 172.30.0.0/24 may reach the published container address 172.30.0.10:8443 directly. Proposed rule for `nftables.conf` raw prerouting: `iifname != { "lo", "tailscale0", "docker0", "br-*" } ip daddr 172.16.0.0/12 drop`. NET-002 is the second line of defence.
   - The Docker 28 minimum. The `br-*` bridge names are kept.
5. **Root files, edited as an approved handoff:** `"deploy"` in `package.json` workspaces, `bun.lock`, and `deploy/**/*.ts` added to the limits block in `eslint.config.js`. Without that last change the deploy tests would have no limits (CQ-010).

## Deferred items (for issue #1, R-007)

- Pin all five base images by digest (`postgres`, `guacd`, `goacme/lego`, `rust`, `debian`). They are pinned by version tag now.
- An egress filter for guacd: it parses untrusted RDP, VNC and SSH data and can reach any host (GW-003 defence in depth).
- Run `certs` as non-root (10002:10001, key 0440 group 10001) so it needs no capability.
- C-4: a oneshot unit that waits for the VPN address before the hub's port bind, if step 22 shows the race.
- An all-or-nothing check pass in `make-secrets`: a bad existing file can fail after other files were made. A rerun is safe.
- The SIGINT prompt test runs only on macOS `/bin/sh`; it has not run under dash.
- SELinux behaviour is not tested in an automated way: macOS Docker has no SELinux. Steps 6, 7 and 10 of the runbook are the first real check.

## Open questions and risks

- Members of host gid 999 or 10001 can read the database secrets. On AlmaLinux 10, gid 999 is `systemd-oom` with no members.
- The postgres entrypoint keeps `POSTGRES_PASSWORD` in PID 1's environment inside the container. Only uid 999 and `docker exec` users can read it.
- Dumps hold credential ciphertext and password hashes. restic encrypts them off-site; on the VPS they are mode 0600 in a 0700 folder.

## Contract change requests

None open. K2 CREATEROLE wording was settled by R-021, and the start check by R-023.
