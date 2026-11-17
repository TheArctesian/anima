---
kind: Proposal: spec delta for the hub as a background program
state: Draft
evidence: Not approved. It changes no requirement until the operator approves it.
---

# S4 spec delta: proposal

Progress: the plan register in [work-plan.md](../../work-plan.md) Section 9 holds the state of this plan (PLN-005).

Version 1.0. Date: 2026-11-15. Status: **Draft: a proposal for open item OI-2 of `docs/plans/one-click/one-click-plan.md`. Nothing in
`docs/spec/design-spec.md` or `contracts/` is changed by this file.** The operator approves the text
before it goes into the spec.

Context: plan item S4. In the application mode the hub runs as a background program of the
user (`crates/anima-local-hub`). The Docker installation does not change, and no requirement
below changes it.

## 1. New requirements (prefix `LHB`, "local hub")

The prefix is free on `main` at `4c27cceb`. Each one is built and tested in
`crates/anima-local-hub`, except where the last column says another item.

| ID | Requirement | Where |
| --- | --- | --- |
| LHB-001 | In the application mode the hub must run when the application window is closed and must start when the user signs in. On macOS it is a LaunchAgent, on Linux a systemd user unit (as MET-019). No step must need administrator rights. | S4 |
| LHB-002 | One program must start the hub's programs in this order: the database, the migration, the gateway, the hub. It must stop them in the reverse order. (This is DBL-012 of the S3 delta, for the whole group.) | S4 |
| LHB-003 | When one of the hub's programs ends by itself, the others must be stopped in order and the group must start again after 10 seconds. | S4 |
| LHB-004 | When the hub cannot start until the operator acts, the group must not start in a loop. The state must give the reason: the hub key is missing, the database is of another PostgreSQL major version, or another program runs the database. | S4 |
| LHB-005 | The hub key must be made only when the database is new. When the key is missing and the database is not new, no key must be made. | S4 |
| LHB-006 | The hub key, the logs and the state must be in a folder that only the user can open. The key must not be in a log, a command line or the state (LOG-007). | S4 |
| LHB-007 | The unit must run a copy of the programs that an application update does not replace while the hub runs. An update must stop the hub, replace the copy and start the hub. (This is DBL-013 of the S3 delta, for every program of the group.) | S4 |
| LHB-008 | To remove the hub must stop it and remove its unit and its programs. It must not remove the database, the hub key or the logs. | S4 |
| LHB-009 | The Tauri shell must install, remove and read the state of the hub (REPO-003). Its commands must take no path, no program and no address from the web view, and only the main window may call them (WEB-007). | S4 |
| LHB-010 | The application must show the state of the hub on this computer: starting, ready, blocked with the reason, or failed with the program that ended. | S6 |
| LHB-011 | The backup of the application mode must hold the hub key with the database. | S9 |

## 2. Requirements of other items that S4 touches and does not change

| ID | State after S4 |
| --- | --- |
| NET-001, NET-002 | Not changed. S4 sets no permitted network, so the hub of the application mode refuses every client and answers only `/health`. S5 sets the rules. |
| NET-003 | Kept: the hub listens on `127.0.0.1` only, with plain HTTP, which the hub permits on loopback only. S5 owns the address. |
| GW-002, GW-003 | Kept: the gateway listens on a loopback port. It has no sign-in, so each program of the computer can use it; this is the state that S2 accepted. |
| K7 (`contracts/conventions.md`) | Not changed. The hub gets `ANIMA_DATABASE_URL_FILE`, `ANIMA_KEY_FILE`, `ANIMA_GUACD`, `ANIMA_LOG` and `ANIMA_LISTEN`, each with its K7 meaning. |
| CMD-030, CMD-031 | Do not apply: S4 adds no root field to `contracts/schema.graphql`. `anima-local-hub` is the command line of this function. It gets no MCP tool, because it changes what runs at login. |

## 3. Not proposed

- No lingering on Linux (`loginctl enable-linger`): the hub runs while the user is signed in.
  A hub that runs with nobody signed in is the self-host mode (Docker), or a later item.
- No sandbox directives in the systemd user unit beyond `NoNewPrivileges` and `LimitCORE=0`.
  The hub must reach every address of the LAN and PostgreSQL needs shared memory; a tighter
  unit needs a test on a real Linux desktop first (task T10).
