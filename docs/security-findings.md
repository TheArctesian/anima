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
| SF-015 | The profile `contained` is a list of refused things, and many keys of Compose that are not in it give the rights of the machine | Container work types (proposed text for contract K10, not built) | Open |
| SF-016 | A bind path below the folder of the application passes the rule with a symbolic link, and a container can write the Compose file that the daemon will start | Container work types (proposed text for contract K10, not built) | Open |
| SF-017 | The check and the start do not read the same input | Container work types (proposed text for contract K10, not built) | Open |
| SF-018 | The right to use Docker makes the user of the daemon an administrator of the machine, and the profile is a restraint of the same program | Container work types (proposed text for contract K10, not built) | Open |
| SF-019 | `adopted_apply` takes the path of the file from labels that each user of Docker can write, and runs it with no profile | Container work types (proposed text for contract K10, not built) | Open |
| SF-020 | `container_action` works on each container of the machine, so the adopted list does not limit it, and an API key can stop the things that the daemon and the hub need | Container work types (proposed text for contract K10, not built) | Open |
| SF-021 | `docker_prune` and `application_remove` let an API key delete each container of the machine, and the rule for volumes is only in the hub | Container work types (proposed text for contract K10, not built) | Open |
| SF-022 | `container_follow` gives the hub the log of each container of the machine | Container work types (proposed text for contract K10, not built) | Open |
| SF-023 | The check of the hub reads the files and the addresses of the hub | Container work types (proposed text for contract K10, not built) | Open |
| SF-024 | The reader opens files outside the folder before a rule refuses them, and the text does not say what the log of an `application_apply` holds | Container work types (proposed text for contract K10, not built) | Open |
| SF-025 | `application_back` starts a file that the profile of today did not check, and the text does not say how it sets the images | Container work types (proposed text for contract K10, not built) | Open |
| SF-026 | The environment file breaks three rules of Section 8, and the device list of SF-006 does not protect it | Container work types (proposed text for contract K10, not built) | Open |
| SF-027 | A published port and the network of a container are permitted with no rule | Container work types (proposed text for contract K10, not built) | Open |
| SF-028 | The project name is the first 12 characters of a UUID, and the form of the UUID has no rule | Container work types (proposed text for contract K10, not built) | Open |
| SF-029 | The limits of an item do not apply to the containers that it starts, and one item holds the one slot of the machine | Container work types (proposed text for contract K10, not built) | Open |
| SF-030 | To set the switch off, or the profile back to `contained`, does not change what runs | Container work types (proposed text for contract K10, not built) | Open |
| SF-031 | An image is named by a tag, and a file can start each image that the machine has or can pull | Container work types (proposed text for contract K10, not built) | Open |
| SF-032 | The log of the machine has no members for the container types | Container work types (proposed text for contract K10, not built) | Open |
| SF-033 | Points that an implementer must guess | Container work types (proposed text for contract K10, not built) | Open |

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

## The security review of the container work types (task H14, 2026-11-19)

R-209 (operator, 2026-11-19) approves Section 4 of [hosted-applications-stage-2.md](proposals/hosted-applications-stage-2.md) as the text of the container work types for a security review; it becomes version 0.4 of contract K10 when the review has no finding that he has not decided. A separate reviewer agent did the review on 2026-11-19. It got Sections 4.1 to 4.3 of the proposal as one file, the contract [work-route.md](../contracts/work-route.md) version 0.3, K8 and the entries SF-003 to SF-014. It was told not to read the proposal itself, which holds the judgement of the session that wrote the text, and it says that it did not. No code exists, so each entry is about the text. The entries below are the text of the reviewer; only the identifiers were changed, from its provisional numbers to SF-015 to SF-033, and a line "State" was added to each.

The reviewer found 19 weaknesses: 7 of severity High, 10 Medium and 2 Low. The largest: the profile `contained` is a list of refused things, and many keys of Compose that it does not name give the rights of the machine (SF-015). Thus the sentence of the proposed text, that a file of the profile `contained` cannot ask for the rights of the machine, is not true as written.

Two facts of Docker were checked by the reviewer against public documentation (a named volume of the `local` driver calls `mount` with the options of the file; `--volumes-from` copies the volume definitions of a different container). Each other behaviour of Docker or Compose in the entries is marked "not verified" where it was not checked.

Eight choices need the operator. They are the open decisions OD-HOST-14 to OD-HOST-21 of [hosted-applications-plan.md](plans/hosted-applications-plan.md), each with the options and the recommendation of the reviewer. The text of the reviewer for them:

1. **SF-018, how the daemon holds the right to use Docker.** Options: (A) the user of the daemon is in the group `docker`: simple; a compromise of that user or of the daemon is a compromise of the machine, and the profile protects only against an honest daemon. (B) a small helper that runs as root, holds the profile, the switch and the lists, does the check of 6.4.3 and is the only program that speaks to Docker; the daemon that speaks to the hub has no right to Docker. (C) rootless Docker for Anima's applications only: the limit is the kernel, but adopted projects and other containers are out of reach. Recommendation: B. With A, write in Section 3 that the switch makes the user of the daemon an administrator, and do not offer the profile as a control against a hub that is not trusted.
2. **SF-020, SF-021, SF-022, the scope of the container types.** Options: (A) each container of the machine, as the text; (B) only applications of Anima and adopted projects, with a setting "All containers" at the machine, off by default. Recommendation: B. The adopted list has no meaning with A.
3. **SF-021, removal of volumes with a hosted hub.** The actor rule is in the hub only. Options: (A) accept that a person who controls the hub can delete the data of each application; (B) a setting at the machine, off by default, "The hub may delete volumes"; (C) wait for the key of the device (W7) and a signature of the operator on such an item. Recommendation: B now, C with W7.
4. **SF-019, `adopted_apply` with no profile and with `pull`.** Options: (A) as the text, with the recorded paths of SF-019; (B) no `pull` for an adopted project; (C) the owner sets `pull` for each adopted project at the machine. Recommendation: C, off by default.
5. **SF-027, published ports and the network of a container.** Options: (A) each port on each address, as the text; (B) `127.0.0.1` by default and all addresses by a setting at the machine. Recommendation: B. The reach of a container into the local network and the overlay network cannot be closed in this version; it must be said in Section 3 and accepted.
6. **The profile `full` at all (Section 3, 6.4.3).** With `full`, each API key and each agent that can make `application_apply` can become the administrator of the machine, and the text has no actor rule for it. Options: (A) keep `full`, and the hub makes an `application_apply` for a `full` machine only for a login or a browser session; (B) no `full` in this version: a file that needs more goes through adoption at the machine. Recommendation: B; `adopted_apply` already serves the owner who wants a file with no limit.
7. **SF-031, images by tag.** Options: (A) accept tags; (B) a digest for each recipe of the catalogue, tags for the files of the operator. Recommendation: B.
8. **SF-026, `application_apply` before task W7.** A stolen daemon token of the machine gets the environment file. Options: (A) accept until W7; (B) no `environment_file` through the route before W7 on a machine that the operator does not own alone. Recommendation: B, the same condition as R-186.

What the reviewer checked and found sound:

- `container_action` and `container_follow` take only a full identifier of 64 hexadecimal characters that the engine must have; there is no name, no short form, no `kill`, `pause` or `remove`. No injection through that member.
- The daemon starts `docker` with a list of arguments and no shell; no container type takes a path or a command line as a member of `parameters`.
- `file_not_permitted` reports a fixed code and no text of the file; the results (`health`, the counts of the prune, `data_not_restored`) are fixed codes and numbers, as Section 7 asks.
- The check of the machine is named as the control and the check of the hub as a courtesy. That order is correct (the fault of the hub check is SF-023).
- `adopted_apply` has no `down` and does not change the file; the hub names no path in any type.
- The project name has a fixed prefix that the hub does not choose (the fault is the length, SF-028).
- The SHA-256 of the two files binds the item to the file that the hub sent, and gives the log of the machine a value to record. It is not a control against a person who controls the hub, and the text does not claim that it is.
- The log of `container_follow` takes the rules of Section 7 (control characters, line limit, 2 MiB), and no MCP tool returns it; the rest of SF-010 is unchanged and not reopened.
- No type exists for `exec`, a terminal, or reading a file of a volume; the terminal is left to its own review (the limit of that statement is SF-031 c).
- SF-004 (the link port), SF-005, SF-008, SF-009, SF-011 and SF-014 are not reopened by the new text, apart from what SF-018, SF-026, SF-029 and SF-033 name.

### SF-015: The profile `contained` is a list of refused things, and many keys of Compose that are not in it give the rights of the machine

Requirements: task H14; the proposed text of the container work types, Section 6.4.3; Section 3 (the new paragraph). Severity: High.

What it is: The table refuses seven groups. Each key that the table does not name is permitted. Compose has many more keys, and each new version of Compose adds keys. The daemon reads the file with the Compose program of the machine, so a newer Compose gives a final form with keys that the daemon does not know. The sentence "such a file cannot ask for the rights of the machine" is not true with this table.

Where: Section 6.4.3, the table and the sentence "A named volume, a published port and a network of the project are permitted". The rules speak only of "a service"; no rule speaks of the top-level `volumes`, `networks`, `secrets`, `configs`, `name`, `include`.

Effect, and who can cause it: A person who controls the hub, and an API key or MCP tool if the hub lets it make `application_apply` (the text has no rule), on a machine with `contained`. Concrete ways:

- A named volume with `driver: local` and `driver_opts` `type: none`, `o: bind`, `device: /` mounts each path of the machine. With `type: ext4`, `device: /dev/sda1` it mounts a disk. With `type: nfs` or `cifs` it mounts a remote share. This passes `bind_outside` and `docker_socket`. (Verified in part: the Docker documentation shows the block device and says that the driver calls `mount`.)
- `volumes_from: ["container:<name>"]` gives each mount of a different container of the machine, also its bind mounts. If one container of the machine has the Docker socket (Portainer, Traefik, Watchtower), the service gets the socket. It also gives the data of each other project. (Verified in part.)
- A top-level volume with `external: true` and a `name` mounts the volume of a different project.
- `device_cgroup_rules: ["b *:* rwm"]`. The default capabilities of Docker include `MKNOD`, so the container makes a node of the disk and reads and writes it. `privileged` names only "a device", an implementer will read that as `devices`. (Not verified.)
- `pid: "container:<id>"`, `ipc: "container:<id>"`, `network_mode: "container:<id>"` or `"service:<x>"`. `host_namespace` names only the namespaces "of the machine". With the process list of a different container the service reads `/proc/<pid>/environ` and `/proc/<pid>/root` of it. (Not verified.)
- `security_opt` with `systempaths=unconfined` or `label=disable`. The code `security_off` names three examples and the judgement "turns a protection off".
- `gpus`, `deploy.resources.reservations.devices`, a CDI device name, `runtime`, `cgroup: host`, `cgroup_parent`, `userns_mode: host` (the text says "the user list", which is not a term of Docker), `uts: host`, `sysctls`, `oom_score_adj: -1000`, `use_api_socket` (a newer key that gives the socket of the engine to the service; not verified), `provider` (Compose starts a program of the machine, not a container; not verified), `models`.
- A top-level network with `external: true` (the network of a different project), with the driver `macvlan` or `ipvlan` and a `parent` interface, or with `driver_opts` such as `com.docker.network.bridge.name`.
- `secrets` and `configs` with `environment:` read a variable of the process that runs Compose, which is the daemon.
- `logging` with a driver that sends lines to an address (`syslog`, `gelf`, `fluentd`).
- A top-level `name` changes the project name if the daemon does not give `-p` (the text does not say that it does).

Mitigation now: The rules are applied to the final form, which is correct. The list itself is the fault.

Proposed change: Replace the table of refused things by a list of permitted things, with the rule of Section 5 ("a daemon refuses a member that it does not know"):

"With the profile `contained` the daemon refuses a file whose final form has a key, at any level, that is not in the list below, or a value that breaks its rule (`file_not_permitted`, code `key_not_permitted`). The list is fixed in the daemon program. Services: `image`, `command`, `entrypoint`, `environment`, `restart`, `depends_on`, `healthcheck`, `user`, `working_dir`, `hostname`, `stop_signal`, `stop_grace_period`, `read_only`, `tmpfs`, `init`, `cap_drop`, `expose`, `mem_limit`, `cpus`, `pids_limit`, `labels` (no key that starts with `com.docker.`), `logging` (driver `json-file` or `local` only), `security_opt` (only `no-new-privileges:true`), `ports` (rule of SF-027), `volumes` (type `volume` with a volume of this file, or type `bind` with the rule of SF-016; no other type), `networks` (networks of this file only). Top level: `services`; `volumes` with no `driver`, no `driver_opts`, no `external` and no `name`; `networks` with no `driver` other than `bridge`, no `driver_opts`, no `external`, no `name`; `secrets` and `configs` with `content`, or `file` below the folder of the application. `name`, `include`, `extends` with `file`, and each other key are refused. The daemon always gives the project name and the file path as arguments."

Correct the sentence of Section 3 only after this change.

Confirmed from the text; each named way depends on Docker behaviour as marked.

State: Open. The text is a proposal and no code exists. The proposed change is a change of the text, for the session that writes version 0.4; it needs no decision of the operator unless that session cannot make it.

### SF-016: A bind path below the folder of the application passes the rule with a symbolic link, and a container can write the Compose file that the daemon will start

Requirements: task H14; the proposed text of the container work types, Sections 6.4.3 (`bind_outside`, `file_outside`), 6.4.4, 6.4.7. Severity: High.

What it is: The folder of an application holds the Compose file, the environment file, the kept files of earlier versions and the bind mounts. A container is root in its bind mount (Docker has no user remap by default). The text does not say that the path check follows links, and it does not refuse a bind mount of the folder itself.

Where: Section 6.4.3, "A bind mount whose path on the machine is not below the folder of the application"; Section 6.4.4, "The folder holds the Compose file, the environment file, and the bind mounts".

Effect, and who can cause it: (a) A person who controls the hub, in two items, on a `contained` machine. Item 1: a service with the bind mount `./d:/d` and the command `ln -s / /d/x`. Item 2: a service with the bind mount `./d/x:/host`. The text of the path is below the folder; Docker follows the link and mounts `/`. (b) The author of an image, with a recipe that binds `.` or a folder that holds the files: the container changes `compose.yaml`, the `.env` file, a kept earlier file, or adds `compose.override.yaml`. The next `application_back`, the next start by the daemon, or the "ordinary `docker compose` command" of the owner then starts a file that no check saw. (c) The author of an image and a local user: the container writes a set-user-ID file of root in its bind mount; a local user who can reach the folder runs it. The text sets the access only for the environment file, not for the folder.

Mitigation now: None in the text.

Proposed change: Section 6.4.4: "The folder of an application has two parts. `files/` holds the Compose file, the environment file and the kept files; no mount can name it. `binds/` holds one folder for each bind mount. A bind mount of the profile `contained` names exactly `binds/<name>`, where `<name>` is 1 to 64 characters of `a-z`, `0-9`, `-`, `_`; a longer path is refused. The daemon makes that folder itself, and directly before the start it checks, without following a link, that it is a real folder below a real `binds/`. The top folder of all applications is owned by the user of the daemon (or root) and has no access for the group or others. The daemon gives the path of the Compose file as an argument, so that no override file is read." Add to 6.4.3 that the check of a path is on the real path, not on its text. Say that the owner must not run `docker compose` in `files/` with files that he did not read, or drop the sentence "An ordinary `docker compose` command in that folder works".

(a) depends on Docker following a link in a bind source: not verified, but well known. (b) and (c) are confirmed from the text.

State: Open. The text is a proposal and no code exists. The proposed change is a change of the text, for the session that writes version 0.4; it needs no decision of the operator unless that session cannot make it.

### SF-017: The check and the start do not read the same input

Requirements: task H14; the proposed text of the container work types, Sections 6.4.2, 6.4.3. Severity: High.

What it is: The daemon checks "the final form" and then starts `docker compose up`, which reads the file again. The text does not say that the two readings have the same input. The SHA-256 covers only the main file.

Where: Section 6.4.3, first paragraph; Section 6.4.2, "The machine checks the file ... before it writes anything".

Effect, and who can cause it: A person who controls the hub, or the author of an image with a bind mount. Ways:

- `include`, `extends: file:` or `env_file` name a file in a bind mount. A container of the earlier version, which still runs, changes that file after the check.
- `include` with a remote source (a Git or OCI address, where Compose permits it; not verified) gives one content to the check and a different content to the start.
- A variable `${X}` of the file comes from the `.env` file and from the environment of the process. If the check runs in a different folder, with a different environment, or before the environment file is written ("before it writes anything"), the final form of the check is not the final form of the start. Example: `privileged: ${P:-false}`.
- The `.env` file can set `COMPOSE_FILE`, `COMPOSE_PROFILES` and `COMPOSE_PROJECT_NAME` (not verified). A service with `profiles` is not in the default final form, and the owner or a later command can start it.
- The text does not say what "the environment file" is: the `.env` file that fills variables of the Compose file, or a file that `env_file` names. An implementer must guess.
- A variable of the file also reads the environment of the daemon, so a file can copy a variable of the daemon into a container.

Mitigation now: The rule is applied to the final form. That is necessary and not sufficient.

Proposed change: Section 6.4.3: "The daemon makes the final form one time, with all profiles on, in the folder `files/`, with an empty environment and the environment file of the item as the only source of variables. It refuses a file whose reading needs a file outside `files/` or an address. It refuses an environment file that sets a name that starts with `COMPOSE_` or `DOCKER_`. It applies the rules to that final form, writes the final form to `files/`, and starts the project from the final form only, with the project name and the file path as arguments and the same empty environment. The log of the machine records the SHA-256 of the final form." Section 6.4.4 then says that the file in the folder is the final form (this changes "Anima keeps no ... private form": the final form is still a plain Compose file). Define "environment file" in Section 6.4.2.

Confirmed from the text; the single ways depend on Compose behaviour as marked.

State: Open. The text is a proposal and no code exists. The proposed change is a change of the text, for the session that writes version 0.4; it needs no decision of the operator unless that session cannot make it.

### SF-018: The right to use Docker makes the user of the daemon an administrator of the machine, and the profile is a restraint of the same program

Requirements: task H14; the proposed text of the container work types, Section 13, W1-P6; Section 3 (two new rows); K8 Sections 1 and 2. Severity: High.

What it is: On Linux the right to use Docker is membership of the group `docker` or root. Each is the right of the administrator. K8 says that the daemon runs as a user, that its token is in a file of that user, and that "a program of the same user ... can also read the token store". After the switch goes on, each program of that user can use Docker with no profile. The profile, the switch and the adopted list are then checks inside a program that already holds the full right. Three more gaps: (1) the text says that the administrator is needed for the switch, but not for the profile `full` and not for the adopted list; (2) "setting the switch off takes the right back" is not true for a process that runs (a group stays until the process ends) and not possible where the user owns the socket (Docker Desktop on a Mac, rootless Docker); (3) the text does not say what happens to the switch, the profile and the adopted list when the daemon is enrolled at a different hub. `anima-daemon enrol --hub` is a command of the user of the daemon.

Where: W1-P6; Section 3, the row "If the machine takes container work"; K8 Section 2, the token file.

Effect, and who can cause it: (a) A program of the user of the daemon (a faulty program, a job if W2 fails): administrator of the machine. (b) A fault in the daemon that a hub or a Compose file can reach (the daemon now parses 256 KiB of YAML from the hub and the output of Docker): administrator, not a user. (c) A local user who is not the administrator and who runs the daemon: after the administrator set the switch on, that user enrols the daemon at a hub of their own and, with `full` or with SF-015, becomes the administrator. (d) A person with a stolen daemon token cannot make an item, but can claim the items of that machine and get the environment file (SF-026).

Mitigation now: W1-P6 names the question and gives it to task W2. The text gives no limit.

Proposed change: Decide W1-P6 before the contract is accepted, not in W2; the contract cannot be judged without it. The choice is the owner's (see the list below). Whatever the choice, add to Section 3: "The switch 'Manage applications', the profile and the list of adopted projects are in a file that only the administrator of the machine can write; the daemon reads it. They are bound to the origin of the hub and the device identifier at the time they were set. An enrolment at a different origin, or as a different device, sets the switch off, the profile to `contained` and the list to empty." And add to the paragraph of Section 3: "On a machine whose switch is on, a person who controls the user of the daemon is the administrator of the machine."

Confirmed from the text and from K8. The statement on the group `docker` is general Docker knowledge.

State: Open. The text is a proposal and no code exists. It needs a decision of the operator: OD-HOST-14 of [hosted-applications-plan.md](plans/hosted-applications-plan.md).

### SF-019: `adopted_apply` takes the path of the file from labels that each user of Docker can write, and runs it with no profile

Requirements: task H14; the proposed text of the container work types, Section 6.4.5. Severity: High.

What it is: The adopted list holds only project names. The path of the file comes from the labels `com.docker.compose.project.config_files` and `working_dir` of a container at the time of the item. Each party that can make a container can write those labels. The file at that path, and its `.env`, its includes and its `build` folders, can be writable by a user who is not the administrator. The profile does not apply, on the reason "the file is the file of the owner"; the daemon cannot know that.

Where: Section 6.4.5, second and fifth items.

Effect, and who can cause it: (a) A local user who can write the folder of an adopted project (a project in a home folder or a shared folder, started one time by the administrator): the user changes the file to ask for `privileged`, then waits for the next `adopted_apply`, or asks for it through the hub. Effect: administrator. (b) A party that can make one container with the label of an adopted project name and a path of its choice. If a file of `application_apply` can set those labels on a service (Compose puts its own labels last, as far as I know; not verified), a person who controls the hub goes from `contained` to no limit. (c) A person who controls the hub, with `pull: true`: the adopted project gets the image that the registry has now. If that project has the Docker socket, the author of that image, or a person who took the tag, is the administrator. (d) `up` applies a file that the owner changed and did not yet want to apply, and builds a `build` section.

Mitigation now: The owner adopts by name at the machine. `down` does not exist.

Proposed change: "At adoption the daemon records, for each project, the real path of each Compose file and of the project folder, as the labels give them at that time, and shows them to the owner. `adopted_apply` uses the recorded paths, never the labels of the time of the item. It refuses the item when a recorded file, or a folder above it, can be written by a user other than root or the user that adopted it (`not_adopted`). It gives the paths and the project name as arguments. `up` never builds (`--no-build`)." Add to 6.4.3 that a label whose key starts with `com.docker.` is refused in each profile. For `pull` on an adopted project the owner must choose (list below).

(a), (c) and (d) are confirmed from the text. (b) depends on Compose behaviour that I did not verify.

State: Open. The text is a proposal and no code exists. It needs a decision of the operator: OD-HOST-17 of [hosted-applications-plan.md](plans/hosted-applications-plan.md).

### SF-020: `container_action` works on each container of the machine, so the adopted list does not limit it, and an API key can stop the things that the daemon and the hub need

Requirements: task H14; the proposed text of the container work types, Section 6.4.1; Section 3 (the new paragraph). Severity: High.

What it is: The only test is that the identifier exists. The text gives no rule on which actor may make the item. The control `not_adopted` exists for `adopted_apply`, but `stop`, `start` and `restart` of the same containers need no adoption.

Where: Section 6.4.1.

Effect, and who can cause it: An API key, an AI agent through an MCP tool, or a person who controls the hub. (a) `stop` of the container of the VPN, the DNS, the reverse proxy or the link of the machine: the machine loses its network and the daemon cannot reach the hub; only a person at the machine can correct it. (b) Where the hub and its PostgreSQL run in Docker on a machine with the daemon: `stop` of the database of the hub. No item can then start it again. (c) `stop` of the isolation of a `build` or a `runner` item, if task W2 uses the same Docker (the text does not say). (d) `start` of a container that stopped: a one-time container runs its command again (a migration, a restore, a clean-up, a stopped `--privileged` tool). (e) An agent that reads text that is not trusted (SF-010) can be led to do each of these.

Mitigation now: The identifier is full and checked; there is no `kill`, `pause` or `remove`. The Section 3 paragraph says that a person who controls the hub "can stop each container".

Proposed change: The owner chooses the scope (list below). My proposal: "`container_action` works only on a container of an application that Anima installed, or of an adopted project (the daemon reads the project label and checks it against its own records). Each other container is refused (`not_adopted`), unless the owner set 'All containers' at the machine. The daemon holds a list of protected containers that the owner sets at the machine; it refuses each item for them. `start` is permitted only for a container whose restart policy is not `no`, or that Anima installed." And in the hub requirements: which actor kinds may make each container type.

Confirmed from the text.

State: Open. The text is a proposal and no code exists. It needs a decision of the operator: OD-HOST-15 of [hosted-applications-plan.md](plans/hosted-applications-plan.md).

### SF-021: `docker_prune` and `application_remove` let an API key delete each container of the machine, and the rule for volumes is only in the hub

Requirements: task H14; the proposed text of the container work types, Sections 6.4.6, 6.4.8. Severity: High.

What it is: `docker_prune` with `containers` removes each stopped container of the machine, not only those of Anima. With `container_action` `stop` first (SF-020), an API key or MCP tool removes each container: its writable layer, its log, and for a container made with `docker run` its whole configuration. The rule "`volumes` only for a login or a browser session" is a rule of the hub; the machine cannot see the actor kind. The text does not say if `images` means images with no tag or each image with no container, and if `volumes` means anonymous volumes or each volume with no container.

Where: Section 6.4.8; Section 6.4.6, first item.

Effect, and who can cause it: An API key or an AI agent: each container of the machine is deleted, and each image, also the images that `keep` holds for `application_back` (they have no container). A person who controls the hub: the same, and then `volumes`, which deletes the data of each project of the machine, also of projects that the owner did not adopt. This is more than the Section 3 paragraph says ("can stop each container").

Mitigation now: The hub rule for volumes. `application_remove` names only an application of Anima.

Proposed change: "`docker_prune` removes only things that have the project label of an application that Anima installed, or, for `images`, images with no tag and images that the daemon pulled for an application and no container uses. The daemon never removes an image that `keep` holds. `containers` and `volumes` of other projects need the setting 'All containers' of the owner at the machine. A volume is removed only by `application_remove` of its application." For a hosted hub, the owner must choose if removal of volumes needs a second control at the machine (list below). Define the two words exactly, with the Docker command and its flags.

Confirmed from the text; the meaning of the Docker commands is the undefined point.

State: Open. The text is a proposal and no code exists. It needs a decision of the operator: OD-HOST-16 (and OD-HOST-15) of [hosted-applications-plan.md](plans/hosted-applications-plan.md).

### SF-022: `container_follow` gives the hub the log of each container of the machine

Requirements: task H14; the proposed text of the container work types, Section 6.4.9. Severity: Medium.

What it is: The item names any container. The rules "only to a login or a browser session" and "the hub deletes the lines when the item ends" are rules of the hub. `tail: 200` sends old lines, not only new lines; the text gives `tail` no limit.

Where: Section 6.4.9.

Effect, and who can cause it: A person who controls the hub (with a hosted hub, not the owner): the logs of each container of the machine, also of projects that are not adopted, with the secrets that they hold, by one item after the other. An API key, if the hub lets it make the item: the lines leave the machine and are in the hub for 900 seconds, and the item holds the one slot of the machine (SF-029). The Section 3 paragraph does not name this power.

Mitigation now: The redaction and the character rules of Section 7; the hub rules above.

Proposed change: "`container_follow` works on the containers that `container_action` works on (SF-020). `tail` is 0 to 1,000. The hub makes the item only for an act with a login or a browser session." Add the power to the Section 3 paragraph: "can read the log of each such container".

Confirmed from the text.

State: Open. The text is a proposal and no code exists. It needs a decision of the operator: OD-HOST-15 of [hosted-applications-plan.md](plans/hosted-applications-plan.md).

### SF-023: The check of the hub reads the files and the addresses of the hub

Requirements: task H14; the proposed text of the container work types, Section 6.4.3, last paragraph. Severity: Medium (High for a hosted hub that has more than one operator).

What it is: "The hub applies the same rules" to the final form. To make the final form, the reader opens each file that `include`, `extends`, `env_file`, a secret `file` or a config `file` names, and can get a remote include. On the hub those paths are paths of the hub.

Where: Section 6.4.3, "The hub applies the same rules before it makes an item".

Effect, and who can cause it: An operator of a hosted hub, an API key, or an agent that can send a Compose file. `env_file: /proc/self/environ` or a path of a secret of the hub goes into the final form or into an error text that the editor shows (HOST-022). A remote include makes the hub send a request to an address in its own network.

Mitigation now: None in the text.

Proposed change: "The hub checks the text of the file only. It refuses, before it reads anything, a file with `include`, `extends` with `file`, `env_file`, or a `file` of a secret or config that is not a relative path with no `..`. It opens no file and no address for the check, and it shows no text of an error of the reader."

Confirmed from the text; it depends on how the hub makes the final form.

State: Open. The text is a proposal and no code exists. The proposed change is a change of the text, for the session that writes version 0.4; it needs no decision of the operator unless that session cannot make it.

### SF-024: The reader opens files outside the folder before a rule refuses them, and the text does not say what the log of an `application_apply` holds

Requirements: task H14; the proposed text of the container work types, Sections 6.4.2, 6.4.3, Section 7 (log). Severity: Medium.

What it is: `file_outside` is applied to the final form, thus after the reader opened `env_file: /home/<user>/.ssh/id_ed25519` or a file of the daemon. The report has a fixed code, but the text does not say if the output of the Compose program goes into `log.text` of the item. An error of the environment reader can quote the line that it could not read (not verified).

Where: Section 6.4.3, first paragraph; Section 7, "Log" has rules for `build` and `runner` only.

Effect, and who can cause it: A person who controls the hub, or an API key that can make the item: lines of a file that the user of the daemon can read reach the hub in the log of the item. The redaction finds only known patterns.

Mitigation now: "The report names the rule by a fixed code, never text of the file."

Proposed change: Section 7: "For a container type other than `container_follow`, the log holds only lines of the daemon: the step, the fixed code, the identifier of each image. It holds no output of the Docker program." Section 6.4.3: "Before it gives the file to the reader, the daemon refuses a file that names a path that is absolute or has `..`" (with SF-017, a file outside `files/` is never opened).

Confirmed from the text; the content of the error text is not verified.

State: Open. The text is a proposal and no code exists. The proposed change is a change of the text, for the session that writes version 0.4; it needs no decision of the operator unless that session cannot make it.

### SF-025: `application_back` starts a file that the profile of today did not check, and the text does not say how it sets the images

Requirements: task H14; the proposed text of the container work types, Section 6.4.7; Section 6.4.2 (`keep`). Severity: Medium.

What it is: The item brings back "the Compose file and environment file of that time". The text does not say that the daemon applies the profile again. It does not say how the project gets the earlier image identifiers: a change of the file, or a new tag on the old image.

Where: Section 6.4.7.

Effect, and who can cause it: (a) An API key or a person who controls the hub: a machine was `full`, the owner set it back to `contained`; `application_back` starts the earlier file with its `privileged` service. The same after an update of the daemon with a new rule. (b) If the daemon sets the tag `name:tag` on the old image, each other project of the machine that uses that tag gets the old image at its next start. (c) The kept file is in the folder, so SF-016 (b) applies. (d) The item brings an application back to an image with a known fault (as SF-014). (e) `docker_prune` with `images` removes the kept images (SF-021).

Mitigation now: The identifiers of the images are kept, not the tags. `data_not_restored` is said.

Proposed change: "`application_back` applies the rules of the profile of the machine at that time to the kept file, as `application_apply` does, and records the profile. The kept files are in `files/` and no mount can name them. The daemon starts the earlier images by identifier in a copy of the final form; it sets no tag. One `application_back` goes back one step; a second one is refused until the next `application_apply`."

Confirmed from the text.

State: Open. The text is a proposal and no code exists. The proposed change is a change of the text, for the session that writes version 0.4; it needs no decision of the operator unless that session cannot make it.

### SF-026: The environment file breaks three rules of Section 8, and the device list of SF-006 does not protect it

Requirements: task H14; the proposed text of the container work types, Section 8 (two new purposes); Sections 6.4.4, 6.4.7, 9. Severity: Medium.

What it is: Section 8 says that the machine "writes the value to no file outside the isolation", "deletes the value when the step has it", and asks "only after the isolation exists". An item of the kind `host` has no isolation, and the environment file stays in the folder, with the copies of earlier versions for `application_back`. The table says "Life: the item", which is not true. The control of SF-006 (the operator names the devices of a project) becomes "the one machine of the application": a stolen token of that machine passes it. The text does not say what `project` and `repository` of the `Item` are for a container type.

Where: Section 8 of the proposed text; Section 6.4.4, "Only the daemon and the administrator of the machine can read the environment file"; Section 9, "if it made one outside the folder".

Effect, and who can cause it: A person with the daemon token of the machine (SF-018 says who can read it): they send claims in place of the daemon, hold the next `application_apply`, and get the environment file with each secret of the application. This reopens SF-006 for a new purpose; task W7 (the key of the device) is the planned control and is not built. Each user of the machine with the right to use Docker reads the same values with `docker inspect`; "only the daemon and the administrator" is thus not true where a different user has that right.

Mitigation now: The file goes to the holder only, one time for one attempt, with an audit entry.

Proposed change: Section 8: "For an item of the kind `host`, the rules 'no file outside the isolation' and 'deletes the value' do not apply to `compose_file` and `environment_file`: the machine keeps them in `files/` of the application, with mode 0600, for the life of the application and of its kept versions. `application_remove` deletes them in each case, not only with `volumes` true." Change "Life" to "The application". Say that the values are also readable by each user of the machine who can use Docker. Say that an item of a container type has `project` null and no `repository`, and that the daemon refuses one that has them. Record that W7 must come before a machine that the operator does not own alone takes `application_apply`.

Confirmed from the text.

State: Open. The text is a proposal and no code exists. It needs a decision of the operator: OD-HOST-21 of [hosted-applications-plan.md](plans/hosted-applications-plan.md).

### SF-027: A published port and the network of a container are permitted with no rule

Requirements: task H14; the proposed text of the container work types, Section 6.4.3 ("a published port ... permitted"). Severity: Medium.

What it is: A `contained` file can publish each port on each address of the machine, and its containers have the network of the machine.

Where: Section 6.4.3.

Effect, and who can cause it: A person who controls the hub; the author of an image. (a) A published port of Docker passes the firewall of the machine on Linux (`ufw`, `firewalld`; not verified here, well known), so a service is open to the network of the machine. (b) A port on `127.0.0.1` takes the place of a local service that is not running, and local programs trust it. K8 Section 1 names this class for the old port of the link. (c) A container reaches the local network, the overlay network of Anima, the services of the machine on the gateway address and, on a cloud server, the metadata address. The default capability `NET_RAW` lets it send false ARP answers on its bridge. For a job, SF-004 asked for "no loopback address, no link interface and no private range"; an application has no such rule.

Mitigation now: None in the text. To host an application is to give it a network; the question is what the text says and what the default is.

Proposed change: "With `contained`, a published port names a port of the machine from 1024 to 65535 and no address, or the address `127.0.0.1`; the owner sets at the machine if a port is published to all addresses. The daemon adds `cap_drop: [NET_RAW]` to each service, or refuses a file that does not have it." Add to the Section 3 paragraph: "The containers of such a file can reach each address that the machine can reach, and can open a port of the machine." The default is the owner's choice (list below).

Confirmed from the text; the firewall behaviour is not verified.

State: Open. The text is a proposal and no code exists. It needs a decision of the operator: OD-HOST-18 of [hosted-applications-plan.md](plans/hosted-applications-plan.md).

### SF-028: The project name is the first 12 characters of a UUID, and the form of the UUID has no rule

Requirements: task H14; the proposed text of the container work types, Sections 6.4.2, 6.4.4, 6.4.6. Severity: Medium.

What it is: The example UUID (`0192f6b0-…`) is of version 7. Its first 12 characters are 8 hexadecimal digits, a hyphen and 3 digits: only time. Two applications that the hub makes in the same 65 seconds have the same project name; with 12 hexadecimal digits, in the same millisecond. A person who controls the hub can also choose two UUIDs with one prefix. The folder is named by the full UUID, so the two applications have two folders and one project. The text does not say that the daemon checks the form of `application` before it makes a path from it.

Where: Section 6.4.2, first item; Section 6.4.4.

Effect, and who can cause it: With an honest hub: `application_apply` of the second application removes or replaces the containers of the first (Compose removes containers that the file no longer names, if the daemon uses that option; undefined), and `application_remove` of one removes the volumes of the two. With a hub that sends `"application": "../../x"` and a daemon that does not check: a path outside the folder of the applications.

Mitigation now: Section 5 `member_unknown` refuses "a value that breaks a rule of this contract", but no rule is written for this member.

Proposed change: "`application` is a UUID in the form of 36 characters, lowercase; the daemon refuses each other value. The project name is `anima-` and the 32 hexadecimal digits of the UUID. The daemon refuses an `application_apply` whose project name exists with a different folder."

Confirmed from the text.

State: Open. The text is a proposal and no code exists. The proposed change is a change of the text, for the session that writes version 0.4; it needs no decision of the operator unless that session cannot make it.

### SF-029: The limits of an item do not apply to the containers that it starts, and one item holds the one slot of the machine

Requirements: task H14; the proposed text of the container work types, Sections 6.4.2, 6.4.9; Section 3 ("The limits of an item"), Section 4 (one slot). Severity: Medium.

What it is: `limits.memoryBytes` and `limits.diskBytes` are for the item. The containers stay after the item and have no limit: no memory limit, no process limit, no limit on the size of the pulled images. A `container_follow` holds the slot for up to 900 seconds, and an `application_apply` for `limits.seconds`.

Where: Section 6.4.2 (`limits.seconds`), Section 6.4.9.

Effect, and who can cause it: A person who controls the hub, an API key, or the author of an image: the disk of the machine is full, or its memory, and the machine of a person stops to be usable. An API key with one `container_follow` after the other holds the slot, so no `build`, no `runner` and no `container_action` (a `stop` that the operator needs now) runs on that machine. The text does not say if the device policy (USE-009: quiet hours, mains power) and the state `unclean` refuse a `container_action`.

Mitigation now: None in the text.

Proposed change: "An item of a container type does not use the slot of jobs. A machine runs one item that changes Docker at a time and, apart from it, at most two `container_follow` items. `container_action` with `stop` is never refused for `policy` or `unclean`. Before a pull the daemon refuses the item when the free disk is below its own limit (`limit_disk`). With `contained`, each service must have `mem_limit` and `pids_limit`, or the daemon sets its own values."

Confirmed from the text.

State: Open. The text is a proposal and no code exists. The proposed change is a change of the text, for the session that writes version 0.4; it needs no decision of the operator unless that session cannot make it.

### SF-030: To set the switch off, or the profile back to `contained`, does not change what runs

Requirements: task H14; the proposed text of the container work types, Section 3 (two new rows); Section 9. Severity: Medium.

What it is: The text says what the daemon refuses when the switch is off. It does not say what happens to the containers that items started. With `restart: always` they start again at each start of the machine. A service that was started under `full` stays privileged after the owner sets `contained`, and `container_action` still starts it.

Where: Section 3, the two new rows; Section 9, "A container that the item started is not deleted".

Effect, and who can cause it: A person who controls the hub, before the owner sets the switch off: code of their choice continues on the machine. The owner believes that "off" ends it.

Mitigation now: None in the text.

Proposed change: "When the owner sets the switch off, or the profile from `full` to `contained`, the machine shows each application that Anima installed, with the profile of its last apply, and asks if it stops them. `container_action` refuses `start` and `restart` of a container of an application whose last apply was under `full` while the profile is `contained`." Say in the text that a container continues after the switch goes off unless the owner stops it.

Confirmed from the text.

State: Open. The text is a proposal and no code exists. The proposed change is a change of the text, for the session that writes version 0.4; it needs no decision of the operator unless that session cannot make it.

### SF-031: An image is named by a tag, and a file can start each image that the machine has or can pull

Requirements: task H14; the proposed text of the container work types, Section 6.4.2 (`pull`), Section 6.4.3. Severity: Medium.

What it is: The SHA-256 fixes the Compose file, not the images. `image: name:tag` is what the registry gives at the time of the pull. The pull uses the registry logins of the user of the daemon. A file can name any image and give it a `command` or an `entrypoint` of the hub's choice.

Where: Section 6.4.2, fourth item; Section 6.4 ("No container type runs a command inside a container").

Effect, and who can cause it: (a) The author of an image, or a person who takes over its tag or its registry: new code on each machine at the next `pull: true`, with what the file permits. (b) A person who controls the hub: a service with a private image of the owner (one that is on the machine, or that the logins of the machine can pull) and a command that sends its files to an address. (c) The sentence "No container type runs a command inside a container" is true only for `exec`: `command`, `entrypoint`, `healthcheck.test` and `post_start` are commands of the hub in a container, and a health check is a scheduled command, which Section 4.3 says the route forbids.

Mitigation now: None in the text.

Proposed change: "A recipe of the catalogue names each image with a digest. The log of the machine records the digest of each image that an item started. With `contained`, the daemon pulls with no login of the machine, unless the owner named the registry at the machine." Correct the sentence of Section 6.4 to say what is true: the hub names the image, the command and the environment of each container through the Compose file.

Confirmed from the text.

State: Open. The text is a proposal and no code exists. It needs a decision of the operator: OD-HOST-20 of [hosted-applications-plan.md](plans/hosted-applications-plan.md).

### SF-032: The log of the machine has no members for the container types

Requirements: task H14; the proposed text of the container work types, Section 9 (SF-012); Section 6.4.3. Severity: Low.

What it is: Section 9 lists the members of the entry for `build` and `runner`. The proposed text adds only "the profile with which each file was applied".

Where: Section 9; Section 6.4.3.

Effect, and who can cause it: A person who controls the hub. The owner cannot see afterwards which container was stopped, which file was applied, what was removed. This reopens SF-012 for the new types.

Mitigation now: The profile is recorded.

Proposed change: "For a container type the entry has: the type; the container identifier and the action; the application or the project; `fileSha256`, `environmentSha256` and the SHA-256 of the final form; `pull`, `keep`, `volumes`, `images`, `what`; the profile; the rule code of a refusal; the digest of each image started; the counts of what was removed. For `container_follow`: the container and the time, never a line."

Confirmed from the text.

State: Open. The text is a proposal and no code exists. The proposed change is a change of the text, for the session that writes version 0.4; it needs no decision of the operator unless that session cannot make it.

### SF-033: Points that an implementer must guess

Requirements: task H14; the proposed text of the container work types, Sections 3, 4, 5, 6.4, 7. Severity: Low (each can become a fault of an implementation).

What it is, and where:

- The two switches. `Claim.switch` has one value, and the hub "always answers 204" to `off`. The text does not say if a machine with work off and "Manage applications" on takes container items, or the reverse.
- `kinds` of the claim: is `host` a member, and when.
- "The Docker of that machine" and "a remote Docker address is refused": no rule says how. `DOCKER_HOST`, `DOCKER_CONTEXT` and the current context of the configuration file of the user select the engine. The daemon must name one fixed socket path as an argument and say which engine it is when a machine has two (the system Docker and Docker Desktop, or rootless). On a Mac and on Windows the engine is in a virtual machine: "the rights of the machine" then has a different meaning, and the bind paths are shares of the home folder.
- A refusal "is not an attempt: the hub gives the item to a different machine". An application has one machine, and a container identifier has meaning on one machine.
- The lease and the lost hub (Section 7, SF-005): what an `application_apply` does when the 90 seconds pass in the middle of `up`; if a second attempt of `container_action` `restart` restarts a second time.
- If `application_apply` removes the containers that the new file no longer names.
- Which file `application_remove` uses to remove the project if the folder is gone, and what it does with files in a bind mount that root owns. What "the images that only this application used" means.
- `adopted_apply` with `pull: true` and `action: stop`: a combination with no meaning; refuse it.
- If the rules of 6.4.3 are applied when `pull` is false and the file is the same.
- The size of the `Secret` answer: Section 2 has limits for requests only; `compose_file` is 256 KiB in a JSON string.
- If the YAML reader has a limit for aliases (a file of 256 KiB can grow in memory).
- Which actor kinds may make each container type. The text has a rule only for `volumes`.

Mitigation now: None.

Proposed change: One sentence for each point in the section named. For the first: "`Claim` has a second member `applications`: `on` or `off`. The hub gives a container item only when it is `on`; the work switch does not apply to container types."

Confirmed from the text.

State: Open. The text is a proposal and no code exists. The proposed change is a change of the text, for the session that writes version 0.4; it needs no decision of the operator unless that session cannot make it.

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
