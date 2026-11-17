# WP-21 report (package orchestrator)

Status: **done**. Branch `claude/wp-21-remote-access`, rebased onto `phase-2` at 76f7232 (R-067). Not pushed.
Plan, decisions D-1 to D-12 and the review log: [WP-21.md](WP-21.md).

Merged early into `phase-2` (R-067): T1 and T2 at 60ff355 (as 5a36769); T3 sent at 290db32. T5 and the final-review
fix are the rest of the branch; gates 12/12 on 7a07c3c (`BASE_REF=76f7232`).

## What is delivered

| Output | Where (under `crates/hub/`) | Spec |
| --- | --- | --- |
| RDP, VNC settings tables with K1 defaults in the schema; MAC, metrics source, transfer flags, site wake relay | `migrations/0110_devices_remote_access.sql` | DB-011, RA-030–033 |
| `RdpSettings`, `VncSettings`, `TransferPolicy`, `MacAddress`, `MetricsSource`, `wake_relay` (K3) | `src/devices.rs`, `devices/remote.rs`, `mac.rs`, `store/remote.rs`, `rules/remote.rs` | RA-030, RA-031, DEV-010 |
| `rdp_password`, `vnc_password` kinds; `PasswordKind`, `store/load/with/clear_password` (K3); SSH paths scoped to SSH kinds | `migrations/0210_…`, `src/credentials/` | SEC-003, SEC-005 |
| K1 devices members of D-1, the four password mutations, `setSiteWakeRelay`, access warnings | `src/devices/graphql/remote/`, `devices/warnings.rs` | RA-020, DEV-011, LOG-005, LOG-007 |
| `Device.webInterfaces`, `webInterfaceSuggestions` (WP-30 H-2) | `src/devices/graphql/standing/interfaces.rs` | WEB-001, WEB-002 |

`@pending` was removed from every member the hub now declares. `RdpSettings.serverCertificate` keeps a field-level
`@pending` for WP-22.

## WP-25 assumptions

- **Clearing (D-2).** `rdp`, `vnc` and `transfer` replace the whole object when present; absent or null leaves
  it. Inside an object, a null or blank `domain` or VNC `username` clears it, and an absent field takes its K1
  default. `macAddress`: absent or null keeps it, `""` clears it. The client's shapes match.
- **New certificate (D-3).** `serverCertificate` follows the `hostKey` rule: present only when none is accepted
  (`certificateChanged: false`, new) or the server showed another one (`true`); null when it equals the accepted
  one. The client's reading gives the same answer. WP-22 builds it.

## Handoffs

- **H-1 WP-22.** D-3; `RdpSettings.serverCertificate` as an R-011 edit in `devices/graphql/remote/`;
  `ConnectionTestInput` reuses `RdpSettingsInput`, `VncSettingsInput` (`validated()`) and `PasswordCredential`.
  `devices/graphql.rs` is at 292 code lines, `store.rs` near 297: add a seam before growing them.
- **H-2 programme (`schema.rs`).** Register `rdp_credential_loader`, `vnc_credential_loader`, `device_loader`,
  `web_interfaces_loader`, `web_interface_suggestions_loader` (`.delay(BATCH_WINDOW)`); drop
  `.register_output_type::<web_interfaces::Suggestion>()`. Without them those fields answer INTERNAL.
- **H-3 K3.** The devices row may also import `credentials::{PasswordKind, store_password, clear_password,
  with_password, PasswordCredential parse types}`.
- **H-4 WP-27.** `devices::MacAddress` (`octets()`, Display) and `devices::wake_relay(db, site)`. The relay always
  has SSH on and sits at its site: set, move and SSH-off are all guarded.
- **H-5 WP-22, WP-29.** D-10 test literals gained the new `Device` fields with defaults.

## Open coulds (issue #1)

- The automatic relay clear on a move or SSH-off has no audit entry of its own.
- The relay event has no device link when cleared.
- `standing/interfaces.rs` sits under a checks-flavoured folder name.
- No cross-check that `metricsSource: SSH` has SSH on.
- No test for SSH turned off in a race with an already-set relay beyond the set path.
