# S6 spec delta: proposal

Progress: the plan register in [work-plan.md](../../work-plan.md) Section 9 holds the state of this plan (PLN-005).

Version 1.0. Date: 2026-11-15. Status: **Draft: a proposal for open item OI-2 of `docs/one-click-plan.md`. Nothing in
`docs/design-spec.md` or `contracts/` is changed by this file, except the K1 fields of Section 3,
which the code needs (OR-S6-3 in [s6-first-run-tasks.md](s6-first-run-tasks.md)).** The operator
approves the text before it goes into the spec.

## 1. New requirements (prefix `FRN`, "first run")

The prefix is free on `main` at `a8274e6b`.

| ID | Requirement |
| --- | --- |
| FRN-001 | At the first start, the desktop application must show three choices: "Sign in", "Join a hub" and "Create a hub on this computer". One build must hold all three. A choice that has no service yet must say that it is not yet available. |
| FRN-002 | "Create a hub on this computer" must install and start the local hub (LHB-001) with its database, its gateway and its link, then make the operator account, then show an invite for the next device as a QR code and as text. |
| FRN-003 | The operator account of a local hub must be made only on the hub computer, by the hub's own commands, with each secret on standard input. No hub API must make an operator account. |
| FRN-004 | The first run must enrol TOTP (OD-2). The hub must save the TOTP secret only after the operator gives a correct code, and that code must not then sign in. |
| FRN-005 | "Join a hub" must take an invite, make a new device key, join the hub over the link, start the device link while the application runs, and show the sign-in for that hub. A device that the hub revoked must be able to join again with a new invite. |
| FRN-006 | The application must start and stop the local hub's programs together, and must show the state of the local hub and of its link (LHB-010). |
| FRN-007 | On the hub computer, the operator must be able to set a new password and a new TOTP secret. All tokens of the operator must then stop. A joined device, a token or an API key must not be able to do this. |
| FRN-008 | The client must treat the hub as "this computer" only when the local hub is installed and the client uses its address (AWK-005). |
| FRN-009 | The operator must be able to make an invite, list the devices of the link and revoke a device through the hub API, the `anima` command and the client. The device list must also be an MCP tool; the invite and the revocation must not (MCP-004, MCP-005). Each invite and each revocation must have an audit entry that holds no invite text (LOG-005, LOG-007). |

## 2. Changes to requirements

| ID | Change |
| --- | --- |
| CLI-002 | Add: "A hub address is `https:`, or `http:` for a loopback host only (the local hub or the local port of the device link)." |
| NAV-009 | Add: "On the desktop, before a hub is chosen, the window shows the first-run choices (FRN-001) instead of the sign-in page." |
| AWK-005 | "When the computer is the hub" is FRN-008. |
| LNK-004 | Add: "through the hub API (FRN-009)". |

## 3. Contracts

- **K1** (`contracts/schema.graphql`): `Query.linkDevices`, `Mutation.createLinkInvite`,
  `Mutation.revokeLinkDevice`, and the types `LinkDevice`, `LinkInvite`, `LinkPath`.
- **K7** (`contracts/conventions.md`), proposed rows, not applied:

| Variable | Meaning | Default |
| --- | --- | --- |
| `ANIMA_LINK_SOCKET` | (S5) Path of the Unix socket for `anima-link`; set means the link mode | empty |
| `ANIMA_LINK_CONTROL` | Path of the control socket of `anima-link hub`; the hub uses it for invites and the device list | empty: the link fields answer `LINK_OFF` |

- **The link's control socket** (`link/control`): the verbs `status`, `devices.list`,
  `devices.revoke` and `invites.make` become an interface between the hub and the link. Proposed
  as a contract (K9) after S6 is accepted; until then a change to them must change `fleet/link`
  in the same commit.
