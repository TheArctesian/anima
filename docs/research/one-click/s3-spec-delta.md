# S3 spec delta: proposal

Progress: the plan register in [work-plan.md](../../work-plan.md) Section 9 holds the state of this plan (PLN-005).

Version 1.0. Date: 2026-11-14. Status: **Draft: a proposal for open item OI-2 of `docs/one-click-plan.md`. Nothing in
`docs/design-spec.md` or `contracts/` is changed by this file.** The operator approves the text
before it goes into the spec.

Context: plan item S3. The hub in the desktop application uses a PostgreSQL that the
application carries (`crates/anima-database`). The Docker installation does not change, and
no requirement below changes it.

## 1. Requirements that name Docker

Today's text is right for the Docker installation and stays. Add a second sentence to each.

| ID | Today | Add |
| --- | --- | --- |
| DB-001 | Only the hub must connect to the database. Do not publish port 5432 outside the Docker network. | In the application mode the database server must have no network listener. It must listen only on a Unix socket in a folder that only the hub's user can open. |
| DB-002 | The hub must read the database password from a Docker secret or from an environment file. Do not write the password in the Compose file. | In the application mode the hub must read the database URL from a file with mode 0600 in a folder with mode 0700 below the user's own folder. |
| DB-007 | The Docker Compose file must include a backup procedure that uses `pg_dump`. | In the application mode `anima-database backup` makes the same kind of backup. Its schedule is plan item S9. |

DB-003 needs no change: the application mode makes the same roles as `deploy/postgres/roles.sql`.

## 2. Raw data retention in the application mode (decision D-9)

| ID | Today | Proposal |
| --- | --- | --- |
| DB-009 | The hub must keep raw heartbeats and metrics for 30 days. Keep minute aggregates for 7 days, hour aggregates for 90 days and day aggregates for 2 years. | Add: In the application mode the default for raw heartbeats and raw metrics is 3 days. The aggregates do not change. The operator can change the periods (HUB-005). |

Reason: 10 devices with a metrics agent make about 163 MB of raw samples each day, 4.9 GB in
30 days (spike report, Section 10.2). The charts read the minute and hour aggregates, not the
raw samples. With 3 days the same devices need about 0.5 GB.

This is a hub setting: the two rows `heartbeats` and `metrics` of `retention.periods`, which
accept 1 to 400 days. S3 does not set it. The work that makes the first run of the application
hub (plan item S6) sets it, after this delta is approved. The default of the Docker
installation stays 30 days.

To check before approval: does any function read raw samples or raw heartbeats that are older
than 3 days? The spike found none in the chart queries; it did not read every query.

## 3. New requirements (prefix `DBL`, "database, local")

The prefix is free on `main` at `604fec8d`. Each one is built and tested in
`crates/anima-database`, except where the last column says "S4".

| ID | Requirement | Where |
| --- | --- | --- |
| DBL-001 | The application must carry its PostgreSQL. The operator must not have to install a database, and no step must need administrator rights. | S3 |
| DBL-002 | The PostgreSQL tree must be built from the official source, pinned by version and SHA-256. It must link only system libraries. On macOS it must be universal and must run on the oldest macOS that the application supports. | S3 (`database-tree/`) |
| DBL-003 | One copy at most must run a database. A second copy must end with a clear message and must change nothing. | S3 |
| DBL-004 | The database superuser must have no password. Only the operating-system user who runs the database may sign in as the superuser. | S3 |
| DBL-005 | A password of a database login must never be in a statement that is sent to the server. The server must be sent the SCRAM verifier. (Adds to LOG-007.) | S3 |
| DBL-006 | The server log must hold no statement and no error message of a session. | S3 |
| DBL-007 | After a kill of the application, of the program or of the server, the next start must bring the database up with every change that was reported as saved. It must tell that it recovered. | S3 |
| DBL-008 | On macOS the database must ask the drive to empty its cache at each commit. | S3 (decision D-4) |
| DBL-009 | A database of another PostgreSQL major version must not be started and must not be changed. The message must name both versions and what to install. | S3 (decision D-6) |
| DBL-010 | The release that changes the PostgreSQL major version must carry the tree of the version before, must make a backup, must upgrade the database and must check it before the old one is removed. | Later: the release that brings PostgreSQL 18 |
| DBL-011 | The database folder must be excluded from Time Machine. The backups must not be. | S3 (decision D-8) |
| DBL-012 | The hub service must start the database before the hub and stop it after the hub. It must give the database 30 seconds or more to stop. | S4 |
| DBL-013 | The database tree must run from a folder that an application update does not replace while the server runs. | S4 |

## 4. K2 (`contracts/database.md`): one added rule

Proposed text for the "Rules" list. It does not change a table, a role or a range.

> A migration may use an extension only when it is in the official `postgres` Docker image and
> in the application's database tree (`apps/desktop/src-tauri/database-tree/build.sh`). Today
> no migration uses an extension. The tree carries `plpgsql`, `amcheck` and
> `pg_stat_statements`. A third-party extension needs a programme ruling: it needs a custom
> image for the VPS and a second build for each platform of the application.

Reason: with two ways to run the database, an extension that only one of them has would make
a migration fail on the other.

## 5. Not proposed

- No change to K7: the hub reads `ANIMA_DATABASE_URL_FILE` as today.
- No change to the hub's roles, schemas or migration ranges.
- No requirement for a TCP listener (decision D-5: a home folder path that is too long for
  the socket is refused with a message).
