# WP-40 report — Phase 3 skeleton and contract gate

Status: **done** (all six tasks accepted and merged into `phase-3`). Package branch `claude/wp-40-skeleton`; plan,
decisions and review log in [WP-40.md](WP-40.md). Started on the Mac, finished on `pop-os` after the handoff.

## Accepted tasks

| Task | SHA | Merge on `phase-3` | Gates | Rounds |
| --- | --- | --- | --- | --- |
| T1 contracts K1–K7 (R-081, P3-002) | ada5f5ab | e76ef128 | passed on the Mac | 1 |
| T1b exhaustive check config (P3-002, D-29) | 3186779f | 2a36751c | passed on the Mac | 1 |
| T2 ids and K7 config (K7, D-2, D-3, LOG-007) | aa9c7983 | cba65915 | 12/12 on `pop-os` | 3 (third: short review of the could fixes) |
| T3 module map (K3, REPO-004) | be6274b7 | 9293b4b8 | passed on the Mac | 1 |
| T4 K6 fixtures (K6, CON-003, SEC-005) | b39419e5 | 83747e5f | 12/12 on `pop-os` | 3 (third granted under R-066, scope M1/S1/S2) |
| T5 spec changes (R-081) | f810fd92 | f3d52e32 | passed on the Mac | 2 |

Which gate set each Mac task ran (12, or 4–12 under R-068) is in the Mac session's records, not in this
branch. T2 and T4 were rebased onto `phase-3` 784355b1 before their gate runs. Their 12/12 runs used a probe commit with
the Linux clippy line that is now on `phase-3` as 2150d394 (D-WP40-9). T2's first full run hit the `ra_031`
woken-opening flake under load 30; it passed alone, then 12/12 on rerun.

## What later packages get

- **K1–K7 frozen in `contracts/`** (T1), with `@pending` on every K1 addition; `CheckType.DOMAIN_EXPIRY` compiles
  in the client but stays out of the type picker until WP-50 (D-WP40-2).
- **`crates/hub/src/ids.rs`:** `ApiKeyId`, `ChannelId`, `DeliveryId`, `ShareId`, `RecordingId`.
- **`crates/hub/src/config.rs`:** `public_origin` (one canonical browser spelling, D-3), `web_root` and
  `recording_root` (absolute, not `/`, no `..` part, D-2), `recording_max_bytes`. Nothing reads them yet; WP-43
  `main.rs` and WP-46 are the first users. Origin errors never echo userinfo (LOG-007).
- **`quality/hub-modules.json`:** domains `notifications` and `retention` with their K3 edges. `ssh` and `trust`
  come from WP-41's handoff commits.
- **`contracts/fixtures/phase3.json`** (K6 segments 17–24, synthetic, no secret values) and the typed loader
  `apps/client/src/lib/fixtures/phase3.ts` for tests and stories. Its enum lists and slice keys are tested
  against K1. The Phase 3 client packages split the slices into their domain folders (D-WP40-6).
- **Spec:** the R-081 decisions are in `docs/design-spec.md` (T5).

## Handoffs

None outstanding. No WP-40 task edited `schema.rs`, `main.rs`, `lib.rs` or `upkeep.rs` (PKG-001).

- For the programme orchestrator: on Linux, clippy compiles the `cfg(not(target_os = "macos"))` code that the
  Mac never compiled. The WebKitGTK doc line (2150d394) was the only lint found there.

## Should items for issue #1

None open. The T2 round-1 shoulds (IPv4-mapped IPv6; non-browser host forms; userinfo echoed in the error;
relative and `/` roots) were fixed in round 2. The T1 shoulds were fixed in T1b. The T5 shoulds were fixed in
round 2. T4's S3 (`lib/fixtures/` is not a domain folder) is answered by D-WP40-6.

## Could items for issue #1

- **T2, `crates/hub/src/config.rs:130-131` (Predictable):** the `optional_root` doc comment reads as if a `..`
  path would serve the whole file system. Suggested: "The root `/` and any path with a `..` part are refused:
  `/` would serve the whole file system, and `..` could climb out of the folder."
- **T2, `crates/hub/src/config/roots_tests.rs:31-32` (Predictable, test strength):** the positive case checks
  `web_root.or(recording_root).is_some()`. It should assert that the named field equals `/srv/web`.
- **T2, `crates/hub/src/config/roots_tests.rs` (Idiomatic):** it is the only `*_tests.rs` file in the repository.
  Its neighbours use a `tests/` folder (`config/origin/tests.rs`, `checks/tests/probe/`).
- **T4, `quality/tests/fixtures.test.ts:~222` (Idiomatic):** a stray blank line before `});`, left by the test
  split.
- **Already in `phase-3-coulds.md`:** T5 (NET-004 hub address in the nav bar; UI-005 holds several rules) and T1b
  (unexport `BareProbeConfig`; comment the exhaustiveness `default`).
- **Round-1 coulds without recorded text:** T1 (3), T2 (5 from reviewer, 3 from reviewer-security), T3 (3) and T4
  (3). The Mac session logged only their counts; their text is not in any commit, package file or
  `phase-3-coulds.md`. T2's round 2 (499d77ce) fixed the cheap ones, and its round 3 fixed the `..` roots and
  scheme-less userinfo. The rest are lost with the Mac session's transcripts.

## Open risks

- Timing flakes under load on `pop-os` as on the Mac: `ra_031` failed once in a full gate run and passed alone.
  It is already on the issue #1 should list.
- The K6 loader reads `contracts/fixtures/phase3.json` from outside the client root. No route may import it until
  a client package moves the slices into its domain folder (Vite `fs.allow` in a build has not been shown).
