# Restore a hub on a new server, or move it

| Item | Value |
| --- | --- |
| Document | The procedure to restore the daily backup of a hub on a new server and continue (FND-006). The same steps move a hub to a different server |
| Version | 1.0 |
| Date | 2026-11-17 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | Written and proved in part (R-140, task F1). The database steps were done on a new, empty setup in Docker on 2026-11-17; see "Proof". The steps on a real second server are not checked: HT-023 |
| Kind | Procedure. It adds no requirement (PLN-010) |

This procedure is for a person who runs the hub on a server with `deploy/compose.yaml`.

- It is not for a hosted hub. The pool makes and restores the backup of a hosted hub (HMD-014).
- It is not for a hub on your own computer. `anima-database` writes, checks and restores that backup.
- It moves no data into a hosted hub (R-139).

The owner of each step is the document that the step names. This document gives the order and the
facts that those documents do not give. If a step here and its owner disagree, the owner is correct.

## What you must have before the old server is lost

The dump holds the database only. Keep these at a place that is not the server.

| Item | Where it is on the server | If you lose it |
| --- | --- | --- |
| The dump `anima-<time>.dump` and its file `.counts` | `/var/backups/anima`, and the `restic` repository on the home PC ([deploy/backup/README.md](../../deploy/backup/README.md), "Off-site copy") | The data of the hub is lost |
| The two `restic` secret files | `deploy/secrets/restic_repository`, `deploy/secrets/restic_password` | The copies on the home PC cannot be read |
| The key `anima_key` | `deploy/secrets/anima_key` | The restored credentials cannot be read. Each credential must be typed again |
| The file `.env` | `deploy/.env` | Write it again from the table in [deploy/README.md](../../deploy/README.md), "First start", step 2 |
| The updater key and its password, if this hub publishes releases | `deploy/secrets/updater/` | See [releases.md](releases.md): each installed application must be installed again by hand |
| The R2 keys, if this hub publishes releases | `deploy/secrets/r2_access_key_id`, `deploy/secrets/r2_secret_access_key` | Make new keys in Cloudflare |
| The Headscale backup | The `restic` repository of Headscale ([deploy/headscale/backup/README.md](../../deploy/headscale/backup/README.md)) | Each machine must join the VPN again |

You do not need: the database passwords, the Cloudflare DNS token and the certificate. A new server makes
new passwords, you make a new token, and the `certs` container orders a new certificate.

## Steps

Do each step as root on the new server, in `/opt/anima/deploy`, unless the step says a different place.

1. **Prepare the server.** Do the prerequisites of [deploy/README.md](../../deploy/README.md), and put the
   repository at `/opt/anima` at the commit that the old server had, or a later one. Never an earlier
   one: the hub cannot use a database that a later hub migrated.
2. **Restore the VPN first, if the server of Headscale is also lost.** Follow "Restore" of
   [deploy/headscale/backup/README.md](../../deploy/headscale/backup/README.md), steps 1 to 5. The new
   server then has a VPN address. If only the hub moves and Headscale stays, join the new server to
   the VPN with a new `tag:hub` key and delete the old `hub` node (step 4 of that document).
3. **Put the key in place before you make the other secrets.**

   ```sh
   install -d -m 0700 secrets
   install -m 0600 <your copy of anima_key> secrets/anima_key
   ```

   Then make the other secret files as "First start", step 1, of
   [deploy/README.md](../../deploy/README.md). `bin/make-secrets` makes only the files that are missing,
   so it keeps your key and makes new database passwords. If you run it before you copy the key, it
   makes a new key, and the restored credentials cannot be read: delete that new `secrets/anima_key`
   and copy yours.
4. **Put the other files back:** `.env`, with `ANIMA_HUB_ADDRESS` and `ANIMA_VPS_PUBLIC_ADDRESS` of the
   new server; `secrets/restic_repository` and `secrets/restic_password`; and `secrets/updater/` and the
   R2 keys if this hub publishes releases.
5. **Check the server:** the three `bin/preflight` commands of "First start", step 3.
6. **Start only the database.**

   ```sh
   docker compose up -d postgres
   ```

   Wait until `docker compose ps` shows `postgres` as healthy. The first start makes the roles and an
   empty database `anima`, with the new passwords.
7. **Get the dump.** Restore the newest snapshot from the home PC with `restic restore` into a folder
   that is not `/var/backups/anima` ([deploy/backup/README.md](../../deploy/backup/README.md), "Restore",
   step 2). Keep the file `.counts` beside the dump.
8. **Restore.**

   ```sh
   backup/restore.sh --yes <dump>
   ```

   It prints "Restored ... into anima." The empty database of step 6 stays as `anima_previous_<time>`.
9. **Migrate.**

   ```sh
   docker compose up migrate
   ```

   It must end with "exited with code 0". On a later commit it adds what that commit needs.
10. **Compare the restored database with the backup.**

    ```sh
    backup/restore-check.sh <dump>
    ```

    Expected: "Restore check passed: N tables match the manifest."
11. **Start the hub:** `docker compose up -d`. The first certificate can take some minutes
    ("Order of start" in [deploy/README.md](../../deploy/README.md)).
12. **Follow the addresses.** If the VPN address of the hub changed, change the `hub` DNS record and the
    allow list of the home PC (step 5 of the Headscale restore).
13. **Check in the client.** Open the client and sign in as before. Expected: the devices and the audit
    log are there, and a session to one device with a stored credential opens. If the session says
    that the credential cannot be read, the key of step 3 is not the key of the old server.
14. **Start the backups of the new server:** "Schedule" and "Operator changes" of
    [deploy/backup/README.md](../../deploy/backup/README.md). Then drop the empty database with the
    command that `restore.sh` printed.

To move a hub that still runs: on the old server, run `docker compose stop hub`, then `backup/backup.sh`,
and use that dump in step 7. Do not start the old hub again after the new hub runs.

## What is lost

Each change after the dump: at most one day with the daily timer. Metrics and events of that time do
not come back. A daemon keeps at most 1 hour of points and sends them when the hub answers again (K8
Section 7).

## Proof

On 2026-11-17 an agent did steps 3 and 6 to 10 on this repository at the commit of R-140, with Docker
29.9.3 on a Mac, from two copies of the repository (an "old" and a "new" server):

| Fact | Result |
| --- | --- |
| The old setup: `make-secrets`, `postgres`, `migrate`, one site, two devices, one tag, then `backup.sh` | One dump and its counts file |
| The old setup was then removed with its volume | The new setup had an empty volume |
| The new setup: the old `anima_key` copied first, then `make-secrets` | The key was the same file; the owner password was different |
| `restore.sh --yes` on the new setup | "Restored ... into anima." |
| `migrate` | Exited with code 0 |
| `restore-check.sh` | "Restore check passed: 65 tables match the manifest." |
| The two devices by name, and the rights of `anima_runtime` | Present; it can read `devices.devices` and cannot delete from `events.audit_log` |

Not proved there: the steps for the VPN, the certificate, the start of the hub and a session with a
restored credential (steps 2, 5 and 11 to 13). They need a real server: HT-023 of
[need-human-testing.md](../need-human-testing.md).
