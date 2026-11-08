# Programme handoff — 2026-11-07

The programme moves from the Mac session to a new Claude account run from the Linux machine
(`pop-os`). This file is the state at the stop. Read it first, then the rulings listed in
section 5.

## 1. Where things are

- **Integration branch:** `phase-3`. It was cut from `phase-2` at 04ab9b61 (R-081) and has
  `origin/main` merged in. Gates: `env -u DATABASE_URL BASE_REF=04ab9b61 bun run gates`
  (12 gates). Client-only changes can run gates 4–12 (R-068).
- **Phase 1 and 2:** finished and merged into `main` (PR #3, PR #51). The hub at fd8cc03c is
  live on the VPS.
- **Phase 3:** started 2026-11-07 (R-081). The plan is `docs/programme/phase-3-plan.md`, with
  packages WP-40 to WP-54. R-084 (the metrics push agent) was approved and added to the spec
  the same day.
- **Stop rule used:** every accepted task is merged into `phase-3`. No builder, reviewer or fix
  step is half-done. Each task below that is not merged is built and committed, and waits for
  its review.
- **Not touched:** `wp-36-builds` (WP-36 Builds runs on the Linux machine).

## 2. Finished

| Package | State |
| --- | --- |
| WP-42 client groundwork | **Done.** All 7 tasks are merged (last merge a329eebf). Report: `docs/programme/packages/WP-42-report.md`. |
| WP-40 skeleton | T1 (K1–K7 contract freeze), T1b, T3 (module map) and T5 (spec changes) are merged. |
| WP-41 hub groundwork | T1 `api_errors` and T2 `events::Trail` are merged (065989af). |
| WP-52 deploy | T1 web image, T2 Apprise and T3 egress check are merged. |
| R-084 / CR-8 | The spec has MET-011 to MET-020, the agent crates in §13 and the changed INT-003 (845b9660). WP-51 is narrowed, and WP-54 is new. |

## 3. In progress (built, committed, not merged)

Each branch lives in a worktree under `.claude/worktrees/`, and all of them are pushed with this
handoff.

| Package / task | Branch | Last commit | State | Next step |
| --- | --- | --- | --- | --- |
| WP-40 T2 ids/config | `wp/40-skeleton/ids-config` | d77d8667 | Could fixes: `..` refused in `ANIMA_WEB_ROOT` and `ANIMA_RECORDING_ROOT`; userinfo hidden without `://`. Mutation-proved. fmt, clippy and quality pass on d77d8667; cargo-test through build passed on 3dd6c01f. | Short review of 3dd6c01f..d77d8667, then the 12 gates, then merge. WP-43 main.rs and WP-46 need its `public_origin` and `web_root`. |
| WP-40 T4 fixtures | `wp/40-skeleton/fixtures` | 737fad78 | Round-2 fixes (M1 timestamps, S1, S2); 12/12 gates. | Round-3 review, then merge. |
| WP-40 package | `claude/wp-40-skeleton` | 5014d5b8 | Package plan and log. | Write WP-40-report.md when T2 and T4 merge. |
| WP-41 T3 text | `wp/41-groundwork/text` | d6b6a0de | Review accepted (round 2). Gate 9 fails: `crates/hub/src/checks/tests/probe/secrets.rs` is 306 lines, over the limit of 300. | Move the strip cases to a sibling test file, then the 12 gates, then merge. The PR must say that names now refuse U+200C/U+200D, U+00AD and U+FEFF. |
| WP-41 T4 targets | `wp/41-groundwork/targets-r2` | a0f81930 | Round 1 accepted by both reviewers (39 mutants). Round 2 fixes are in fa3c0e5c. fmt, clippy, it (710) and lib pass; limits fails only on T3's file. The old branch `wp/41-groundwork/targets` (8ff0c592) is round 1. | Reviewer check of fa3c0e5c, then gates after T3, then merge. Handoff: remove `mod target;` from `sessions.rs`. |
| WP-41 T5–T8 | — | — | Not started. | Order per CR-7: T6 trust (migration 0920, PublicKey move), then T5 ssh, then T7 relay and Serving (security), then T8 fingerprint and seal fallbacks. |
| WP-41 package | `claude/wp-41-groundwork` | ac01df9a | Plan, decisions and review log. | — |
| WP-43 T1 web-client | `wp/43-access/web-client` | 3608d0f8 | 12/12 gates (builder run), not reviewed. | Review round 1: reviewer-security with mutations, plus reviewer. |
| WP-43 T2 browser | `wp/43-access/browser` | c14bfb26 | 12/12 gates (builder run), not reviewed. Migration 0010_access_token_kinds. | Same review. T1 and T2 both add `access/origin.rs`: keep one copy at merge. |
| WP-43 T3, T4 | — | — | Not started. | The T3 API-key deny list must include `enrolAgent` and `revokeAgentToken` (R-084). |
| WP-43 package | `claude/wp-43-access` | db2294f0 | Plan, state and the main.rs handoff lines. | — |
| WP-46 T1 retention | `wp/46-checks/retention` | e34b3f46 | 12/12 gates (builder run), not reviewed. Migration 0900; CR-6 floors. | **Needs a ruling:** two K3 additions, `AuditTarget::Setting` and `events::drop_expired_partitions(events_keep)`. Then review. |
| WP-46 T2 defaults | `wp/46-checks/defaults` | 445fb372 | 12/12 gates (builder run), not reviewed. Migration 0321. | Review. Change its audit target to `AuditTarget::Setting`. |
| WP-46 T3 exposition | `wp/46-checks/exposition` | 7bf4638e | 12/12 gates (builder run), not reviewed. | Review (reviewer plus reviewer-security). Mount it after WP-43 T4. |
| WP-46 T4 domain expiry | — | — | Not started. | Waits on WP-41 T4. |
| WP-46 package | `claude/wp-46-checks` | f60100d9 | Plan and log. | — |
| WP-52 T4 docs + T5 firewall | `wp/52-deploy/apprise-firewall` | cd36c290 | T5 Apprise nftables rules and the T4 round-2 doc fixes. Lint, check, deploy 442/0, headscale 483/0 and quality pass. Not reviewed. | Review (reviewer-security for nftables), then full gates, then merge. Nothing has run on the live VPS. |
| WP-52 package | `claude/wp-52-deploy` | e4fb8a43 | Plan and log. | — |

**Not started:**
- WP-44, after WP-42 (done) and WP-43.
- WP-45, after WP-41 T1–T4.
- WP-47, WP-48 and WP-49.
- WP-50.
- WP-51 (R-084: T1 creates `agent-core` and moves the parsers; T1 and T2 do not need WP-41 T5).
- WP-53.
- WP-54, after WP-51 T1.

No more than 7 package orchestrators may run at once (R-066).

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
- **Issue #1:** posting `docs/programme/phase-2-coulds.md` needs approval. Later, also post
  `phase-3-coulds.md` and the shoulds in the package files.
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

- WP-46 T1: the two K3 additions (see section 3).
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
