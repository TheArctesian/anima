# Security Findings

| Item | Value |
| --- | --- |
| Document | The list of security findings and the state of each |
| Date | 2026-11-16 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Kind | Record. It adds no requirement (PLN-010). |

This document lists each security weakness that a person or an agent found in Anima.ing. It is
not a GitHub issue list. A requirement that corrects a finding goes in the plan or the proposal
that owns the area.

## How to use this list

- Each entry has an identifier `SF-nnn`. Do not use an identifier again. `SEC-nnn` identifiers
  are requirements of the design specification. Do not use them here.
- An entry gives: what the weakness is, where it is (files), the effect and who can cause it,
  the mitigation that is present now, the state, and a link to its commit or its human test
  (`HT-nnn` in [need-human-testing.md](need-human-testing.md)).
- The states are Open, Accepted and Corrected:
  - Open: the weakness is present, or a correction is built but a person did not test it.
  - Accepted: the operator accepts the weakness as it is. The entry gives the reason.
  - Corrected: the correction is merged and its tests and its human test passed.
- Only the operator sets an entry to Accepted or Corrected. If the operator tells an agent the
  decision, the agent writes it in the entry with the date.
- When a correction is built, write "built" and the commit. Do not write "corrected" until the
  operator closes the entry (`AGENTS.md`, section "Human testing").
- Do not delete an entry. Move a closed entry to "Closed" at the end of this document.

## Open

| Entry | Subject | Area | State |
| --- | --- | --- | --- |
| SF-001 | "This machine" can show and probe the address of another overlay network | Desktop app | Open |
| SF-002 | The web view chooses the login server that the desktop shell trusts | Desktop app | Open |
| SF-003 | The hub selects the code that runs on a machine | Work route (contract K10, not built) | Open |
| SF-004 | A job can take the daemon token through the port of the link | Work route (contract K10, not built) | Open |
| SF-005 | A machine keeps a job after its lease ended | Work route (contract K10, not built) | Open |
| SF-006 | Each enrolled device can get the secrets of a project | Work route (contract K10, not built) | Open |
| SF-007 | The switch at the machine depended on the hub | Work route (contract K10, not built) | Open |
| SF-008 | A registration of a runner stays after its item | Work route (contract K10, not built) | Open |
| SF-009 | A project secret did not depend on the code that runs | Work route (contract K10, not built) | Open |
| SF-010 | The log of a job carries text to the client and to an AI agent | Work route (contract K10, not built) | Open |
| SF-011 | A device can hold the queue | Work route (contract K10, not built) | Open |
| SF-012 | The log of the machine could not check the hub | Work route (contract K10, not built) | Open |
| SF-014 | The hub can name an old runner program | Work route (contract K10, not built) | Open |

### SF-001: "This machine" can show and probe the address of another overlay network

Requirements: ADR-005
([cve-watch-and-network-awareness.md](proposals/cve-watch-and-network-awareness.md)),
R-060 ([rulings.md](rulings.md)).

What it is: the desktop app found its own address on the Anima VPN as the lowest IPv4 address
of this computer in `100.64.0.0/10`. Headscale, Tailscale and NetBird all give addresses in
that range. On a computer that also runs NetBird or another overlay network, the app could
select an address that is not an Anima node.

Where: `apps/desktop/src-tauri/src/this_machine.rs` (`read()`) and
`apps/desktop/src-tauri/src/this_machine/address.rs` (`vpn_address_among`, now removed).

Effect: "This machine" in Add device showed the wrong address, and the SSH check (TCP port 22)
connected to it. The device that the operator added then had a host on a different network.
"Already added" (R-060), the device `/32` in the VPN policy (R-061) and the VPN match
(VPN-011) used the same wrong address. Nobody must attack to cause it: a second overlay network
on the same computer is sufficient. An administrator of that other network controls the node
that receives the probe and, later, the sessions of the operator.

Mitigation now: built and merged to `main` (fast-forward of `claude/vpn-own-address`). No
person tested it yet.

- bdb321f1: the desktop shell asks the Tailscale client for its own address
  (`tailscale status --json`, `Self.TailscaleIPs`). It accepts the address only when the client
  runs and is logged in to the Anima login server (`tailscale debug prefs`, `ControlURL`). It
  does not read the interfaces. Each question to the client has a limit of 1 second.
- 71bfcbdb: the hub is the one source of the login server (`Query.vpnLoginServer`).

State: Open, until the human test [HT-017](need-human-testing.md#ht-017-this-machine-with-the-anima-vpn-and-netbird-together)
passes.

### SF-002: the web view chooses the login server that the desktop shell trusts

Requirements: ADR-005, R-060. Found while SF-001 was built (71bfcbdb).

What it is: the shell command `this_machine_read` takes the login server URL as an argument
from the web view (`loginServer`). The shell accepts the VPN address of a Tailscale client that
is logged in to that URL. The shell does not check the URL against the hub.

Where: `apps/desktop/src-tauri/src/this_machine.rs` (`this_machine_read`),
`apps/desktop/src-tauri/src/this_machine/vpn_client.rs` (`signed_in_to`),
`apps/client/src/lib/desktop/this-machine.ts` (`readThisMachine`).

Effect: script that runs in the web view, for example after a cross-site scripting fault, can
send a different URL. The shell then reports, and probes with a TCP connection to port 22, the
own address of this computer on a tailnet that the script chooses. The address can be the
address on a tailnet that is not Anima's, so the operator can add the wrong host. Only script in
the web view can cause it.

Mitigation now:

- The shell accepts only an IPv4 address inside `100.64.0.0/10`.
- The shell accepts only an address that the local Tailscale client reports for this computer.
  The script cannot name a different computer.
- Each question to the Tailscale client has a limit of 1 second, and the SSH check has a limit
  of 300 ms. The script cannot make the shell wait for a long time.
- No secret crosses: the shell sends no credential, token or key to the address. The SSH check
  only opens and closes a TCP connection.

Options to close it (not built):

- The shell compares the URL with an allow list, for example the login server of the hub that
  the app signed in to, kept by the shell.
- The shell takes the login server only from the hub, with its own request, and the web view
  passes nothing.

State: Open.

## The security review of the work route (task W1, 2026-11-18)

R-160 and [jobs.md](proposals/jobs.md), "Needs", ask for a security review of the work route before code. The review read the proposed contract K10, [work-route.md](../contracts/work-route.md), version 0.1. A separate reviewer agent did it; it got the text and the limits of [jobs.md](proposals/jobs.md), "Decision", and of R-160, and no opinion of the session that wrote the text. No code exists for the contract, so each entry is about the text. Version 0.2 of the contract has the changes that the entries name. Each entry stays Open: only the operator closes one (see "How to use this list"). Four needed his decision. He decided SF-004 on 2026-11-18 (R-172: correct it first). SF-003, SF-006 and SF-013 still wait for him. "Severity" is the word of the reviewer.

What the reviewer found sound in version 0.1: the result of an item is fixed codes only; the daemon refuses a member that it does not know; the machine uses the lower of two limits; a secret is given one time for one attempt, with one refusal for each cause; only the daemon opens a connection; the stop path.

The eight limits of [jobs.md](proposals/jobs.md) and R-160, as the reviewer judged version 0.1:

| Limit | Judgement | Entry |
| --- | --- | --- |
| Only the work types of a fixed list; no command from the hub | Holds in part: the hub selects the repository and the JIT configuration | SF-003 |
| A job has no access to the daemon token or the home folder | Holds in part: it depends on task W2, and the port of the link is a gap that W2 cannot close | SF-004 |
| A machine that is set off at the machine stays off | Holds in part: true with an honest hub | SF-007 |
| Each item is in the audit log, and the machine keeps its own log | Holds in part: the log of the machine was thin | SF-012 |
| The daemon listens on no port and connects only to the hub | Holds in part: no port holds; the runner program comes from GitHub (W1-P1 of the contract) | — |
| A secret for one job, recorded and deleted | Holds in part | SF-005, SF-006, SF-008 |
| A job reads the local copy and does not call the service (PRO-006) | Holds: this version gives a job no credential of such a service and no token of Anima | — |
| An installation builds only the projects of its own hub | Holds in part: only the hub says so; the daemon cannot check it | SF-003, SF-006 |

### SF-003: The hub selects the code that runs on a machine

Requirements: task W1; [work-route.md](../contracts/work-route.md). Severity: High.

What it is: The fixed list of work types has no type that runs a command (USE-006). But the hub names the repository and the commit of a `build`, and gives a `runner` a JIT configuration that the daemon passes on with no check. The code of that repository, or the steps of a workflow, then run on the machine.

Where: [work-route.md](../contracts/work-route.md), Sections 3, 4 and 6.2.

Effect, and who can cause it: A person who controls the hub, with nothing else. With R-160 that can be a person who is not the owner of the machine: a hosted hub runs on the pool. Effect: code of their choice on each machine that takes work, inside the isolation of task W2.

Mitigation now: Version 0.2 says this in plain words in Section 3. It adds a list of owners and repositories that the owner of a machine can set at the machine, and the check of the JIT configuration if task B1 shows that it can be read. The isolation of task W2 is the limit.

State: Open. The operator decided on 2026-11-18 (R-185: "A: allow-list at machine"): he accepts that the hub selects the repository, with the list at the machine. The owner of each machine lists the owners and the repositories of GitHub that it builds; the machine refuses each other item, and a machine with no list refuses each item that names a repository. Version 0.3 of the contract has this rule. No code exists: task W4 builds the list and its test. The entry stays Open until then; only the operator closes it.

### SF-004: A job can take the daemon token through the port of the link

Requirements: task W1; [work-route.md](../contracts/work-route.md). Severity: High.

What it is: In the link mode the daemon sends its token to a local port. K8 Section 1 says that a program of a local user that listens on that port when the link does not would receive the token. The work route puts code that is not trusted on the same machine. The isolation design cannot close it: it is the transport.

Where: [work-route.md](../contracts/work-route.md), Section 2; K8 Section 1.

Effect, and who can cause it: The author of a job (for example a pull request from a fork), when the link is down or starts again. Effect: the token of the machine, and with it claim, report and secret in the name of the machine.

Mitigation now: Version 0.2: a machine in the link mode takes no item until K8 has its Unix socket, with a ruling. Task W2 gets the requirement that the network of a job has no loopback address, no link interface and no private range.

State: Open. The operator decided on 2026-11-18 (R-172: "A: fix it first"): the daemon talks to the link over a Unix socket that only it can open, before any runner work. The risk is not accepted. Task W0 of [projects-and-builds-plan.md](plans/projects-and-builds-plan.md) is built on the branch `daemon/w0-link-socket` (2026-11-18): the link makes a Unix socket for the daemon that only its user can open, and the daemon sends its token to no TCP port. It is not checked on a real machine: [HT-037](need-human-testing.md). The entry stays Open until the operator closes it. What the change does not close: a program of the same user as the daemon can open the socket, so a job must run as a different user (task W2). Until then a machine that reaches its hub through the link takes no item, so the runner path does not work through a hosted hub.

### SF-005: A machine keeps a job after its lease ended

Requirements: task W1; [work-route.md](../contracts/work-route.md). Severity: High.

What it is: Version 0.1 let a job continue for 600 seconds with no contact, and the hub gave a `build` to a second machine after 90 seconds. Two machines then ran one build with its secrets. A machine whose token was revoked continued for 10 minutes.

Where: [work-route.md](../contracts/work-route.md), Sections 7 and 11.

Effect, and who can cause it: A network fault, an attacker who drops packets, or a stolen token. Effect: one build runs two times, with side effects; a recipe that publishes can run two times.

Mitigation now: Version 0.2: a `build` ends on the machine when the time of the lease has passed with no accepted report. A `runner` continues, because the hub makes no second attempt of it. A refused token ends each job at once.

State: Open. The text is changed; no code exists.

### SF-006: Each enrolled device can get the secrets of a project

Requirements: task W1; [work-route.md](../contracts/work-route.md). Severity: High.

What it is: The only test of the secret route was that the device holds the item. A second machine of the hub, or a stolen daemon token, can claim with the types that it invents and then get a project secret, a repository token or a JIT configuration.

Where: [work-route.md](../contracts/work-route.md), Sections 2 and 8; W1-P4.

Effect, and who can cause it: A second machine that is enrolled at the hub, or a person with a stolen daemon token. Effect: the secrets of a project; a JIT configuration that is used on a different computer gets the secrets of the workflow.

Mitigation now: Version 0.2: the operator names the devices that get items with secrets for each project, and the placement and the secret route check it. The audit entry of each release stays. Not in the text: a key of the device that signs each request, so that a token alone is not sufficient.

State: Open. The operator decided on 2026-11-18 (R-186: "A: device list, key later"): the device list for each project now (task W3 builds it), and the key of the device that signs each request as task W7 of [projects-and-builds-plan.md](plans/projects-and-builds-plan.md), before a machine that he does not own alone takes work. No code exists. The entry stays Open until the list is built and tested; the key is then the rest of the finding.

### SF-007: The switch at the machine depended on the hub

Requirements: task W1; [work-route.md](../contracts/work-route.md). Severity: Medium.

What it is: A daemon whose switch is off still sends a claim, and version 0.1 only said that the hub answers 204. A hub that answers with an item was not covered.

Where: [work-route.md](../contracts/work-route.md), Sections 3, 4 and 5.

Effect, and who can cause it: A person who controls the hub. Effect: work on a machine that its owner set off (USE-004).

Mitigation now: Version 0.2: the daemon drops such an answer and starts nothing; the switch is kept where no job and no other user can write it, and an update does not change it.

State: Open. The text is changed; no code exists.

### SF-008: A registration of a runner stays after its item

Requirements: task W1; [work-route.md](../contracts/work-route.md). Severity: Medium.

What it is: The hub makes the registration at GitHub when the machine asks for the secret. Version 0.1 removed it only when a lease ended. After a lost answer, or after a report `failed`, a configuration that works was left.

Where: [work-route.md](../contracts/work-route.md), Sections 6.2, 7 and 8.

Effect, and who can cause it: A machine that reports a false state, or a lost answer. Effect: a runner that the hub does not know can take a job of the repository.

Mitigation now: Version 0.2: the hub removes the registration at each end of an attempt and does not trust the report; it revokes the repository token.

State: Open. The text is changed; no code exists.

### SF-009: A project secret did not depend on the code that runs

Requirements: task W1; [work-route.md](../contracts/work-route.md). Severity: Medium.

What it is: `commit` was any 40 characters, and nothing in an item said that the code is of a fork or of a scratch ref. The rule that such code gets no project secret (PRO-030, JOB-020) was only in the hub, outside the contract.

Where: [work-route.md](../contracts/work-route.md), Sections 6.1 and 8.

Effect, and who can cause it: The author of a pull request from a fork, if the hub makes an error; a caller with an API key (JOB-020). Effect: a project secret in code that is not trusted.

Mitigation now: Version 0.2: the member `origin`; the hub writes `default_branch` only after it verified the commit; an item with `other` names no project secret, and the daemon refuses one that does.

State: Open. The text is changed; no code exists.

### SF-010: The log of a job carries text to the client and to an AI agent

Requirements: task W1; [work-route.md](../contracts/work-route.md). Severity: Medium.

What it is: The log of a `build` is up to 2 MiB of output of code that is not trusted. A length limit does not remove control characters or direction marks, and it does not stop text that gives instructions to an AI agent. A job that has a secret can write it in a form that no pattern finds.

Where: [work-route.md](../contracts/work-route.md), Section 7.

Effect, and who can cause it: The author of a job. Effect: false text in the client, instructions to an agent that reads the log through MCP, and a way out for a project secret.

Mitigation now: Version 0.2: the two sides remove control and direction characters and cut each line; MCP gives a log only as text that is not trusted; the text says that redaction is a best effort and that the control is to give no secret to code that is not trusted (SF-009).

State: Open. The text is changed; no code exists. The risk of instructions in a log stays for each agent that reads logs (MCP-008).

### SF-011: A device can hold the queue

Requirements: task W1; [work-route.md](../contracts/work-route.md). Severity: Medium.

What it is: A refusal was not an attempt and had no limit. A device could claim and refuse with no end, or report `preparing` with no end. A lost answer to a claim used one of the three attempts.

Where: [work-route.md](../contracts/work-route.md), Sections 4, 5 and 7.

Effect, and who can cause it: An enrolled machine with a fault, or a stolen token. Effect: items do not run; a build fails after three lost answers.

Mitigation now: Version 0.2: the hub ends an attempt at its time limit; a device with 5 refusals or lost leases in 1 hour gets no item for 1 hour; a lease that the machine does not name in `holds` ends with no attempt counted.

State: Open. The text is changed; no code exists.

### SF-012: The log of the machine could not check the hub

Requirements: task W1; [work-route.md](../contracts/work-route.md). Severity: Low.

What it is: The machine's own log had only the id, the type, the repository, the times and the outcome, and the text did not say when it is written or who can write it.

Where: [work-route.md](../contracts/work-route.md), Section 9.

Effect, and who can cause it: A person who controls the hub; a job. Effect: the owner of a machine cannot see afterwards what the hub made it run.

Mitigation now: Version 0.2: the entry is written before the start, also for a refused item, with the commit, the recipe or the runner program and the purposes of the secrets, outside the isolation.

State: Open. The text is changed; no code exists.

### SF-014: The hub can name an old runner program

Requirements: task W1; [work-route.md](../contracts/work-route.md). Severity: Note.

What it is: The hub names the version and the SHA-256 of the runner program. A hub could name an old version with a known fault.

Where: [work-route.md](../contracts/work-route.md), Section 6.2.

Effect, and who can cause it: A person who controls the hub. Effect: small, inside the isolation.

Mitigation now: Version 0.2: the daemon holds the lowest version that it accepts.

State: Open. The text is changed; no code exists.

## Closed

| Entry | Subject | Area | State |
| --- | --- | --- | --- |
| SF-013 | A job can take its machine out of work | Work route (contract K10, not built) | Accepted (R-187) |

### SF-013: A job can take its machine out of work

Requirements: task W1; [work-route.md](../contracts/work-route.md). Severity: Low.

What it is: A job that makes the clean-up fail leaves the machine `unclean` until its owner acts. With warm runners on a public repository, one pull request from a fork for each machine does it.

Where: [work-route.md](../contracts/work-route.md), Section 9.

Effect, and who can cause it: The author of a job. Effect: no machine takes work.

Mitigation now: None in the contract: a machine that is not clean must not take the next job. The isolation of task W2 and the approval rule of GitHub (F18) are the controls. Version 0.2 says so.

State: Accepted (operator, 2026-11-18, R-187: "A: accept"; relayed by the orchestrating session). The reason: a machine that could not clean up must take no more work until its owner clears it, and the other choice, a machine that takes the next job when it is not clean, is worse. The controls stay the isolation of task W2 and the approval rule of GitHub (F18).
