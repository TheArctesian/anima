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
with `--append-only --private-repos`. The VPS can add snapshots and cannot delete them.

Pruning the remote repository is the job of the home PC, not of the VPS. Use a `restic forget`
rule there, as for the Headscale repository.

A dump holds credential ciphertext and password hashes. `restic` encrypts it before it leaves
the VPS.

Secret files that you create (never commit them, never print them):

| File | Content |
|---|---|
| `/opt/anima/deploy/secrets/restic_repository` | `rest:https://anima-db:<rest-server password>@<home PC address>:8000/anima-db/anima-db` |
| `/opt/anima/deploy/secrets/restic_password` | The restic encryption password. Keep a copy offline. |

The command that `offsite.sh` runs:

```sh
restic --repository-file /opt/anima/deploy/secrets/restic_repository \
  --password-file /opt/anima/deploy/secrets/restic_password \
  backup --host anima-vps --tag anima-db <dump> <dump>.counts
```

## Schedule

Run the backup and the off-site copy every day. Copy these two files to `/etc/systemd/system/`.

`anima-backup.service`:

```ini
[Unit]
Description=Anima database backup and off-site copy
Wants=network-online.target
After=network-online.target docker.service

[Service]
Type=oneshot
UMask=0077
LimitCORE=0
CacheDirectory=anima-offsite
CacheDirectoryMode=0700
Environment=ANIMA_RESTIC_CACHE_DIR=%C/anima-offsite
ExecStart=/opt/anima/deploy/backup/backup.sh
ExecStart=/opt/anima/deploy/backup/offsite.sh
```

`anima-backup.timer`:

```ini
[Unit]
Description=Daily Anima database backup

[Timer]
OnCalendar=*-*-* 03:30:00 UTC
RandomizedDelaySec=15m
Persistent=true

[Install]
WantedBy=timers.target
```

If `backup.sh` fails, systemd does not run `offsite.sh`. The run fails, and `journalctl -u anima-backup.service` shows why.

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
backup/restore-check.sh "$(ls /var/backups/anima/anima-*.dump | tail -n 1)"
```

The script restores the dump into `anima_restore_check`. It compares the row count of every
table with the `.counts` manifest from the same snapshot. It does not read the live database.
It checks that `anima_runtime` can read and append to `events.audit_log` and `events.event_log`,
and cannot update, delete or truncate them. Then it drops the check database. It prints table
names and counts, never row contents. It exits with a non-zero code on any difference or when
the manifest is missing.

## Operator changes

1. Copy the two unit files from this README to `/etc/systemd/system/anima-backup.service` and `/etc/systemd/system/anima-backup.timer`.
2. Run `systemctl daemon-reload`.
3. Create `/opt/anima/deploy/secrets/restic_repository` and `/opt/anima/deploy/secrets/restic_password` (mode 0600, root), and add an `anima-db` user to the `rest-server` on the home PC.
4. Run `systemctl enable --now anima-backup.timer`.
5. Copy `/opt/anima/deploy/secrets/anima_key` and `restic_password` to an offline place.
6. Run the first backup: `systemctl start anima-backup.service`.
7. Run the first drill: `/opt/anima/deploy/backup/backup.sh`.
8. Then run `/opt/anima/deploy/backup/restore-check.sh <the path that backup.sh printed>`.
