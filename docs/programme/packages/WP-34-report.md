# WP-34 report — Phase 2 integration and acceptance

Status: **done.** Package branch `claude/wp-34-acceptance`, rebased onto `phase-2` after each programme merge. Plan,
decisions and review log: [WP-34.md](WP-34.md). Matrix with evidence and operator steps:
[phase-2-acceptance.md](../phase-2-acceptance.md).

## Tasks

| Task | What | Result |
| --- | --- | --- |
| T1 matrix | One row per Section 5 criterion, with evidence, status and operator steps | `docs/programme/phase-2-acceptance.md` |
| T2 motion audit | Section 5.8 on the Phase 2 screens, WebKit and Chromium, fake hub on K6 data | `docs/research/motion/phase-2.md` |
| T3 log audit | Criterion 16, 55 Phase 2 call sites, 6 mutations | Pass; two test gaps became F6 |
| T4 CQ-050 | Code-quality audit, audit mode, nothing applied (D-3) | `docs/research/code-quality/phase-2.md`, Q2-01 to Q2-16 |
| F1 settings-wiring | Settings mounts Web interfaces; RDP and VNC passwords stored; accept certificate and clear password wired | Merged as f9007b86 |
| F2 live-rows | Live sessions rows: no 64 px jumps, failed Stop moves nothing, keyboard reach, reduced-motion test | Merged as a88af73c |
| F3 strip-close | No 32 px step after the last tab closes (round 3, R-075); wake icon column | Merged (58177a27) |
| F4 overlay | `overlayOpen` hides the native web view under menus, popovers and modals | Merged (e5e3f181) |
| F5 narrow | Header buttons wrap at 390 px; card badge icon-only below bp-compact | Merged as af0952a5 |
| F6 log-tests | Process-log tests for files, wake, tab warnings, guacd level; `MacAddress` Debug hides the octets | Merged (393393ec), 12/12 |
| F7 loaders | Files listing skeleton; chart skeleton keeps each device's chart shape | Sent as 8b899792 (two rounds) |
| F8 scrim | Session modal scrims use `--scrim` | Merged as ade3e8a1 |

Every task ran gates 4–12 (client-only, R-068) or all 12 (F6), and had a review by an agent that did not build it.
Rounds: at most two, except F3 (three, R-075).

## Acceptance status per criterion

| # | Criterion | Status |
| --- | --- | --- |
| 1 | RDP to Windows | operator (O-1) |
| 2 | RDP certificate pin, change, badge | operator (O-2) |
| 3 | VNC to macOS and Linux | operator (O-3) |
| 4 | Credential prompt, Save | pass |
| 5 | Copy and paste off | pass |
| 6 | SFTP panel, flags, audit | operator (O-4) |
| 7 | Live sessions, Stop | pass |
| 8 | Wake-on-LAN before connect | operator (O-5) |
| 9 | Web interface tab | operator (O-6 macOS, O-7 Linux OD-7) |
| 10 | Web interface suggestions | pass (after F1; it failed at 551ee1da) |
| 11 | SSH metrics, GPU discovery | **fail for GPU discovery** (not built, deferred by R-073); operator (O-8) for the rest |
| 12 | Card bars | pass |
| 13 | Detail view | pass (GPU charts on synthetic data only) |
| 14 | Metric aggregates and retention | pass |
| 15 | Phase 2 check types | operator (O-9) |
| 16 | No secrets or content in logs | pass |
| 17 | Motion: reduced forms, no open layout jump or wrong motion, nothing over a live display | pass, with coulds named below |
| 18 | Gates 12/12, baseline `{}`, no `@pending` | pass |

Criterion 18: the programme orchestrator ran all 12 gates on `phase-2` after the F3, F4 and F6 merge (ba02666e): pass.
`quality/baseline.json` is `{}`. K1 keeps only the `@pending` directive declaration and its comment; nothing uses it.

## Operator list

Hardware: a Windows machine with RDP and NLA; a Mac with Screen Sharing; a Linux desktop with VNC; a real OpenSSH
server; a sleeping LAN device and a wake relay at its site; a device web interface on an untrusted certificate; a
Linux desktop for OD-7; Docker and systemd on an SSH device; an SNMP v3 SHA-256/AES agent; the WP-23 LocalAPI proxy
on the VPS. Steps O-1 to O-9 are in the acceptance matrix. Before O-1, the Windows machine needs its Headscale grant
on tcp:3389 (VPS-005: the operator confirms each VPN or VPS change).

## Findings found here and fixed

- **Q2-01 (criteria 1–3, 10).** The WP-25 H-2 handoff was never applied. Settings did not mount the Web interfaces
  section, and typed RDP and VNC passwords were never stored. Fixed by F1.
- **C-4 (MAT-008, WP-31 handoff).** `overlayOpen` was the constant `false`. Fixed by F4; desktop check in O-6.
- **M-1, M-2, M-3 (MOT-007).** Fixed by F3 and F2, re-measured in both engines.
- **T3 F-1, F-2 and the `MacAddress` Debug (LOG-007).** Fixed by F6.

## Open items (issue #1 or operator decision)

- **Operator decision (CQ-050, D-3):** Q2-02, CQ-023 `expect` on the OS random source in `credentials/vault.rs:71`,
  `checks/graphql/write.rs:44` and `checks/probe/snmp.rs:45`. The other CQ-050 items are in the research report.
- **Spec wording (C-10):** 5.7.3 Chart update says "the line moves left continuously". The audit settled WP-33 D-2:
  one spring slide per point meets 5.7.3 and MOT-011. Reword the row. The Modal row still names blur (D-11 E-5).
- **Coulds from the audit and re-measures (D-5):** S-2 Files panel arrives in one frame; S-4 missing hover colours
  (programme-wide MOT-006 pattern); C-5 table Connect column off screen; C-6 transfer fill by `inline-size`;
  C-7 Files toggle at 390 px; C-8 badge text 3.04:1; C-9 SSH tab frame stall; F7 skeleton 49.5 px short at 390 px (legend wraps); F2 N-1 panel bottom edge steps 88 px
  while rows glide; F3 Chromium-only layout-shift entry 0.0198 (logged at f590e0cd); chart grid stays `chart-grid`
  under `prefers-contrast: more` (1.69:1, not a MAT-006 failure).
- **Settled WP-34 coulds:** D-2 chart slide (meets spec); popover reduced form (correct, now tested); Stop row shift
  (fixed); button borders 7.45:1 under `contrast: more` (MAT-006 met); 4003 text comes only from a fake hub (the real
  hub shows the certificate text; check in O-2).
- **Not checkable here:** the WebKit clipboard write in the macOS app (O-1), a real download in the Tauri app (O-4),
  MAT-007 frame time on Linux (O-7), the 30 s collector ticker has no test, no test reads the desktop capability files.

## Contract change requests

None.

## Handoffs

- Programme: phase-2 is ready for operator checkpoint 3 (test machines) and then checkpoint 4 (merge into `main`).
- `lib/overlay/` is a new client folder (F4); `lib/motion/roles/reorder.svelte.ts` moved from `lib/devices/card/` (F2).
