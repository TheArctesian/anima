# Backup and restore

These scripts back up the Anima database each day (DB-007, VPS-006). They run `pg_dump` and
`pg_restore` inside the `postgres` container, over its Unix socket. No password is needed.

| Script | Purpose |
|---|---|
| `backup.sh` | Writes one checked dump to the backup folder and removes old dumps. |
| `restore.sh --yes <dump>` | Replaces the database `anima` with the dump. |
| `restore.sh --check <dump>` | Restores into `anima_restore_check`, compares it, drops it. |

## Where dumps go

`backup.sh` writes `anima-<UTC time>.dump` to `/var/backups/anima`. It creates the folder with
mode 0700 and each dump with mode 0600. It prints the path of the new dump.

| Variable | Default | Meaning |
|---|---|---|
| `ANIMA_BACKUP_DIR` | `/var/backups/anima` | Backup folder. |
| `ANIMA_BACKUP_KEEP_DAYS` | `14` | Dumps older than this are deleted after a good backup. |
| `ANIMA_COMPOSE_FILE` | `deploy/compose.yaml` | Compose file that has the `postgres` service. |

The script writes to a `.partial` file first. It checks the file with `pg_restore --list`, then
renames it. A failed run leaves no `.dump` file and deletes nothing.

## Schedule

Run the backup every day. This example is a systemd timer. Replace `/opt/anima` with the
folder that holds the repository.

`/etc/systemd/system/anima-backup.service`:

```ini
[Unit]
Description=Anima database backup

[Service]
Type=oneshot
ExecStart=/opt/anima/deploy/backup/backup.sh
```

`/etc/systemd/system/anima-backup.timer`:

```ini
[Unit]
Description=Daily Anima database backup

[Timer]
OnCalendar=*-*-* 03:15:00 UTC
Persistent=true

[Install]
WantedBy=timers.target
```

Enable it with `systemctl enable --now anima-backup.timer`. Cron works too:
`15 3 * * * /opt/anima/deploy/backup/backup.sh`.

## Copy dumps off the VPS

A dump on the VPS does not survive the loss of the VPS. Copy each dump every day to a site that
is not the VPS (VPS-006). You choose the target: another machine, or object storage.

A dump holds credential ciphertext and password hashes. Encrypt it with `age` before you copy it:

```sh
age -r <your-public-key> -o anima-<time>.dump.age anima-<time>.dump
```

Keep the `age` private key off the VPS.

## What a dump does not hold

- **`anima_key`.** This key opens the stored credentials. Without it, the restored
  credentials cannot be read. Keep a copy offline. It is not in any dump.
- **The Cloudflare token.** Create a new one if you lose it.
- **The certificate.** The certificate service issues it again.
- **The database passwords.** On a new host, run `make-secrets` and set the role passwords to match.

## Restore

Use this when the database is lost or damaged.

1. Find the dump. If it is encrypted, decrypt it with `age -d`.
2. Stop the hub: `docker compose stop hub`. Make sure `migrate` is not running.
3. Run `deploy/backup/restore.sh --yes /var/backups/anima/anima-<time>.dump`.
   The script refuses to run while `hub` or `migrate` runs. It drops the database, creates it
   again with the same owner and grants, and loads the dump in one transaction.
4. Start the migrations: `docker compose up -d migrate`. Wait until it exits with code 0.
5. Start the hub: `docker compose up -d hub`.
6. Open the client. Check that the devices and the audit log are there.

If `restore.sh` fails, the database may be missing or empty. Fix the cause and run it again.

## Monthly restore drill

Once a month, prove that the newest dump restores. This needs no downtime.

1. Run `deploy/backup/backup.sh`.
2. Run `deploy/backup/restore.sh --check <the new dump>`.

The script restores the dump into `anima_restore_check`. It compares the row count of every
table with the live database. It checks that `anima_runtime` can read and append to
`events.audit_log` and `events.event_log`, and cannot update, delete or truncate them. Then it
drops the check database. It prints table names and counts, never row contents.

It exits with a non-zero code on any difference. Run the drill right after the backup. The hub
may write events between the two steps, and then the counts differ. Run the drill again.

## Operator changes

1. Install the timer: create the two files above, then run `systemctl enable --now anima-backup.timer`.
2. Choose the off-site target and add a daily copy job for `/var/backups/anima`. Encrypt with `age` first.
3. Copy `anima_key` to an offline place (a password manager or a printed copy in a safe).
4. Run the first restore drill: `backup.sh`, then `restore.sh --check` on the new dump.
