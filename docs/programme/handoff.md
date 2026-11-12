# Programme handoff — 2026-11-11

The programme orchestrator stopped at the operator's usage limit. This file is the state at the stop.
Read it first, then the rulings listed in section 5 and R-093 to R-111, CR-17 to CR-21 in `programme.md`.

## 1. Where things are

- **Integration branch:** `phase-3`, pushed. `main` was merged into it on 2026-11-10 with the operator's OK
  (`efe75d8d`: client redesign, distribution, the hub image toolchain, the extensions plan), and `main` is
  fast-forwarded to `phase-3` again as a save. Push only fast-forwards; ask before anything else on `main`.
- **Last Rust merge gated 12/12:** `26a5a9bd` (WP-45 T6 and the dispatcher wiring in `main.rs`). After it came the WP-36
  deploy merges (`fbdd525f`, `e1e7ff2c`) and the R-109 to R-111 docs commits, through `9baf6aeb`.
- **R-101:** every gate and review run uses its own `CARGO_TARGET_DIR` and `CARGO_INCREMENTAL=0`; delete
  the target folder when the task is accepted. A full disk shows up as sqlx test-database failures or a
  linker SIGBUS. About 116 GB free on 2026-11-09.
- **Merge loop:** merges ran from a small script (merge `--no-ff`, then gates, stop at the first
  failure). Rewrite it in the new session's scratchpad if needed.

## 2. Finished in Phase 3

| Package | State |
| --- | --- |
| WP-40, WP-41, WP-42, WP-52 | Done. |
| WP-44 web client | Complete. The `/shared` bare route is gone: since WP-49 T9 (`0dd50644`), `/shared` needs sign-in like any other route (R-111). |
| WP-46 checks | Done. Last merges: package fixes `a28410c4`, R-102 domain retries and hourly floor `53f0a031`, log `2821cc2d`. |
| WP-50 settings | Done. Last merges: check-row `5fbebf60`, log `0cbc06d4`, R-102 client `e12da074`. |
| WP-51 parsers | Done. T4 `a67ae69e`, vendor retry `4080416c`, log `8f44adc3`. |
| WP-43 access | Done. T4 bearer `2446f367` (router assembly in lib.rs `all_routes`), T5 rotation `4ee029fe`, T6 throttle `15b8f9d0`, log `8d064256`. |
| WP-45 notifications | Done. T6 `99258714`, with the dispatcher wired in `main.rs` (`26a5a9bd`); T7 checks edits `e34e9060`; log `ab598609`. Follow-up: narrow the `notifications.rs` public face (lines 31–37). |
| WP-47 session sharing | Done. T2 session limit `6f8d8356`, waking-flake `09555c56`, T7 share removal `301d1b5e` (R-111), log `347cc2cc`. T4–T6 cancelled. Migrations 0121–0129 are unused. |
| WP-48 recording | Closing under R-113 (recording deferred). Merged: T1 playback proof `a4192d68`, T2 format and T3 asciicast `e091bfa8`, T4 recorder `711260b3`, T5a start `ab7f2d83`, T5b catalogue `3a1305f5`. T6–T9 cancelled; T10 removes the merged recording code. |
| WP-49 client sharing and recording | Closed. T5 asciicast `ad50bac3`, T6 playback tunnel `4d3eebab`, T8a badge `917281e9`, T9 `/shared` sign-in `0dd50644`. T1–T4 cancelled (R-111); T7 and T8b cancelled (R-113). WP-48 T10 removes the recording parts of T5, T6 and T8a. |
| WP-54 agent | Done. T2, T3a, T3b `7ea84ecc`, T4, T5, T6a `49119a71`, T6b+T6c `b2775938`, T6e `c6517bce`, T7 `176cb4b1`, T8a hardening `792d4441` (CR-22), log `11e96064`. T6d is a handoff note to WP-36: build the agent for three targets with `release.json`, and ship it in the desktop release (`binaries/anima-agent-<triple>`, `--config tauri.release.conf.json`). |

## 3. Work in progress (2026-11-11)

R-112 (speed without dropping quality) and R-113 (recording deferred and removed) apply. At most 3 builders and reviewers work at once.

| Package | State | Next |
| --- | --- | --- |
| WP-48 | T10 recording removal (1 slot). | Merge T10, then the package log. |
| WP-55 | Gate speed: nextest, a template database, sccache (1 slot). | Its gates are compared with the current gates on the same commit before it merges; record the R-101 amendment. |
| WP-53 | Preparation: the acceptance document, scripts and fixtures (1 slot). | Acceptance, after WP-48 T10 and the operator's steps. |

**Operator's critical path for WP-53, in order:** live checks O-4 to O-9; the spec 5.7.3 wording; the Headscale agent policy (R-105, confirmed in chat); `TAURI_SIGNING_PRIVATE_KEY`; R2 setup and the agent binary upload; the test machines and accounts in plan Section 6, with each real secret typed into Settings by the operator.

**Load flake that needs a fix:** `builds::publishing::fences::d_25_a_try_past_its_hold_writes_nothing` (WP-36) failed three merge gates on 2026-11-10 and passes alone.

**Load flakes seen in merge gates this stretch, all passing alone:**
- the three `ra_031` tests, with WP-47;
- `access::listen` sec_001 large-body tests;
- `k4_8_5`;
- `met_012`, fixed in WP-54 T3b;
- `d11_gpu`;
- `totp::log_007`, which is a real needle bug: WP-43 T6 fixes it.

The issue #1 items not yet posted are listed in the orchestrator's scratchpad, `issue1-pending.md`. If the scratchpad is lost, rebuild the list from the package logs' Outcome sections.

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
- WP-47 T3's full run also hit
  `k4_5_a_client_write_stalled_past_the_idle_wait_is_4002_timeout`. It reported client silence
  instead of a stalled write. The full run passed on retry with `RUST_TEST_THREADS=8`.

## 6. Decisions and follow-ups for the next session

- D-23 / R-100: INT-003 already requires refusing `shareSession` for API keys. WP-43 T3 (`f4eee5ed`)
  includes it in the deny list and its test. A merge preview with WP-47 T3 found conflicts in
  `contracts/schema.graphql` and `crates/hub/src/events/model.rs`. Keep the login/browser-only wording
  alongside WP-47's removal of `@pending`, plus both audit target variants.
- WP-47 Q-5: K4 §4's general relay rule already forwards the owner's gateway instructions, which
  includes `msg`. The recorded notice carries a guacd user ID (`@` and a UUID) and the viewer name.
  T4 cleans the name before T6 sends it to guacd. Keep the existing owner relay rule unless T6 adopts
  an explicit contract exception.
- WP-47 T3 security round 2 passed with 16/17 mutants killed. Coulds: cover the remaining
  `keep()` signal-read mutant (N2), consider
  `Shares::take` and a drop guard for the panic gap, and wrap the long line in `keeper.rs`.
- COULD-3: three copies of `random_token` (sessions/start.rs, sessions/files/opening.rs, sessions/share/opening.rs); needs an owner.
- New HTTP routes go in `crates/hub/src/lib.rs` `beside_graphql`, not in `main.rs` (WP-43 T4).
- Operator steps added this stretch:
  - Apply the Headscale agent policy (R-105, VPS-005).
  - Upload the agent binaries to R2 for the D-10 install command.
- Background agents still running at the stop (WP-43 T4 security review, WP-45 T6 reviews, WP-54 T3a
  and T4 builds) may have pushed commits to their branches after this file:
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
  - `.claude/worktrees/wp-47-t1b-evidence/` (local synthetic captures and scripts)
  - every branch in section 3, until it merges or the Linux machine has it from the push
