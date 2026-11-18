# WP-25 report (package orchestrator)

Status: **done**. Branch `claude/wp-25-device-form` from `phase-2` at a6c57db. Not pushed, not merged.
`env -u DATABASE_URL BASE_REF=a6c57db bun run gates` on the code tip `5f67a7f`: **12/12 pass**, exit 0. This
commit adds only this file and the review log. Plan, decisions D-1 to D-7 and the review log: [WP-25.md](WP-25.md).

Built on a mock (R-066 early start): WP-20 was not merged, so `apps/client/src/lib/forms/hub-pending.ts`
holds the K1 `@pending` shapes by hand, with codegen's names (D-1). `contracts/` is not changed.

## What is delivered

| Output | Path (under `apps/client/src/`) | Spec |
| --- | --- | --- |
| RDP section: port, user, domain, security mode, ignore certificate, resize method, audio, microphone, shared drive, hidden password, stored credential, recorded certificate, warnings | `lib/forms/device/remote/RdpSection.svelte`, `warnings.ts` | 6.2, 6.3.2, RA-020, DEV-011 |
| VNC section: port, optional user, hidden password, the hub's unencrypted-VNC warning | `remote/VncSection.svelte` | RA-030, RA-020 |
| MAC address (normalised, empty clears) and metrics source in General | `remote/values.ts`, `form/GeneralSection.svelte` | RA-031, 6.2 |
| Files and clipboard: upload, download, copy, paste | `remote/TransferSection.svelte` | RA-032, RA-033 |
| Values, validation, create and update inputs; an untouched form sends the Phase 1 input | `remote/values.ts`, `validate.ts`, `input.ts` | D-2 |
| RDP and VNC credentials on Add device; an unstored password is reported | `lib/forms/add-device.ts` | DEV-011 |
| RDP and VNC connection test; new, changed and accepted server certificate | `remote/test/` | DEV-013, SEC-007 |
| Web interfaces section: rows with their own save and delete, suggestions | `lib/forms/device/web/`, `routes/devices/[id]/settings/SettingsPanel.svelte` | WEB-001, WEB-002 |
| Docker, systemd, SNMP v2c and v3, and Tailscale ping check editors; names that look like shell commands are refused | `lib/forms/check/probes/` | 6.4, CHK-009, CHK-012, SEC-005 |

Tests: client suite 2161 tests in 173 files. Phase 1 tests changed only where the values object grew, plus
the RDP placeholder test.

## Handoffs (wiring outside the owned paths, D-4)

- **H-1, WP-26 (`lib/devices/api.ts`).**
  - The `DeviceDetailFields` fields, the credential, certificate and web interface mutations, and the
    `testConnection` fields, as listed in WP-25.md.
  - After this, the K1 fields in `DeviceFields` become required.
- **H-2, programme (`routes/devices/[id]/save.ts`, `[id]/+page.svelte`, `routes/devices/+page.svelte`).**
  - `saveDevice` must store `rdpCredentialInput(values)` and `vncCredentialInput(values)` and report a
    failure. Until it does, the edit form drops a typed RDP or VNC password. H-2 must land in the same
    integration as WP-25.
  - Pass `onClearRdpCredential`, `onClearVncCredential`, `onAcceptServerCertificate` and `webInterfaces`.
  - Add `setRdpCredential` and `setVncCredential` to `AddDeviceApi`.
- **H-3, WP-29 (`lib/checks/api.ts`).** Config fragments `DockerConfig { container timeoutMs }`,
  `SystemdConfig { unit timeoutMs }`, `SnmpConfig { port version oid expected username timeoutMs }` and
  `TailscalePingConfig { target timeoutMs }`. The client omits an empty SNMP secret to mean "keep".
- **H-4, WP-33 (`routes/devices/[id]/checks/rows.ts`).** Labels for DOCKER, SYSTEMD, SNMP and TAILSCALE_PING.
- **After WP-20 merges:**
  - Rebase onto `phase-2` and regenerate the types.
  - Swap the `hub-pending.ts` imports to `$lib/hub/generated/graphql`, then delete the file and the one cast
    in `check/model/config-input.ts`.

## What depends on WP-21

- The hub must accept `rdp`, `vnc`, `macAddress`, `metricsSource` and `transfer` on create and update.
  - An empty `macAddress` must clear the address.
  - A `null` domain or VNC user name inside a whole settings object must clear the field. This is assumed;
    K1 does not say it.
- `setRdpCredential`, `clearRdpCredential`, `setVncCredential`, `clearVncCredential`.
- `accessWarnings` and `rdp.serverCertificate` on `Device`.
- The RDP and VNC `testConnection`, and `acceptServerCertificate`. The client reads a returned certificate as
  new unless it equals the accepted one or `certificateChanged` is true. This is assumed; WP-21 or WP-22
  must confirm it.
- Validation field paths: `rdp.*`, `vnc.*` and `macAddress` are assumed.

Web interfaces depend on WP-30, and the check types depend on WP-29.

## Open findings (could, or Phase 1 patterns for one change everywhere; issue #1)

- UI-013: checkboxes are 13 px, the same as Phase 1's "SSH enabled".
- MAT-006: under more contrast, a hovered Disclosure button is 1.17:1.
- UI-004: the primary button is 2.98 to 3.5:1.
- MOT-006: the press role animates the background colour.
- COM-002: comments and test titles cite package decision IDs, as Phase 1 does.
- The port rule is written three times, and the http(s) URL check twice.
- `sameJson` depends on key order.
- `WithRdp` and `WithVnc` are single-use types.
- No warning when an SNMP check's destination changes.
- Saving an unchanged web interface row does nothing and gives no feedback.
