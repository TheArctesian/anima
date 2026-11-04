# Backup and restore

These scripts back up the Anima database each day (DB-007, VPS-006). `pg_dump` and
`pg_restore` run inside the `postgres` container, over its Unix socket. No password is needed.
The install folder on the VPS is `/opt/anima`. The VPS runs AlmaLinux 10 with SELinux
enforcing. The scripts need only `bash`, `coreutils`, `findutils` and `flock` (util-linux),
which that system has.

| Script | Purpose |
|---|---|
| `backup.sh` | Writes one checked dump and its counts manifest, and removes old ones. |
| `offsite.sh` | Sends the newest dump and manifest to the home PC with `restic`. |
| `restore.sh --yes <dump>` | Restores the dump into a new database and swaps it in. |
| `restore-check.sh <dump>` | Restore drill: restores into `anima_restore_check`, compares, drops it. |

## Where dumps go

`backup.sh` writes `anima-<UTC time>.dump` and `anima-<UTC time>.dump.counts` to
`/var/backups/anima`. It creates the folder with mode 0700 and the files with mode 0600. It
prints the path of the dump. That folder is on the host, outside every container, so SELinux
does not restrict it.

The dump and the row counts come from one database snapshot. So they agree, even while the hub
writes. The `.counts` file holds table names and row counts only.

| Variable | Default | Meaning |
|---|---|---|
| `ANIMA_BACKUP_DIR` | `/var/backups/anima` | Backup folder. |
| `ANIMA_BACKUP_KEEP_DAYS` | `14` | Backups older than this are deleted after a good backup. |
| `ANIMA_COMPOSE_FILE` | `/opt/anima/deploy/compose.yaml` | Compose file that has the `postgres` service. |

The script writes `.partial` files first. It checks the dump with `pg_restore --list`, then
renames the files. A failed run leaves no `.dump` file and deletes nothing. A lock (`flock`, or
the folder `.lock.d` if `flock` is missing) stops two runs at once. After a crash, remove a
stale `.lock.d` by hand.

Age is read from the time in the file name, not from the file time. If you copy an old dump back
into the folder, it keeps its old name, so the next backup deletes it. Copy restored files to
another folder.

## Off-site copy

A dump on the VPS does not survive the loss of the VPS. `offsite.sh` sends each backup to the
`rest-server` on the home PC, over the VPN. This is the same pattern as the Headscale backup
(`deploy/headscale/backup/README.md`): `restic` encrypts on the VPS, and the `rest-server` runs
with `--append-only --private-repos`. The VPS can add snapshots and cannot delete them. The
`rest-server` speaks plain HTTP, because the traffic stays inside the VPN.

A dump holds credential ciphertext and password hashes. `restic` encrypts it before it leaves
the VPS.

**Install restic on the VPS.** AlmaLinux 10 has no `restic` in its own repositories. EPEL 10
has it (version 0.19.1 when this was written; checked in an `almalinux:10` container):

```sh
dnf install -y epel-release
dnf install -y restic
```

**Secret files** that you create on the VPS (mode 0600, owner root; never commit them, never print them). The recipe below keeps the values out of the shell history and the process list:

| File | Content |
|---|---|
| `/opt/anima/deploy/secrets/restic_repository` | `rest:http://anima-db:<rest-server password>@<home PC LAN IP>:8000/anima-db/` |
| `/opt/anima/deploy/secrets/restic_password` | The restic encryption password. Keep a copy offline. |

Make the rest-server password hex (`openssl rand -hex 32`). It sits inside a URL.

```bash
# bash, as root, on the VPS
umask 077
cd /opt/anima/deploy/secrets
read -rs -p 'restic repository URL: ' value; echo
printf '%s' "$value" > restic_repository
read -rs -p 'restic encryption password: ' value; echo
printf '%s' "$value" > restic_password
unset value
```

**Home PC steps** (the Headscale README lists the packages and the `rest-server` unit):

1. Add the user: `htpasswd -B /etc/restic/htpasswd anima-db`. Use the same password as in the URL above.
2. Put the restic password in `/etc/restic/anima-db-password` (mode 0600, root).
3. Check the size limit. WP-03's `rest-server` unit starts with `--max-size` (1 GiB at the time of writing). Check in `/etc/systemd/system/rest-server.service` whether the limit covers each repository or all of them together, and whether Headscale already uses part of it. Daily dumps are small and `restic` stores only the changed parts, but 90 days of history grows. After the first week, run `restic stats --mode raw-data` and multiply by 13. Raise `--max-size` if the result is more than half of the limit.
4. Prune on the home PC only. This repository is append-only for the VPS, so the VPS cannot
   delete anything. Install the pair in `deploy/backup/home-pc/` (copy the folder to the home
   PC). The service runs as `User=restic`, the owner of `/srv/restic/anima-db`, so restic
   never writes a root-owned file into the repository. Do not run `restic forget` as root
   against it. Nor does the command take `--no-lock`: restic refuses that flag outside
   `--dry-run`.

```sh
install -m 0644 anima-db-prune.unit /etc/systemd/system/anima-db-prune.service
install -m 0644 anima-db-prune.timer /etc/systemd/system/anima-db-prune.timer
systemctl daemon-reload
systemctl start anima-db-prune.service
systemctl enable --now anima-db-prune.timer
```

The service runs this command, with the repository, the password file (a systemd credential
read from `/etc/restic/anima-db-password`) and the cache folder set in its environment:

```sh
restic forget --host anima-vps --tag anima-db --group-by host,tags --keep-within 90d --prune
```

Keep 90 days, not 14. Damage such as a wrong delete can go unseen for weeks. Ninety days also
matches the retention of `events.event_log`, so a restore from the oldest dump still holds the
logs of its own period. `--keep-within` counts from the newest snapshot, and the VPS chooses
snapshot times. Look at `restic snapshots --tag anima-db` now and then (as the `restic` user,
`runuser -u restic -- restic ...`) for a snapshot dated in the future, because it would shift
the window. `deploy/headscale/backup/prune.sh` has a guard for this, but it is fixed to the
Headscale host and tag, so it cannot be used here.

**Initialise the repository** once, from the VPS. This needs the user and password above:

```sh
restic --repository-file /opt/anima/deploy/secrets/restic_repository \
  --password-file /opt/anima/deploy/secrets/restic_password init
```

The command that `offsite.sh` runs after each backup:

```sh
restic --repository-file /opt/anima/deploy/secrets/restic_repository \
  --password-file /opt/anima/deploy/secrets/restic_password \
  backup --host anima-vps --tag anima-db <dump> <dump>.counts
```

## Schedule

Run the backup and the off-site copy every day with a systemd timer.

| Repository file | Installed as | Mode |
|---|---|---|
| `deploy/backup/anima-backup.unit` | `/etc/systemd/system/anima-backup.service` | `0644` |
| `deploy/backup/anima-backup.timer` | `/etc/systemd/system/anima-backup.timer` | `0644` |

The repository file ends in `.unit` because the name check (CQ-030) refuses the word
"service" in file names. The unit runs `backup.sh`, then `offsite.sh`. If `backup.sh` fails,
systemd does not run `offsite.sh`, and `journalctl -u anima-backup.service` shows why. The unit
stops a run after two hours. It is sandboxed: the file system is read-only except
`/var/backups/anima`, and the Docker socket stays reachable.

## What a dump does not hold

- **`anima_key`.** This key opens the stored credentials. Without it, restored
  credentials cannot be read. Keep a copy offline: `deploy/secrets/anima_key`.
- **The Cloudflare token.** Create a new one if you lose it.
- **The certificate.** The certificate container issues it again.
- **The database passwords.** On a new host, run `deploy/bin/make-secrets` and set the role passwords to match.

## Restore

Use this when the database is lost or damaged.

1. `cd /opt/anima/deploy`
2. Find the dump in `/var/backups/anima`. To fetch one from the home PC, use `restic restore`
   with the two secret files above, into a folder outside `/var/backups/anima`.
3. `docker compose stop hub`. Make sure `migrate` is not running.
4. `backup/restore.sh --yes <dump>`.
   The script reads the dump list first. It refuses to run while `hub` or `migrate` runs. It
   restores into a new database, and only then swaps it in for `anima`. A bad dump never
   touches `anima`.
5. `docker compose up -d migrate`. Wait until it exits with code 0.
6. `docker compose up -d hub`.
7. Open the client. Check that the devices and the audit log are there.
8. The old database stays as `anima_previous_<time>`. When you are sure, drop it with the
   command that `restore.sh` printed.

If `restore.sh` fails, `anima` is unchanged. Fix the cause and run it again.

## Monthly restore drill

Once a month, prove that the newest dump restores. This needs no downtime and runs any time.

```sh
cd /opt/anima/deploy
backup/restore-check.sh "$(find /var/backups/anima -regextype posix-extended \
  -regex '.*/anima-[0-9]{8}T[0-9]{6}Z\.dump' | sort | tail -n 1)"
```

The script restores the dump into `anima_restore_check`. It compares the row count of every
table with the `.counts` manifest from the same snapshot. It does not read the live database.
It checks that `anima_runtime` can read and append to `events.audit_log` and `events.event_log`,
and cannot update, delete or truncate them. Then it drops the check database. It prints table
names and counts, never row contents. It exits with a non-zero code on any difference or when
the manifest is missing.

## Operator changes

Do these in order.

1. On the VPS: `dnf install -y epel-release && dnf install -y restic`.
2. On the home PC: `htpasswd -B /etc/restic/htpasswd anima-db`, and create `/etc/restic/anima-db-password`. Check `--max-size` as described above.
3. On the VPS: create `/opt/anima/deploy/secrets/restic_repository` and `/opt/anima/deploy/secrets/restic_password` (mode 0600, root) with the bash recipe above (`umask 077`, `read -rs`, `printf '%s' ... > file`).
4. On the VPS: `restic --repository-file /opt/anima/deploy/secrets/restic_repository --password-file /opt/anima/deploy/secrets/restic_password init`.
5. On the VPS: `install -m 0644 /opt/anima/deploy/backup/anima-backup.unit /etc/systemd/system/anima-backup.service`.
6. On the VPS: `install -m 0644 /opt/anima/deploy/backup/anima-backup.timer /etc/systemd/system/anima-backup.timer`.
7. On the VPS: `systemctl daemon-reload`.
8. On the VPS: `systemctl start anima-backup.service`. Wait for it to finish, then read the printed dump path with `journalctl -u anima-backup.service -n 20`.
9. On the VPS: `/opt/anima/deploy/backup/restore-check.sh <the printed path>`.
10. On the VPS: `systemctl enable --now anima-backup.timer`.
11. Copy `/opt/anima/deploy/secrets/anima_key` and `restic_password` to an offline place.
12. On the home PC: install `deploy/backup/home-pc/anima-db-prune.unit` and `.timer` as in Home PC step 4 (they run as the `restic` user), then `systemctl enable --now anima-db-prune.timer`.
