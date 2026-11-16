# Operator checks

This document is a record, not a plan (PLN-010, [work-plan.md](work-plan.md) Section 9). It lists each check that only the operator can do: a check on a real machine, a real account or a real network.

| Item | Value |
| --- | --- |
| Document | Operator checks: the list and the result of each check |
| Version | 1.0 |
| Date | 2026-11-16 |
| Status | In use. This table holds the result of each check (PLN-014). |
| Origin | The checks were in 10 documents. Their steps stay in those documents. The results in this version come from the text of those documents and of [handoff.md](programme/handoff.md) (2026-11-12). No person or agent ran a check for this version. |

Rules:

- This document holds the result of each check. The document in the column "Steps" holds the steps. Do not copy the steps here.
- A task that waits for a check is not Done until the row of the check has the result Pass (PLN-009, PLN-014).
- A new check gets its row here in the commit that defers it. The row names the task or the plan that it blocks.
- The operator, or an agent on the instruction of the operator, writes the result with its date. An agent must not write Pass for a check that it did not see.
- The words for a result: Not run, Pass, Fail, Blocked. A Fail names the defect in [work-items/](work-items/README.md) or the line in [deferred.md](deferred.md).
- The operator types each real secret personally. An agent never types, reads or asks for a real secret.

## 1. Summary

| Group | Checks | Pass | Blocks |
| --- | --- | --- | --- |
| Phase 2 (O-1 to O-9) | 9 | 1 | The Phase 2 live criteria, and the Phase 3 acceptance run (work plan Section 4, tier 0) |
| Phase 3 (P-1 to P-17) | 17 | 0 | The Phase 3 acceptance run, WP-53 (tier 0) |
| One-click (S1 to S8) | 26 | 0 | The task of each row. The one-click work has no tier (work plan Section 4, notes) |
| Command line and MCP | 6 | 0 | The register row of [cli-mcp-plan.md](cli-mcp-plan.md) |

## 2. Phase 2

Steps: [programme/phase-2-acceptance.md](programme/phase-2-acceptance.md), "Operator steps". Blocks: the criterion in the column "Criterion" of that document.

| Check | What it proves | Needs | Criterion | Result |
| --- | --- | --- | --- | --- |
| O-1 | RDP: the certificate, the picture, resize, copy and paste, Ctrl+Alt+Delete | A machine with RDP. R-076: no Windows; the operator tests on a second Mac. A Headscale grant on tcp:3389 | 1 | Not run. It waits for the second Mac (handoff, 2026-11-12) |
| O-2 | RDP: a changed certificate stops the session; the Warning badge | The machine of O-1 | 2 | Not run. It waits for the second Mac (handoff, 2026-11-12) |
| O-3 | VNC to a Mac and to a Linux desktop | A Mac with Screen Sharing on. A Linux desktop with a VNC server | 3 | Pass on the Mac and on `pop-os` (handoff, 2026-11-12; the Linux run was 2026-11-07, with input off in wayvnc) |
| O-4 | SFTP: list, upload, download, the refusals, the audit entries | A real OpenSSH server with an accepted host key | 6 | Not run |
| O-5 | Wake-on-LAN through the site relay | A device that sleeps on a LAN. A relay device at that site with `wakeonlan` | 8 | Not run |
| O-6 | The web view on macOS: fingerprint, a changed certificate, the Down page | A device web interface with an untrusted certificate | 9 | Not run |
| O-7 | OD-7 on Linux: the trust hook spike. Also the frame time of the Devices page | A Linux desktop | 9 | Not run |
| O-8 | SSH metrics: a new point each 30 s, the bars on the card | One Linux device and one macOS device with SSH | 11 | Not run |
| O-9 | Docker, SNMP, Tailscale ping and systemd checks | An SSH device with Docker and systemd. An SNMP v3 agent. The LocalAPI proxy on the VPS | 15 | Not run |

## 3. Phase 3

Steps: [programme/phase-3-acceptance.md](programme/phase-3-acceptance.md), "Operator steps". Run order: P-1 to P-15, P-17, then P-16. Blocks: the acceptance run of Phase 3 (WP-53). The handoff of 2026-11-12 names what must come first: O-4 to O-9, the wording of the design specification Section 5.7.3, the Headscale agent policy (R-105), the R2 set-up and the agent binaries.

| Check | What it proves | Needs | Result |
| --- | --- | --- | --- |
| P-1 | The deployed image and the compose file are the accepted ones | A shell on the VPS | Not run |
| P-2 | The T4 probe, with no key | A VPN machine with `bun` | Not run |
| P-3 | Browsers and sign-in on macOS; the cookie attributes | A Mac on the VPN with Safari and Chrome | Not run |
| P-4 | A browser on Linux, and the desktop application | A Linux machine with Firefox or Chromium | Not run |
| P-5 | The web client: the device list and the detail view | A signed-in browser (P-3) | Not run |
| P-6 | SSH and VNC sessions in the web client | A device with SSH. A device with VNC | Not run |
| P-7 | The notification channels are entered | A signed-in client. The operator types each secret | Not run |
| P-8 | Channels are selected on a check | A signed-in client | Not run |
| P-9 | "Send test" on each channel | The receiving application of each channel | Not run |
| P-10 | A Down and Up cycle sends the messages | The check of P-8, and a way to stop its service | Not run |
| P-11 | Maintenance sends no message | The check of P-8 | Not run |
| P-12 | A live RDAP check | A signed-in client | Not run |
| P-13 | GPU metrics on the Apple silicon Mac | The Mac with the push agent in operation | Not run |
| P-14 | Optional: a real Prometheus scrape | A Prometheus server on the VPN. A Headscale grant (ask first) | Not run |
| P-15 | Optional: the API key checks in Settings | A signed-in client | Not run |
| P-17 | The desktop application: the update banner | A build with an older version installed, and a newer release | Not run |
| P-16 | Last: the T3 log scan of the live hub log | A shell on the VPS, after all other steps | Not run |

## 4. One-click

Each row gives the document that has the steps and the task that the check blocks.

| Check | What it proves | Needs | Steps and task | Result |
| --- | --- | --- | --- | --- |
| S1-T14 | The link between two machines: the direct rate, the first-connection time, relayed Screen Sharing, the firewall prompt. It sets the keepalive numbers (LNK-009) | A second Mac | [s1-link-tasks.md](research/one-click/s1-link-tasks.md), T14 | Not run |
| S2-T9 | The native VNC gateway against macOS Screen Sharing and TigerVNC, with the figures | This Mac; the operator types the password | [s2-vnc-tasks.md](research/one-click/s2-vnc-tasks.md), T9 | Not run |
| S2-RDP | The native RDP gateway against a GNOME Remote Desktop host | The operator's GNOME Remote Desktop host | [s2-rdp-tasks.md](research/one-click/s2-rdp-tasks.md), Section 6 | Not run |
| S3-T10 | The database tree recipe on the Linux builder | The `x86_64` Linux build computer | [s3-database-tasks.md](research/one-click/s3-database-tasks.md), Section 7.2 | Not run |
| S3-T12 | Signature and notary check of the database tree | The Developer ID of the operator. A second Mac or a new user account | [s3-database-tasks.md](research/one-click/s3-database-tasks.md), Section 7.2 | Not run |
| S3-T13 | No data loss after a power cut | A Mac that can lose power | [s3-database-tasks.md](research/one-click/s3-database-tasks.md), Section 7.2 | Not run |
| S3-T14 | The database on a real Linux desktop | A Linux desktop with PostgreSQL 17 | [s3-database-tasks.md](research/one-click/s3-database-tasks.md), Section 7.2 | Not run |
| S4-T10 | The systemd user unit on Linux. The Local Network prompt on a Mac | A Linux desktop. A Mac with a LAN device | [s4-local-hub-tasks.md](research/one-click/s4-local-hub-tasks.md), Section 7.2 | Not run |
| S6-1 | The first run in the desktop window: create a hub, TOTP, the key copy, an invite | This Mac and a phone | [s6-first-run-tasks.md](research/one-click/s6-first-run-tasks.md), Section 6.3, check 1 | Not run |
| S6-2 | Join from a second Mac, then revoke | A second Mac | Section 6.3, check 2 | Not run |
| S6-3 | The first run on Linux with the systemd user unit; finding F-10 | A Linux desktop | Section 6.3, check 3 | Not run |
| S6-4 | No port mapping on a real router; no TCP port answers from outside | The router of the hub computer, and a machine outside the network | Section 6.3, check 4 | Not run |
| S6-5 | A relayed path shows `RELAYED` | A network where UDP is blocked | Section 6.3, check 5 | Not run |
| S8-1 to S8-8 | Stay awake on macOS: the tray, keep awake, the set-up, the lid, quit and failure, the battery floor, remove, eight hours (8 checks) | A Mac laptop. A second computer for the `ping` of check 4 | [s8-stay-awake-tasks.md](research/one-click/s8-stay-awake-tasks.md), Section 6, checks 1 to 8 | Not run |
| S8-9 to S8-13 | Stay awake on Linux: the tray, keep awake, the lid on GNOME, quit, the battery (5 checks) | A Linux laptop | Section 6, checks 9 to 13 | Not run |

## 5. Command line and MCP

Steps and comments: [GH-60](work-items/device-manager/GH-60.md). A suite on loopback covers the other items of that list (its comment of 2026-11-13). Blocks: the register row of [cli-mcp-plan.md](cli-mcp-plan.md).

| Check | What it proves | Needs | Result |
| --- | --- | --- | --- |
| CLI-1 | `anima mcp` with a real MCP client: the tool list, one read, one change, and the approval request for a tool in state "ask" (MCP-013) | Claude Code and a hub | Not run. A first change through the deployed hub was sent on 2026-11-16 (`anima issue comment`); the approval request is not checked |
| CLI-2 | The MCP section in Settings in the real desktop application | The desktop application | Not run |
| CLI-3 | Linux: the crate compiles; the Secret Service token store and its file fallback | A Linux desktop | Not run |
| CLI-4 | macOS: the Keychain item and its access control | This Mac | Not run |
| CLI-5 | TLS against a hub with a bad certificate | A hub with a bad certificate | Not run |
| CLI-6 | The list of GH-60 against the production hub: sign-in, `anima ssh`, `anima files`, a confirmation prompt | The production hub | Not run. The read commands for GitHub were checked on 2026-11-16 |

## 6. Machines that the checks need

One visit to a machine can close many rows.

| Machine or access | Checks |
| --- | --- |
| A second Mac | O-1, O-2, S1-T14, S3-T12, S6-2 |
| A Linux desktop or laptop | O-7, P-4, S3-T14, S4-T10, S6-3, S8-9 to S8-13, CLI-3 |
| The `x86_64` Linux build computer | S3-T10 |
| This Mac only | S2-T9, S6-1, S8-1 to S8-8, P-3, P-13, CLI-2, CLI-4 |
| A shell on the VPS | P-1, P-16, O-9 |
| Special set-up | O-5 (a relay and a device that sleeps), O-9 (SNMP v3 agent), S3-T13 (a Mac that can lose power), S6-4 (the router), S6-5 (no UDP), P-14 (Prometheus) |
