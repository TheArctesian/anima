# Programme handoff — 2026-11-08

The programme orchestrator stopped at the operator's usage limit. This file is the state at the stop.
Read it first, then the rulings listed in section 5 and R-093 to R-101 in `programme.md`.

## 1. Where things are

- **Integration branch:** `phase-3`, pushed with `main` fast-forwarded to it as a save (operator,
  2026-11-08). Gates: `CARGO_TARGET_DIR=$PWD/target CARGO_INCREMENTAL=0 env -u DATABASE_URL
  BASE_REF=04ab9b61 bun run gates` (R-101; 12 gates). Client-only changes run gates 4–12 (R-068).
  PATH needs nvm Node 22 and `/usr/lib/postgresql/17/bin`.
- **Last fully gated commit:** `4d4242fa` (WP-43 T3 merge, 12/12). `c42b17c2` (K1 bounds doc
  strings) passed gates 4–12 and the hub contract test.
- **R-101:** every gate and review run uses its own `CARGO_TARGET_DIR` and `CARGO_INCREMENTAL=0`; delete
  the target folder when the task is accepted. A full disk shows up as sqlx test-database failures or a
  linker SIGBUS. About 116 GB free on 2026-11-09.
- **Merge loop:** merges ran from a small script (merge `--no-ff`, then gates, stop at the first
  failure). Rewrite it in the new session's scratchpad if needed.

## 2. Finished in Phase 3

| Package | State |
| --- | --- |
| WP-40, WP-41, WP-42, WP-52 | Done. |
| WP-44 web client | Complete except its rebase onto WP-43's last tasks; parked. |
| WP-46 checks | All tasks merged (T4 domain expiry `43babf5e`). Final package review running. |
| WP-43 access | T1, T2 and T3 api-keys (`4d4242fa`) merged. |
| WP-45 notifications | T1, T2, T3 channels (`92091f20`), T4 templates (`89ec75b1`), T5 merged. |
| WP-47 session sharing | T1a devices (`6eb66ca6`), T1b join-record (`52f87d1f`), T3 shares (`d398a8f1`) merged. |
| WP-50 settings | T1–T13 merged (last: T8 `6d0e1822`, T9 `4088bccf`, T12 `f4fa5abc`, T11 `5e660958`). Close-out running. |
| WP-51 parsers | T1, T2, T3 gpu-discovery (`22284904`) merged. |
| WP-54 agent | T2 points merged (`595c624c`; K8 frozen). |

## 3. Work in progress (2026-11-09)

The WP-43, WP-45 and WP-54 orchestrators were restarted as new agents (operator OK); WP-46, WP-47,
WP-50 and WP-51 were resumed. Each package log on its `claude/wp-NN-*` branch is the detail.

| Package | State | Next |
| --- | --- | --- |
| WP-43 | T4 bearer (`wp/43-access/bearer`), T5 rotation, T6 throttle (stacked on T3; CR-16 b). | T4 must move router assembly out of `serve()` (`main.rs` 60/60). |
| WP-45 | T6 dispatcher (`wp/45-notifications/dispatcher` f222ab84): reviews restarted, rebase onto 89ec75b1. | T7: checks/ edits with WP-46, `Check.notifications`. |
| WP-46 | Final package review; also checks WP-50's domain-rows against the T4 K1 shape. | Package done after the review. |
| WP-47 | T2 limit from 6eb66ca6. | T4 needs CR-16 b from WP-43 T6; then T5, T6. Q-5 ruled: keep the K4 §4 owner relay (forward `msg`). |
| WP-50 | Final review asked for fixes: `wp/50-settings/domain-rows` (R-094 rows + M2, ruled in scope) and `wp/50-settings/package-close` (M1 focus, S3–S7, `/settings` CLS handoff). | Gate both, review round 2, interface pass, report for merge, package log onto phase-3. |
| WP-51 | T4 read side (MET-008, MET-009), on 22284904. | `store/chart.rs` mean overflow. |
| WP-54 | T3a token (`wp/54-agent/token` 708db9ec) to gates; T4 binary (`wp/54-agent/binary` 2a1cc5d4) after its handoff reorder. | `SeriesKey::of` GPU index; cap pushed GPU temperature/power. |

**Not started:** WP-48 (can start now that WP-47 T1a is in), WP-49, WP-53. At most 7 package
orchestrators (R-066); all 7 slots are in use.

The T1b recording scripts and captures are in `.claude/worktrees/wp-47-t1b-evidence/`.

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
- `crates/hub/src/main.rs` `serve()` is at the 60-line limit: WP-43 T4 (or the next wiring change)
  moves router assembly into its own function (at most 4 params).
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
