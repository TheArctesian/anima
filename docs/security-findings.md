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
| SF-034 | Section 4.5: three rules are on the final form, where they cannot apply, and a variable passes the check before the reader | Container work types (proposed text for contract K10, not built) | Open |
| SF-035 | Section 4.1: an administrator program runs `docker` and the Compose plug-in, and no rule says who can write them | Container work types (proposed text for contract K10, not built) | Open |
| SF-036 | Section 4.5: "at any level" with a list of two levels | Container work types (proposed text for contract K10, not built) | Open |
| SF-037 | Section 4.5, top level: `networks` and `volumes` are "no X" lists again | Container work types (proposed text for contract K10, not built) | Open |
| SF-038 | `secrets` and `configs`: three sentences that do not agree | Container work types (proposed text for contract K10, not built) | Open |
| SF-039 | Section 4.2: the binding to the origin and the device has no mechanism; Section 4.1 does not say what the socket gives | Container work types (proposed text for contract K10, not built) | Open |
| SF-040 | Section 4.9: "or the user that owned it at the adoption" | Container work types (proposed text for contract K10, not built) | Open |
| SF-041 | Images: "no login" has no way, and an image that is on the machine can be started | Container work types (proposed text for contract K10, not built) | Open |
| SF-042 | The environment: the prefix rule is a list of refused names | Container work types (proposed text for contract K10, not built) | Open |
| SF-043 | `labels` | Container work types (proposed text for contract K10, not built) | Open |
| SF-044 | Values with no upper limit | Container work types (proposed text for contract K10, not built) | Open |
| SF-045 | Section 4.12, "All containers" on | Container work types (proposed text for contract K10, not built) | Open |
| SF-046 | Section 4.14: "where the table says 'User only', the settings at the machine are the second control" | Container work types (proposed text for contract K10, not built) | Open |
| SF-047 | Contradictions with K10 0.3 that Section 4.3 does not change | Container work types (proposed text for contract K10, not built) | Open |
| SF-048 | Points that an implementer must guess | Container work types (proposed text for contract K10, not built) | Open |
| SF-049 | Section 4.1, how the kind of engine is decided. High on a Mac, Low elsewhere | Container work types (proposed text for contract K10, not built) | Open |
| SF-050 | Section 4.1, engine of a user: the accepted user need not be the owner of the engine. Medium | Container work types (proposed text for contract K10, not built) | Open |
| SF-051 | Section 4.2, "The owner sets the settings off when he moves a machine to a different hub". Medium | Container work types (proposed text for contract K10, not built) | Open |
| SF-052 | Section 4.5 steps 1 and 5, the reader and the writer of YAML have no rules. Medium (not verified) | Container work types (proposed text for contract K10, not built) | Open |
| SF-053 | Section 4.5 step 3, the filling of variables is not fully said. Medium to Low | Container work types (proposed text for contract K10, not built) | Open |
| SF-054 | Section 4.5, `image`: "the characters that a registry permits". Low | Container work types (proposed text for contract K10, not built) | Open |
| SF-055 | Section 4.5, value rules that are missing for permitted keys | Container work types (proposed text for contract K10, not built) | Open |
| SF-056 | Section 4.5, "the lower of the value of the file and its own limit". Medium (not verified) | Container work types (proposed text for contract K10, not built) | Open |
| SF-057 | Section 4.5, `content` of a top-level secret. Low (not verified) | Container work types (proposed text for contract K10, not built) | Open |
| SF-058 | Sections 4.6, 4.7, 4.10: `application_back` checks the final form with a list that refuses a final form. Medium | Container work types (proposed text for contract K10, not built) | Open |
| SF-059 | Section 4.8, "It runs Compose in the recorded folder, as the owner would". Medium | Container work types (proposed text for contract K10, not built) | Open |
| SF-060 | Sections 4.3 and 4.7, the queue. Medium to Low | Container work types (proposed text for contract K10, not built) | Open |
| SF-061 | Section 4.3, contradictions with K10 0.3 that remain. Medium for the first, Low for the others | Container work types (proposed text for contract K10, not built) | Open |
| SF-062 | Section 4.13, what the key gives to a catalogue install. Medium to Low | Container work types (proposed text for contract K10, not built) | Open |
| SF-063 | Sections 4.1, 4.2 and 4.3 (Section 9), sentences that are false on an engine of a user. Low | Container work types (proposed text for contract K10, not built) | Open |
| SF-064 | Section 4.1, the path-write rule and the plug-in. Low (not verified) | Container work types (proposed text for contract K10, not built) | Open |
| SF-065 | Section 4.3, the paragraph of K10 Section 3 does not name each power. Low | Container work types (proposed text for contract K10, not built) | Open |
| SF-066 | The table of actors gave an API key the stop and the restart of an adopted project, which R-234 does not list | Container work types (proposed text for contract K10, not built) | Open |
| SF-067 | The table of actors gave an API key the update of a catalogue application, which R-234 does not list | Container work types (proposed text for contract K10, not built) | Open |
| SF-068 | The text said the risk of R-235 more softly and more narrowly than the ruling | Container work types (proposed text for contract K10, not built) | Open |
| SF-069 | The text said the risk of R-231 without the words "on each machine" | Container work types (proposed text for contract K10, not built) | Open |
| SF-090 | A routed session is a full login of the operator for each agent key, and it defeats each setting at the machine | A command line or a session for an agent (proposal, no code) | Open |
| SF-091 | The isolation that the command line depends on is a proposal: not accepted, not built, not measured on Linux | A command line or a session for an agent (proposal, no code) | Open |
| SF-092 | The option "as the user of the daemon" of OD-RCL2 gives the daemon token and the applications socket | A command line or a session for an agent (proposal, no code) | Open |
| SF-093 | No permission at the hub for a key: each write key gets the function, and the MCP state is not a control | A command line or a session for an agent (proposal, no code) | Open |
| SF-094 | The setting of RCL-006: one switch for two mechanisms, with the weaker protection, and for a session it is not at the machine | A command line or a session for an agent (proposal, no code) | Open |
| SF-095 | A hub that the owner of the machine does not control: the list of R-185 is lost, and RCL-007 names the wrong control | A command line or a session for an agent (proposal, no code) | Open |
| SF-096 | A command shares its isolation with `build` and `runner`: what it leaves can reach a later job that has a secret | A command line or a session for an agent (proposal, no code) | Open |
| SF-097 | The network of a command: the public internet is open, the limits are not measured, and "unless the owner set otherwise" is a setting with no text | A command line or a session for an agent (proposal, no code) | Open |
| SF-098 | Section 8 states as fact two things that the W2 record says are not checked | A command line or a session for an agent (proposal, no code) | Open |
| SF-099 | The text does not say what file system a container command has; the inventory is of the tools of the host | A command line or a session for an agent (proposal, no code) | Open |
| SF-100 | The audit entry: 4 KiB of a command line of up to 1 MiB, a secret that stays one year, and no entry in the log of the machine | A command line or a session for an agent (proposal, no code) | Open |
| SF-101 | The limit of 30 starts does not limit: 30 commands of one hour hold each slot, "caller" has no definition, queued work counts later | A command line or a session for an agent (proposal, no code) | Open |
| SF-102 | The queue: a command starts when the owner sets the switch on later; a refusal takes the machine out of work; two names for one cause | A command line or a session for an agent (proposal, no code) | Open |
| SF-103 | The output of a command instructs the agent that has a shell; a stolen daemon token can take commands and give false results | A command line or a session for an agent (proposal, no code) | Open |
| SF-104 | The working tree: the text says a command has no repository and then gives it one; a private checkout with no secret; source of a private repository can leave | A command line or a session for an agent (proposal, no code) | Open |
| SF-105 | A session needs no tool of Anima to be used by a key; the start command of an SSH session; the idle limit does not bind an agent | A command line or a session for an agent (proposal, no code) | Open |
| SF-106 | A command line removes the click rules that R-196 made absolute (install of Hermes, AXT-021) | A command line or a session for an agent (proposal, no code) | Open |
| SF-107 | A command can run two times or continue with no hub: the lease rules for `command` are not written | A command line or a session for an agent (proposal, no code) | Open |
| SF-108 | The hub selects the kind, and `native` is the weaker isolation; `actor` has the value `hub`; PRO-024 is not named | A command line or a session for an agent (proposal, no code) | Open |
| SF-109 | The arguments are visible on the machine and in the lists of the hub; two daemons on one Mac share one build user | A command line or a session for an agent (proposal, no code) | Open |
| SF-110 | The environment, the search path and the input of the command are not written | A command line or a session for an agent (proposal, no code) | Open |
| SF-111 | The texts that Section 2 missed | A command line or a session for an agent (proposal, no code) | Open |
| SF-112 | The gate of Section 9 is weaker than ROU-014 | A command line or a session for an agent (proposal, no code) | Open |
| SF-113 | "No shell" and "`startSession` stays closed" are true and protect nothing | A command line or a session for an agent (proposal, no code) | Open |
| SF-114 | A kept session on a hub of Anima.ing can be reached only if that hub gets a login, and then it is a shell of the vendor | The agent layer of Anima (proposal, no code) | Open |
| SF-115 | Nothing at the machine proves that a start comes from the operator | The agent layer of Anima (proposal, no code) | Open |
| SF-116 | The interface from the daemon to the unit with no sandbox is not written | The agent layer of Anima (proposal, no code) | Open |
| SF-117 | "No AI agent starts, stops or types into a kept session" has no mechanism | The agent layer of Anima (proposal, no code) | Open |
| SF-118 | "By its name" is a lookup in the search path, and the configuration of a repository runs at the start | The agent layer of Anima (proposal, no code) | Open |
| SF-119 | A setting "that the hub cannot set" is kept by a daemon of the user, so each program of that user can write it | The agent layer of Anima (proposal, no code) | Open |
| SF-120 | A signal does more than change a state: it makes rows, causes a push and sends an alert | The agent layer of Anima (proposal, no code) | Open |
| SF-121 | The signal drops the folder, so the exclusion of a repository is known too late | The agent layer of Anima (proposal, no code) | Open |
| SF-122 | A hub of Anima.ing gets a timeline of the work of a person, joined to the usage counts | The agent layer of Anima (proposal, no code) | Open |
| SF-123 | The installer: one `notify` program of Codex, the path of the entry, a link, the mode of the copy | The agent layer of Anima (proposal, no code) | Open |
| SF-124 | No rule that the hook prints nothing, logs nothing and leaves no crash report | The agent layer of Anima (proposal, no code) | Open |
| SF-125 | The alert gives a third party the name of the machine, the address of the hub and the times of work; it conflicts with NOT-003, NOT-005 and NOT-007 | The agent layer of Anima (proposal, no code) | Open |
| SF-126 | The alphabet of a branch name stops an injection, not a disclosure | The agent layer of Anima (proposal, no code) | Open |
| SF-127 | The signal file has no size limit and no rule for a link, a pipe or a truncation | The agent layer of Anima (proposal, no code) | Open |
| SF-128 | `Notification` is not only "waiting", nothing ends Waiting, and "Unknown, active" is a seventh state | The agent layer of Anima (proposal, no code) | Open |
| SF-129 | The version of a program and "hooks installed" leave the device but are not in the list of fields | The agent layer of Anima (proposal, no code) | Open |
| SF-130 | The gate covers the kept sessions and High findings only | The agent layer of Anima (proposal, no code) | Open |
| SF-131 | A unit of a user ends at logout unless linger is set | The agent layer of Anima (proposal, no code) | Open |
| SF-NEW-thr01 | "Only a person answers an approval" has no mechanism: an agent of the same user reaches the socket, the command and the login token (High) | Agent threads (proposal, no code) | Accepted by the operator (2026-11-20, relayed; R-NEW-threadshub, OD-THR18: "Accept it across any device. If my other desicons are contridicarty to this I want them changed."). The weakness stays; Section 8.2 of the proposal has the guard that is left |
| SF-NEW-thr02 | Settings of the project or the user of the agent program (allow rules, hooks, MCP servers, the policy of Codex) can answer before the approval tool is asked (not verified) (High) | Agent threads (proposal, no code) | Open |
| SF-NEW-thr03 | "Relay without a copy" is a promise of the hub: a hub controller can pull each thread of each device that is online (High) | Agent threads (proposal, no code) | Accepted by the operator (2026-11-20, relayed; R-NEW-threadshub, OD-THR4: "Yes, plain on the hub"). The weakness stays; Section 8.2 of the proposal has the guard that is left |
| SF-NEW-thr04 | W7 proves the device to the hub, not the hub to the machine; a prompt of the hub is a command that the hub names, with no isolation (High) | Agent threads (proposal, no code) | Accepted by the operator (2026-11-20, relayed; R-NEW-threadshub, OD-THR12 and OD-THR20: "No, with the guards only"). The weakness stays; Section 8.2 of the proposal has the guard that is left |
| SF-NEW-thr05 | The daemon token and the login token of the command line are in reach of an agent of the same user; in stage 3 one poisoned file reaches each machine (High) | Agent threads (proposal, no code) | Accepted by the operator (2026-11-20, relayed; R-NEW-threadshub, OD-THR18: "Accept it across any device."). The weakness stays; Section 8.2 of the proposal has the guard that is left |
| SF-NEW-thr06 | THR-034 names a key of a client that no plan owns, and does not say what is signed (High) | Agent threads (proposal, no code) | Accepted by the operator (2026-11-20, relayed; R-NEW-threadshub, OD-THR5: "Through the hub"). The weakness stays; Section 8.2 of the proposal has the guard that is left |
| SF-NEW-thr07 | THR-029 bans only MCP tools; an API key and the `anima` command can read entries (High) | Agent threads (proposal, no code) | Open |
| SF-NEW-thr08 | The main web view is a new reader with open `https:` egress, and later would send prompts and answer approvals; P9 was too kind (High) | Agent threads (proposal; the read side is built on a branch) | Open |
| SF-NEW-thr09 | The switch, the folder list and the mode can be written by the user of the agent; TH33 misread USE-004; nothing stops the hub from sending Full access (Medium) | Agent threads (proposal, no code) | Accepted by the operator (2026-11-20, relayed; R-NEW-threadshub, OD-THR18: "Accept it across any device."). The weakness stays; Section 8.2 of the proposal has the guard that is left |
| SF-NEW-thr10 | The recommendation of OD-THR8 conflicts with MCP-006 and AGL-003; change tools are `allow` by default (Medium) | Agent threads (proposal, no code) | Accepted by the operator (2026-11-20, relayed; R-NEW-threadsdrive, OD-THR8: "Yes, within limits"). The weakness stays; Section 8.2 of the proposal has the guard that is left |
| SF-NEW-thr11 | The built MCP tool gave the full title, the path of the folder and the branch, with no exclusion (Medium) | Agent threads (proposal; the read side is built on a branch) | Accepted by the operator (2026-11-20, relayed; R-NEW-threadsshape, OD-THR13: "Yes, cleaned and capped"). The weakness stays; Section 8.2 of the proposal has the guard that is left |
| SF-NEW-thr12 | Nobody is named as the holder of the setting of option A of the event log (Medium) | Agent threads (proposal, no code) | Open |
| SF-NEW-thr13 | The long poll of THR-026 is not written against K10 and K8 (Medium) | Agent threads (proposal, no code) | Open |
| SF-NEW-thr14 | The idle stop and the life of a thread have no limits, no containment of child processes, no rule for a restart of the daemon (Medium) | Agent threads (proposal, no code) | Open |
| SF-NEW-thr15 | The `git` of the runner runs hooks and configuration that the agent can write; checkpoints take untracked secrets; worktree names come from the prompt (Medium) | Agent threads (proposal, no code) | Open |
| SF-NEW-thr16 | The label of the source of a prompt and the audit rest on the hub; no log at the machine (Medium) | Agent threads (proposal, no code) | Open |
| SF-NEW-thr17 | The event log is a second copy with no retention, deletion or limit; redaction is best effort (Medium) | Agent threads (proposal, no code) | Open |
| SF-NEW-thr18 | Wider conflicts with BOT-020, BOT-032, BOT-033 and AXT-045: a subscription login driven through a hosted hub (Medium) | Agent threads (proposal, no code) | Accepted by the operator (2026-11-20, relayed; R-NEW-threadsdrive, OD-THR3: "No we can use account methods everyone else is doing it"). The weakness stays; Section 8.2 of the proposal has the guard that is left |
| SF-NEW-thr19 | Gaps of THR-040: no deny list, the source of `PATH`, `SSH_AUTH_SOCK`, a prompt in the process list (Medium) | Agent threads (proposal, no code) | Open |
| SF-NEW-thr20 | A forged session file shows as a thread, and a resume would load its history; the ID of a Codex row came from the content (Medium) | Agent threads (proposal; the read side is built on a branch) | Open |
| SF-NEW-thr21 | No rule to fail closed for an unknown event, kind of request or version of the program (Medium) | Agent threads (proposal, no code) | Open |
| SF-NEW-thr22 | Limits missing in the code: the summary title, the patch, the whole thread, the line of a glance (Low) | Agent threads (proposal; the read side is built on a branch) | Open |
| SF-NEW-thr23 | The file reader followed a link at the root folders and checked no owner and no hard link (Low) | Agent threads (proposal; the read side is built on a branch) | Open |
| SF-NEW-thr24 | The reader can show what did not happen: the output wrapper of Codex, the rule for typed text, no end of a turn for Claude Code (Low) | Agent threads (proposal; the read side is built on a branch) | Open |
| SF-NEW-thr25 | TH4 was marked built, but the page was not on the branch at the time of the read (Low) | Agent threads (proposal; the read side is built on a branch) | Open |
| SF-NEW-thr26 | Rows of Section 8 that were wrong or too kind (P2, P3, P10, P13), and rows that were missing (Low) | Agent threads (proposal, no code) | Open |
| SF-NEW-thr27 | Open decisions and conflicts that were missing (Low) | Agent threads (proposal, no code) | Open |
| SF-NEW-thr28 | Members about the account and its limits in the streams of the programs conflict with AIS-022 (not verified) (Low) | Agent threads (proposal, no code) | Open |
| SF-NEW-thr29 | A start of a child on a different machine has no signer: no person is at a client, and the rule that a child has no more than its parent is kept only by the hub (High) | Agent threads, version 2 (proposal, no code) | Accepted by the operator (2026-11-20, relayed; R-NEW-threadshub, OD-THR20 and OD-THR35: "Trust the hub, keep the guards"). The weakness stays; Section 8.2 of the proposal has the guard that is left |
| SF-NEW-thr30 | A machine of the pool has no person at the machine, so each guard at the machine (the switch, the folder list, the trust question) is set by the pool and the hub (High) | Agent threads, version 2 (proposal, no code) | Accepted by the operator (2026-11-20, relayed; R-NEW-threadshub, OD-THR20 and OD-THR35: "Trust the hub, keep the guards"). The weakness stays; Section 8.2 of the proposal has the guard that is left |
| SF-NEW-thr31 | "A device of the account" gives a machine of the pool a path to the devices of the owner, a daemon token that claims work, and a place in "any machine" (High) | Agent threads, version 2 (proposal, no code) | Accepted by the operator (2026-11-20, relayed; R-NEW-threadspool, OD-THR36: "Yes, like any device"). The weakness stays; Section 8.2 of the proposal has the guard that is left |
| SF-NEW-thr32 | "Inside the allowance" cannot be checked at the start: the hours are not known, parallel requests race, no cap on machines or size, no end (High) | Agent threads, version 2 (proposal, no code) | Open |
| SF-NEW-thr33 | The offer of a larger plan can be caused by text that steers an agent and answered by a program; the card rests on figures of the hub (High) | Agent threads, version 2 (proposal, no code) | Open |
| SF-NEW-thr34 | The hard stops for what cannot be undone were named in THR-065 and enforced nowhere; THR-081 itself deletes a disk with no question (High) | Agent threads, version 2 (proposal, no code) | Open |
| SF-NEW-thr35 | Who may ask the pool for a machine was a note in P18, not a requirement; a call from the hub to the pool is against CVM-015 (Medium) | Agent threads, version 2 (proposal, no code) | Accepted by the operator (2026-11-20, relayed; R-NEW-threadshub, OD-THR20 and OD-THR35: "Trust the hub, keep the guards"). The weakness stays; Section 8.2 of the proposal has the guard that is left |
| SF-NEW-thr36 | Reports and messages are text that steers, and they go up to the machine that is trusted most; no limit on messages or turns (Medium) | Agent threads, version 2 (proposal, no code) | Open |
| SF-NEW-thr37 | The tools of a tree give content of a thread, against THR-029; briefs and reports go through the hosted hub (Medium) | Agent threads, version 2 (proposal, no code) | Open |
| SF-NEW-thr38 | The binding of the tools to a tree binds the tools only; a child with full access can use the command, the socket or the login token (Medium) | Agent threads, version 2 (proposal, no code) | Accepted by the operator (2026-11-20, relayed; R-NEW-threadshub, OD-THR18: "Accept it across any device."). The weakness stays; Section 8.2 of the proposal has the guard that is left |
| SF-NEW-thr39 | The placement is not written against the routing rules of jobs.md; THR-049 and THR-071 disagree; OD-THR25 changes BOT-032 without saying so (Medium) | Agent threads, version 2 (proposal, no code) | Open |
| SF-NEW-thr40 | Each option of OD-THR28 was against a rule or against THR-079; an API key is a bill beyond the plan (Medium) | Agent threads, version 2 (proposal, no code) | Open |
| SF-NEW-thr41 | The removal rested on the word of the hub; more stays than the disk; the kind of isolation of a machine of the pool was not said (Medium) | Agent threads, version 2 (proposal, no code) | Open |
| SF-NEW-thr42 | Where the settings "ask first" and the limits are kept was not said; the hub or a steered agent could loosen them (Medium) | Agent threads, version 2 (proposal, no code) | Accepted by the operator (2026-11-20, relayed; R-NEW-threadshub, OD-THR18: "Accept it across any device."). The weakness stays; Section 8.2 of the proposal has the guard that is left |
| SF-NEW-thr43 | The turned OD-THR7 had no condition for a prompt from the hub; the environment of a child on a different machine was not defined (Medium) | Agent threads, version 2 (proposal, no code) | Accepted by the operator (2026-11-20, relayed; R-NEW-threadsdrive, OD-THR7: "Your login's environment"). The weakness stays; Section 8.2 of the proposal has the guard that is left |
| SF-NEW-thr44 | The one stop of the turned OD-THR21 (trust a folder one time) does not fit a worktree, a fresh clone or a machine with no person (Medium) | Agent threads, version 2 (proposal, no code) | Accepted by the operator (2026-11-20, relayed; R-NEW-threadsdrive, OD-THR21: "Yes, with a notice"). The weakness stays; Section 8.2 of the proposal has the guard that is left |
| SF-NEW-thr45 | Stage 8 disagreed with cloud-machines.md and HMD-073, and Section 11 did not list it (Medium) | Agent threads, version 2 (proposal, no code) | Open |
| SF-NEW-thr46 | The turned text of OD-THR8 was in the wrong table; some rows of Section 8 were stale (Low) | Agent threads, version 2 (proposal, no code) | Open |
| SF-NEW-thr47 | A side chat with a different program or model sends the whole conversation to a second vendor (Low) | Agent threads, version 2 (proposal, no code) | Open |
| SF-NEW-thr48 | A stop by one process group leaves commands alive (High; stage 1b) | Agent threads, version 3 (proposal; no code of the drive side) | Open |
| SF-NEW-thr49 | With THR-048 withdrawn, "text is drawn as text" is the only wall between a remote author and a command with full access through the main web view, and it was not named as a guard (High; stage 1b) | Agent threads, version 3 (proposal; no code of the drive side) | Open |
| SF-NEW-thr50 | The text did not say what leaves the computer in stage 1b, mixed two switches, and did not say if a title goes with the switch off (High; stage 1b, 2) | Agent threads, version 3 (proposal; no code of the drive side) | Open |
| SF-NEW-thr51 | The account methods of Codex had no closed list, the sign-in was not held to the machine, and nothing said if the hub gets the account, the plan and the limits (High; stage 1b, 3) | Agent threads, version 3 (proposal; no code of the drive side) | Open |
| SF-NEW-thr52 | Sentences that bind still said the opposite of an answer (THR-065, THR-021, THR-051, THR-031, THR-033, THR-040, THR-046) (Medium; stage 1b) | Agent threads, version 3 (proposal; no code of the drive side) | Open |
| SF-NEW-thr53 | Rows of Section 8 still claimed a guard that is withdrawn (Medium; stage 1b to 8) | Agent threads, version 3 (proposal; no code of the drive side) | Open |
| SF-NEW-thr54 | THR-049 said that no program runs after a restart, against the holder; the ID of a request after a restart of the daemon was not defined (Medium; stage 1b) | Agent threads, version 3 (proposal; no code of the drive side) | Open |
| SF-NEW-thr55 | "The holder takes commands only from the daemon" cannot hold against a program of the same user (Medium; stage 1b) | Agent threads, version 3 (proposal; no code of the drive side) | Open |
| SF-NEW-thr56 | The end of the holder, its update, the versions of its protocol and agents with no holder were not written (Medium; stage 1b) | Agent threads, version 3 (proposal; no code of the drive side) | Open |
| SF-NEW-thr57 | "No byte is lost" had no limit of size, and the kept output is one more copy (Medium; stage 1b) | Agent threads, version 3 (proposal; no code of the drive side) | Open |
| SF-NEW-thr58 | THR-040 named variables only: descriptors, the environment of the holder, the working folder and a secret by its value were missing (Medium; stage 1b) | Agent threads, version 3 (proposal; no code of the drive side) | Open |
| SF-NEW-thr59 | The credential of the MCP server of a thread was not defined (Medium; stage 1b, 7) | Agent threads, version 3 (proposal; no code of the drive side) | Open |
| SF-NEW-thr60 | The soft limits did not say who counts or what a tool gets at the limit; the start tool came before the limits (Medium; stage 1b, 7) | Agent threads, version 3 (proposal; no code of the drive side) | Open |
| SF-NEW-thr61 | "Allow for the session" had no rule; "a missing answer is a decline" was against THR-087 (Medium; stage 1b) | Agent threads, version 3 (proposal; no code of the drive side) | Open |
| SF-NEW-thr62 | The log at the machine "that the hub cannot change" claimed too much (Medium; stage 1b, 3) | Agent threads, version 3 (proposal; no code of the drive side) | Open |
| SF-NEW-thr63 | The socket was not said to be a file; the client did not check its owner; the runner was not forbidden in a system unit (Medium; stage 1b) | Agent threads, version 3 (proposal; no code of the drive side) | Open |
| SF-NEW-thr64 | Full access, the default, was not measured in TH10 (Medium; stage 1b) | Agent threads, version 3 (proposal; no code of the drive side) | Open |
| SF-NEW-thr65 | The values of the arguments had no rule (Medium; stage 1b, 3) | Agent threads, version 3 (proposal; no code of the drive side) | Open |
| SF-NEW-thr66 | TH33 was one line (Medium; stage 3) | Agent threads, version 3 (proposal; no code of the drive side) | Open |
| SF-NEW-thr67 | One machine with the switch on can, through an agent, turn the switch on at each machine that it reaches (Medium; stage 3, 7) | Agent threads, version 3 (proposal; no code of the drive side) | Open |
| SF-NEW-thr68 | No rule for how long the hub keeps the content of threads, for its delete, its copies and the log of the database (Medium; stage 2) | Agent threads, version 3 (proposal; no code of the drive side) | Open |
| SF-NEW-thr69 | The redaction covers tool outputs only, and its clause of known secrets is empty now (Medium; stage 2) | Agent threads, version 3 (proposal; no code of the drive side) | Open |
| SF-NEW-thr70 | Task rows that were stale; `approve` had no task in stage 1b (Low; stage 1b to 6) | Agent threads, version 3 (proposal; no code of the drive side) | Open |
| SF-NEW-thr71 | Section 11 missed MCP-004, AIS-018, BOT-066, BOT-032, LOG-007 (Low; stage 1a to 3) | Agent threads, version 3 (proposal; no code of the drive side) | Open |
| SF-070 | Sections 6.4.1 and 6.4.2: "the administrator" has two meanings. Medium | Container work types (contract K10 Section 6.4, not built) | Open |
| SF-071 | Section 6.4.1: the write check names "a real file", and the socket is one. Medium | Container work types (contract K10 Section 6.4, not built) | Open |
| SF-072 | Sections 6.4.2 and 6.4.7: who writes the adoption record on an engine of a user. Medium | Container work types (contract K10 Section 6.4, not built) | Open |
| SF-073 | Section 7 paragraph: "A hub keeps a member of `result` as the machine sent it." Medium | Container work types (contract K10 Section 6.4, not built) | Open |
| SF-074 | Section 4 paragraph: a `stop` waits behind a running item, and a key has no limit on installs. Medium | Container work types (contract K10 Section 6.4, not built) | Open |
| SF-075 | Sections 4 and 7 paragraphs: an item that no machine claims has no end. Medium | Container work types (contract K10 Section 6.4, not built) | Open |
| SF-076 | Section 7 paragraph and 6.4.1: the machine has no exclusion and no time limit of its own. Medium | Container work types (contract K10 Section 6.4, not built) | Open |
| SF-077 | Sections 6.4.1 and 6.4.4, `ports`: no lowest version of the engine. Medium (not verified) | Container work types (contract K10 Section 6.4, not built) | Open |
| SF-078 | Section 6.4.7: the comparison at adoption reads three fields. Low to Medium (not verified) | Container work types (contract K10 Section 6.4, not built) | Open |
| SF-079 | Section 12: a `container_action` item makes a machine busy. Low (not verified against K8) | Container work types (contract K10 Section 6.4, not built) | Open |
| SF-080 | Section 6.4.4 steps 1, 3 and 5: rules that two honest implementers build differently. Low each | Container work types (contract K10 Section 6.4, not built) | Open |
| SF-081 | Section 6.4.4 step 3: the variable form and the environment file. Low | Container work types (contract K10 Section 6.4, not built) | Open |
| SF-082 | Section 6.4.4 step 5: characters that the reader of Compose can take as a line end or refuse. Low (not verified) | Container work types (contract K10 Section 6.4, not built) | Open |
| SF-083 | Section 6.4.1: the environment and configuration rules are said for `docker` only. Low | Container work types (contract K10 Section 6.4, not built) | Open |
| SF-084 | Section 7 paragraph: `limits.seconds` has no maximum for three types, and some refusals have no cause. Low | Container work types (contract K10 Section 6.4, not built) | Open |
| SF-085 | Section 6.4.2: the pair of origin and device. Low | Container work types (contract K10 Section 6.4, not built) | Open |
| SF-086 | Section 6.4.1: the paths are checked "at each start" only. Low | Container work types (contract K10 Section 6.4, not built) | Open |
| SF-087 | Section 6.4.2, row "Registries with a login". Low | Container work types (contract K10 Section 6.4, not built) | Open |
| SF-088 | Eleven sentences of contract K10 version 0.4 contradict each other | Container work types (contract K10 Section 6.4, not built) | Open |
| SF-089 | The clean-up with "All containers" on can remove an adopted project, against R-211 | Container work types (contract K10 Section 6.4, not built) | Open |
| SF-139 | A failed stop or a refused image removal left `files/` with its secret values. High | The applications program (task H31, part 1) | Open |
| SF-137 | The clean-up compared names of images, not digests, and could remove a kept image. Medium | The applications program (task H31, part 1) | Open |
| SF-132 | It is not proven that each Compose version obeys `--host` and `--config`. Medium | The applications program (task H31, part 1) | Open |
| SF-138 | The records said that a new version runs before its start. Medium | The applications program (task H31, part 1) | Open |
| SF-134 | The debug text of a plan showed values of the environment file. Low | The applications program (task H31, part 1) | Open |
| SF-136 | A record of the pair that cannot be read looked like no record. Low | The applications program (task H31, part 1) | Open |
| SF-140 | The settings file is checked for its owner and mode only. Low | The applications program (task H31, part 1) | Open |
| SF-133 | Forms that compose-rules accepted: carriage returns, `=` in a name, two texts for one target. Low | The applications program (task H31, part 1) | Open |
| SF-135 | One origin of a hub could have two texts. Low | The applications program (task H31, part 1) | Open |
| SF-144 | The converter writes each network alias for each network, so a small command makes a very large file. High | Compose file check and `docker run` converter (task H29) | Open |
| SF-141 | One request can hold about 250 checks of the largest file. Medium | Compose file check and `docker run` converter (task H29) | Open |
| SF-146 | The writer of the converter leaves characters that YAML does not print. Low | Compose file check and `docker run` converter (task H29) | Open |
| SF-145 | The reader of shell words keeps as text an expansion that a shell does. Low | Compose file check and `docker run` converter (task H29) | Open |
| SF-142 | A problem of the check shows a secret that the operator wrote in the place of a key. Low | Compose file check and `docker run` converter (task H29) | Open |
| SF-143 | The MCP cleaning can change the Compose file that an agent gets. Low | Compose file check and `docker run` converter (task H29) | Open |

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

## The security read of the agent threads proposal (2026-11-20)

R-NEW-t3native asks for the behaviour of T3 Code as a native part of Anima; the proposal is [agent-threads.md](proposals/agent-threads.md). A separate reviewer agent read version 1 on 2026-11-20. It got the proposal, the documents that it stands on, the code of the read side (the branch `threads/read-side`) and eight positions of an attacker, and no opinion of the session that wrote the proposal. Its report, with the scenario and the proposed change of each finding, is the record [agent-threads-review-2026-11-20.md](research/agent-threads/agent-threads-review-2026-11-20.md). The findings F1 to F28 of that record are the entries SF-NEW-thr01 to SF-NEW-thr28, in the same order: 8 High, 13 Medium, 7 Low. Section 9 of the proposal says what was done for each. No code of the drive side exists; the entries about the read side (F8, F11, F20, F22 to F25) have corrections that are built on the branch and that no person and no second reviewer checked. The choices that only the operator can make are OD-THR18 to OD-THR22 and OD-THR2 to OD-THR8 of the proposal.

A second separate reviewer read the parts that version 2 of the proposal added (the principle of reach, the tree of threads and the tools of an agent, a thread on a chosen machine, machines of the pool inside the allowance of a plan, and the four recommendations that were turned), with the frame of the operator: "maximum agent reach is a good thing". It looked for the ways by which a different party uses the reach of the agent, and for a cost or a loss that cannot be undone. Its report is the record [agent-threads-review-2-2026-11-20.md](research/agent-threads/agent-threads-review-2-2026-11-20.md). Its findings G1 to G19 are the entries SF-NEW-thr29 to SF-NEW-thr47, in the same order: 6 High, 11 Medium, 2 Low. Section 9.1 of the proposal says what was done for each. The choices for the operator are OD-THR20 and OD-THR35 to OD-THR37, and OD-THR25 to OD-THR28.

On 2026-11-20 the operator answered 36 open decisions of the proposal (relayed). Seventeen entries above have the state "Accepted by the operator", with his words; Section 8.2 of the proposal lists each accepted risk and the guard that is left. A third separate reviewer then read version 3 with those answers as given facts, to check that the text says what he accepted and nothing wider, and that each guard that is left can be built and tested. Its report is the record [agent-threads-review-3-2026-11-20.md](research/agent-threads/agent-threads-review-3-2026-11-20.md). Its findings H1 to H24 are the entries SF-NEW-thr48 to SF-NEW-thr71: 4 High, 18 Medium, 2 Low. Section 9.2 of the proposal says what was done for each, and Section 6.2.1 is the list for the review of the code of stage 1b.

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

The operator answered the eight choices on 2026-11-19 (R-226 to R-233), and each entry has an "Update" in its State line. The session that wrote the corrected text then wrote here that no entry had a finding that he had not decided. A second read by a separate reviewer on the same day says that this is not true: see "The second read of the container work types" below. SF-019, SF-020 with SF-033, and SF-031 still have a part that he has not decided, and the corrected text has new weaknesses (SF-034 to SF-048). Two parts are Accepted against the recommendation: the update of an adopted stack (SF-019, R-232) and the secrets of an application before the key of the device (SF-026, R-231). Before his answers this paragraph said: eight choices need the operator. They were the open decisions OD-HOST-14 to OD-HOST-21 of [hosted-applications-plan.md](plans/hosted-applications-plan.md), each with the options and the recommendation of the reviewer. The text of the reviewer for them:

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

State: Open. The text is a proposal and no code exists. The proposed change is a change of the text, for the session that writes version 0.4; it needs no decision of the operator unless that session cannot make it. Update, 2026-11-19, after the answers of the operator (R-226 to R-233): R-228: no profile `full` in the first version. The list of refused things is replaced by a list of permitted things (Section 4.5). The corrected text is version 3 of [hosted-applications-stage-2.md](proposals/hosted-applications-stage-2.md), Section 4. No code exists; the entry stays Open until its code exists and only the operator closes it. The second read of 2026-11-19 says: corrected in part: SF-034, SF-036, SF-037, SF-038.

### SF-016: A bind path below the folder of the application passes the rule with a symbolic link, and a container can write the Compose file that the daemon will start

Requirements: task H14; the proposed text of the container work types, Sections 6.4.3 (`bind_outside`, `file_outside`), 6.4.4, 6.4.7. Severity: High.

What it is: The folder of an application holds the Compose file, the environment file, the kept files of earlier versions and the bind mounts. A container is root in its bind mount (Docker has no user remap by default). The text does not say that the path check follows links, and it does not refuse a bind mount of the folder itself.

Where: Section 6.4.3, "A bind mount whose path on the machine is not below the folder of the application"; Section 6.4.4, "The folder holds the Compose file, the environment file, and the bind mounts".

Effect, and who can cause it: (a) A person who controls the hub, in two items, on a `contained` machine. Item 1: a service with the bind mount `./d:/d` and the command `ln -s / /d/x`. Item 2: a service with the bind mount `./d/x:/host`. The text of the path is below the folder; Docker follows the link and mounts `/`. (b) The author of an image, with a recipe that binds `.` or a folder that holds the files: the container changes `compose.yaml`, the `.env` file, a kept earlier file, or adds `compose.override.yaml`. The next `application_back`, the next start by the daemon, or the "ordinary `docker compose` command" of the owner then starts a file that no check saw. (c) The author of an image and a local user: the container writes a set-user-ID file of root in its bind mount; a local user who can reach the folder runs it. The text sets the access only for the environment file, not for the folder.

Mitigation now: None in the text.

Proposed change: Section 6.4.4: "The folder of an application has two parts. `files/` holds the Compose file, the environment file and the kept files; no mount can name it. `binds/` holds one folder for each bind mount. A bind mount of the profile `contained` names exactly `binds/<name>`, where `<name>` is 1 to 64 characters of `a-z`, `0-9`, `-`, `_`; a longer path is refused. The daemon makes that folder itself, and directly before the start it checks, without following a link, that it is a real folder below a real `binds/`. The top folder of all applications is owned by the user of the daemon (or root) and has no access for the group or others. The daemon gives the path of the Compose file as an argument, so that no override file is read." Add to 6.4.3 that the check of a path is on the real path, not on its text. Say that the owner must not run `docker compose` in `files/` with files that he did not read, or drop the sentence "An ordinary `docker compose` command in that folder works".

(a) depends on Docker following a link in a bind source: not verified, but well known. (b) and (c) are confirmed from the text.

State: Open. The text is a proposal and no code exists. The proposed change is a change of the text, for the session that writes version 0.4; it needs no decision of the operator unless that session cannot make it. Update, 2026-11-19, after the answers of the operator (R-226 to R-233): The folder has `files/` and `binds/` (Section 4.6). The corrected text is version 3 of [hosted-applications-stage-2.md](proposals/hosted-applications-stage-2.md), Section 4. No code exists; the entry stays Open until its code exists and only the operator closes it. The second read of 2026-11-19 says: corrected in part: SF-038.

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

State: Open. The text is a proposal and no code exists. The proposed change is a change of the text, for the session that writes version 0.4; it needs no decision of the operator unless that session cannot make it. Update, 2026-11-19, after the answers of the operator (R-226 to R-233): One final form, made one time; the start reads only it (Section 4.5). The corrected text is version 3 of [hosted-applications-stage-2.md](proposals/hosted-applications-stage-2.md), Section 4. No code exists; the entry stays Open until its code exists and only the operator closes it. The second read of 2026-11-19 says: corrected in part: SF-034, SF-042.

### SF-018: The right to use Docker makes the user of the daemon an administrator of the machine, and the profile is a restraint of the same program

Requirements: task H14; the proposed text of the container work types, Section 13, W1-P6; Section 3 (two new rows); K8 Sections 1 and 2. Severity: High.

What it is: On Linux the right to use Docker is membership of the group `docker` or root. Each is the right of the administrator. K8 says that the daemon runs as a user, that its token is in a file of that user, and that "a program of the same user ... can also read the token store". After the switch goes on, each program of that user can use Docker with no profile. The profile, the switch and the adopted list are then checks inside a program that already holds the full right. Three more gaps: (1) the text says that the administrator is needed for the switch, but not for the profile `full` and not for the adopted list; (2) "setting the switch off takes the right back" is not true for a process that runs (a group stays until the process ends) and not possible where the user owns the socket (Docker Desktop on a Mac, rootless Docker); (3) the text does not say what happens to the switch, the profile and the adopted list when the daemon is enrolled at a different hub. `anima-daemon enrol --hub` is a command of the user of the daemon.

Where: W1-P6; Section 3, the row "If the machine takes container work"; K8 Section 2, the token file.

Effect, and who can cause it: (a) A program of the user of the daemon (a faulty program, a job if W2 fails): administrator of the machine. (b) A fault in the daemon that a hub or a Compose file can reach (the daemon now parses 256 KiB of YAML from the hub and the output of Docker): administrator, not a user. (c) A local user who is not the administrator and who runs the daemon: after the administrator set the switch on, that user enrols the daemon at a hub of their own and, with `full` or with SF-015, becomes the administrator. (d) A person with a stolen daemon token cannot make an item, but can claim the items of that machine and get the environment file (SF-026).

Mitigation now: W1-P6 names the question and gives it to task W2. The text gives no limit.

Proposed change: Decide W1-P6 before the contract is accepted, not in W2; the contract cannot be judged without it. The choice is the owner's (see the list below). Whatever the choice, add to Section 3: "The switch 'Manage applications', the profile and the list of adopted projects are in a file that only the administrator of the machine can write; the daemon reads it. They are bound to the origin of the hub and the device identifier at the time they were set. An enrolment at a different origin, or as a different device, sets the switch off, the profile to `contained` and the list to empty." And add to the paragraph of Section 3: "On a machine whose switch is on, a person who controls the user of the daemon is the administrator of the machine."

Confirmed from the text and from K8. The statement on the group `docker` is general Docker knowledge.

State: Open. The text is a proposal and no code exists. It needs a decision of the operator: OD-HOST-14 of [hosted-applications-plan.md](plans/hosted-applications-plan.md). Update, 2026-11-19, after the answers of the operator (R-226 to R-233): R-226: a separate program with the rights of the administrator is the only one that talks to Docker; the settings are bound to the hub and the device (Sections 4.1 and 4.2). The corrected text is version 3 of [hosted-applications-stage-2.md](proposals/hosted-applications-stage-2.md), Section 4. No code exists; the entry stays Open until its code exists and only the operator closes it. The second read of 2026-11-19 says: each program of the user of the daemon can ask the applications program with no hub (SF-039); OD-HOST-24.

### SF-019: `adopted_apply` takes the path of the file from labels that each user of Docker can write, and runs it with no profile

Requirements: task H14; the proposed text of the container work types, Section 6.4.5. Severity: High.

What it is: The adopted list holds only project names. The path of the file comes from the labels `com.docker.compose.project.config_files` and `working_dir` of a container at the time of the item. Each party that can make a container can write those labels. The file at that path, and its `.env`, its includes and its `build` folders, can be writable by a user who is not the administrator. The profile does not apply, on the reason "the file is the file of the owner"; the daemon cannot know that.

Where: Section 6.4.5, second and fifth items.

Effect, and who can cause it: (a) A local user who can write the folder of an adopted project (a project in a home folder or a shared folder, started one time by the administrator): the user changes the file to ask for `privileged`, then waits for the next `adopted_apply`, or asks for it through the hub. Effect: administrator. (b) A party that can make one container with the label of an adopted project name and a path of its choice. If a file of `application_apply` can set those labels on a service (Compose puts its own labels last, as far as I know; not verified), a person who controls the hub goes from `contained` to no limit. (c) A person who controls the hub, with `pull: true`: the adopted project gets the image that the registry has now. If that project has the Docker socket, the author of that image, or a person who took the tag, is the administrator. (d) `up` applies a file that the owner changed and did not yet want to apply, and builds a `build` section.

Mitigation now: The owner adopts by name at the machine. `down` does not exist.

Proposed change: "At adoption the daemon records, for each project, the real path of each Compose file and of the project folder, as the labels give them at that time, and shows them to the owner. `adopted_apply` uses the recorded paths, never the labels of the time of the item. It refuses the item when a recorded file, or a folder above it, can be written by a user other than root or the user that adopted it (`not_adopted`). It gives the paths and the project name as arguments. `up` never builds (`--no-build`)." Add to 6.4.3 that a label whose key starts with `com.docker.` is refused in each profile. For `pull` on an adopted project the owner must choose (list below).

(a), (c) and (d) are confirmed from the text. (b) depends on Compose behaviour that I did not verify.

State: Open. The text is a proposal and no code exists. It needs a decision of the operator: OD-HOST-17 of [hosted-applications-plan.md](plans/hosted-applications-plan.md). Update, 2026-11-19, after the answers of the operator (R-226 to R-233): The paths are recorded at the adoption, `up` never builds, and a file that others can write is refused (Section 4.9). The part `pull` is decided by R-232: the operator accepts that the hub can make an adopted stack get and start new images. For that part the state is Accepted (R-232, 2026-11-19), with his reason: no setting for each stack; the screen says what an update does. The corrected text is version 3 of [hosted-applications-stage-2.md](proposals/hosted-applications-stage-2.md), Section 4. No code exists; the entry stays Open until its code exists and only the operator closes it. The second read of 2026-11-19 says: ways (a) and (d) are not corrected: a user who is not the administrator can still change the file of an adopted project (SF-040); OD-HOST-23. R-235 (2026-11-19): for ways (a) and (d) the state is Accepted: Anima does not check if the file of an adopted stack changed.

### SF-020: `container_action` works on each container of the machine, so the adopted list does not limit it, and an API key can stop the things that the daemon and the hub need

Requirements: task H14; the proposed text of the container work types, Section 6.4.1; Section 3 (the new paragraph). Severity: High.

What it is: The only test is that the identifier exists. The text gives no rule on which actor may make the item. The control `not_adopted` exists for `adopted_apply`, but `stop`, `start` and `restart` of the same containers need no adoption.

Where: Section 6.4.1.

Effect, and who can cause it: An API key, an AI agent through an MCP tool, or a person who controls the hub. (a) `stop` of the container of the VPN, the DNS, the reverse proxy or the link of the machine: the machine loses its network and the daemon cannot reach the hub; only a person at the machine can correct it. (b) Where the hub and its PostgreSQL run in Docker on a machine with the daemon: `stop` of the database of the hub. No item can then start it again. (c) `stop` of the isolation of a `build` or a `runner` item, if task W2 uses the same Docker (the text does not say). (d) `start` of a container that stopped: a one-time container runs its command again (a migration, a restore, a clean-up, a stopped `--privileged` tool). (e) An agent that reads text that is not trusted (SF-010) can be led to do each of these.

Mitigation now: The identifier is full and checked; there is no `kill`, `pause` or `remove`. The Section 3 paragraph says that a person who controls the hub "can stop each container".

Proposed change: The owner chooses the scope (list below). My proposal: "`container_action` works only on a container of an application that Anima installed, or of an adopted project (the daemon reads the project label and checks it against its own records). Each other container is refused (`not_adopted`), unless the owner set 'All containers' at the machine. The daemon holds a list of protected containers that the owner sets at the machine; it refuses each item for them. `start` is permitted only for a container whose restart policy is not `no`, or that Anima installed." And in the hub requirements: which actor kinds may make each container type.

Confirmed from the text.

State: Open. The text is a proposal and no code exists. It needs a decision of the operator: OD-HOST-15 of [hosted-applications-plan.md](plans/hosted-applications-plan.md). Update, 2026-11-19, after the answers of the operator (R-226 to R-233): R-227: only applications of Anima and adopted projects, with "All containers" at the machine; protected containers; the rule for `start` (Sections 4.7 and 4.8). The corrected text is version 3 of [hosted-applications-stage-2.md](proposals/hosted-applications-stage-2.md), Section 4. No code exists; the entry stays Open until its code exists and only the operator closes it. The second read of 2026-11-19 says: which actor may ask for each type is not decided: OD-HOST-22. R-234 (2026-11-19) decides which actor may ask for each type.

### SF-021: `docker_prune` and `application_remove` let an API key delete each container of the machine, and the rule for volumes is only in the hub

Requirements: task H14; the proposed text of the container work types, Sections 6.4.6, 6.4.8. Severity: High.

What it is: `docker_prune` with `containers` removes each stopped container of the machine, not only those of Anima. With `container_action` `stop` first (SF-020), an API key or MCP tool removes each container: its writable layer, its log, and for a container made with `docker run` its whole configuration. The rule "`volumes` only for a login or a browser session" is a rule of the hub; the machine cannot see the actor kind. The text does not say if `images` means images with no tag or each image with no container, and if `volumes` means anonymous volumes or each volume with no container.

Where: Section 6.4.8; Section 6.4.6, first item.

Effect, and who can cause it: An API key or an AI agent: each container of the machine is deleted, and each image, also the images that `keep` holds for `application_back` (they have no container). A person who controls the hub: the same, and then `volumes`, which deletes the data of each project of the machine, also of projects that the owner did not adopt. This is more than the Section 3 paragraph says ("can stop each container").

Mitigation now: The hub rule for volumes. `application_remove` names only an application of Anima.

Proposed change: "`docker_prune` removes only things that have the project label of an application that Anima installed, or, for `images`, images with no tag and images that the daemon pulled for an application and no container uses. The daemon never removes an image that `keep` holds. `containers` and `volumes` of other projects need the setting 'All containers' of the owner at the machine. A volume is removed only by `application_remove` of its application." For a hosted hub, the owner must choose if removal of volumes needs a second control at the machine (list below). Define the two words exactly, with the Docker command and its flags.

Confirmed from the text; the meaning of the Docker commands is the undefined point.

State: Open. The text is a proposal and no code exists. It needs a decision of the operator: OD-HOST-16 (and OD-HOST-15) of [hosted-applications-plan.md](plans/hosted-applications-plan.md). Update, 2026-11-19, after the answers of the operator (R-226 to R-233): R-227 and R-230: the prune reaches only things of Anima; a volume goes only with its application and only with the setting "The hub may delete volumes" at the machine; the signature of the operator with task W7 (Sections 4.10 and 4.12). The corrected text is version 3 of [hosted-applications-stage-2.md](proposals/hosted-applications-stage-2.md), Section 4. No code exists; the entry stays Open until its code exists and only the operator closes it.

### SF-022: `container_follow` gives the hub the log of each container of the machine

Requirements: task H14; the proposed text of the container work types, Section 6.4.9. Severity: Medium.

What it is: The item names any container. The rules "only to a login or a browser session" and "the hub deletes the lines when the item ends" are rules of the hub. `tail: 200` sends old lines, not only new lines; the text gives `tail` no limit.

Where: Section 6.4.9.

Effect, and who can cause it: A person who controls the hub (with a hosted hub, not the owner): the logs of each container of the machine, also of projects that are not adopted, with the secrets that they hold, by one item after the other. An API key, if the hub lets it make the item: the lines leave the machine and are in the hub for 900 seconds, and the item holds the one slot of the machine (SF-029). The Section 3 paragraph does not name this power.

Mitigation now: The redaction and the character rules of Section 7; the hub rules above.

Proposed change: "`container_follow` works on the containers that `container_action` works on (SF-020). `tail` is 0 to 1,000. The hub makes the item only for an act with a login or a browser session." Add the power to the Section 3 paragraph: "can read the log of each such container".

Confirmed from the text.

State: Open. The text is a proposal and no code exists. It needs a decision of the operator: OD-HOST-15 of [hosted-applications-plan.md](plans/hosted-applications-plan.md). Update, 2026-11-19, after the answers of the operator (R-226 to R-233): R-227: the scope of Section 4.7; `tail` has a limit; only a login or a browser session (Section 4.13). The corrected text is version 3 of [hosted-applications-stage-2.md](proposals/hosted-applications-stage-2.md), Section 4. No code exists; the entry stays Open until its code exists and only the operator closes it.

### SF-023: The check of the hub reads the files and the addresses of the hub

Requirements: task H14; the proposed text of the container work types, Section 6.4.3, last paragraph. Severity: Medium (High for a hosted hub that has more than one operator).

What it is: "The hub applies the same rules" to the final form. To make the final form, the reader opens each file that `include`, `extends`, `env_file`, a secret `file` or a config `file` names, and can get a remote include. On the hub those paths are paths of the hub.

Where: Section 6.4.3, "The hub applies the same rules before it makes an item".

Effect, and who can cause it: An operator of a hosted hub, an API key, or an agent that can send a Compose file. `env_file: /proc/self/environ` or a path of a secret of the hub goes into the final form or into an error text that the editor shows (HOST-022). A remote include makes the hub send a request to an address in its own network.

Mitigation now: None in the text.

Proposed change: "The hub checks the text of the file only. It refuses, before it reads anything, a file with `include`, `extends` with `file`, `env_file`, or a `file` of a secret or config that is not a relative path with no `..`. It opens no file and no address for the check, and it shows no text of an error of the reader."

Confirmed from the text; it depends on how the hub makes the final form.

State: Open. The text is a proposal and no code exists. The proposed change is a change of the text, for the session that writes version 0.4; it needs no decision of the operator unless that session cannot make it. Update, 2026-11-19, after the answers of the operator (R-226 to R-233): The hub checks the text only and opens no file (Section 4.5). The corrected text is version 3 of [hosted-applications-stage-2.md](proposals/hosted-applications-stage-2.md), Section 4. No code exists; the entry stays Open until its code exists and only the operator closes it.

### SF-024: The reader opens files outside the folder before a rule refuses them, and the text does not say what the log of an `application_apply` holds

Requirements: task H14; the proposed text of the container work types, Sections 6.4.2, 6.4.3, Section 7 (log). Severity: Medium.

What it is: `file_outside` is applied to the final form, thus after the reader opened `env_file: /home/<user>/.ssh/id_ed25519` or a file of the daemon. The report has a fixed code, but the text does not say if the output of the Compose program goes into `log.text` of the item. An error of the environment reader can quote the line that it could not read (not verified).

Where: Section 6.4.3, first paragraph; Section 7, "Log" has rules for `build` and `runner` only.

Effect, and who can cause it: A person who controls the hub, or an API key that can make the item: lines of a file that the user of the daemon can read reach the hub in the log of the item. The redaction finds only known patterns.

Mitigation now: "The report names the rule by a fixed code, never text of the file."

Proposed change: Section 7: "For a container type other than `container_follow`, the log holds only lines of the daemon: the step, the fixed code, the identifier of each image. It holds no output of the Docker program." Section 6.4.3: "Before it gives the file to the reader, the daemon refuses a file that names a path that is absolute or has `..`" (with SF-017, a file outside `files/` is never opened).

Confirmed from the text; the content of the error text is not verified.

State: Open. The text is a proposal and no code exists. The proposed change is a change of the text, for the session that writes version 0.4; it needs no decision of the operator unless that session cannot make it. Update, 2026-11-19, after the answers of the operator (R-226 to R-233): No output of Docker in the log of an item; no path outside `files/` is opened (Sections 4.3 and 4.5). The corrected text is version 3 of [hosted-applications-stage-2.md](proposals/hosted-applications-stage-2.md), Section 4. No code exists; the entry stays Open until its code exists and only the operator closes it. The second read of 2026-11-19 says: corrected in part: SF-034.

### SF-025: `application_back` starts a file that the profile of today did not check, and the text does not say how it sets the images

Requirements: task H14; the proposed text of the container work types, Section 6.4.7; Section 6.4.2 (`keep`). Severity: Medium.

What it is: The item brings back "the Compose file and environment file of that time". The text does not say that the daemon applies the profile again. It does not say how the project gets the earlier image identifiers: a change of the file, or a new tag on the old image.

Where: Section 6.4.7.

Effect, and who can cause it: (a) An API key or a person who controls the hub: a machine was `full`, the owner set it back to `contained`; `application_back` starts the earlier file with its `privileged` service. The same after an update of the daemon with a new rule. (b) If the daemon sets the tag `name:tag` on the old image, each other project of the machine that uses that tag gets the old image at its next start. (c) The kept file is in the folder, so SF-016 (b) applies. (d) The item brings an application back to an image with a known fault (as SF-014). (e) `docker_prune` with `images` removes the kept images (SF-021).

Mitigation now: The identifiers of the images are kept, not the tags. `data_not_restored` is said.

Proposed change: "`application_back` applies the rules of the profile of the machine at that time to the kept file, as `application_apply` does, and records the profile. The kept files are in `files/` and no mount can name them. The daemon starts the earlier images by identifier in a copy of the final form; it sets no tag. One `application_back` goes back one step; a second one is refused until the next `application_apply`."

Confirmed from the text.

State: Open. The text is a proposal and no code exists. The proposed change is a change of the text, for the session that writes version 0.4; it needs no decision of the operator unless that session cannot make it. Update, 2026-11-19, after the answers of the operator (R-226 to R-233): The rules are applied again, one step back, by identifier (Section 4.11). The corrected text is version 3 of [hosted-applications-stage-2.md](proposals/hosted-applications-stage-2.md), Section 4. No code exists; the entry stays Open until its code exists and only the operator closes it.

### SF-026: The environment file breaks three rules of Section 8, and the device list of SF-006 does not protect it

Requirements: task H14; the proposed text of the container work types, Section 8 (two new purposes); Sections 6.4.4, 6.4.7, 9. Severity: Medium.

What it is: Section 8 says that the machine "writes the value to no file outside the isolation", "deletes the value when the step has it", and asks "only after the isolation exists". An item of the kind `host` has no isolation, and the environment file stays in the folder, with the copies of earlier versions for `application_back`. The table says "Life: the item", which is not true. The control of SF-006 (the operator names the devices of a project) becomes "the one machine of the application": a stolen token of that machine passes it. The text does not say what `project` and `repository` of the `Item` are for a container type.

Where: Section 8 of the proposed text; Section 6.4.4, "Only the daemon and the administrator of the machine can read the environment file"; Section 9, "if it made one outside the folder".

Effect, and who can cause it: A person with the daemon token of the machine (SF-018 says who can read it): they send claims in place of the daemon, hold the next `application_apply`, and get the environment file with each secret of the application. This reopens SF-006 for a new purpose; task W7 (the key of the device) is the planned control and is not built. Each user of the machine with the right to use Docker reads the same values with `docker inspect`; "only the daemon and the administrator" is thus not true where a different user has that right.

Mitigation now: The file goes to the holder only, one time for one attempt, with an audit entry.

Proposed change: Section 8: "For an item of the kind `host`, the rules 'no file outside the isolation' and 'deletes the value' do not apply to `compose_file` and `environment_file`: the machine keeps them in `files/` of the application, with mode 0600, for the life of the application and of its kept versions. `application_remove` deletes them in each case, not only with `volumes` true." Change "Life" to "The application". Say that the values are also readable by each user of the machine who can use Docker. Say that an item of a container type has `project` null and no `repository`, and that the daemon refuses one that has them. Record that W7 must come before a machine that the operator does not own alone takes `application_apply`.

Confirmed from the text.

State: Open. The text is a proposal and no code exists. It needs a decision of the operator: OD-HOST-21 of [hosted-applications-plan.md](plans/hosted-applications-plan.md). Update, 2026-11-19, after the answers of the operator (R-226 to R-233): The text says the true life of the two files and what a stolen token gets (Section 4.3). The part "before task W7" is decided by R-231: the operator accepts that, until task W7, a stolen daemon token of a machine yields the secrets of the applications of that machine. For that part the state is Accepted (R-231, 2026-11-19); task W7 is the planned correction. The corrected text is version 3 of [hosted-applications-stage-2.md](proposals/hosted-applications-stage-2.md), Section 4. No code exists; the entry stays Open until its code exists and only the operator closes it.

### SF-027: A published port and the network of a container are permitted with no rule

Requirements: task H14; the proposed text of the container work types, Section 6.4.3 ("a published port ... permitted"). Severity: Medium.

What it is: A `contained` file can publish each port on each address of the machine, and its containers have the network of the machine.

Where: Section 6.4.3.

Effect, and who can cause it: A person who controls the hub; the author of an image. (a) A published port of Docker passes the firewall of the machine on Linux (`ufw`, `firewalld`; not verified here, well known), so a service is open to the network of the machine. (b) A port on `127.0.0.1` takes the place of a local service that is not running, and local programs trust it. K8 Section 1 names this class for the old port of the link. (c) A container reaches the local network, the overlay network of Anima, the services of the machine on the gateway address and, on a cloud server, the metadata address. The default capability `NET_RAW` lets it send false ARP answers on its bridge. For a job, SF-004 asked for "no loopback address, no link interface and no private range"; an application has no such rule.

Mitigation now: None in the text. To host an application is to give it a network; the question is what the text says and what the default is.

Proposed change: "With `contained`, a published port names a port of the machine from 1024 to 65535 and no address, or the address `127.0.0.1`; the owner sets at the machine if a port is published to all addresses. The daemon adds `cap_drop: [NET_RAW]` to each service, or refuses a file that does not have it." Add to the Section 3 paragraph: "The containers of such a file can reach each address that the machine can reach, and can open a port of the machine." The default is the owner's choice (list below).

Confirmed from the text; the firewall behaviour is not verified.

State: Open. The text is a proposal and no code exists. It needs a decision of the operator: OD-HOST-18 of [hosted-applications-plan.md](plans/hosted-applications-plan.md). Update, 2026-11-19, after the answers of the operator (R-226 to R-233): R-229: a port only on the machine itself unless "Open to the network" is set at the machine; no `NET_RAW` (Section 4.5). The operator was told that a container can reach what the machine can reach on the local network and on the network of Anima, and selected with that sentence in front of him (R-229). The corrected text is version 3 of [hosted-applications-stage-2.md](proposals/hosted-applications-stage-2.md), Section 4. No code exists; the entry stays Open until its code exists and only the operator closes it.

### SF-028: The project name is the first 12 characters of a UUID, and the form of the UUID has no rule

Requirements: task H14; the proposed text of the container work types, Sections 6.4.2, 6.4.4, 6.4.6. Severity: Medium.

What it is: The example UUID (`0192f6b0-…`) is of version 7. Its first 12 characters are 8 hexadecimal digits, a hyphen and 3 digits: only time. Two applications that the hub makes in the same 65 seconds have the same project name; with 12 hexadecimal digits, in the same millisecond. A person who controls the hub can also choose two UUIDs with one prefix. The folder is named by the full UUID, so the two applications have two folders and one project. The text does not say that the daemon checks the form of `application` before it makes a path from it.

Where: Section 6.4.2, first item; Section 6.4.4.

Effect, and who can cause it: With an honest hub: `application_apply` of the second application removes or replaces the containers of the first (Compose removes containers that the file no longer names, if the daemon uses that option; undefined), and `application_remove` of one removes the volumes of the two. With a hub that sends `"application": "../../x"` and a daemon that does not check: a path outside the folder of the applications.

Mitigation now: Section 5 `member_unknown` refuses "a value that breaks a rule of this contract", but no rule is written for this member.

Proposed change: "`application` is a UUID in the form of 36 characters, lowercase; the daemon refuses each other value. The project name is `anima-` and the 32 hexadecimal digits of the UUID. The daemon refuses an `application_apply` whose project name exists with a different folder."

Confirmed from the text.

State: Open. The text is a proposal and no code exists. The proposed change is a change of the text, for the session that writes version 0.4; it needs no decision of the operator unless that session cannot make it. Update, 2026-11-19, after the answers of the operator (R-226 to R-233): The form of the UUID and a project name of all 32 digits (Section 4.8). The corrected text is version 3 of [hosted-applications-stage-2.md](proposals/hosted-applications-stage-2.md), Section 4. No code exists; the entry stays Open until its code exists and only the operator closes it.

### SF-029: The limits of an item do not apply to the containers that it starts, and one item holds the one slot of the machine

Requirements: task H14; the proposed text of the container work types, Sections 6.4.2, 6.4.9; Section 3 ("The limits of an item"), Section 4 (one slot). Severity: Medium.

What it is: `limits.memoryBytes` and `limits.diskBytes` are for the item. The containers stay after the item and have no limit: no memory limit, no process limit, no limit on the size of the pulled images. A `container_follow` holds the slot for up to 900 seconds, and an `application_apply` for `limits.seconds`.

Where: Section 6.4.2 (`limits.seconds`), Section 6.4.9.

Effect, and who can cause it: A person who controls the hub, an API key, or the author of an image: the disk of the machine is full, or its memory, and the machine of a person stops to be usable. An API key with one `container_follow` after the other holds the slot, so no `build`, no `runner` and no `container_action` (a `stop` that the operator needs now) runs on that machine. The text does not say if the device policy (USE-009: quiet hours, mains power) and the state `unclean` refuse a `container_action`.

Mitigation now: None in the text.

Proposed change: "An item of a container type does not use the slot of jobs. A machine runs one item that changes Docker at a time and, apart from it, at most two `container_follow` items. `container_action` with `stop` is never refused for `policy` or `unclean`. Before a pull the daemon refuses the item when the free disk is below its own limit (`limit_disk`). With `contained`, each service must have `mem_limit` and `pids_limit`, or the daemon sets its own values."

Confirmed from the text.

State: Open. The text is a proposal and no code exists. The proposed change is a change of the text, for the session that writes version 0.4; it needs no decision of the operator unless that session cannot make it. Update, 2026-11-19, after the answers of the operator (R-226 to R-233): No job slot; limits of memory and processes; the free disk; `stop` is never refused for the policy (Sections 4.3 and 4.5). The change of the slot is a change of Section 4 of K10, which the owner of that contract must take. The corrected text is version 3 of [hosted-applications-stage-2.md](proposals/hosted-applications-stage-2.md), Section 4. No code exists; the entry stays Open until its code exists and only the operator closes it. The second read of 2026-11-19 says: corrected in part: SF-044, SF-047.

### SF-030: To set the switch off, or the profile back to `contained`, does not change what runs

Requirements: task H14; the proposed text of the container work types, Section 3 (two new rows); Section 9. Severity: Medium.

What it is: The text says what the daemon refuses when the switch is off. It does not say what happens to the containers that items started. With `restart: always` they start again at each start of the machine. A service that was started under `full` stays privileged after the owner sets `contained`, and `container_action` still starts it.

Where: Section 3, the two new rows; Section 9, "A container that the item started is not deleted".

Effect, and who can cause it: A person who controls the hub, before the owner sets the switch off: code of their choice continues on the machine. The owner believes that "off" ends it.

Mitigation now: None in the text.

Proposed change: "When the owner sets the switch off, or the profile from `full` to `contained`, the machine shows each application that Anima installed, with the profile of its last apply, and asks if it stops them. `container_action` refuses `start` and `restart` of a container of an application whose last apply was under `full` while the profile is `contained`." Say in the text that a container continues after the switch goes off unless the owner stops it.

Confirmed from the text.

State: Open. The text is a proposal and no code exists. The proposed change is a change of the text, for the session that writes version 0.4; it needs no decision of the operator unless that session cannot make it. Update, 2026-11-19, after the answers of the operator (R-226 to R-233): The text says that a container continues, and the machine asks (Section 4.2). The part about `full` no longer applies (R-228). The corrected text is version 3 of [hosted-applications-stage-2.md](proposals/hosted-applications-stage-2.md), Section 4. No code exists; the entry stays Open until its code exists and only the operator closes it.

### SF-031: An image is named by a tag, and a file can start each image that the machine has or can pull

Requirements: task H14; the proposed text of the container work types, Section 6.4.2 (`pull`), Section 6.4.3. Severity: Medium.

What it is: The SHA-256 fixes the Compose file, not the images. `image: name:tag` is what the registry gives at the time of the pull. The pull uses the registry logins of the user of the daemon. A file can name any image and give it a `command` or an `entrypoint` of the hub's choice.

Where: Section 6.4.2, fourth item; Section 6.4 ("No container type runs a command inside a container").

Effect, and who can cause it: (a) The author of an image, or a person who takes over its tag or its registry: new code on each machine at the next `pull: true`, with what the file permits. (b) A person who controls the hub: a service with a private image of the owner (one that is on the machine, or that the logins of the machine can pull) and a command that sends its files to an address. (c) The sentence "No container type runs a command inside a container" is true only for `exec`: `command`, `entrypoint`, `healthcheck.test` and `post_start` are commands of the hub in a container, and a health check is a scheduled command, which Section 4.3 says the route forbids.

Mitigation now: None in the text.

Proposed change: "A recipe of the catalogue names each image with a digest. The log of the machine records the digest of each image that an item started. With `contained`, the daemon pulls with no login of the machine, unless the owner named the registry at the machine." Correct the sentence of Section 6.4 to say what is true: the hub names the image, the command and the environment of each container through the Compose file.

Confirmed from the text.

State: Open. The text is a proposal and no code exists. It needs a decision of the operator: OD-HOST-20 of [hosted-applications-plan.md](plans/hosted-applications-plan.md). Update, 2026-11-19, after the answers of the operator (R-226 to R-233): R-233: a digest for each recipe of the catalogue. The digest of each image in the log of the machine; a pull with no login unless the owner named the registry; the sentence of K10 Section 3 is corrected (Sections 4.3 and 4.8). The corrected text is version 3 of [hosted-applications-stage-2.md](proposals/hosted-applications-stage-2.md), Section 4. No code exists; the entry stays Open until its code exists and only the operator closes it. The second read of 2026-11-19 says: way (b), an image that is already on the machine, is not corrected; the text change of SF-041 corrects it.

### SF-032: The log of the machine has no members for the container types

Requirements: task H14; the proposed text of the container work types, Section 9 (SF-012); Section 6.4.3. Severity: Low.

What it is: Section 9 lists the members of the entry for `build` and `runner`. The proposed text adds only "the profile with which each file was applied".

Where: Section 9; Section 6.4.3.

Effect, and who can cause it: A person who controls the hub. The owner cannot see afterwards which container was stopped, which file was applied, what was removed. This reopens SF-012 for the new types.

Mitigation now: The profile is recorded.

Proposed change: "For a container type the entry has: the type; the container identifier and the action; the application or the project; `fileSha256`, `environmentSha256` and the SHA-256 of the final form; `pull`, `keep`, `volumes`, `images`, `what`; the profile; the rule code of a refusal; the digest of each image started; the counts of what was removed. For `container_follow`: the container and the time, never a line."

Confirmed from the text.

State: Open. The text is a proposal and no code exists. The proposed change is a change of the text, for the session that writes version 0.4; it needs no decision of the operator unless that session cannot make it. Update, 2026-11-19, after the answers of the operator (R-226 to R-233): The members of the entry are listed (Section 4.3). The corrected text is version 3 of [hosted-applications-stage-2.md](proposals/hosted-applications-stage-2.md), Section 4. No code exists; the entry stays Open until its code exists and only the operator closes it.

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

State: Open. The text is a proposal and no code exists. The proposed change is a change of the text, for the session that writes version 0.4; it needs no decision of the operator unless that session cannot make it. Update, 2026-11-19, after the answers of the operator (R-226 to R-233): Each point has its sentence (Sections 4.1, 4.3, 4.8 to 4.14). The corrected text is version 3 of [hosted-applications-stage-2.md](proposals/hosted-applications-stage-2.md), Section 4. No code exists; the entry stays Open until its code exists and only the operator closes it. The second read of 2026-11-19 says: the actor table is not decided (OD-HOST-22) and the engine on a Mac is left open (SF-035, OD-HOST-25).

## The second read of the container work types (task H14, 2026-11-19)

After R-226 to R-233 the text was written again (version 3 of [hosted-applications-stage-2.md](proposals/hosted-applications-stage-2.md), Section 4). A second separate reviewer agent read the new text against the entries SF-015 to SF-033 and the rulings. It got Sections 4.1 to 4.3 and 4.5 to 4.15 as one file, and not Section 4.4, which is the claim of the writer of where each finding is corrected. No code exists. The behaviour of Docker and Compose in these entries is from the knowledge of the reviewer and was not run; each such point is marked "not verified" by the reviewer.

The judgement of the reviewer, in its words: "Yes: SF-019 (ways a and d), SF-020 with the last point of SF-033 (which actor may ask for each type, Section 4.14), and SF-031 (way b, an image already on the machine). SF-018 has the consequence of SF-039 that no ruling states. SF-015, SF-016, SF-017, SF-024 and SF-029 are corrected only in part, but they need changes of the text (SF-034, SF-036, SF-037, SF-038, SF-042, SF-044, SF-047), not a decision."

The operator answered the five points on 2026-11-19 (R-234 to R-238). By R-237 the text does not become version 0.4 until the design of task H30 for a Mac exists and he has approved it. Before his answers this paragraph said: thus the condition of R-209 is not met: the text is not version 0.4 of the contract. Five points need the operator; they were the open decisions OD-HOST-22 to OD-HOST-26 of [hosted-applications-plan.md](plans/hosted-applications-plan.md). Two points that the reviewer put to the owner (an image that is already on the machine, SF-041; the labels of a file, SF-043) are taken as changes of the text, as the reviewer proposes, and need no decision unless he wants the wider form.

How the reviewer judged each first finding:

| Finding | Result | The sentence that corrects it, or what is missing |
| --- | --- | --- |
| SF-015 | In part (R-228 removes `full`) | "The applications program refuses a file whose final form has a key, at any level, that is not in this list, or a value that breaks its rule." Missing: the list has two levels only, so the keys below a permitted key are a guess (SF-036). The top-level `volumes` and `networks` rows are again written as "no X" (SF-037). Service keys `secrets` and `configs` are not in the service row, yet the top level permits them (SF-038). |
| SF-016 | In part | "A bind mount names exactly `binds/<name>` … Directly before the start it checks, without following a link, that it is a real folder below a real `binds/`." and "`files/` … No mount can name it." Missing: the top-level row permits `secrets`/`configs` with "`file` below `files/`", which is a mount that names `files/` (SF-038). |
| SF-017 | In part | "makes the final form one time … starts the project from that final form only … Thus the check and the start read one input." Missing: `include`, `extends` with a file and `env_file` cannot be refused on a final form, and a variable can carry a path past the check before the reader (SF-034). Compose reads `.env` of the folder at the start by default (SF-042). |
| SF-018 | In part (R-226) | "It has no right to use Docker, and is in no group that has it." and "Only the administrator of the machine can set them." Missing: the binding to the origin and the device has no way to be true (SF-039); the programs that the administrator program runs have no rule (SF-035). |
| SF-019 | In part; `pull` accepted (R-232) | "`adopted_apply` uses the recorded paths, never the labels at the time of the item." and "`up` never builds an image." Not corrected: way (a), because the text permits "the user that owned it at the adoption" to write the file where the reviewer wrote "the user that adopted it" (SF-040). Not corrected: way (d), `up` still applies a file that the owner changed. R-232 does not speak of (a) or (d). |
| SF-020 | Corrected in the text (R-227); the actor rule is not decided | "Each other container is refused (`not_adopted`)." and "`start` is permitted only for a container of an application that Anima installed, or whose restart policy is not `no`." The table of Section 4.14 gives each type to an API key and an MCP tool; no ruling decides that (Needs the owner, 1). |
| SF-021 | Corrected (R-227, R-230) | "`containers`: the stopped containers that have the project label of an application that Anima installed." and "The applications program refuses it unless 'The hub may delete volumes' is on." New contradiction: SF-045. |
| SF-022 | Corrected (R-227) | "`tail` is 0 to 1,000." and "The hub makes the item only for an act with a login or a browser session." New contradiction in Section 4.14: SF-046. |
| SF-023 | Corrected | "The hub checks the text of the file only: it opens no file and no address … and it shows no text of an error of a reader." |
| SF-024 | In part | "It holds no output of the Docker program and no text of a file." Missing: the check "before the reader opens anything" passes a path that a variable supplies (SF-034). The reader now runs as the administrator, so it can open more files than before. |
| SF-025 | Corrected | "applies the rules of Section 6.4.5 of today to the kept file" and "It sets no tag on an image." and "A second `application_back` is refused until the next `application_apply`." Kept images with "All containers" on: SF-045. |
| SF-026 | Corrected in the text; the part before W7 accepted (R-231) | "the machine keeps the files for the life of the application" and "The values of an environment file can also be read by each user of the machine who can use Docker." Wrong reference: Section 8 names "Section 6.4.4"; the files are in 6.4.6. |
| SF-027 | Corrected (R-229) | "With 'Open to the network' off, the address of the machine is `127.0.0.1`; a file that names a different address, or none, is refused." and "adds to each service `cap_drop: [NET_RAW]`". |
| SF-028 | Corrected | "The project name is `anima-` and the 32 hexadecimal digits of that UUID. The applications program refuses the item when a project of that name exists with a different folder." |
| SF-029 | In part | "An item of a container type does not use the slot of jobs." Missing: `mem_limit` and `pids_limit` are set only "when the service has none", so a file sets its own high value. `logging` has no size limit. An `application_apply` that waits for health for `limits.seconds` holds the one place for an item that changes Docker, so a `stop` waits. Contradictions with K10 0.3: SF-047. |
| SF-030 | Corrected | "To set 'Manage applications' off does not stop a container. … the applications program shows each application that Anima installed and asks if it stops them." The automatic reset at a new enrolment has no such question (Low). |
| SF-031 | In part (R-233) | "A recipe of the catalogue of Anima names each image by a digest" and "The pull uses no login of the machine, unless the owner named the registry at the machine." Missing: no way is named for "no login" (SF-041). Way (b) stays: a file can start an image that is already on the machine (SF-041). |
| SF-032 | Corrected | "For a container type the entry of the log of the machine has: the type; the container identifier and the action; … The applications program writes this log; the daemon and the hub cannot change it." |
| SF-033 | In part | Each point has a sentence, apart from two. The engine on a Mac is left open: "Task H30 says what the applications program is there." The actor table exists and is not decided (Needs the owner, 1). |

What the reviewer says needs the owner:

1. **The column "An API key or an MCP tool" of Section 4.14.** No ruling R-226 to R-233 decides it. It lets an agent, which can be led by text that is not trusted (SF-020 e), send a Compose file of its choice (`application_apply`), stop containers, prune, and make an adopted stack get new images. For the last, the table names R-232. R-232 speaks of "the hub" and has the condition that "the screen says" the sentence at each update; an API key has no screen.
2. **SF-019 way (d), and way (a) if SF-040 is not taken.** `up` applies a changed file of an adopted project with no rule. R-232 accepts `pull` and "no profile", not this.
3. **SF-039.** With R-226 B, each program of the user of the daemon can do what the rules permit with no hub and outside the rules for actors. The reviewer's text for option B said "the daemon that speaks to the hub has no right to Docker". That sentence is still true, but the effect is not stated in a ruling.
4. **SF-041, if the text is not changed.** The hub can start an image that is on the machine.
5. **SF-043, if the text is not changed.** A label of a file from the hub can give orders to a reverse proxy or an updater of the owner.
6. **A Mac with Docker Desktop (SF-035).** The contract would become version 0.4 while the limit on that platform is left to task H30.
7. **"The hub may delete volumes" off does not protect the data.** A file from the hub runs a command of its choice with the volume of its own application mounted, and can empty it. R-230 decides the removal of a volume, not this. One sentence in Section 3 of K10 and his acceptance are needed.

### SF-034: Section 4.5: three rules are on the final form, where they cannot apply, and a variable passes the check before the reader

Requirements: task H14; the text of the container work types, version 3 of the proposal, Section 4. Severity: High.

- Way: `env_file: ${P}`, with `P=/path/of/a/file` in the environment file. The text of the file has no absolute path and no `..`. The reader, which now runs as the administrator, opens the file and puts its `KEY=VALUE` lines into `environment` of the final form. The final form then has only permitted keys (not verified: that Compose drops `env_file` from its output). The container sends the values out. Targets: the environment file of a different application, `/etc/environment`, files in `/etc/default`.
- `include` and `extends` with a file are resolved before the final form in the same way. Only the hub refuses them by text, and the hub is not the control.
- Who: a person who controls the hub; an API key or MCP tool (Section 4.14).
- Change: "The applications program applies the list of keys two times: to the file as it came, before a variable is filled, and to the final form. In the file as it came it refuses `env_file`, `include` and `extends` at any level. It fills the variables itself, and after that, before a reader opens anything, it refuses a value of `file` or of a `volumes` source that is absolute or has `..`."

State: Open. The text is a proposal and no code exists. Found by the second read of 2026-11-19.

### SF-035: Section 4.1: an administrator program runs `docker` and the Compose plug-in, and no rule says who can write them

Requirements: task H14; the text of the container work types, version 3 of the proposal, Section 4. Severity: High on a Mac and with a rootless engine; Low on Linux with the system engine.

- Way: with Docker Desktop the command line, the plug-in folder `~/.docker/cli-plugins` and the socket belong to the user. A program of that user replaces the plug-in, sends one request on the Unix socket, and its code runs as the administrator.
- The text gives this to task H30; the contract would be accepted with this limit not written.
- Who: each program of the user of the daemon.
- Change: "The applications program runs `docker` and its Compose plug-in only at paths that are fixed in the program. It refuses to start when one of them, or a folder above, can be written by a user other than the administrator. Where the engine or its command line belongs to a user (Docker Desktop, rootless), the applications program runs as that user and not as the administrator, and this version gives such a machine no container work until task H30 is accepted by a ruling."

State: Open. The text is a proposal and no code exists. Found by the second read of 2026-11-19. R-237 (2026-11-19): not accepted. Version 0.4 waits for the design of task H30 for a Mac and for a rootless Docker. R-246 (2026-11-19): on a machine whose Docker belongs to a user the applications program runs as that user and never as the administrator, and it refuses to start when a Docker program can be written by a user other than the one it runs as. The third form of the text has this (Section 4.1).

### SF-036: Section 4.5: "at any level" with a list of two levels

Requirements: task H14; the text of the container work types, version 3 of the proposal, Section 4. Severity: Medium.

- The members below a permitted key have no rule: `healthcheck`, `logging.options`, the long forms of `ports`, `volumes` (`bind.propagation`, `bind.selinux`, `volume.subpath`), `networks` of a service (`ipv4_address`, `mac_address`, `aliases`, `driver_opts`) and `depends_on`.
- A final form that Compose writes always has a top-level `name`, a `name` on each network, absolute bind paths and long forms (not verified). Read exactly, the list refuses each file; an implementer will then permit by guess.
- The text does not say if the final form is made by `docker compose config` or by code of the program.
- Who: the implementer; then a person who controls the hub.
- Change: name the reader. List the permitted members below each key. Add: "the members that the reader adds are permitted only with these exact values: top-level `name` equal to the project name; `name` of a network equal to `<project>_<key>`; a bind source equal to the real path of `binds/<name>`."

State: Open. The text is a proposal and no code exists. Found by the second read of 2026-11-19.

### SF-037: Section 4.5, top level: `networks` and `volumes` are "no X" lists again

Requirements: task H14; the text of the container work types, version 3 of the proposal, Section 4. Severity: Medium.

- Way: `ipam` with a subnet of the local network, of the network of Anima or of the address of the hub. The machine then sends that range to the bridge: it loses the hub until a person is at the machine, or a container answers for a local address.
- Who: a person who controls the hub; an API key.
- Change: "A top-level volume has no member. A top-level network has only `driver: bridge` and `internal`. `ipam`, `enable_ipv6`, `attachable`, `labels` and each other member are refused."

State: Open. The text is a proposal and no code exists. Found by the second read of 2026-11-19.

### SF-038: `secrets` and `configs`: three sentences that do not agree

Requirements: task H14; the text of the container work types, version 3 of the proposal, Section 4. Severity: Medium.

- The service row does not permit `secrets` or `configs`, so the top-level permission has no use, or the implementer adds the service key with no rule.
- "`file` below `files/`" can name only the final form, the environment file and the kept files. It contradicts "No mount can name it" in Section 4.6. If Compose mounts the file writable (not verified), SF-016 (b) is open again.
- Change: "`secrets` and `configs` at the top level have `content` only; `file`, `environment`, `external` and `name` are refused. A service may have `secrets` and `configs` with `source`, `target` and `mode` only."

State: Open. The text is a proposal and no code exists. Found by the second read of 2026-11-19.

### SF-039: Section 4.2: the binding to the origin and the device has no mechanism; Section 4.1 does not say what the socket gives

Requirements: task H14; the text of the container work types, version 3 of the proposal, Section 4. Severity: Medium.

- The applications program "never talks to the hub" and "does not trust the daemon". It can learn the origin only from the daemon or from a file of the user of the daemon.
- Way: a local user who runs the daemon (SF-018 c), or each program of that user, sends requests on the socket with no hub. The actor table, the audit log of the hub, the rule "User only" for `container_follow` and for `volumes` true, and the sentence of R-232 do not apply to such a request.
- With "All containers" on, that program stops each container and reads each log. With "The hub may delete volumes" on, it deletes the data.
- A different local user can make a socket at the path if the folder permits, and the daemon then gives it the environment file.
- Changes:
- Section 3 of K10: "On a machine whose setting is on, each program of the user of the daemon can do, with no hub, each thing that this paragraph gives to a person who controls the hub; the rules for actors are rules of the hub and do not limit it."
- Section 4.2: "The binding protects against a change of the hub by the owner; it does not limit a program of the user of the daemon."
- Section 4.1: "The socket is in a folder that only the administrator can write. The daemon checks that the peer is the administrator. No request on the socket changes a setting."
- Section 4.10: "The applications program checks the signature of the operator (task W7)."

State: Open. The text is a proposal and no code exists. Found by the second read of 2026-11-19. R-236 (2026-11-19): Accepted until task W7; the contract says it.

### SF-040: Section 4.9: "or the user that owned it at the adoption"

Requirements: task H14; the text of the container work types, version 3 of the proposal, Section 4. Severity: High.

- Way: the administrator started one time a stack whose file is in the home folder of a user who has no right to Docker, and adopted it. That user adds `privileged` or a mount of `/`, then waits for the next `up`. This is way (a) of SF-019.
- The check also covers only the Compose files and the folders above them. It does not cover `.env`, an `env_file`, or a file that `include` or `extends` names.
- `adopted_apply` also does not say with which environment and which login Compose runs. With an empty environment, a file with `${X:-default}` gives a different stack than the owner started.
- Who: a local user who is not the administrator.
- Change: "… can be written by a user other than the administrator. At the adoption the applications program records the SHA-256 of each file that the reader opens for the project. `adopted_apply` with `up` refuses the item when one is different (`not_adopted`), until the owner adopts the project again at the machine." This also corrects way (d).

State: Open. The text is a proposal and no code exists. Found by the second read of 2026-11-19. R-235 (2026-11-19): Accepted. Anima does not check the file and starts it as it is; no fingerprint is built. The text must say that Anima does not check, not that the owner of the file may change it.

### SF-041: Images: "no login" has no way, and an image that is on the machine can be started

Requirements: task H14; the text of the container work types, version 3 of the proposal, Section 4. Severity: Medium.

- With an empty environment the `docker` program reads the configuration of the home folder of the administrator, with its logins and helper programs (not verified).
- A file from the hub with `pull` false, or with a name that the engine has, starts a private image of the owner or of an adopted project, with an `entrypoint` of the hub and a network. This is way (b) of SF-031.
- Who: a person who controls the hub; an API key.
- Change: "The applications program gives `docker` a configuration folder of its own (`--config`) that holds no login. A login of a registry that the owner named is put there by the owner at the machine. `application_apply` starts only an image that the applications program pulled for this application, by the digest of that pull; it never starts an image that was on the machine for a different reason."

State: Open. The text is a proposal and no code exists. Found by the second read of 2026-11-19.

### SF-042: The environment: the prefix rule is a list of refused names

Requirements: task H14; the text of the container work types, version 3 of the proposal, Section 4. Severity: Low to Medium.

- "It refuses an environment file that sets a name that starts with `COMPOSE_` or `DOCKER_`" leaves `HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY`, `SSL_CERT_FILE`, `HOME`, `PATH`, `BUILDKIT_*`. Not verified: whether a Compose version puts the variables of the file into its own process.
- The environment file is in `files/`. If its name is `.env`, Compose reads it again at the start.
- A value of the final form that holds `${…}` is filled a second time at the start.
- An empty environment has no `PATH` and no `HOME`; the text does not say that the program gives each path as an argument.
- Change: "The applications program fills the variables itself. The environment file is never given to Compose and has a name that Compose does not read. The final form holds no `$` that a reader fills (each is written `$$`). Each path that `docker` needs is an argument."

State: Open. The text is a proposal and no code exists. Found by the second read of 2026-11-19.

### SF-043: `labels`

Requirements: task H14; the text of the container work types, version 3 of the proposal, Section 4. Severity: Medium, when the machine has a program that reads labels.

- Only `com.docker.` is refused. Traefik, caddy-docker-proxy, Watchtower and similar programs take orders from the labels of each container.
- Way: a label makes a route in the reverse proxy of the owner for a name of the owner, or opens the service to the internet with "Open to the network" off.
- Who: a person who controls the hub; an API key.
- Change: "`labels` is permitted only for a key whose prefix the owner named at the machine; after the install no prefix is named." Or say the power in Section 3 and let the owner accept it.

State: Open. The text is a proposal and no code exists. Found by the second read of 2026-11-19.

### SF-044: Values with no upper limit

Requirements: task H14; the text of the container work types, version 3 of the proposal, Section 4. Severity: Low.

- `mem_limit` and `pids_limit` of a file (a very high number, `-1`); `stop_grace_period`, which holds the one place for an item that changes Docker; the `interval` of a `healthcheck`; the size of a `tmpfs`; `logging` with no `max-size`; the count of services.
- Change: "The applications program uses the lower of the value of the file and its own limit for `mem_limit`, `pids_limit` and `stop_grace_period`, sets `max-size` and `max-file` of the log, and refuses a file with more services than its limit."

State: Open. The text is a proposal and no code exists. Found by the second read of 2026-11-19.

### SF-045: Section 4.12, "All containers" on

Requirements: task H14; the text of the container work types, version 3 of the proposal, Section 4. Severity: Low.

- `docker image prune` removes images with no tag. An image that `keep` holds by identifier can have no tag, so it is removed. This contradicts "never an image that `keep` holds".
- Change: put "never an image that `keep` holds" in a sentence that applies to both cases.

State: Open. The text is a proposal and no code exists. Found by the second read of 2026-11-19.

### SF-046: Section 4.14: "where the table says 'User only', the settings at the machine are the second control"

Requirements: task H14; the text of the container work types, version 3 of the proposal, Section 4. Severity: Low.

- This is false for `container_follow`: no setting of Section 4.2 is for logs.
- Change: "… for `application_remove` with `volumes` true, the setting at the machine is the second control. `container_follow` has no second control: a person who controls the hub, and a program of the user of the daemon, read the logs that Section 6.4.3 permits."

State: Open. The text is a proposal and no code exists. Found by the second read of 2026-11-19.

### SF-047: Contradictions with K10 0.3 that Section 4.3 does not change

Requirements: task H14; the text of the container work types, version 3 of the proposal, Section 4. Severity: Low.

- Section 3, the row of the switch: "If the hub answers such a claim with an `Item`, the daemon drops the answer and starts nothing." The new text gives container items to a machine with `switch` `off`.
- Section 4: a claim each 10 seconds only "while its switch is on", and one in each hour when off. A machine with `switch` `off` and `applications` `on` then gets a `stop` after as much as one hour.
- Section 4: "`unclean` … The hub gives no item to such a machine." The sentence "`stop` is never refused for `policy` or `unclean`" then has no effect.
- Section 11, `WORK_OFF` after 5 refused items: 5 refusals of a file take the machine out of each work, also a `stop`.
- Section 6, first lines: "There is no type that runs a command which the hub names", and "The list is fixed in the daemon program."
- Section 7: `NOT_HOLDER`, `TOKEN_REFUSED`, a stop and `expired` "end the job at once, for each type". The new text does not say what that is in the middle of an `up`.
- `types` and `free.slots` of the claim for container types are not said.
- Change: one sentence in each of these places.

State: Open. The text is a proposal and no code exists. Found by the second read of 2026-11-19.

### SF-048: Points that an implementer must guess

Requirements: task H14; the text of the container work types, version 3 of the proposal, Section 4. Severity: Low.

- What "an address" is in "a file that names … an address": an image name holds a registry address.
- What "a path that is absolute" covers: `working_dir`, `tmpfs` and the target of a volume are absolute paths in the container.
- How a protected container is named (an identifier changes at each new make), and if `adopted_apply` and `application_apply` respect the list. Section 4.2 says "no item"; Section 4.7 names two types.
- If the adoption of a project whose name starts with `anima-` is refused.
- The limit of the YAML reader has no numbers.
- 256 KiB in a JSON string can be more than 512 KiB.
- "With all profiles on" while `profiles` is not a permitted key.
- The final form holds the values of the environment file; Section 4.6 says "The owner can read it" and gives the rule of access only for the environment file.
- The numbers 6.4.7 and 6.4.8 do not exist.

State: Open. The text is a proposal and no code exists. Found by the second read of 2026-11-19.

## The third read of the container work types (task H14, 2026-11-19)

After R-234 to R-238 and R-246 the text was written a third time (version 4 of [hosted-applications-stage-2.md](proposals/hosted-applications-stage-2.md), Section 4). A third separate reviewer agent read it against SF-015 to SF-048, the rulings R-209 to R-246 and the record of task H30. It did not read the proposal. Nothing was run; the behaviour of Docker and Compose in these entries is from the knowledge of the reviewer, marked "not verified" where it carries a finding.

The last line of the reviewer, in its words: "No finding remains that the owner has not decided; the open points are changes of the text." It gave one condition, in its words:

None, on the condition that the changes of SF-051, SF-066 and SF-067 are taken. If the writer keeps the text of one of them, that point needs him:
- SF-051: accept that settings made for one hub serve a different hub (I recommend the change).
- SF-066: give the stop and restart of an adopted project to an API key (I recommend "No").
- SF-067: give the update of a catalogue application to an API key (I recommend "No" until he says yes).

No finding remains that the owner has not decided; the open points are changes of the text.

The writer took the three changes (SF-051, SF-066, SF-067), so no point goes to the operator from this read. The open points are 17 weaknesses of the text (SF-049 to SF-065) and four places where the text was wider or softer than a ruling (SF-066 to SF-069). The fourth form of the text (version 5 of the proposal) has the change that each entry proposes. No reviewer has read the fourth form; the owner of `contracts/work-route.md` reviews the text when it is put in that file.

How the reviewer judged the 34 earlier findings:

| Finding | Result | Sentence of the third form, or what is missing |
| --- | --- | --- |
| SF-015 | Corrected (R-228 removes `full`) | "A key, at any level, that is not here is refused (`file_not_permitted`)." Value rules that are still missing are SF-053 to SF-057. |
| SF-016 | Corrected | "No mount can name `files/`." and "A bind mount names exactly `binds/<name>` … it checks, without following a link, that it is a real folder below a real `binds/`." |
| SF-017 | Corrected | "It starts the project from the final form only, with the project name and the path of the file as arguments. It never gives the environment file to Compose". Residue: SF-052, SF-055. |
| SF-018 | Decided by R-226, R-241, R-236 | The reset of the settings at a new enrolment, which the finding proposed, is replaced by a duty of the owner (SF-051). |
| SF-019 | Decided by R-232 (`pull`) and R-235 (ways a, d); way (b) corrected | "`adopted_apply` uses the recorded paths, never the labels at the time of the item." and "`up` never builds an image." |
| SF-020 | Corrected (R-227); actors decided by R-234 | "Each other container is refused (`not_adopted`)." The table of 4.13 is wider than R-234 in two cells (SF-066, SF-067). |
| SF-021 | Corrected (R-227, R-230) | "`containers`: the stopped containers that have the project label of an application that Anima installed." and "refuses it unless 'The hub may delete volumes' is on". |
| SF-022 | Corrected (R-227, R-234) | "`tail` is 0 to 1,000." and, in the table, "No. No MCP tool returns a log". |
| SF-023 | Corrected | "It opens no file and no address, and shows no text of an error of a reader". |
| SF-024 | Corrected | "It holds no output of the Docker program and no text of a file." and "No file of the machine other than the two files of the item is opened for this." |
| SF-025 | Corrected in part | Present: "It sets no tag on an image." and the one step back. Missing: "applies the rules of Section 4.5 of today to the kept file", but the kept file is the final form, which has `name`, real bind paths and added members that the list refuses (SF-058). |
| SF-026 | Corrected in the text; the part before W7 decided by R-231 | "the applications program keeps the two files in the folder of the application (Section 4.6) for the life of the application, and `application_remove` deletes them." Not changed: the K10 Section 8 rule "asks for a secret only after the isolation exists" (SF-061). |
| SF-027 | Corrected (R-229) | "With 'Open to the network' off, `host_ip` is `127.0.0.1`; a different address, or none, is refused". |
| SF-028 | Corrected | "The project name is `anima-` and the 32 hexadecimal digits of that UUID. The item is refused when a project of that name exists with a different folder". |
| SF-029 | Corrected in part | Present: "`free.slots` counts jobs only." and the disk check. Missing: zero and negative limits (SF-056); the place of the health wait has no mechanism (SF-060); the machine still "claims no item" when `unclean` (SF-061). |
| SF-030 | Corrected | "To set 'Manage applications' off does not stop a container. The applications program then shows each application that Anima installed and asks if it stops them". |
| SF-031 | Corrected (R-233) | "The applications program starts only an image that it pulled for this application, by the digest of that pull." |
| SF-032 | Corrected in part | The members are listed. Missing: "writes this log where the daemon and the hub cannot change it" is false on an engine of a user, where the daemon is the same user (SF-063). |
| SF-033 | Corrected in part | Each point has a sentence except one: what the machine does with a container item when it cannot reach the hub (K10 Section 7, the SF-005 paragraph, has `build` and `runner` only; `limits.orphanSeconds` has no meaning for the kind `host`). |
| SF-034 | Corrected | "A key that is not in the list is refused at any level: thus `include`, `extends`, `env_file`, `profiles`, `build` and `name` are refused before a variable is filled." |
| SF-035 | Decided by R-241 | The text has both rules. The way the kind is decided is open (SF-049). |
| SF-036 | Corrected in part | The reader is named and the members below each key are listed. Missing: the short forms, the kept final form (SF-058), and the YAML rules of the reader and the writer (SF-052, SF-053). |
| SF-037 | Corrected | "A top-level volume: No member" and "A top-level network: `driver` with the value `bridge`; `internal`". |
| SF-038 | Corrected | "A top-level secret or config: `content` only" and "`source`, `target`, `mode`, with a source of this file". See SF-057 for `content` of a secret. |
| SF-039 | Decided by R-236 | "No request on the socket changes a setting of Section 4.2." The binding is dropped (SF-051). |
| SF-040 | Decided by R-235 | The third point of the finding (which environment and login Compose gets) is not corrected (SF-059). |
| SF-041 | Corrected | "a configuration folder of its own that holds no login of a registry". |
| SF-042 | Corrected | "each `$` written `$$`" and "the file has a name that Compose does not read" and "Each path that `docker` needs is an argument." |
| SF-043 | Corrected | "After the install no prefix is named, so no label is permitted". |
| SF-044 | Corrected in part | Missing: a value of zero or below is "the lower" and means no limit (SF-056); `cpus` and `healthcheck.interval` have no limit; no count limit for networks and volumes. |
| SF-045 | Corrected | "In each case it never removes an image that `keep` holds, and nothing of the list 'Protected'." |
| SF-046 | Corrected | "No setting of the machine is for logs: a person who controls the hub, and a program of the user of the daemon, can read the logs". |
| SF-047 | Corrected in part | Corrected: the switch row, the Section 6 lines, `types`, `free.slots`, the end in the middle of an `up`. Missing: three contradictions with K10 0.3 (SF-061). |
| SF-048 | Corrected | "The list holds names of containers and names of projects." and "A project whose name starts with `anima-` cannot be adopted." and "a depth of more than 32, and more than 32 services". Residue: a 256 KiB file in a JSON string can exceed the 1 MiB answer when it has many escaped characters (Low). |

### SF-049: Section 4.1, how the kind of engine is decided. High on a Mac, Low elsewhere

Requirements: task H14; the text of the container work types, third form (version 4 of the proposal, Section 4).

- Way: "The socket of the engine belongs to the administrator" has no rule for a link. On the measured Mac `/var/run/docker.sock` and `/usr/local/bin/docker` are links that root owns, to a socket and a program that the user owns.
- An implementation that reads the owner of the path itself decides "system engine" and runs as the administrator. If the write check is also made on the link, it starts a program that the user can replace. That is SF-035 again and breaks R-241 point 2.
- Who: each program of the user of the Mac.
- The text also does not say when the kind is decided, or what happens when the owner of the socket changes later.
- Change: "The administrator records the kind when he sets 'Manage applications' on. The kind is that of the owner of the real socket, after each link is resolved. At each start the applications program resolves each link of the socket path and of the two program paths. It refuses to start when the owner of the real socket is not the user that it runs as, or when the kind is different from the recorded one. When it runs as the administrator it also refuses when a link, the real file, or a folder above one of them can be written by a user who is not an administrator."

State: Open. The text is a proposal and no code exists. Found by the third read of 2026-11-19. The fourth form of the text (version 5 of the proposal) has the change that this entry proposes.

### SF-050: Section 4.1, engine of a user: the accepted user need not be the owner of the engine. Medium

Requirements: task H14; the text of the container work types, third form (version 4 of the proposal, Section 4).

- Way: a computer has daemons for two accounts (R-206, R-208). The socket belongs to A; the administrator names B. The program runs as A and takes requests of B, so B gets A's engine through Anima. The sentence of R-241, "to which Anima gives nothing that it did not have", is then false for B.
- Who: a program of account B.
- Change: "On an engine of a user the accepted user is the owner of the socket of the engine. The applications program refuses to start when the two are different."

State: Open. The text is a proposal and no code exists. Found by the third read of 2026-11-19. The fourth form of the text (version 5 of the proposal) has the change that this entry proposes.

### SF-051: Section 4.2, "The owner sets the settings off when he moves a machine to a different hub". Medium

Requirements: task H14; the text of the container work types, third form (version 4 of the proposal, Section 4).

- Way: the owner moves a machine and does not set them off. "All containers", "The hub may delete volumes", the adopted projects and the label prefixes then serve the new hub.
- R-236 accepts requests with no hub; no ruling says that settings made for one hub serve a different one.
- Who: a person who controls the new hub.
- Change: "The daemon sends the origin of its hub and its device identifier with each request. The applications program records the first pair in a file of its own. When a request has a different pair, it refuses each container item (`applications_off`) until the administrator sets 'Manage applications' on again. This protects an owner who forgets. It does not limit a program of the user of the daemon, which can send the old pair (R-236)."
- With this change the rest of the risk is the one R-236 accepts.

State: Open. The text is a proposal and no code exists. Found by the third read of 2026-11-19. The fourth form of the text (version 5 of the proposal) has the change that this entry proposes.

### SF-052: Section 4.5 steps 1 and 5, the reader and the writer of YAML have no rules. Medium (not verified)

Requirements: task H14; the text of the container work types, third form (version 4 of the proposal, Section 4).

- Way: the program's writer and the reader of Compose can read one scalar differently: `no`, `yes`, `on`, `0444`, `1e3`, `~`, a string that starts with `!`, `&`, `*`, `{` or `[`, or a value with a line end (`content` of a config always has them). A writer that does not quote every string lets a value become structure in what Compose reads, after the check.
- Who: a person who controls the hub; an operator file.
- Change, step 1: "It also refuses a second document, a tag, a key that is not a string, a key that is there two times, and the merge key."
- Change, step 5: "It writes the final form as JSON: each string in quotes with each control character escaped, each number and each truth value from its own typed value. `mode` is written as a string of octal digits."
- Add to 4.1: "It refuses to start when the Compose program is older than the lowest version that it knows."

State: Open. The text is a proposal and no code exists. Found by the third read of 2026-11-19. The fourth form of the text (version 5 of the proposal) has the change that this entry proposes.

### SF-053: Section 4.5 step 3, the filling of variables is not fully said. Medium to Low

Requirements: task H14; the text of the container work types, third form (version 4 of the proposal, Section 4).

- Not said: whether the filling is on the text or on a scalar after the read; what `$NAME`, `$$`, `${NAME-x}`, `${NAME:?x}` and a nested variable in the file are; what a name that the environment file does not have gives; whether a filled value is filled again; whether quotes in `NAME="a b"` are part of the value; a name that is there two times.
- Step 4 checks the result again, so the effect is a difference between the hub and the machine, or structure through a filled value if the filling is on the text.
- Change: "The variables are filled in a scalar value after the YAML is read. The result is one scalar of text; it is never read again as YAML, and it is not filled a second time. `$$` is one `$`. Each other `$` that is not one of the two forms is refused. A name that the environment file does not have and that has no `:-text` is refused. In the environment file the value is the text after the first `=`, with no quote rule; a name that is there two times is refused."
- Add: "A name of a service, a volume, a network, a secret or a config has 1 to 63 characters of `a-z`, `0-9`, `-`, `_`, and starts with a letter or a digit."

State: Open. The text is a proposal and no code exists. Found by the third read of 2026-11-19. The fourth form of the text (version 5 of the proposal) has the change that this entry proposes.

### SF-054: Section 4.5, `image`: "the characters that a registry permits". Low

Requirements: task H14; the text of the container work types, third form (version 4 of the proposal, Section 4).

- Way (a): a value that starts with `-` is an option to `docker pull` (not verified what it can do there).
- Way (b): the registry host can be `localhost:5000`, an address of the local network or of the network of Anima. The engine then makes requests there. This is not in the paragraph of K10 Section 3.
- Who: a person who controls the hub; an operator file.
- Change: "`image` starts with `a-z` or `0-9` and has the form of a reference; the program gives it after `--`." Add to the Section 3 paragraph: "The engine gets an image from each address that the file names, also an address of the local network."

State: Open. The text is a proposal and no code exists. Found by the third read of 2026-11-19. The fourth form of the text (version 5 of the proposal) has the change that this entry proposes.

### SF-055: Section 4.5, value rules that are missing for permitted keys

Requirements: task H14; the text of the container work types, third form (version 4 of the proposal, Section 4).

- `environment`, Low (not verified): an entry with a name and no value is filled by Compose from its own process. Change: "Each entry of `environment` has a value; the form with a name only is refused."
- `tmpfs`, Low: the short form takes options (`/run:exec,suid,dev,size=…`). Change: "`tmpfs` is an absolute path with no `:` and no `,`; the program sets each option."
- `ports`, `volumes`, `secrets`, `configs`, Low: the short forms (`"8080:80"`, `"data:/data"`, a name only) are not said. Change: "Only the long form is permitted" for these four keys. For `ports`: "`published` and `target` are one whole number, not a range; `protocol` is `tcp` or `udp`."
- Volume `target`, secret and config `target`, Low: `/`, `/proc`, `/sys`, `/dev`, or a path over a different mount only harm the same container (not verified that the engine refuses `/`). Change: "A `target` is an absolute path with no `..`; it is not `/`; two mounts of one service have different targets."
- A volume mount with no `source` makes a volume with no name, which `application_remove` does not count. Change: "`source` is always present."
- `user`, `entrypoint`, `command`, `hostname`, `expose`, `restart`, `stop_signal`, `aliases`, and a named volume in two services of one file: I found nothing that reaches outside the application. One sentence with the form of each value is enough.
- `labels`, Low: the administrator can name a prefix that covers the labels of Compose. Change: "A prefix that starts with `com.docker.` cannot be named."

State: Open. The text is a proposal and no code exists. Found by the third read of 2026-11-19. The fourth form of the text (version 5 of the proposal) has the change that this entry proposes.

### SF-056: Section 4.5, "the lower of the value of the file and its own limit". Medium (not verified)

Requirements: task H14; the text of the container work types, third form (version 4 of the proposal, Section 4).

- Way: `mem_limit: 0`, `pids_limit: 0` or `-1` is the lower number and means no limit in the engine. `cpus` has no limit of the program. Thousands of top-level networks use all the address ranges of the engine, and no other project can then make a network.
- Who: a person who controls the hub; the author of a recipe.
- Change: "A value of `mem_limit`, `pids_limit`, `cpus` or `stop_grace_period` that is zero or below is refused. `cpus` has the same rule as `mem_limit`. A file has at most 16 networks and 64 volumes. `healthcheck.interval` is at least 5 seconds."

State: Open. The text is a proposal and no code exists. Found by the third read of 2026-11-19. The fourth form of the text (version 5 of the proposal) has the change that this entry proposes.

### SF-057: Section 4.5, `content` of a top-level secret. Low (not verified)

Requirements: task H14; the text of the container work types, third form (version 4 of the proposal, Section 4).

- As far as I know Compose has `content` for a config and not for a secret. An implementer who finds that it fails will write the value to `files/` and use `file`, which is a mount that names `files/` (SF-038, SF-016 b).
- Change: remove `secrets` from the top level and from the service row, or add: "If Compose does not take `content` for a secret, the program refuses the file; it never writes a `file` member."

State: Open. The text is a proposal and no code exists. Found by the third read of 2026-11-19. The fourth form of the text (version 5 of the proposal) has the change that this entry proposes.

### SF-058: Sections 4.6, 4.7, 4.10: `application_back` checks the final form with a list that refuses a final form. Medium

Requirements: task H14; the text of the container work types, third form (version 4 of the proposal, Section 4).

- The kept things are "the final form, the environment file and the image digests". The file as it came is not kept. The final form has `name`, real bind paths, `cap_drop`, the limits and the log options.
- Read exactly, each `application_back` is refused; else the implementer writes a second list by guess. A rule of today on a variable also cannot be applied to a form in which the variables are filled.
- Change in 4.6 and 4.7: "With `keep` 1 the machine keeps the Compose file as it came, the environment file and the image digests of the version before." Change in 4.10: "It makes the final form again from the kept Compose file and the kept environment file by the six steps of Section 4.5 of today, with the kept digests and no pull."

State: Open. The text is a proposal and no code exists. Found by the third read of 2026-11-19. The fourth form of the text (version 5 of the proposal) has the change that this entry proposes.

### SF-059: Section 4.8, "It runs Compose in the recorded folder, as the owner would". Medium

Requirements: task H14; the text of the container work types, third form (version 4 of the proposal, Section 4).

- It contradicts Section 4.1 ("an environment that holds only what the program sets", "a configuration folder … that holds no login").
- With the environment of 4.1 a file with `${BIND:-0.0.0.0}` or `${TAG:-latest}` gives a different stack than the owner started: a port open to the network, a different image. A private image fails at `pull`. No ruling accepts this; R-235 is about a changed file.
- Not said: that each recorded file is given with `-f`, the project folder and the project name as arguments; that `.env` of the folder is read; the flags of `up`.
- On a system engine Compose runs as the administrator and reads `.env`, `env_file`, `include` and `extends` files with that right; the sentence of R-235 in the text names only "such a file" (see SF-068).
- Change: "It gives each recorded file, the recorded folder as the project folder, and the project name as arguments, with the environment and the configuration folder of Section 4.1. Compose reads the `.env` file of the recorded folder. A variable that the owner had in his shell is not there. At the adoption the applications program makes the form of the project in this way and refuses the adoption when its services have a different image, port or mount than the containers that run. The adoption screen says: 'Anima starts this stack with no variable of your shell and with no registry login of yours.' `up` is `up -d --no-build`, with `--pull always` when `pull` is true. It runs as the user that the applications program runs as."

State: Open. The text is a proposal and no code exists. Found by the third read of 2026-11-19. The fourth form of the text (version 5 of the proposal) has the change that this entry proposes.

### SF-060: Sections 4.3 and 4.7, the queue. Medium to Low

Requirements: task H14; the text of the container work types, third form (version 4 of the proposal, Section 4).

- Order of choice: only "the container item first" is said. A `stop` waits behind an `application_apply` in its pull or its `up` (up to 1,800 seconds), and behind each older container item. An API key may make `container_action`, so it can keep each job of a machine waiting.
- "The wait for health does not hold the place" has no mechanism: the hub counts leases, and no state of K10 means "waits for health". If the place is free, a second `application_apply`, `application_back` or `application_remove` of the same application runs while the first waits, and the first reports the health of containers that the second made.
- Change: "The hub gives, in this order: `container_action` with `stop`; each other container item, the oldest first; a job. The state `finishing` of a container item is the wait for health; the hub counts an item that changes Docker only in `claimed`, `preparing` and `running`. While an item of an application or of a project is not final, the hub gives no other item of it, apart from `container_action` with `stop`. The hub gives one machine at most 6 `container_action` items in a minute for one API key."

State: Open. The text is a proposal and no code exists. Found by the third read of 2026-11-19. The fourth form of the text (version 5 of the proposal) has the change that this entry proposes.

### SF-061: Section 4.3, contradictions with K10 0.3 that remain. Medium for the first, Low for the others

Requirements: task H14; the text of the container work types, third form (version 4 of the proposal, Section 4).

- `refused` after `preparing`. K10 permits `refused` only from `claimed`, and gives a file of Section 8 only in `preparing` or `running`. `file_not_permitted`, `program_mismatch` and `limit_disk` are found after the files are got. A report `refused` then gets `409 NOT_HOLDER`, the code is lost and the item shows "Waiting for the machine" until it expires. Change: "A cause that the machine finds after `preparing` is reported with the state `failed` and that cause; the diagram does not change."
- K10 Section 4: "while its switch is on and it has a free slot". A machine whose one slot holds a job (a `runner`, up to 6 hours) sends no claim, so no `stop` reaches it. Change: "The daemon also asks when it has no free slot and `applications` is `on`; the hub gives no job to a claim whose `free.slots` is 0."
- K10 Section 9: an unclean machine "claims no item". Change: "A machine that is `unclean` still claims while `applications` is `on`, and no container item is refused for `unclean`."
- K10 Section 11: `403 WORK_OFF` is the answer to a claim, and the daemon then "ends each item that it holds; claims again in 1 hour". The hub cannot then give a `stop`. `WORK_OFF` to a report of a container item is not in the new list. Change: "To a claim whose `applications` is `on` the hub does not answer `WORK_OFF`: it answers 204, or the one item that this rule permits. `403 WORK_OFF` to a report of a container item is as `409 NOT_HOLDER`."
- K10 Section 7, `result`: `health`, the rule code, the position of the service, the counts and `data_not_restored` have no member. Change: name them as members of `result` for the kind `host`, each a fixed code or a number, and say that `cleanup` is `none`.
- K10 Section 8: "asks for a secret only after the isolation exists" and "An MCP tool, a command and a query never return a value of this route". The second forbids showing the Compose file in the editor. Change: "For the kind `host` the first rule does not apply. The second applies to `environment_file` only."
- A stop and `expired`: the text does not say that the machine reports `stopped` after the one command ends.

State: Open. The text is a proposal and no code exists. Found by the third read of 2026-11-19. The fourth form of the text (version 5 of the proposal) has the change that this entry proposes.

### SF-062: Section 4.13, what the key gives to a catalogue install. Medium to Low

Requirements: task H14; the text of the container work types, third form (version 4 of the proposal, Section 4).

- "the key or the tool gives no file" does not say that it gives no environment file and no value of a variable. A recipe with a variable in `command`, a port or a volume then takes a value of the agent.
- Who: an API key; an AI agent led by text that is not trusted.
- Change: "The key or the tool gives no Compose file, no environment file and no value of a variable, apart from a value that the recipe names as a choice with a fixed list."

State: Open. The text is a proposal and no code exists. Found by the third read of 2026-11-19. The fourth form of the text (version 5 of the proposal) has the change that this entry proposes.

### SF-063: Sections 4.1, 4.2 and 4.3 (Section 9), sentences that are false on an engine of a user. Low

Requirements: task H14; the text of the container work types, third form (version 4 of the proposal, Section 4).

- "writes this log where the daemon and the hub cannot change it": the daemon is the same user.
- "The hub cannot set one, the daemon cannot": true for the file; but a fault of the daemon that the hub can reach uses the engine with no setting. The record of H30 names this under option A; the text does not.
- The folder of the socket and the program file have no rule for who installs them.
- Change: "On an engine of a user the log is in a file of that user; a program of that user, also the daemon, can change it. A fault of the daemon that the hub can reach is not limited there by the settings. On each kind of engine the administrator installs the program file and the unit of the applications program, and the user cannot write them."

State: Open. The text is a proposal and no code exists. Found by the third read of 2026-11-19. The fourth form of the text (version 5 of the proposal) has the change that this entry proposes.

### SF-064: Section 4.1, the path-write rule and the plug-in. Low (not verified)

Requirements: task H14; the text of the container work types, third form (version 4 of the proposal, Section 4).

- "can be written by a user other than the user that it runs as": on a Mac `/Applications` can be written by the group `admin` (not measured in the record), and root can write each path. Read exactly, the program refuses to start on each Mac. Change: "… by a user who is neither an administrator nor the user that it runs as."
- With a configuration folder of its own, `docker` does not find the plug-in in `~/.docker/cli-plugins`; the measured Mac has no system plug-in folder. Change: "It starts the Compose program directly at its recorded path, not through `docker`."
- The applications program has no update rule. Change: "An update through the hub (K8 Section 8) replaces `anima-daemon` only. It does not replace the applications program and changes no setting."

State: Open. The text is a proposal and no code exists. Found by the third read of 2026-11-19. The fourth form of the text (version 5 of the proposal) has the change that this entry proposes.

### SF-065: Section 4.3, the paragraph of K10 Section 3 does not name each power. Low

Requirements: task H14; the text of the container work types, third form (version 4 of the proposal, Section 4).

- Missing: remove an application and its kept files; bring an application back; remove stopped containers and images; fill the disk with the volumes of an application; and what each setting adds when on.
- Change: add these to the list in one sentence.

State: Open. The text is a proposal and no code exists. Found by the third read of 2026-11-19. The fourth form of the text (version 5 of the proposal) has the change that this entry proposes.

### SF-066: The table of actors gave an API key the stop and the restart of an adopted project, which R-234 does not list

Requirements: task H14; the text of the container work types, third form.

What it is, in the words of the reviewer: R-234, Section 4.13, `adopted_apply`.** The table gives an API key or an MCP tool "`stop` and `restart` only". R-234 lists the start, stop and restart "of a container", the way back and the install of a catalogue entry. A stop or restart of a whole adopted project is not in the set. The effect equals `container_action` on each container, but it is wider than the ruling. Change: "No" in that cell, or a ruling.

State: Open. A contradiction of the text with a ruling, found by the third read of 2026-11-19. The fourth form of the text has the change that the reviewer proposes; the ruling is not changed.

### SF-067: The table of actors gave an API key the update of a catalogue application, which R-234 does not list

Requirements: task H14; the text of the container work types, third form.

What it is, in the words of the reviewer: R-234, Section 4.13, `application_apply`.** "Only for an entry of the tested catalogue" covers an update of a catalogue application (a new recipe, `pull` true). R-234 gives "the install of an entry". Change: "Only the first install of an entry of the tested catalogue … Not an update."

State: Open. A contradiction of the text with a ruling, found by the third read of 2026-11-19. The fourth form of the text has the change that the reviewer proposes; the ruling is not changed.

### SF-068: The text said the risk of R-235 more softly and more narrowly than the ruling

Requirements: task H14; the text of the container work types, third form.

What it is, in the words of the reviewer: R-235, Section 4.8, narrower and softer than the ruling.** The text: "A user of the machine who can write such a file … can thus get the rights that the file asks for." The card of R-235: such a user "can become administrator through Anima". The text also names only the recorded Compose files; the same holds for `.env`, an `env_file`, and a file that `include` or `extends` names. Change: "A user who is not the administrator and who can write a file of the project, a file that it names or its folder, can thus become the administrator of the machine on a system engine, and get the rights of the owner of the engine on an engine of a user."

State: Open. A contradiction of the text with a ruling, found by the third read of 2026-11-19. The fourth form of the text has the change that the reviewer proposes; the ruling is not changed.

### SF-069: The text said the risk of R-231 without the words "on each machine"

Requirements: task H14; the text of the container work types, third form.

What it is, in the words of the reviewer: R-231, Section 4.3 (Section 8), slightly narrower.** The ruling says "on each machine, also on one that he does not own alone". Change: add those words.

State: Open. A contradiction of the text with a ruling, found by the third read of 2026-11-19. The fourth form of the text has the change that the reviewer proposes; the ruling is not changed.

## The security review of a command line or a session for an AI agent (2026-11-19)

R-258 asks for a security review of [routed-command-lines.md](proposals/routed-command-lines.md) before any code. A separate reviewer agent read version 1 of that text; it got the text and the rules of the project, and no opinion of the session that wrote the text. No code exists for the function, so each finding is about the text. The report, with the effect of each finding, who can cause it, what mitigates it now and what would correct it, is the record [command-lines-review-2026-11-19.md](research/work-route/command-lines-review-2026-11-19.md); each entry below names its finding there. 5 findings are of severity High, 12 Medium, 5 Low, and 2 are notes. Nothing that gives an agent a shell is merged before the operator has decided each finding that needs him (R-258).

### SF-090: A routed session is a full login of the operator for each agent key, and it defeats each setting at the machine

Requirements: [routed-command-lines.md](proposals/routed-command-lines.md). Severity: High.

What it is, where, the effect and who can cause it, and the mitigation now: finding F1 of the record [command-lines-review-2026-11-19.md](research/work-route/command-lines-review-2026-11-19.md).

State: Open. It needs the operator.

### SF-091: The isolation that the command line depends on is a proposal: not accepted, not built, not measured on Linux

Requirements: [routed-command-lines.md](proposals/routed-command-lines.md). Severity: High.

What it is, where, the effect and who can cause it, and the mitigation now: finding F2 of the record [command-lines-review-2026-11-19.md](research/work-route/command-lines-review-2026-11-19.md).

State: Open. It needs the operator.

### SF-092: The option "as the user of the daemon" of OD-RCL2 gives the daemon token and the applications socket

Requirements: [routed-command-lines.md](proposals/routed-command-lines.md). Severity: High.

What it is, where, the effect and who can cause it, and the mitigation now: finding F3 of the record [command-lines-review-2026-11-19.md](research/work-route/command-lines-review-2026-11-19.md).

State: Open. It needs the operator.

### SF-093: No permission at the hub for a key: each write key gets the function, and the MCP state is not a control

Requirements: [routed-command-lines.md](proposals/routed-command-lines.md). Severity: High.

What it is, where, the effect and who can cause it, and the mitigation now: finding F4 of the record [command-lines-review-2026-11-19.md](research/work-route/command-lines-review-2026-11-19.md).

State: Open. It needs the operator.

### SF-094: The setting of RCL-006: one switch for two mechanisms, with the weaker protection, and for a session it is not at the machine

Requirements: [routed-command-lines.md](proposals/routed-command-lines.md). Severity: High.

What it is, where, the effect and who can cause it, and the mitigation now: finding F5 of the record [command-lines-review-2026-11-19.md](research/work-route/command-lines-review-2026-11-19.md).

State: Open. It needs the operator.

### SF-095: A hub that the owner of the machine does not control: the list of R-185 is lost, and RCL-007 names the wrong control

Requirements: [routed-command-lines.md](proposals/routed-command-lines.md). Severity: Medium.

What it is, where, the effect and who can cause it, and the mitigation now: finding F6 of the record [command-lines-review-2026-11-19.md](research/work-route/command-lines-review-2026-11-19.md).

State: Open. It needs the operator.

### SF-096: A command shares its isolation with `build` and `runner`: what it leaves can reach a later job that has a secret

Requirements: [routed-command-lines.md](proposals/routed-command-lines.md). Severity: Medium.

What it is, where, the effect and who can cause it, and the mitigation now: finding F7 of the record [command-lines-review-2026-11-19.md](research/work-route/command-lines-review-2026-11-19.md).

State: Open. It is a correction of the text.

### SF-097: The network of a command: the public internet is open, the limits are not measured, and "unless the owner set otherwise" is a setting with no text

Requirements: [routed-command-lines.md](proposals/routed-command-lines.md). Severity: Medium.

What it is, where, the effect and who can cause it, and the mitigation now: finding F8 of the record [command-lines-review-2026-11-19.md](research/work-route/command-lines-review-2026-11-19.md).

State: Open. It needs the operator.

### SF-098: Section 8 states as fact two things that the W2 record says are not checked

Requirements: [routed-command-lines.md](proposals/routed-command-lines.md). Severity: Medium.

What it is, where, the effect and who can cause it, and the mitigation now: finding F9 of the record [command-lines-review-2026-11-19.md](research/work-route/command-lines-review-2026-11-19.md).

State: Open. It is a correction of the text.

### SF-099: The text does not say what file system a container command has; the inventory is of the tools of the host

Requirements: [routed-command-lines.md](proposals/routed-command-lines.md). Severity: Medium.

What it is, where, the effect and who can cause it, and the mitigation now: finding F10 of the record [command-lines-review-2026-11-19.md](research/work-route/command-lines-review-2026-11-19.md).

State: Open. It is a correction of the text.

### SF-100: The audit entry: 4 KiB of a command line of up to 1 MiB, a secret that stays one year, and no entry in the log of the machine

Requirements: [routed-command-lines.md](proposals/routed-command-lines.md). Severity: Medium.

What it is, where, the effect and who can cause it, and the mitigation now: finding F11 of the record [command-lines-review-2026-11-19.md](research/work-route/command-lines-review-2026-11-19.md).

State: Open. It needs the operator.

### SF-101: The limit of 30 starts does not limit: 30 commands of one hour hold each slot, "caller" has no definition, queued work counts later

Requirements: [routed-command-lines.md](proposals/routed-command-lines.md). Severity: Medium.

What it is, where, the effect and who can cause it, and the mitigation now: finding F12 of the record [command-lines-review-2026-11-19.md](research/work-route/command-lines-review-2026-11-19.md).

State: Open. It needs the operator.

### SF-102: The queue: a command starts when the owner sets the switch on later; a refusal takes the machine out of work; two names for one cause

Requirements: [routed-command-lines.md](proposals/routed-command-lines.md). Severity: Medium.

What it is, where, the effect and who can cause it, and the mitigation now: finding F13 of the record [command-lines-review-2026-11-19.md](research/work-route/command-lines-review-2026-11-19.md).

State: Open. It is a correction of the text.

### SF-103: The output of a command instructs the agent that has a shell; a stolen daemon token can take commands and give false results

Requirements: [routed-command-lines.md](proposals/routed-command-lines.md). Severity: Medium.

What it is, where, the effect and who can cause it, and the mitigation now: finding F14 of the record [command-lines-review-2026-11-19.md](research/work-route/command-lines-review-2026-11-19.md).

State: Open. It needs the operator.

### SF-104: The working tree: the text says a command has no repository and then gives it one; a private checkout with no secret; source of a private repository can leave

Requirements: [routed-command-lines.md](proposals/routed-command-lines.md). Severity: Medium.

What it is, where, the effect and who can cause it, and the mitigation now: finding F15 of the record [command-lines-review-2026-11-19.md](research/work-route/command-lines-review-2026-11-19.md).

State: Open. It is a correction of the text.

### SF-105: A session needs no tool of Anima to be used by a key; the start command of an SSH session; the idle limit does not bind an agent

Requirements: [routed-command-lines.md](proposals/routed-command-lines.md). Severity: Medium.

What it is, where, the effect and who can cause it, and the mitigation now: finding F16 of the record [command-lines-review-2026-11-19.md](research/work-route/command-lines-review-2026-11-19.md).

State: Open. It is a correction of the text.

### SF-106: A command line removes the click rules that R-196 made absolute (install of Hermes, AXT-021)

Requirements: [routed-command-lines.md](proposals/routed-command-lines.md). Severity: Medium.

What it is, where, the effect and who can cause it, and the mitigation now: finding F17 of the record [command-lines-review-2026-11-19.md](research/work-route/command-lines-review-2026-11-19.md).

State: Open. It needs the operator.

### SF-107: A command can run two times or continue with no hub: the lease rules for `command` are not written

Requirements: [routed-command-lines.md](proposals/routed-command-lines.md). Severity: Low.

What it is, where, the effect and who can cause it, and the mitigation now: finding F18 of the record [command-lines-review-2026-11-19.md](research/work-route/command-lines-review-2026-11-19.md).

State: Open. It is a correction of the text.

### SF-108: The hub selects the kind, and `native` is the weaker isolation; `actor` has the value `hub`; PRO-024 is not named

Requirements: [routed-command-lines.md](proposals/routed-command-lines.md). Severity: Low.

What it is, where, the effect and who can cause it, and the mitigation now: finding F19 of the record [command-lines-review-2026-11-19.md](research/work-route/command-lines-review-2026-11-19.md).

State: Open. It is a correction of the text.

### SF-109: The arguments are visible on the machine and in the lists of the hub; two daemons on one Mac share one build user

Requirements: [routed-command-lines.md](proposals/routed-command-lines.md). Severity: Low.

What it is, where, the effect and who can cause it, and the mitigation now: finding F20 of the record [command-lines-review-2026-11-19.md](research/work-route/command-lines-review-2026-11-19.md).

State: Open. It is a correction of the text.

### SF-110: The environment, the search path and the input of the command are not written

Requirements: [routed-command-lines.md](proposals/routed-command-lines.md). Severity: Low.

What it is, where, the effect and who can cause it, and the mitigation now: finding F21 of the record [command-lines-review-2026-11-19.md](research/work-route/command-lines-review-2026-11-19.md).

State: Open. It is a correction of the text.

### SF-111: The texts that Section 2 missed

Requirements: [routed-command-lines.md](proposals/routed-command-lines.md). Severity: Low.

What it is, where, the effect and who can cause it, and the mitigation now: finding F22 of the record [command-lines-review-2026-11-19.md](research/work-route/command-lines-review-2026-11-19.md).

State: Open. It is a correction of the text.

### SF-112: The gate of Section 9 is weaker than ROU-014

Requirements: [routed-command-lines.md](proposals/routed-command-lines.md). Severity: Note.

What it is, where, the effect and who can cause it, and the mitigation now: finding F23 of the record [command-lines-review-2026-11-19.md](research/work-route/command-lines-review-2026-11-19.md).

State: Open. It needs the operator.

### SF-113: "No shell" and "`startSession` stays closed" are true and protect nothing

Requirements: [routed-command-lines.md](proposals/routed-command-lines.md). Severity: Note.

What it is, where, the effect and who can cause it, and the mitigation now: finding F24 of the record [command-lines-review-2026-11-19.md](research/work-route/command-lines-review-2026-11-19.md).

State: Open. It is a correction of the text.
### SF-114: A kept session on a hub of Anima.ing can be reached only if that hub gets a login, and then it is a shell of the vendor

Requirements: [agent-layer.md](proposals/agent-layer.md) (AGL-026, OD-AGL7). Severity: High.

What it is, where, the effect and who can cause it, and the mitigation now: finding F1 of the record [agent-layer-review-2026-11-20.md](research/work-route/agent-layer-review-2026-11-20.md).

State: Open. It needs the operator.

### SF-115: Nothing at the machine proves that a start comes from the operator

Requirements: [agent-layer.md](proposals/agent-layer.md) (AGL-025, AGL-028). Severity: High.

What it is, where, the effect and who can cause it, and the mitigation now: finding F2 of the record [agent-layer-review-2026-11-20.md](research/work-route/agent-layer-review-2026-11-20.md).

State: Open. It needs the operator.

### SF-116: The interface from the daemon to the unit with no sandbox is not written

Requirements: [agent-layer.md](proposals/agent-layer.md) (AGL-023). Severity: High.

What it is, where, the effect and who can cause it, and the mitigation now: finding F3 of the record [agent-layer-review-2026-11-20.md](research/work-route/agent-layer-review-2026-11-20.md).

State: Open. It needs the operator.

### SF-117: "No AI agent starts, stops or types into a kept session" has no mechanism

Requirements: [agent-layer.md](proposals/agent-layer.md) (AGL-027, AGL-003). Severity: High.

What it is, where, the effect and who can cause it, and the mitigation now: finding F4 of the record [agent-layer-review-2026-11-20.md](research/work-route/agent-layer-review-2026-11-20.md).

State: Open. It needs the operator.

### SF-118: "By its name" is a lookup in the search path, and the configuration of a repository runs at the start

Requirements: [agent-layer.md](proposals/agent-layer.md) (AGL-024). Severity: Medium.

What it is, where, the effect and who can cause it, and the mitigation now: finding F5 of the record [agent-layer-review-2026-11-20.md](research/work-route/agent-layer-review-2026-11-20.md).

State: Open. It needs the operator.

### SF-119: A setting "that the hub cannot set" is kept by a daemon of the user, so each program of that user can write it

Requirements: [agent-layer.md](proposals/agent-layer.md) (AGL-011, AGL-025). Severity: Medium.

What it is, where, the effect and who can cause it, and the mitigation now: finding F6 of the record [agent-layer-review-2026-11-20.md](research/work-route/agent-layer-review-2026-11-20.md).

State: Open. It needs the operator.

### SF-120: A signal does more than change a state: it makes rows, causes a push and sends an alert

Requirements: [agent-layer.md](proposals/agent-layer.md) (AGL-010, AGL-018, AGL-020). Severity: Medium.

What it is, where, the effect and who can cause it, and the mitigation now: finding F7 of the record [agent-layer-review-2026-11-20.md](research/work-route/agent-layer-review-2026-11-20.md).

State: Open. It is a correction of the text; version 2 of the proposal says what it changed.

### SF-121: The signal drops the folder, so the exclusion of a repository is known too late

Requirements: [agent-layer.md](proposals/agent-layer.md) (AGL-008 against AGL-015, AGL-019). Severity: Medium.

What it is, where, the effect and who can cause it, and the mitigation now: finding F8 of the record [agent-layer-review-2026-11-20.md](research/work-route/agent-layer-review-2026-11-20.md).

State: Open. It is a correction of the text; version 2 of the proposal says what it changed.

### SF-122: A hub of Anima.ing gets a timeline of the work of a person, joined to the usage counts

Requirements: [agent-layer.md](proposals/agent-layer.md) (AGL-015 to AGL-018, OD-AGL9). Severity: Medium.

What it is, where, the effect and who can cause it, and the mitigation now: finding F9 of the record [agent-layer-review-2026-11-20.md](research/work-route/agent-layer-review-2026-11-20.md).

State: Open. It needs the operator.

### SF-123: The installer: one `notify` program of Codex, the path of the entry, a link, the mode of the copy

Requirements: [agent-layer.md](proposals/agent-layer.md) (AGL-012, AGL-013). Severity: Medium.

What it is, where, the effect and who can cause it, and the mitigation now: finding F10 of the record [agent-layer-review-2026-11-20.md](research/work-route/agent-layer-review-2026-11-20.md).

State: Open. It is a correction of the text; version 2 of the proposal says what it changed.

### SF-124: No rule that the hook prints nothing, logs nothing and leaves no crash report

Requirements: [agent-layer.md](proposals/agent-layer.md) (AGL-008, AGL-009). Severity: Medium.

What it is, where, the effect and who can cause it, and the mitigation now: finding F11 of the record [agent-layer-review-2026-11-20.md](research/work-route/agent-layer-review-2026-11-20.md).

State: Open. It is a correction of the text; version 2 of the proposal says what it changed.

### SF-125: The alert gives a third party the name of the machine, the address of the hub and the times of work; it conflicts with NOT-003, NOT-005 and NOT-007

Requirements: [agent-layer.md](proposals/agent-layer.md) (AGL-020, AGL-021). Severity: Medium.

What it is, where, the effect and who can cause it, and the mitigation now: finding F12 of the record [agent-layer-review-2026-11-20.md](research/work-route/agent-layer-review-2026-11-20.md).

State: Open. It needs the operator.

### SF-126: The alphabet of a branch name stops an injection, not a disclosure

Requirements: [agent-layer.md](proposals/agent-layer.md) (AGL-017). Severity: Medium.

What it is, where, the effect and who can cause it, and the mitigation now: finding F13 of the record [agent-layer-review-2026-11-20.md](research/work-route/agent-layer-review-2026-11-20.md).

State: Open. It needs the operator.

### SF-127: The signal file has no size limit and no rule for a link, a pipe or a truncation

Requirements: [agent-layer.md](proposals/agent-layer.md) (AGL-009). Severity: Low.

What it is, where, the effect and who can cause it, and the mitigation now: finding F14 of the record [agent-layer-review-2026-11-20.md](research/work-route/agent-layer-review-2026-11-20.md).

State: Open. It is a correction of the text; version 2 of the proposal says what it changed.

### SF-128: `Notification` is not only "waiting", nothing ends Waiting, and "Unknown, active" is a seventh state

Requirements: [agent-layer.md](proposals/agent-layer.md) (AGL-005 to AGL-007). Severity: Low.

What it is, where, the effect and who can cause it, and the mitigation now: finding F15 of the record [agent-layer-review-2026-11-20.md](research/work-route/agent-layer-review-2026-11-20.md).

State: Open. It is a correction of the text; version 2 of the proposal says what it changed.

### SF-129: The version of a program and "hooks installed" leave the device but are not in the list of fields

Requirements: [agent-layer.md](proposals/agent-layer.md) (AGL-014 against AGL-015). Severity: Low.

What it is, where, the effect and who can cause it, and the mitigation now: finding F16 of the record [agent-layer-review-2026-11-20.md](research/work-route/agent-layer-review-2026-11-20.md).

State: Open. It is a correction of the text; version 2 of the proposal says what it changed.

### SF-130: The gate covers the kept sessions and High findings only

Requirements: [agent-layer.md](proposals/agent-layer.md) (Section 7, Section 10). Severity: Low.

What it is, where, the effect and who can cause it, and the mitigation now: finding F17 of the record [agent-layer-review-2026-11-20.md](research/work-route/agent-layer-review-2026-11-20.md).

State: Open. It needs the operator.

### SF-131: A unit of a user ends at logout unless linger is set

Requirements: [agent-layer.md](proposals/agent-layer.md) (AGL-023). Severity: Low.

What it is, where, the effect and who can cause it, and the mitigation now: finding F18 of the record [agent-layer-review-2026-11-20.md](research/work-route/agent-layer-review-2026-11-20.md).

State: Open. It is a correction of the text; version 2 of the proposal says what it changed.

## The fourth read of the container work types (task H14, 2026-11-19)

The third read was of the form before its own 21 changes and before nine changes that the owner of the contract file asked for. Thus no security reviewer had read the text of version 0.4 as it is in `contracts/work-route.md`. A fourth separate reviewer agent read it on `dev`: Section 6.4 and each sentence that version 0.4 added elsewhere. It did not read the proposal. Nothing was run; the behaviour of Docker and Compose in these entries is from the knowledge of the reviewer, marked "not verified" where it carries a finding.

The last line of the reviewer, in its words: "No finding remains that the owner has not decided; the open points are changes of the text." On what needs the owner, in its words:

None. SF-074, SF-075, SF-076 and SF-084 add rules with numbers (10 minutes, 6 and 12, 600 seconds); the owner of the contract file may choose different values, but each is a change of the text and not an accepted risk.

No finding remains that the owner has not decided; the open points are changes of the text.

The reviewer found 18 weaknesses, none of severity High (SF-070 to SF-087), eleven sentences of the file that contradict each other (SF-088), and one contradiction with a ruling (SF-089). Each is a change of the text. Version 0.5 of the contract has them; the owner of the contract file reviews that change. Until this read was recorded, no code that gives a container type to a real machine was to be merged; none exists.

How the reviewer judged the 21 entries of the third read:

| Entry | In the text? | Sentence, or what is missing |
| --- | --- | --- |
| SF-049 | Yes | 6.4.1: "It refuses to start when the owner of the real socket is not the user that it runs as, or when the kind is different from the recorded one." The word "administrator" has no definition (SF-070). |
| SF-050 | Yes | 6.4.1: "On an engine of a user the accepted user is the owner of the socket of the engine; the applications program refuses to start when the two are different". |
| SF-051 | Yes | 6.4.2: "When a request has a different pair, it refuses each container item (`applications_off`) until the administrator sets 'Manage applications' on again." |
| SF-052 | Yes | 6.4.4 step 5: "It writes the final form itself, as JSON … each string in quotes with each control character escaped"; step 1 has the five refusals; 6.4.1 has the lowest Compose version. |
| SF-053 | Yes, in part | Step 3: "The result is one scalar of text: it is never read again as YAML, and it is not filled a second time." Missing: what `text` of `${NAME:-text}` may hold (SF-082). |
| SF-054 | Yes | `image` row: "A reference that starts with `a-z` or `0-9` … The program gives it to a command after `--`"; Section 3: "also an address of the local network". |
| SF-055 | Yes, in part | "the form with a name only is refused"; "Only the long form"; "`source` is always present"; "A prefix that starts with `com.docker.` cannot be named". Missing: a value rule for `working_dir`, `aliases`, `cap_drop`, and the members of `healthcheck` other than `interval`. |
| SF-056 | Yes | "A value of one of them that is zero or below is refused. `healthcheck.interval` is at least 5 seconds."; step 1: "more than 16 networks and more than 64 volumes". |
| SF-057 | Yes | Top level: "There is no `secrets`: a secret of an application is a variable of the environment file". Residue: step 1 still names "a secret or a config". |
| SF-058 | Yes | 6.4.9: "makes the final form again from the kept Compose file and the kept environment file, by the six steps of Section 6.4.4 of today, with the kept image digests and no pull." 6.4.5 disagrees on what is kept (SF-088, point 5). |
| SF-059 | Yes | 6.4.7: "It gives each recorded file, the recorded folder as the project folder and the project name as arguments, with the environment and the configuration folder of Section 6.4.1." |
| SF-060 | Yes | Section 4: "The hub gives, in this order: `container_action` with `stop`; each other container item, the oldest first; a job." A `stop` still waits behind a running item (SF-074). |
| SF-061 | Yes, in part | Six of seven points are present (for example "reported with the state `failed` and that cause; the diagram of Section 7 does not change"). Missing: "no container item is refused for `unclean`"; the text has only "`container_action` with `stop` is never refused for `policy` or for `unclean`" (SF-088, point 2). |
| SF-062 | Yes | 6.4.12: "gives no Compose file, no environment file and no value of a variable, apart from a value that the recipe names as a choice with a fixed list." |
| SF-063 | Yes | Section 9: "On an engine of a user the log is a file of that user"; 6.4.1: "A fault of the daemon that the hub can reach is not limited there by the settings." |
| SF-064 | Yes | "neither an administrator nor the user that it runs as"; "It starts the Compose program directly at its recorded path, not through `docker`"; "replaces `anima-daemon` only". |
| SF-065 | Yes | Section 3: "bring an application back to its earlier version; remove an application with its kept files; remove stopped containers and images … Each setting of the machine that is on adds what its row says." |
| SF-066 | Yes | 6.4.12, `adopted_apply`: "No. A key or a tool starts, stops and restarts one container with `container_action`". |
| SF-067 | Yes | 6.4.12: "Only the first install of an entry of the tested catalogue of Anima … Not an update". |
| SF-068 | Yes | 6.4.7: "can thus become the administrator of the machine through Anima on a system engine, and get the rights of the owner of the engine on an engine of a user." |
| SF-069 | Yes | Section 8: "on each machine, also on one that the operator does not own alone; the operator accepts this (R-231)". |

### SF-070: Sections 6.4.1 and 6.4.2: "the administrator" has two meanings. Medium

Requirements: task H14; contract K10 version 0.4.

- Way: on the measured Mac the one account is an administrator account and owns the socket. Read with "administrator = member of `admin`", the kind is "system engine", and a settings file "that only the administrator can write" can be written by the daemon, which runs as that account. Read with "administrator = identifier 0", the write check "neither an administrator nor the user" refuses `/Applications` (group `admin`).
- Who: the hub, through a fault of the daemon, if the settings file is built group-writable; an honest implementer in either reading.
- Change, Section 1: "The administrator: the user with the identifier 0. An administrator account: a member of the group `admin` on a Mac, of no group elsewhere." In the kind table: "belongs to the identifier 0" and "belongs to a different identifier". In 6.4.2: "one file that the identifier 0 owns, with no write right for a group or for others". In the write check: "a user who is neither the identifier 0, nor an administrator account, nor the user that it runs as".

State: Open. The text is the contract (version 0.4) and no code exists. Found by the fourth read of 2026-11-19. Version 0.5 of the contract has the change that this entry proposes.

### SF-071: Section 6.4.1: the write check names "a real file", and the socket is one. Medium

Requirements: task H14; contract K10 version 0.4.

- Way: on a Linux system engine the socket is `root:docker`, mode 660. Read exactly, the program refuses to start on each such machine; an implementer then makes the rule weaker by guess. The text also does not say how "can be written" is decided (mode bits, group members, access lists).
- Change: "For the socket the rule is for each link and each folder above it, not for the right to connect to the socket. 'Can be written' is decided from the owner, the mode bits and the access list of each one; a group write bit counts unless each member of the group is permitted."
- Add: "It refuses to start on a system engine when the accepted user of the daemon can connect to the socket of the engine."

State: Open. The text is the contract (version 0.4) and no code exists. Found by the fourth read of 2026-11-19. Version 0.5 of the contract has the change that this entry proposes.

### SF-072: Sections 6.4.2 and 6.4.7: who writes the adoption record on an engine of a user. Medium

Requirements: task H14; contract K10 version 0.4.

- Way: the settings file holds "Adopted projects … with its recorded paths", only the administrator writes it, and the applications program "reads it". But 6.4.7 says "the applications program records the real path" and "makes the form of the project", which starts Compose. On an engine of a user the applications program cannot write that file. One implementer moves the list to a file of the user; another runs the adoption as the administrator, who then starts a Compose program that the user can write (SF-035 again). The same holds for the step that records the paths and checks the Compose version.
- Who: each program of the user of a Mac, in the second build.
- Change: "The program that writes the settings file runs as the administrator; it starts no program of Docker and opens no socket of an engine. For an adoption, the applications program, as the user that it runs as, makes the form and the comparison and shows the result; the administrator then writes the record."

State: Open. The text is the contract (version 0.4) and no code exists. Found by the fourth read of 2026-11-19. Version 0.5 of the contract has the change that this entry proposes.

### SF-073: Section 7 paragraph: "A hub keeps a member of `result` as the machine sent it." Medium

Requirements: task H14; contract K10 version 0.4.

- Way: a machine, or a person with its daemon token (R-231 names machines not owned alone), sends `"rule"`, `"health"` or an unknown member as free text. The hub keeps it, and a screen or an MCP tool gives it to the operator or an AI agent. This breaks "fixed codes and numbers, never text" of version 0.3 and Section 2 ("The hub ignores a member that it does not know").
- Change: replace the sentence with: "The hub checks each member of `result` of the kind `host`: a code that is not in the list of its member is stored as `unknown`; a member that this section does not name is dropped; `service` is null or a whole number from 0 to 31; each count and each number of bytes is a whole number from 0 to 2^53, else `400 MALFORMED`."
- With that change neither a code nor a number carries text from a file. Counts and bytes are numbers of the engine only.

State: Open. The text is the contract (version 0.4) and no code exists. Found by the fourth read of 2026-11-19. Version 0.5 of the contract has the change that this entry proposes.

### SF-074: Section 4 paragraph: a `stop` waits behind a running item, and a key has no limit on installs. Medium

Requirements: task H14; contract K10 version 0.4.

- Way: `stop` is "an item that changes Docker", and the hub gives one at a time. A `stop` waits while an `application_apply` is in `claimed`, `preparing` or `running`, up to 1,800 seconds. An API key may start a first install of a catalogue entry, and only `container_action` has a rate limit. The items of the operator ("the oldest first") also wait behind all installs of a key.
- Who: an API key; an AI agent.
- Change: "`container_action` with `stop` is not counted and does not wait for the place of the item that changes Docker. The hub makes at most 6 container items in a minute, and holds at most 12 that are not final, for one API key and one machine. In one class of the order, an item of a login is before an item of an API key."

State: Open. The text is the contract (version 0.4) and no code exists. Found by the fourth read of 2026-11-19. Version 0.5 of the contract has the change that this entry proposes.

### SF-075: Sections 4 and 7 paragraphs: an item that no machine claims has no end. Medium

Requirements: task H14; contract K10 version 0.4.

- Way: the end "`limits.seconds` and 5 minutes … since the claim" needs a claim. Items for a machine that is off stay in the queue; the limit of 6 in a minute is on giving, not on making. When the machine returns after days, it runs each old `restart`, `application_remove` or `application_apply`, and the per-application rule holds back the new items of the operator.
- Who: an API key (`container_action`, `application_back`, a catalogue install); an operator who forgot.
- Change: "A container item that no machine claimed 10 minutes after it was made is final, `failed`, with the cause `orphan`. The hub makes no second item that is equal to one that waits."

State: Open. The text is the contract (version 0.4) and no code exists. Found by the fourth read of 2026-11-19. Version 0.5 of the contract has the change that this entry proposes.

### SF-076: Section 7 paragraph and 6.4.1: the machine has no exclusion and no time limit of its own. Medium

Requirements: task H14; contract K10 version 0.4.

- Way (a): the hub ends an attempt (`holds` after a restart of the daemon, `expired`, no answer), and the applications program "finishes the one command". The operator asks again, and the hub gives a second item for the same application while the first command runs. A request on the socket with no hub (R-236) has no order at all.
- Way (b): nothing ends a command on the machine. A registry that never answers (the hub names the address) holds a pull with no end.
- Change: "The applications program runs one item that changes Docker at a time, apart from `container_action` with `stop`, and refuses a second one (`busy`) until the first command has ended. It ends a command when `limits.seconds` have passed since it took the item, and reports `failed` with `limit_seconds`." Add `busy` to the causes of Section 5.

State: Open. The text is the contract (version 0.4) and no code exists. Found by the fourth read of 2026-11-19. Version 0.5 of the contract has the change that this entry proposes.

### SF-077: Sections 6.4.1 and 6.4.4, `ports`: no lowest version of the engine. Medium (not verified)

Requirements: task H14; contract K10 version 0.4.

- Way: on older engines a port published on `127.0.0.1` could be reached from a different host of the same local network. R-229 ("only on the machine itself") then does not hold. I recall the hardening in engine 28.0; I did not check it.
- Who: a host on the local network.
- Change: "It refuses to start when the engine is older than the lowest version that it knows." The check plan must measure that a port on `127.0.0.1` is closed from a second host.

State: Open. The text is the contract (version 0.4) and no code exists. Found by the fourth read of 2026-11-19. Version 0.5 of the contract has the change that this entry proposes.

### SF-078: Section 6.4.7: the comparison at adoption reads three fields. Low to Medium (not verified)

Requirements: task H14; contract K10 version 0.4.

- Way: a variable of the shell of the owner can also set `privileged`, `network_mode`, `cap_add`, `pid`, `user`, `command` or a value of `environment` (for example one that sets a login off). The first `up` from the hub then makes a different stack, and the check does not see it.
- Change: "… refuses the adoption when the configuration hash that Compose computes for a service is different from the label `com.docker.compose.config-hash` of its container; where that cannot be compared, when the image, a port, a mount, the network mode, `privileged`, a capability, a device, the user, the command or the environment is different. A service with no container is shown and is not compared."

State: Open. The text is the contract (version 0.4) and no code exists. Found by the fourth read of 2026-11-19. Version 0.5 of the contract has the change that this entry proposes.

### SF-079: Section 12: a `container_action` item makes a machine busy. Low (not verified against K8)

Requirements: task H14; contract K10 version 0.4.

- Way: during a rollout the hub still gives `stop`. A key sends 6 in a minute and the machine reports `busy: true` each time, so an update of the daemon can wait with no end.
- Change: "A `container_action` item does not make a machine busy."

State: Open. The text is the contract (version 0.4) and no code exists. Found by the fourth read of 2026-11-19. Version 0.5 of the contract has the change that this entry proposes.

### SF-080: Section 6.4.4 steps 1, 3 and 5: rules that two honest implementers build differently. Low each

Requirements: task H14; contract K10 version 0.4.

- The YAML schema is not named: `restart: no`, `user: 1000:1000`, `mode: 0444` and `published: "8080"` have a different type by implementer. Change: "A scalar is read by the core schema of YAML 1.2. Where a rule asks for a text, a plain scalar is taken as written."
- A filled value is "one scalar of text", but `published`, `target`, `mem_limit`, `pids_limit`, `cpus`, `read_only`, `init` and `retries` need a number or a truth value. Change: "A key whose rule asks for a number or a truth value takes no variable."
- No form for `mem_limit` (unit letters), `cpus` (a decimal number), `stop_grace_period` and the times of `healthcheck`; "the lower of" cannot be computed without one. Change: "`mem_limit` is a whole number of bytes; `cpus` is a decimal number; each time is whole seconds, written as `<n>s`."
- `healthcheck.start_interval` has no minimum, so `interval` "at least 5 seconds" is passed. Change: "`start_interval` is at least 5 seconds; `timeout` and `start_period` are at most `limits.seconds`."
- `mode` has no input rule. Change: "`mode` is a text of 3 or 4 octal digits."
- "A key, at any level, that is not here is refused" also reads on the names below `services`, `volumes`, `networks`, `configs`, `environment`, `labels` and `depends_on`. Change: "These are names, not keys of this list. A name of `environment` has no `=`; a value that is null is refused."
- `published` has no rule for "absent". Change: "`published` is always present."
- "its own limit" (memory, processes, processors, grace time, free disk) has no value and no place. Change: say that each is a constant of the program, with its value, or a setting of 6.4.2.
- The flags of `up` for `application_apply` are not said, though 6.4.7 says them for an adopted project. Change: "`up -d --remove-orphans --pull never --no-build`."

State: Open. The text is the contract (version 0.4) and no code exists. Found by the fourth read of 2026-11-19. Version 0.5 of the contract has the change that this entry proposes.

### SF-081: Section 6.4.4 step 3: the variable form and the environment file. Low

Requirements: task H14; contract K10 version 0.4.

- Not said: whether `text` may hold `}` or `$` (`${A:-${B}}`); an empty line, a line that starts with `#`, a line with no `=`, a carriage return, a NUL.
- Effect: a difference between the check of the hub and the machine; step 4 still checks the result.
- Change: "`text` has no `$` and no `}`. In the environment file an empty line is ignored; each other line with no `=`, with a name that breaks the rule, or with a carriage return or a NUL is refused; `#` has no meaning."

State: Open. The text is the contract (version 0.4) and no code exists. Found by the fourth read of 2026-11-19. Version 0.5 of the contract has the change that this entry proposes.

### SF-082: Section 6.4.4 step 5: characters that the reader of Compose can take as a line end or refuse. Low (not verified)

Requirements: task H14; contract K10 version 0.4.

- Way: U+0085, U+2028, U+2029, U+007F and U+0080 to U+009F inside a JSON string. A YAML reader can fold them, so the value that Compose reads differs from the checked one. It stays one scalar; I found no way to structure.
- Change: "The writer escapes each character below U+0020, U+007F to U+009F, U+2028, U+2029 and U+FEFF as `\uXXXX`, and writes no pair of surrogates; a NUL in a value is refused."

State: Open. The text is the contract (version 0.4) and no code exists. Found by the fourth read of 2026-11-19. Version 0.5 of the contract has the change that this entry proposes.

### SF-083: Section 6.4.1: the environment and configuration rules are said for `docker` only. Low

Requirements: task H14; contract K10 version 0.4.

- Way: step 6 starts the Compose program for an application with no sentence on its environment. `DOCKER_HOST`, `DOCKER_CONTEXT`, `COMPOSE_FILE` and a `.env` in the project folder change what Compose does. A `config.json` with `credsStore`, `credHelpers` or plug-in entries starts other programs, as the administrator on a system engine.
- Change: "The same rules apply to the Compose program. The configuration folder can be written only by the user that the program runs as; its `config.json` has the member `auths` only, else the program refuses to start. The working folder of Compose is `files/`, which holds no file named `.env`."

State: Open. The text is the contract (version 0.4) and no code exists. Found by the fourth read of 2026-11-19. Version 0.5 of the contract has the change that this entry proposes.

### SF-084: Section 7 paragraph: `limits.seconds` has no maximum for three types, and some refusals have no cause. Low

Requirements: task H14; contract K10 version 0.4.

- `application_remove`, `application_back` and `docker_prune` have no maximum; the per-application rule and "Waiting for the machine" then last as long as the hub writes. Change: "at most 600 for `application_remove`, `application_back` and `docker_prune`."
- No cause is named for: a second `application_back`, or one with nothing kept; `pull` false with no digest; a project of the same name in a different folder; a name in "Protected"; a `start` that is not permitted; an identifier that the engine does not have. The hub reads an unknown cause as `failed`, so a code made by guess is lost. Change: name the cause of each in the table of Section 5.

State: Open. The text is the contract (version 0.4) and no code exists. Found by the fourth read of 2026-11-19. Version 0.5 of the contract has the change that this entry proposes.

### SF-085: Section 6.4.2: the pair of origin and device. Low

Requirements: task H14; contract K10 version 0.4.

- The form of the origin is not said (letter case, default port, last `/`); a difference gives a false refusal. Change: "The origin is the scheme, the host in lower case and the port, always written."
- It is not said that the record is deleted when the administrator sets "Manage applications" on again. Change: add that sentence.
- A program of the user of the daemon can send a false first pair, and the real daemon is then refused until the administrator acts. This is inside R-236 as loss of service only. Change, optional: "The administrator can name the pair when he sets the setting on."
- It is not said how the daemon knows what to send in `applications`. Change: "The daemon sends `on` only when the applications program answered on the socket that the setting is on and the pair is the recorded one."

State: Open. The text is the contract (version 0.4) and no code exists. Found by the fourth read of 2026-11-19. Version 0.5 of the contract has the change that this entry proposes.

### SF-086: Section 6.4.1: the paths are checked "at each start" only. Low

Requirements: task H14; contract K10 version 0.4.

- Way: on a Mac a helper of Docker that runs as root makes the link `/var/run/docker.sock` again for the user who starts Docker Desktop. A program that uses the recorded path later talks to a different engine than the one it checked.
- Change: "It gives each command the real paths that it resolved at the start, and reads the owner of the real socket again before each item; a different owner ends the program."

State: Open. The text is the contract (version 0.4) and no code exists. Found by the fourth read of 2026-11-19. Version 0.5 of the contract has the change that this entry proposes.

### SF-087: Section 6.4.2, row "Registries with a login". Low

Requirements: task H14; contract K10 version 0.4.

- Way: with a login named, a file from the hub can pull each image that the login can read and run it with a command of its choice. Thus a person who controls the hub reads the private images of the owner. The row does not say so, though Section 3 says "adds what its row says".
- Change, in the row: "A person who controls the hub can then get and read each image that this login can read."

State: Open. The text is the contract (version 0.4) and no code exists. Found by the fourth read of 2026-11-19. Version 0.5 of the contract has the change that this entry proposes.

### SF-088: Eleven sentences of contract K10 version 0.4 contradict each other

Requirements: task H14; contract K10 version 0.4.

What it is, in the words of the reviewer (its points C1 to C11):

- C1. 6.4.9 says "`data_not_restored`"; the Section 7 paragraph says the member is "`dataNotRestored`, a truth value".
- C2. Whether a container item other than `stop` is refused for `unclean`:
- Section 4 paragraph: "the hub gives a container item to a machine that is `unclean`".
- Section 5 paragraph: only "`stop` is never refused for `policy` or for `unclean`"; the Section 5 table has `unclean` with no pointer.
- Section 9 third point (version 0.3, no pointer): "sets itself `unclean`, claims no item".
- Change: "No container item is refused for `unclean`", and a pointer in Section 9.
- C3. Section 4 first paragraph (version 0.3, no pointer): "asks for work each 10 seconds while its switch is on and it has a free slot" and "While its switch is off … one in each hour". The paragraph at the end says the opposite when `applications` is `on`.
- C4. `WORK_OFF`:
- Section 7 (version 0.3, no pointer): "`403 WORK_OFF` to a report end the job at once, for each type".
- Section 11: "Ends each item that it holds".
- Section 7 paragraph: a container item is not ended by `WORK_OFF`. It is not said what a `WORK_OFF` to a report of a job does to a container item that the same machine holds.
- Change: "`WORK_OFF` ends jobs only."
- C5. 6.4.5 says `files/` holds "the same three of the earlier version" (with the final form); 6.4.6 says `keep` holds "the Compose file as it came, the environment file and the image digests" (no final form; the digests are not in 6.4.5).
- C6. Section 11, column "Daemon does", rows `TOKEN_REFUSED`, `WRONG_DEVICE` and `NOT_HOLDER` ("ends the job at once", "deletes each thing"): no pointer to the Section 7 paragraph, where the applications program "finishes the one command" and "does not undo a start".
- C7. Section 7 (version 0.3, no pointer): "`finishing`: the job ended and the machine deletes each thing"; Section 4 paragraph: `finishing` is the wait for health. The diagram has no change `running` to `done`, and it is not said that a `container_action` or a `docker_prune` goes through `finishing`.
- C8. Section 12, second point: "gives that machine no item until its update ends"; first point: the hub still gives `container_action` with `stop`.
- C9. 6.4.4 step 1 names "a secret", and the top level has "no `secrets`".
- C10. Section 6 first line (version 0.3, no pointer): "There is no type that runs a command which the hub names"; its replacement is after Section 6.3.
- C11. The status line names the paragraphs "in Sections 1 to 9 and 13"; Sections 11 and 12 also have version 0.4 sentences.

State: Open. The text is the contract (version 0.4) and no code exists. Found by the fourth read of 2026-11-19. Version 0.5 of the contract has the change that this entry proposes.

### SF-089: The clean-up with "All containers" on can remove an adopted project, against R-211

Requirements: task H14; contract K10 version 0.4, Section 6.4.10; R-211, R-227.

What it is, in the words of the reviewer:

- SF-089. R-211 point (2) ("does not remove the project") and 6.4.7 ("There is no `down`") against 6.4.10.
- With "All containers" on, `docker_prune` removes "what `docker container prune`, `docker network prune` … remove". After an `adopted_apply` with `stop`, that is each container and network of the adopted project.
- R-227 opens "the other containers", not the adopted ones.
- Change in 6.4.10: "In each case it removes no container and no network of an adopted project."
- K2. R-246: "Each program of that user holds it with no Anima" is true for the engine. It is not said for the settings file under SF-070; with the definition of SF-070 the text agrees with R-246 point (3).
- I found no other contradiction. R-228, R-229, R-230, R-231, R-232, R-233, R-235, R-236 and R-238 are stated in the sense of each ruling, neither softer nor wider. Section 6.4.12 agrees with R-234 row by row; "a choice with a fixed list" is inside "the install of an entry".
- One sentence to add in 6.4.13 (HOST-018), so that R-226 holds on a system engine: "The daemon gets these values from the applications program, not from the engine."

State: Open. The text is the contract (version 0.4) and no code exists. Found by the fourth read of 2026-11-19. Version 0.5 of the contract has the change that this entry proposes.

### SF-139: A failed stop or a refused image removal left `files/` with its secret values. High

Requirements: task H31; contract K10 Section 6.4.8 ("It deletes `files/` of the application in each case").

What it is: the plan of `application_remove` had the steps: no `.env`, `compose down`, one `image rm` for each pulled image, then the deletion of `files/`. The caller stops a plan at the first step that fails. Thus a failed `down`, or an `image rm` that the engine refuses because a different application uses the image, left `files/` with the environment file and the final form.

Where: `crates/anima-applications/src/work/removal.rs`, `crates/anima-applications/src/plan.rs`.

Effect, and who can cause it: the hub, with a removal of an application whose image a different application uses. Effect: the secret values of the application stay on the machine after its removal.

Mitigation now: two new step kinds. `Step::DeleteFolderInEachCase` deletes `files/` directly after the stop and runs also when an earlier step failed. `Step::Attempt` removes an image; its failure does not stop the plan. `binds/` is deleted only after a stop that did not fail. A test shows the order.

State: Open. The correction is built on the branch `hosted/apps-h31`, commit `d2455995`. Found by a separate reviewer of part 1 of task H31 on 2026-11-20.

### SF-137: The clean-up compared names of images, not digests, and could remove a kept image. Medium

Requirements: task H31; contract K10 Section 6.4.10 ("it never removes an image that `keep` holds").

What it is: `docker_prune` held an image when the text `name@digest`, made from the name in the file, was in the `RepoDigests` of the engine. The engine writes the short name (`nginx@sha256:…`); the file can name `docker.io/library/nginx:1`. With "All containers" on, an image of a kept version with no tag and no container was removed.

Where: `crates/anima-applications/src/work/prune.rs`.

Effect, and who can cause it: the hub, with `docker_prune` and `images`. Effect: `application_back` has no image to start.

Mitigation now: an image is held when the digest of one of its names is a digest of the version that runs or of the kept version of an application. The name part is not compared. `application_remove` with `images` true also leaves out an image whose digest a different application holds. Tests show both.

State: Open. The correction is built on the branch `hosted/apps-h31`, commit `d2455995`. Found by a separate reviewer of part 1 of task H31 on 2026-11-20.

### SF-132: It is not proven that each Compose version obeys `--host` and `--config`. Medium

Requirements: task H31; contract K10 Section 6.4.1.

What it is: the program starts the Compose program with `--host unix://<socket> --config <folder>` before the verb, and gave only `DOCKER_CONFIG` in the environment. No test with a real Compose program shows that each version from the lowest that the program knows takes `--config` and obeys `--host`. If a version does not obey `--host`, it uses the engine of its default socket, which the program did not check.

Where: `crates/anima-applications/src/plan.rs`.

Effect, and who can cause it: no attacker; a fault of the program. Effect: a command goes to an engine that the start checks did not check.

Mitigation now: each command also gets `DOCKER_HOST` with the checked socket in the environment that the program sets. The arguments stay. A comment in the code and the row of task H31 say that part 2 proves the exact list of arguments with a real Compose program, the lowest and the current version, before it does a plan.

State: Open until a real Compose program proves the list. The part in the environment is built on the branch `hosted/apps-h31`, commit `2663858d`. Found by a separate reviewer of part 1 of task H31 on 2026-11-20.

### SF-138: The records said that a new version runs before its start. Medium

Requirements: task H31; contract K10 Sections 6.4.6 and 6.4.9.

What it is: the plan of an apply moved the current version to the kept names and wrote the new Compose file, environment file and digests in place before `up`. A failed start thus left records that said that the new version runs, and a `keep` 1 replaced the last good kept version with a version that did not start.

Where: `crates/anima-applications/src/work/apply.rs`, `crates/anima-applications/src/work/back.rs`, `crates/anima-applications/src/plan.rs`.

Effect, and who can cause it: no attacker; a failed start. Effect: `application_back` goes to a wrong version, and `pull` false uses digests that never started.

Mitigation now: an apply and a back write the three records of the new version under staged names before the start, and move them in place only after the start; the kept version is replaced or deleted only after the start. The final form is written in place before the start, because Compose reads it. Tests show the order.

State: Open. The correction is built on the branch `hosted/apps-h31`, commit `2663858d`. Found by a separate reviewer of part 1 of task H31 on 2026-11-20.

### SF-134: The debug text of a plan showed values of the environment file. Low

Requirements: task H31; `LOG-007`.

What it is: `Prepared`, `Files`, `Kept`, `Step::Write`, `Plan`, `FinalForm` and `Checked` of compose-rules had a derived `Debug`. Their debug text holds the bytes of the environment file or the final form, which hold secret values.

Where: `crates/anima-applications/src/work/apply.rs`, `work/back.rs`, `plan.rs`, `final_form.rs`; `crates/compose-rules/src/lib.rs`.

Effect, and who can cause it: no attacker; a log line or a panic message that a later part writes. Effect: a secret value in a log.

Mitigation now: a hand-written `Debug` for each that shows only the length of the bytes. `Plan`, `Planned` and `Accepted` hold only such steps. A test shows that the debug text of each holds no value.

State: Open. The correction is built on the branch `hosted/apps-h31`, commit `2663858d`. Found by a separate reviewer of part 1 of task H31 on 2026-11-20.

### SF-136: A record of the pair that cannot be read looked like no record. Low

Requirements: task H31; contract K10 Section 6.4.2.

What it is: the checks took the record of the pair as `Option`. A file that the caller cannot read was thus the same as no file: the next request became the first pair, and a different hub was accepted.

Where: `crates/anima-applications/src/pair.rs`, `crates/anima-applications/src/work.rs`.

Effect, and who can cause it: a program of the user of the daemon that damages the file, if it can write it; a fault of the disk. Effect: the protection of a machine that moved to a different hub is lost.

Mitigation now: the file has three states: none, cannot be read, a record. "Cannot be read" refuses each container item with `applications_off`. The file must be deleted when the administrator sets "Manage applications" on again; that is a duty of part 2 and of task H32.

State: Open. The correction is built on the branch `hosted/apps-h31`, commit `2663858d`. Found by a separate reviewer of part 1 of task H31 on 2026-11-20.

### SF-140: The settings file is checked for its owner and mode only. Low

Requirements: task H31; contract K10 Sections 6.4.1 and 6.4.2.

What it is: the program checks that the settings file belongs to the identifier 0 and has no write right for a group or for others. Contract Section 6.4.2 asks no more. A link in the path of the file, a folder above it that a different user can write, or an access list, is not checked.

Where: `crates/anima-applications/src/settings/guard.rs`.

Effect, and who can cause it: a user who can write a folder above the settings file. Effect: he can put a settings file of his own in its place.

Mitigation now: none in the code; the code does what the contract says.

Proposed change of the contract text, for the owner of contract K10: in Section 6.4.2, the rule of Section 6.4.1 applies to the settings file and to each folder above it: each link is resolved, and no link, no real file and no folder above it can be written by a user who is neither the identifier 0, nor an administrator account, by the owner, the mode bits and the access list. The contract file is not changed in this branch.

State: Open, for the owner of contract K10. Found by a separate reviewer of part 1 of task H31 on 2026-11-20.

### SF-133: Forms that compose-rules accepted: carriage returns, `=` in a name, two texts for one target. Low

Requirements: task H28, task H31; contract K10 Section 6.4.4.

What it is: the reader of the environment file cut lines with `lines()`, which takes a carriage return before a line feed as part of the line end, and skipped a line of only spaces. The map form of `environment` accepted a name with `=`. A mount target, a config target and a `tmpfs` path accepted `/.`, `//`, an empty part, a `.` part and a `/` at the end, so that two different texts named one path and "different targets" was not exact.

Where: `crates/compose-rules/src/variables.rs`, `declared.rs`, `forms.rs`.

Effect, and who can cause it: the hub, with a file or an environment file. Effect: Compose reads a different value than the check read, or two mounts of one service name one path.

Mitigation now: the lines are cut at the line feed only, and a line with a carriage return is refused. A line of only spaces is not empty and has no `=`, so it is refused (step 3 of Section 6.4.4). A name of `environment` has no `=`. A target and a `tmpfs` path have one form: absolute, not `/`, and no empty part, no `.` part and no `..` part. Tests show each refused case.

State: Open. The correction is built on the branch `hosted/apps-h31`, commit `dccda56b`. Found by a separate reviewer of part 1 of task H31 on 2026-11-20.

### SF-135: One origin of a hub could have two texts. Low

Requirements: task H31; contract K10 Section 6.4.2.

What it is: the origin accepted an IPv6 address in each form that parses (`fd7a:0::1` and `fd7a::1`), an IPv4 address written as IPv6, and a host with an empty label (`http://a..b:1`). One hub could thus send two texts, and the record of the pair would refuse a hub that did not move.

Where: `crates/anima-applications/src/forms.rs`.

Effect, and who can cause it: no attacker; a daemon that writes the origin in a different form. Effect: a false refusal (`applications_off`).

Mitigation now: one text for each address: an IPv6 address only in its canonical short form (RFC 5952), no IPv4 address written as IPv6, an IPv4 address only in its usual form, and a name with no empty label and a last label that is not all digits. `unix:<path>`, the origin of a hosted hub, stays refused; an open decision of the hosted applications plan asks for it.

State: Open. The correction is built on the branch `hosted/apps-h31`, commit `2663858d`. Found by a separate reviewer of part 1 of task H31 on 2026-11-20.
### SF-144: The converter writes each network alias for each network, so a small command makes a very large file. High

Requirements: HOST-022, R-150; task H29 of [hosted-applications-plan.md](plans/hosted-applications-plan.md).

What it is: `networks()` in `crates/hub/src/work/containers/compose_file/draft.rs` gave each network the whole list of the network aliases, so the size of the file was the number of networks times the number of aliases. A command of 65,515 bytes (about 2,800 `--net` options and aliases) made a `composeFile` of 92.6 MB, with 572 MB of memory in the hub. With aliases of GraphQL, one request could ask for tens of GB.

Who can cause it: each signed-in caller and each API key, also a read key.

The correction: a command has at most 32 networks and at most 32 network aliases; more is the refusal `TOO_MANY_NETWORKS`. A file larger than 256 KiB, the most that the check reads, is the refusal `FILE_TOO_LARGE`, so the converter never makes a file that its own check refuses. No other option is written to more than one place more than a fixed number of times. Tests: `tests/refused.rs` (`host_022_more_than_32_networks_or_aliases_is_refused`, `host_022_a_file_larger_than_the_check_reads_is_refused`).

State: Open. Found by a separate reviewer on 2026-11-20 in the first commit of task H29 (`2773ddf1`). The correction is built on the branch `hosted/apps-h29` (commit `0f4bbdb7`), with a test. The entry stays Open until the operator closes it.

### SF-141: One request can hold about 250 checks of the largest file. Medium

Requirements: HOST-022, API-004; task H29.

What it is: `composeFileCheck` and `composeFromDockerRun` had the complexity 1. One request of 1 MiB could ask for about 250 checks of a file of 256 KiB through one variable, about 4 seconds of a worker of the runtime.

Who can cause it: each signed-in caller and each API key.

The correction: each of the two fields has the complexity 100 with the limit of 500 of the schema, so one request holds at most four. Each check and each conversion runs on a thread for blocking work (`spawn_blocking`), under the log and the span of the request, so the test of the log still reads its lines. Test: `crates/hub/tests/it/containers/compose_file.rs` (`host_022_one_request_holds_at_most_four_checks_or_conversions`).

State: Open. Found by a separate reviewer on 2026-11-20 in the first commit of task H29 (`2773ddf1`). The correction is built on the branch `hosted/apps-h29` (commit `0f4bbdb7`), with a test. The entry stays Open until the operator closes it.

### SF-146: The writer of the converter leaves characters that YAML does not print. Low

Requirements: HOST-022; task H29.

What it is: `quoted` in `compose_file/writer.rs` escaped the control characters and three more, but wrote U+FFFE, U+FFFF and other characters outside the printable set of YAML as they are. The reader of Compose refuses such a file.

The correction: the writer escapes each character outside the printable set of YAML 1.2, and also U+0085, U+2028, U+2029 and U+FEFF. Test: `tests/written.rs` (`host_022_each_character_that_yaml_does_not_print_is_an_escape`).

State: Open. Found by a separate reviewer on 2026-11-20 in the first commit of task H29 (`2773ddf1`). The correction is built on the branch `hosted/apps-h29` (commit `0f4bbdb7`), with a test. The entry stays Open until the operator closes it.

### SF-145: The reader of shell words keeps as text an expansion that a shell does. Low

Requirements: HOST-022; task H29.

What it is: the reader of shell words (`compose_file/words.rs`) kept as text a brace expansion (`{80,443}`), a `~` at the start of a word, `*`, `?` and `[` out of quotes, `$[1+2]` and a quote in `${A:-'x'}`, and read a carriage return alone as a space. A shell changes each of these, so the file did not hold what the command does.

The correction: each is refused with a fixed code: `BRACE_EXPANSION`, `TILDE_EXPANSION` (also after `=` or `:`), `PATTERN`, `ARITHMETIC_EXPANSION`, `VARIABLE_NOT_KEPT` for a quote in a kept variable, and `CONTROL_CHARACTER` for a carriage return that is not part of a line end. In quotes, each stays as text. Test: `tests/refused.rs` (`host_022_each_expansion_of_a_shell_with_no_dollar_is_refused`).

State: Open. Found by a separate reviewer on 2026-11-20 in the first commit of task H29 (`2773ddf1`). The correction is built on the branch `hosted/apps-h29` (commit `0f4bbdb7`), with a test. The entry stays Open until the operator closes it.

### SF-142: A problem of the check shows a secret that the operator wrote in the place of a key. Low

Requirements: HOST-022, LOG-007; task H29.

What it is: a problem of the check showed a name of a key or of a service when it was made of safe characters. A secret written in the place of a key (`hunter2: 1`) or as the name of a service (a token of GitHub) was shown in the problem, in the output of the command and in the result of the MCP tool. The text of the first commit said that a problem holds "never a value of the file"; that was not true for such a name.

The correction: a problem gives a key by its name only when Compose defines that name, and a service by its number in `services` (`serviceNumber` in K1 in place of `service`). An unknown key, an extension and the name of a service are not shown. The list of the options that are not converted names an option that Docker does not have as `(an unknown option)`. Test: `tests/secrecy.rs` (`host_022_a_problem_names_only_a_key_that_compose_defines_and_a_service_by_its_number`) and the hub test with a secret as a key and as the name of a service.

State: Open. Found by a separate reviewer on 2026-11-20 in the first commit of task H29 (`2773ddf1`). The correction is built on the branch `hosted/apps-h29` (commit `0f4bbdb7`), with a test. The entry stays Open until the operator closes it.

### SF-143: The MCP cleaning can change the Compose file that an agent gets. Low

Requirements: HOST-022, MCP-008; task H29.

What it is: `mcp/untrusted.rs` gave the cap of 4,000 characters with kept lines to each field named `composeFile` in each result of each tool. The cleaning also removes invisible characters, so an agent could get a Compose file that is different from the file of the hub, with no sign of it.

The correction: only the top-level field `composeFile` of `anima_compose_from_run` keeps its lines (`cleaned_result`). The tool gives the file only when the cleaning does not change it; else `composeFile` is null, `withheld` says why, and `fileLength` gives the length of the whole file. The description of the tool says that an agent must not install a file from the result without a person reading it. Tests: `mcp_008_only_the_file_of_the_compose_tool_keeps_its_lines` and `mcp_008_a_file_that_the_cleaning_would_change_is_withheld`.

State: Open. Found by a separate reviewer on 2026-11-20 in the first commit of task H29 (`2773ddf1`). The correction is built on the branch `hosted/apps-h29` (commit `0f4bbdb7`), with a test. The entry stays Open until the operator closes it.

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
