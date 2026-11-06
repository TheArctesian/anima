# WP-19 report (package orchestrator)

Status: **done**. Branch `claude/wp-19-check-repair` from `phase-1` (`0f7bd83`). Not pushed, not merged.
`env -u DATABASE_URL BASE_REF=0f7bd83 bun run gates` on the code tip `44f6b72`: **12/12 pass**, exit 0 (this commit adds only this file). Plan, decisions D-1..D-8 and the review log: [WP-19.md](WP-19.md).

## What is delivered

| Output | Path | Spec |
| --- | --- | --- |
| Marker columns | `crates/hub/migrations/0302_checks_method_repair.sql`:<br>- `checks.checks.operator_edited boolean not null default false`<br>- `repair_tried_at timestamptz null`<br>- backfill: a check with an `update` audit entry (target `check`) is marked edited, only when `events.audit_log` exists | CHK-013, DB-011, D-1, D-2 |
| Edited flag | `store/writes.rs` `update` sets `operator_edited = operator_edited OR $13`. The flag is set by every `updateCheck` on the check itself. It is not set by regrouping from a group's create or update (`CheckChange.regrouping`), by pause or resume, or by the scheduler. | D-1 |
| Pure rule | `checks/scheduler/repair/rule.rs`:<br>- candidate: ICMP, not inverted, Down, `failures > retries`, never answered, not edited, not tried, not paused, not in maintenance<br>- the device's ports, SSH first (`Protocol`, ready for RDP and VNC)<br>- the repaired name<br>- the R-064 texts<br>- the audit change | CHK-013, R-064, D-3, D-5 |
| I/O edge | `checks/scheduler/repair.rs`, called from `attempt::run` after a Down beat of an ICMP check, inside the attempt task and its permit:<br>- reads the facts<br>- `probe::probe` with TCP, a 5 s timeout, the device host and the scheduler's `AddressPolicy`<br>- one transaction: a locked re-check, then the switch with the Info event and audit entry, or `repair_tried_at` with one Warning | R-032, LOG-005, LOG-007, D-7, D-8 |
| SQL | `checks/store/repairs.rs`:<br>- facts, with history only behind a `CASE` on cheap columns: status, uptime minute/hour/day, heartbeats<br>- the locked re-check on type, config, name, edited, tried, paused and status<br>- switch, mark tried | D-3, D-8 |

**Tests:**
- 28 unit or edge tests under `checks/scheduler/repair/tests/`: rule, ticks, guards, races
- 5 integration tests in `crates/hub/tests/it/checks/api/writes.rs`: the marker, regrouping and the backfill (which runs the real `do $$` block)

**Accepted behaviour:**
- Inverted ICMP checks are never repaired.
- A device with SSH off gives one Warning and is marked tried.
- When "SSH port <n>" and "SSH port <n> (was <old>)" are both taken, " 2", " 3" and so on are added.

## Tasks and verdicts

| Task | Builder | Rounds | Reviewer | reviewer-security |
| --- | --- | --- | --- | --- |
| T1 marker (`203e628`, `4606539`) | builder (sonnet) | 2 | Round 1: changes (must: regroup marked children). Round 2: **accept** (4/4 mutations killed). | n/a (covered in T2's security review of 0302) |
| T2 repair (`0557633`, `d56520f`) | builder-heavy (opus) | 2 | Round 1: changes (S1 untested lock clauses, S2 edge tests, S3 history cost). Round 2: **accept** (every lock clause and history source killed). | Round 1: **accept**. Proved loopback refused and deny list wins: 0 connects, 1 Warning, no second probe. |
| Final package review (`0f7bd83..252475c`) | — | 1 | **accept**. The should finding (no test that a repair leaves `operator_edited` false) was fixed in `44f6b72`, and the mutation is now killed. | — |

## Open findings (all could, accepted)

- The device row is read before the connect and not re-checked in the transaction. A host, port or maintenance change during the 5 s connect still lets the repair commit. The next TCP beat probes the current host and port.
- The audit actor is named "Anima" (nil operator id, client 127.0.0.1). An operator may also be named "Anima", and the API shows only the name. `change.repairedBy = "hub"` and the client address tell them apart. Reserving the name is left for later.
- History older than its retention is not seen (day buckets 2 years). Edits older than the audit log's 1 year are not backfilled.
- If the write keeps failing, the repair probes again once per Down beat. This is bounded by the check's interval and logged.
- The S3 cost guards are not pinned by a test, because they change only cost.

## Contract and migration text

- `contracts/database.md`, `checks.checks` entry: adds `operator_edited bool default false, repair_tried_at timestamptz null (CHK-013)`.
- New migration `crates/hub/migrations/0302_checks_method_repair.sql`. It has a lower number than the applied 0601. sqlx 0.8.6 applies unapplied lower versions, and 0006, 0401, 0501 and 0601 are precedent.
- K1 (`contracts/schema.graphql`) is unchanged. The new config is visible through `Check.config`.

## Files outside `crates/hub/src/checks/**`

- `crates/hub/migrations/0302_checks_method_repair.sql` (new)
- `contracts/database.md` (2 lines)
- `crates/hub/tests/it/checks/api/writes.rs` (T1 tests)
- `docs/programme/packages/WP-19.md`, `WP-19-report.md`

No shared file (`schema.rs`, `main.rs`, `lib.rs`, `Cargo.toml`) is touched, and nothing under WP-18's paths. Inside checks, the only shared-module edits are one `mod` line each in `scheduler.rs` and `store.rs`, and the `CheckChange` field and doc in `model.rs`.

## After deploy

The operator's Mac check (ICMP, Down since creation, never Up, SSH on 22) repairs on its first Down beat after the hub restarts:
1. Migration 0302 leaves it not edited, unless an `updateCheck` audit entry exists for it.
2. Its facts show it never answered.
3. The TCP connect to port 22 answers.
4. It becomes "SSH port", Unknown, due at once, then Up. One Info event and one audit entry are written.

To confirm, look for that event and audit entry. If it is still ICMP, check the audit log for an `update` entry on that check.

## Handoffs

None. The client needs no change.
