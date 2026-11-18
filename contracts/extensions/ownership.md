# K-E8 — Migration ranges and owned paths for the engine and extensions

Status: new (operator, 2026-11-13; ruling number to be assigned). Proposed for the path to Projects; it is not part of the Phase 3 freeze. Change it only through a programme ruling (CON-001). The same rows are in K2 (`contracts/database.md`). Requirement IDs: REPO-E-001, REPO-E-002, ENG-002, EXT-005 of `docs/plans/extensions-plan.md`.

## 1. Ranges

| Range | Domain | Schema | Tables |
| --- | --- | --- | --- |
| 0221 | credentials (engine addition) | `credentials` | `source_secrets` |
| 2000–2019 | engine/sources | `sources` | `accounts`, `sync_runs` |
| 2020–2029 | engine/extensions | `extensions` | `switches` |
| 2030–2039 (operator, 2026-11-16; ruling number to be assigned; K-E4) | engine/links | `links` | `record_link` |
| 2040–2049 (R-148; K-E4) | engine/places | `places` | `place`, `site_place` |
| 2050–2059 (R-148; K-E4) | engine/files | `files` | `piece`, `archive`, `attachment`, `proposal` |
| 2060–2069 (R-148; K-E4) | engine/locations | `locations` | `sample`, `ingest_token`, `transfer_context` |
| 2070–2099 | reserved for the other engine domains (K-E5 and K-E6 take their ranges with E0-6 and E0-7) | — | not assigned now |
| 2100–3599 | the extensions, as Section 5.2 of `docs/plans/extensions-plan.md` (Projects 2200–2299) | — | — |

## 2. Rules

- A foreign key points only into a lower range: `projects -> sources -> credentials`.
- The K2 rule "the string `"<schema>.` may appear only under the domain folder" reads, after the hub regroup, as: only under `crates/hub/src/<group>/<domain>/` (`fleet/`, `engine/`, `work/`).
- Hub paths: the engine is `crates/hub/src/engine/` (`sources`, `extensions`; from E0-4 also `timeline`, `links` and the leaf `records.rs`). The timeline has no schema and no range (TML-001).
- Projects is `crates/hub/src/work/projects/`. The credentials addition stays in `crates/hub/src/fleet/credentials/` (file `source.rs`).

## 3. Owned paths

- **Engine:** `crates/hub/src/engine/**`, `crates/providers/exchange/**`, `$lib/extensions/**`, `$lib/sources/**`.
- **XP-02 (Projects):** `crates/hub/src/work/projects/**`, `crates/providers/github/**`, `crates/anima-cli/src/projects*`, `apps/client/src/routes/projects/**`, `$lib/projects/**`.
