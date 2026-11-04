# Headscale backup (VPS-006, decision D-12)

Every day the VPS takes a consistent snapshot of the Headscale database, adds the two
private keys and the configuration, and sends it with `restic` to a `rest-server` on the
home Linux PC over the VPN. restic encrypts on the VPS. The rest-server runs
`--append-only --private-repos`. A compromised VPS can add snapshots, but it cannot
delete old snapshots as long as `prune.sh` refuses anomalous snapshot lists (see
[Risks](#risks)). Retention and pruning run only on the home PC.

The full setup and the restore drill are in the VPN guide (`docs/setup/vpn.md`). This
file lists what goes where. The hub reaches `10.10.0.0/24` only if the VPS runs
`tailscale up --accept-routes` (also in the guide).

## File map

systemd needs the `.service` suffix; the repository files end in `.unit` because the
name check (CQ-030) refuses the word "service" in file names. Copy them to the
installed names below.

| Repository file | Machine | Installed as | Mode |
| --- | --- | --- | --- |
| `backup.sh` | VPS | `/usr/local/sbin/headscale-backup` | `0700 root` |
| `headscale-backup.unit` | VPS | `/etc/systemd/system/headscale-backup.service` | `0644` |
| `headscale-backup.timer` | VPS | `/etc/systemd/system/headscale-backup.timer` | `0644` |
| `rest-server.unit` | home PC | `/etc/systemd/system/rest-server.service` | `0644` |
| `rest-server-allow.conf` | home PC | `/etc/systemd/system/rest-server.service.d/allow.conf`, placeholder replaced | `0644` |
| `prune.sh` | home PC | `/usr/local/bin/restic-prune` | `0755` |
| `restic-prune.unit` | home PC | `/etc/systemd/system/restic-prune.service` | `0644` |
| `restic-prune.timer` | home PC | `/etc/systemd/system/restic-prune.timer` | `0644` |

Files the operator creates (secrets: never commit them, never print them):

| File | Machine | Content | Mode |
| --- | --- | --- | --- |
| `/etc/anima/backup/restic-repository` | VPS | `rest:http://headscale-vps:<rest-server password>@<home PC LAN IP>:8000/headscale-vps/headscale` | `0600 root` |
| `/etc/anima/backup/restic-password` | VPS | restic encryption password (also kept offline) | `0600 root` |
| `/etc/restic/htpasswd` | home PC | `htpasswd -B` entry for user `headscale-vps` | `0640 root:restic` |
| `/etc/restic/headscale-password` | home PC | the same restic encryption password | `0600 root` |
| `/etc/restic/rest-server.env` | home PC | `HOME_PC_LAN_IP=10.10.0.X` (not a secret) | `0644` |

Make the rest-server password hex (`openssl rand -hex 32`): it sits inside a URL, and
hex needs no URL encoding.

Packages: on the VPS (AlmaLinux 10), `sqlite` (AppStream, the `sqlite3` command) and
`restic` (EPEL 10); both install to `/usr/bin`. Under SELinux the backup unit runs as
`unconfined_service_t` and reads Headscale's `container_file_t` files; the systemd sandbox
confines it. On the home PC, `restic`, `rest-server` and `jq`, with a system user `restic` that owns `/srv/restic` (`0700`). Put `/srv/restic` on its
own filesystem or under a quota, so a flood of uploads cannot fill the home PC's disk
before `--max-size` (1 GiB) stops it.

## What a snapshot holds

From `/opt/anima/headscale`: `lib/db.sqlite` (taken with `VACUUM INTO` and checked with
`PRAGMA integrity_check`), `lib/noise_private.key`, `lib/derp_server_private.key`,
`config/config.yaml`, `config/policy.hujson`. Host `anima-vps`, tag `headscale`. The ACME
cache is not saved; Headscale requests a new certificate.

## Run and check

```sh
# VPS
systemctl daemon-reload
systemctl enable --now headscale-backup.timer
systemctl start headscale-backup.service        # first run by hand
journalctl -u headscale-backup.service -n 50

# home PC
systemctl daemon-reload
systemctl enable --now rest-server.service restic-prune.timer
```

First real run on the VPS, check:

1. The database copy works under `ReadOnlyPaths`. `sqlite3 -readonly` opens the live WAL
   database read-only. If the journal shows `unable to open database file (14)` while
   Headscale is stopped, start Headscale once (it creates `db.sqlite-wal` and
   `db.sqlite-shm`) and run the backup again. If the error persists, add
   `ReadWritePaths=-/opt/anima/headscale/lib/db.sqlite-shm` to the unit (the `-shm` file
   only, never the whole folder; the leading `-` keeps the unit from failing with
   226/NAMESPACE when the file is missing). This lets the backup process write the live
   WAL index, so add it only if the read-only open fails.
2. `ls -ln /opt/anima/headscale/lib` shows owner `0` for every file. The unit has no
   capabilities; if Headscale's files belong to another user, add
   `CapabilityBoundingSet=CAP_DAC_READ_SEARCH` to the unit.
3. On the home PC, `journalctl -u rest-server.service` shows the VPS's request coming
   from the address in `allow.conf`.

## Restore

On the VPS (the append-only repository still allows reads):

```sh
R="restic --repository-file /etc/anima/backup/restic-repository --password-file /etc/anima/backup/restic-password"
$R snapshots --host anima-vps --tag headscale
$R restore latest --host anima-vps --tag headscale --target /root/headscale-restore
src="$(dirname "$(find /root/headscale-restore -name db.sqlite)")"
sqlite3 "$src/db.sqlite" "PRAGMA integrity_check"      # must print ok

cd /opt/anima/headscale
docker compose stop headscale
rm -f lib/db.sqlite-wal lib/db.sqlite-shm              # a stale WAL would corrupt the restored db
install -m 0600 "$src/db.sqlite" lib/db.sqlite
install -m 0600 "$src/noise_private.key" "$src/derp_server_private.key" lib/
install -m 0644 "$src/config.yaml" "$src/policy.hujson" config/
docker compose start headscale
docker compose exec headscale headscale nodes list     # every node is listed
rm -rf /root/headscale-restore
```

If the VPS is lost, restore on the home PC from the local folder. Run as root without a
cache or a lock, so nothing is written into the `restic`-owned repository:

```sh
restic -r /srv/restic/headscale-vps/headscale --password-file /etc/restic/headscale-password \
  --no-cache --no-lock restore latest --host anima-vps --tag headscale --target /root/headscale-restore
src="$(dirname "$(find /root/headscale-restore -name db.sqlite)")"
rsync -a --chmod=D0700,F0600 "$src"/ root@<NEW_VPS_IP>:/root/headscale-restore/
rm -rf /root/headscale-restore
```

Then on the new VPS set `src=/root/headscale-restore` and run the steps above from
`cd /opt/anima/headscale` on.

## Risks

- **Snapshot flooding.** The VPS sets its own snapshot times. Retention is
  `--keep-within 1y` only: real snapshots are kept for one year whatever the VPS
  appends, and anything older than a year (counted from the newest snapshot) is pruned.
  Because keep-within counts from the newest snapshot, `prune.sh` refuses to prune when a
  snapshot is dated more than an hour in the future. It also refuses when a UTC day has
  more than two snapshots, which bounds growth (about 365 snapshots of a database under
  1 MB, well inside `--max-size`). After a refusal, inspect
  `restic snapshots --host anima-vps --tag headscale` and forget the bad snapshot IDs by
  hand. One extra run by hand beside the timer is fine; two or more extra runs on the same
  UTC day trigger a refusal.
- **The home PC holds the restic password**, because pruning needs it. Anyone with root on
  the home PC can decrypt the backups, including the Headscale private keys.
- **rest-server and prune share the `restic` user.** rest-server writes the repository
  files, so its user owns them: a compromised rest-server can delete the repository
  whichever user runs prune. Append-only protects against a compromised client (the
  VPS), not a compromised server. A separate prune user would not fix that. It would also
  have to share write access to the repository folders, and a prune run as root would
  leave files rest-server cannot read. Prune therefore runs as `restic`. The cost: while
  prune runs, a process running as `restic` could read the password credential. The
  defences are around rest-server: only the hub's VPN address may connect, it listens on
  the LAN address only, and it runs sandboxed with no capabilities.
- **Stale backups are not alerted yet** (deferred, issue #1). Check
  `systemctl list-timers headscale-backup.timer` and the journal.
