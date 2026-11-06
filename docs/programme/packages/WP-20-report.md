# WP-20 report — Phase 2 skeleton and contract gate

Status: **done**. Branch `claude/wp-20-phase2-skeleton`, not pushed, not merged into `phase-2`.

## What changed

- `contracts/`: K1–K7 from phase-2-contracts.md, headers "frozen for Phase 2". Every K1 addition carries
  `@pending` (37 declarations, 61 members). K1 header names it as the client's generation source.
- CON-002 (`crates/hub/tests/it/contract.rs`, `contract/pending.rs`, `contract/tests.rs`): a `@pending`
  declaration, member, argument or union member is ignored while the hub lacks it; a hub that declares
  it fails with "still @pending in K1". `Declaration::served_members` for other K1 readers.
- `crates/hub/src/targets.rs`: `AddressPolicy` moved from `checks/probe/reach.rs` (rename, 96 %).
- `ids.rs`: `WebInterfaceId`, `MetricSeriesId`. `config.rs`: `guacd_drive_root`, `tailscale_localapi`;
  `Error::Guacd` is now `Error::HostPort { variable, value }`.
- `contracts/fixtures/phase2.json` and `quality/tests/fixtures.test.ts`.
- K5 chart tokens in `design/tokens/`, outputs rebuilt; client generated types rebuilt.
- Stylelint bans `@keyframes` (MV-008). Baseline default is `phase-2` (P2-001). Spec Section 13 says bun.
  AGENTS.md already said bun; only its baseline default changed.

## Edits outside WP-20 paths (for acceptance at merge)

- `lib.rs` (`pub mod targets;`), `main.rs` (one import), `checks` imports.
- 7cf4f90: `lib/sessions/ending.ts`, `lib/forms/check/*`, `routes/devices/[id]/{checks,sessions}/`
  cover the new enum values (D-WP20-5).
- `crates/hub/tests/it/sessions/k1.rs`: uses `served_members`.

## Handoffs

- **WP-21, WP-22, WP-30:** when the hub declares a K1 addition, the orchestrator removes its `@pending`
  in the same merge, or CON-002 fails. Read K1 enums with `served_members`.
- **WP-22:** `config.guacd_drive_root` is ready for `drive-path`. Owner of `sessions::terminal`: the test
  `k4_8_5_a_client_ping_sent_while_the_hub_s_send_waits_keeps_the_session` fails under machine load.
- **WP-24:** `SERVER_CERTIFICATE_MISMATCH` has placeholder text in `ending.ts`.
- **WP-25:** remove `EditableCheckType`, `FALLBACK_TYPE` and the `default:` in `withConfig` when the new
  editors land. Until then a saved Phase 2 check opens as ICMP.
- **WP-23:** `ANIMA_GUACD_DRIVE_ROOT` must be an absolute path.
- **events owner (CQ1-02):** `events/page.rs:69-75` cursor has no time-range bound.
- **Programme:** `.claude/agents/gate-runner.md:14` still names `phase-1` as the default base.

## Open coulds and shoulds (issue #1)

`host_port` accepts `[]:4822` and port 0. The fixture test does not check field names. Contract-test
tidy-ups: union header built twice, signature stored twice, the hub side's `@pending` is stripped too.
The `hub-domains.md` table is split by a blank line. MV-008 names an allow-list, but the gate has none.

## Contract change requests

None. K7 now states the drive root must be absolute; that adds a stated rule to K7 (final review).
