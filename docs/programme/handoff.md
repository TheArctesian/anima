# Programme handoff — 2026-11-08

The programme orchestrator stopped at the operator's usage limit. This file is the state at the stop.
Read it first, then the rulings listed in section 5 and R-093 to R-101 in `programme.md`.

## 1. Where things are

- **Integration branch:** `phase-3`, pushed, at the commit that adds this handoff. `main` is fast-forwarded
  to it as a save (operator, 2026-11-08). Gates: `CARGO_TARGET_DIR=$PWD/target CARGO_INCREMENTAL=0 env -u
  DATABASE_URL BASE_REF=04ab9b61 bun run gates` (R-101; 12 gates). Client-only changes run gates 4–12 (R-068).
  PATH needs nvm Node 22 and `/usr/lib/postgresql/17/bin`.
- **Last fully gated commit:** cabe1c94 (12/12 at 595c624c, then docs only).
- **441b8384** (WP-50 T13 merge, client only) is on `phase-3`, but its own gates 4–12 run was stopped
  at the hand-off. WP-45 T3, WP-46 T4 and WP-51 T3 each passed 12/12 on top of 441b8384, so it is
  covered. Run gates 4–12 once to confirm before the next merge.
- **R-101:** every gate and review run uses its own `CARGO_TARGET_DIR` and `CARGO_INCREMENTAL=0`; delete
  the target folder when the task is accepted. The shared `~/.cache/cargo-target` was deleted (operator OK).
  A full disk shows up as sqlx test-database failures or a linker SIGBUS, not as code failures.
- **Stop rule used:** nothing half-merged. All seven package orchestrators were running when the session
  stopped; they will stop with it. Resume each with SendMessage if the ids still exist, otherwise start a
  new package-orchestrator per package and point it at its package file.

## 2. Finished in Phase 3

| Package | State |
| --- | --- |
| WP-40, WP-41, WP-42, WP-52 | Done (see their package files and reports). |
| WP-44 web client | Complete except its rebase onto WP-43's last tasks; parked. |
| WP-46 checks | T1–T3 merged; **T4 accepted, queued** (below). Then the final package review. |
| WP-43 access | T1 web client and T2 browser sign-in merged with their `main.rs` lines (8319b999, 5d4860c2). |
| WP-45 notifications | T1 egress, T2 store, T5 decide merged (1ea7bb1a). |
| WP-54 agent | T2 points merged (595c624c; K8 `contracts/agent-push.md` frozen). |
| WP-50 settings | T1–T7, T10 merged; T13 merged as 441b8384. |
| WP-51 parsers | T1, T2 merged. |

## 3. Merge queue and work in progress

**Accepted, ready to merge, in this order** (each merge: all 12 gates if Rust changed, else 4–12):

| # | Task | Branch | Tip | Gated on | Notes |
| --- | --- | --- | --- | --- | --- |
| 1 | WP-50 T8 channels | `wp/50-settings/channels` | 40684cb6 | 4–12 | Client only; on T13. |
| 2 | WP-50 T9 check-channels | `wp/50-settings/check-channels` | a8ce67a6 | 4–12 | On T8. Its handoff touches `routes/devices/[id]/checks/` (ChecksPanel keeps the editor open). |
| 3 | WP-50 T12 hub-row | `wp/50-settings/hub-row` | 1598fbda | 4–12 | On T8. |
| 4 | WP-47 T1a devices | `wp/47-sharing/devices` | 939b7163 | 12/12 R-101 | Clears 6 K1 `@pending` (sessionLimit, recordSessions). WP-48 and WP-50 T11 wait for it. |
| 5 | WP-50 T11 device-policy | `wp/50-settings/device-policy` | 03d41258 | 4–12 | After #4. |
| 6 | WP-45 T3 channels | `wp/45-notifications/channels` | 315ed5ff | 12/12 R-101 on 441b8384 | Handoff: schema.rs notifications roots + loader; 25 K1 members un-pended. |
| 7 | WP-45 T4 templates | `wp/45-notifications/templates` | a6c05227 | 12/12 R-101 | On T3; merge right after #6. |
| 8 | WP-46 T4 domain expiry | `wp/46-checks/domain-expiry` | 1063ee20 | 12/12 R-101 on 441b8384 | 5 K1 members un-pended; no wiring lines. |
| 9 | WP-51 T3 gpu-discovery | `wp/51-parsers/gpu-discovery` | 571064f5 | 12/12 R-101 on 441b8384 | Merge commit only (intermediate commits do not compile). |

**In progress (not accepted yet):**

| Package | State | Next |
| --- | --- | --- |
| WP-43 | T3 api-keys (f4eee5ed) has the R-100 deny-list round; T5 rotation (f3ab2eb3); T4 bearer (`wp/43-access/bearer` 22b5ff16); T6 throttle (`wp/43-access/throttle` d8f880c4, CR-16 b). | Reviews and gates per package file `docs/programme/packages/WP-43.md` (branch `claude/wp-43-access`). T4's handoff must move router assembly out of `serve()` in `main.rs` (60/60 lines). |
| WP-45 | T6 dispatcher built (4 commits on a6c05227), going to gates and both reviewers. | T7: the checks/ handoff edits (with WP-46), `Check.notifications`. |
| WP-47 | T3 shares (`wp/47-sharing/shares` 1daa8922): round-2 fixes in, security review round 2 running, then the Sonnet review and 12 gates. T1b join-record (`wp/47-sharing/join-record` 728bbcc8, on 441b8384): round 1 accepted, 12/12; two should fixes, then round 2. The `guacamole/guacd:1.6.0` image is still pulled; WP-47 removes it after T1b round 2. Package log: `docs/programme/packages/WP-47.md` on `claude/wp-47-sharing` d001b86c. | Resume agent a91042fdb55f93636. T2 after T1a merges; then T4 (needs CR-16 b from WP-43 T6), T5, T6. WP-47 will raise Q-5 before T6: pass or drop guacd's `msg` join notices to the owner (K4 §4 is silent). |
| WP-50 | Above, plus R-094 rows handoff after WP-46 T4. | Final package review and interface pass. |
| WP-51 | T3 queued. | T4 read side (MET-008, MET-009) after T3 merges; owns `store/chart.rs` mean overflow. |
| WP-54 | T3a built and gated (c7874fde), review next; `wp/54-agent/token` 4fa2a6f8, `wp/54-agent/binary` a38195d0. | Fix `SeriesKey::of` (drops the GPU index) and cap pushed GPU temperature/power (WP-51 note). |

**Not started:** WP-48 (after WP-47 T1a), WP-49, WP-53. At most 7 package orchestrators (R-066).

**Programme to-do:** add the bounds to K1 doc strings `CheckNotificationsInput.resendEveryBeats`
(0 to 1000) and `NotificationSettingsInput.certificateDays` (1 to 365, 1 to 5 values, kept largest first).

## 4. Waiting on the operator

- **Live checks O-4 to O-9** (`docs/programme/phase-2-acceptance.md`):
  - O-4 SFTP
  - O-5 Wake-on-LAN
  - O-6 web view on macOS
  - O-7 OD-7 on Linux
  - O-8 SSH metrics
  - O-9 Docker, SNMP, Tailscale ping and systemd checks

  O-3 passed on both the Mac and `pop-os` (recorded on `main`). O-1 and O-2 wait for the final
  test on a second Mac (R-076).
- **Issue #1:** posting is approved (2026-11-08). `phase-2-coulds.md`, `phase-3-coulds.md` and a
  Phase 3 package list (WP-43 to WP-54, up to 595c624c) are posted. Post later items as new comments.
- **Pushing:** approved (2026-11-08). `phase-3` is pushed, and `main` is fast-forwarded to it as a
  save. Push only fast-forwards; ask before anything else on `main`.
- **guacd image:** WP-47 may pull `guacamole/guacd:1.6.0` for its join-handshake record (127.0.0.1
  only, container removed afterwards). The shared `~/.cache/cargo-target` was deleted with the
  operator's OK (R-101).
- **Spec 5.7.3 wording:** still open.
- **Signing key:** `TAURI_SIGNING_PRIVATE_KEY` for the updater signature. The desktop build exits
  1 without it.
- **R2 setup:** releases go to Cloudflare R2 (`releases.anima.ing`). No CI does this
  (`anima-no-ci-gates`).
- **Phase 3 VPS changes:** each needs an OK in chat and a row in `operator-changes.md` (VPS-005).
  They include the WP-52 Apprise firewall rules and, later, the R-084 agent VPN policy.

## 5. Read first

**Rulings** (`docs/programme/programme.md`):
- R-066 (7 orchestrators)
- R-067 (merge each accepted task at once)
- R-068 (client-only gates)
- R-076 (no Windows; RDP checks on a second Mac)
- R-079 (one-button screen sharing)
- R-081 (Phase 3 adopted)
- R-082 and R-083 (handoff commits)
- CR-5, CR-6 and CR-7
- R-084 and CR-8 (metrics push agent)

Also read `docs/programme/phase-3-plan.md`, `phase-3-contracts.md` and the package files in
`docs/programme/packages/`.

**Memory notes** for the new account to recreate from this list (they live in the Mac account's
`~/.claude/projects/.../memory/`):
- Deferred work goes on GitHub issue #1.
- No CI for gates or releases. Gates and builds run locally; releases go to R2.
- Autonomous programme: merge into the integration branch and start packages without asking.
  Ask before pushing, before touching main, VPS, Cloudflare or secrets, and before destructive
  Git.
- Merge each accepted task at once (R-067).
- No Windows anywhere. Do not plan, justify or mention Windows support.
- Phase 3 runs on branch `phase-3`, BASE_REF 04ab9b61.
- Metrics come from the push agent (R-084); SSH is the fallback.

**Working rules learned this phase:**
- Never kill processes by pattern (`pkill -f`, `killall`); other gate runs share the machine.
- Never use bare `git stash`.
- Wait for a gate run to finish before merging.
- Timing tests flake under heavy load and pass alone: `ra_031` waking (terminal and tunnel), the `api_005` statement
  counts, `log_007` terminal refusals, the deploy localapi "endless upstream body" test (on `pop-os` it once spun for
  45 minutes; stop that one test process by its PID, then run the deploy suite alone), and (seen on `pop-os`)
  `k4_8_5` client ping while the hub's send waits, the desktop `process_log` `log_008` opener-after-patience test and
  the `checks::tests::snmp::probes::d_5_*` probes, and the deploy `secrets-token` DNS-003 interrupt test (it waits
  only 5 s for the `stty` shim; run `bun run --cwd deploy test` alone).
  These are logged as a should for issue #1.

## 6. Open decisions for the next session

- K1 doc-string bounds for `resendEveryBeats` and `certificateDays` (section 3, programme to-do).
- D-23 / R-100: should the API-key deny list also refuse `shareSession`? (WP-47's note for WP-43 T3.)
- COULD-3: three copies of `random_token` (sessions/start.rs, sessions/files/opening.rs, sessions/share/opening.rs); needs an owner.
- `crates/hub/src/main.rs` `serve()` is at the 60-line limit: WP-43 T4 (or the next wiring change)
  moves router assembly into its own function (at most 4 params).
- Background agents still running at the stop (WP-43 T4 security review, WP-45 T6 reviews, WP-47 T1b
  and T3 review, WP-54 T3a and T4 builds) may have pushed commits to their branches after this file:
  check each branch tip against section 3 before merging.
- Compute plan (branch `claude/distributed-compute-devices-25e110`): rename its crates to
  `agent` and `agent-core`, and keep "the agent opens no port" (R-084, delta §5).

## 7. Worktrees that can be cleaned up (not deleted)

- **Merged into `phase-3`:**
  - wp-40-check-form, wp-40-contracts, wp-40-modules, wp-40-spec
  - wp-41-api-errors, wp-41-trail
  - wp-42 and all of wp-42-*
  - wp-52-compose, wp-52-egress, wp-52-web-image
  - wp-52-docs (contained in apprise-firewall)
- **Phase 1 and 2:**
  - `.claude/worktrees/phase-2` (merged into main)
  - the `agent-*` worktrees
  - the scratchpad worktrees `wt` and `wt35`
- **Keep:**
  - `phase-3`
  - every branch in section 3, until it merges or the Linux machine has it from the push
