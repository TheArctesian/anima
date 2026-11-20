---
kind: Proposal
state: Draft
evidence: Version 2 (2026-11-19): the security review of version 1 is done (24 findings, 5 High; SF-090 to SF-113), and its questions for the operator are OD-RCL1 to OD-RCL16. Version 1 (2026-11-19). R-258 (operator, relayed: "B: agents too, no question") gives the direction: a routed command line or interactive session is open to an AI agent and to an API key, with no question. No requirement of this document is approved. It changes no contract: the text for K10 in Section 5 is a proposal. A security review of this text comes before any code (Section 9), and nothing that gives an agent a shell is merged before the operator has decided its findings.
---

# Proposal: a command line or a session on a machine for an AI agent

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this proposal (PLN-005).

Version: 2. Date: 2026-11-19. Version 2 has the corrections of the text that the security review asked for and that need no answer of the operator (Section 2, Section 5, Section 8, Section 9), and the questions of the review as open decisions. The other corrections of the text wait for his answers, because the answers change the text; Section 11 lists them.
Status: the header of this document holds the state (PLN-020).
Map: [work-plan.md](../work-plan.md), category Use, row 4.7 (jobs, builds and AI agent runs).
From: R-258. The requirements ROU-001, ROU-015 and ROU-016 of [jobs.md](jobs.md) name the function; this document says how it could work, which texts it changes, and what is not safe in it.
Identifiers: RCL-001 to RCL-024, OD-RCL1 to OD-RCL16.

## 1. What the operator decided, and what he did not

The question came from a review of the routing plan against the accepted contract of the work route (K10). The card put it to him as: the Grok chat says yes with no question (R-244, OD-ROU1); the contract says that the hub names no command and that an agent gets a fixed safe set (R-234). He selected "B: agents too, no question": a routed command line or interactive session is open to an agent and to an API key, with no question to him at each start.

The orchestrating session then told him what follows from that answer, and R-258 records it:

- K10 Section 3 ("the hub names no program, no command, no argument") must change by a ruling with exact text.
- The fixed safe set of R-234 no longer limits an agent that has a shell.
- The function needs a security review of its own before any code.
- Until that review is done and he has decided its findings, nothing that gives an agent a shell is merged.

He did not decide: the mechanism, the user that the command runs as, what is recorded, a switch at the machine, or the texts of the contracts. Sections 3 to 7 propose them. Section 10 holds the questions that only he can answer.

## 2. The texts that this function changes

Each row is a text that is in force today and that the function contradicts. None is changed by this document.

| Text | What it says today | What changes |
| --- | --- | --- |
| R-160, and USE-006 and JOB-015 of [jobs.md](jobs.md) | The daemon accepts only the work types of a fixed list. No command and no MCP tool takes a command line for a machine | One work type of the list takes a command line (Section 5). The list stays fixed; one entry of it is "any command" |
| K10, [work-route.md](../../contracts/work-route.md), Section 3, the row "The steps of an item" | For `build` and `runner` the hub names no program, no command, no argument, no path, no address, no image and no environment variable | For the type `command` the hub names one command line. Section 5 has the proposed sentences |
| K10 Section 6, the first lines | "There is no type that takes a command line from the hub" (version 0.4 text) | That sentence becomes false. It must name the type `command` as the one exception |
| MCP-006 of [cli-mcp-plan.md](../plans/cli-mcp-plan.md) | No tool opens a session or runs a command on a device. R-243 and R-244 wrote an exception that has no effect before the review RT7 | The exception gets its mechanism (this document) and its own review (Section 9) |
| INT-003 of the design specification | An API key must not call `startSession` | For an interactive session (Section 6) an API key that is made for an AI agent would call a new function that opens a routed session. `startSession` itself is not opened to a key |
| `AGENTS.md`, "Command line first" | An MCP tool must not open a session | The same exception as MCP-006 |
| R-234 | An API key and an MCP tool may ask only for a fixed safe set of container actions | The set still limits the container tools. It does not limit an agent that has a command line on the machine: `docker` is a command (Section 7) |
| R-120, MCP-002 | With nothing saved, each change tool is `allow` and runs with no question | It applies as written: the tool that starts a command line is `allow` for a new user. R-258 says so ("no question") |
| CHAT-041 of [compute-plan.md](../plans/compute-plan.md), and R-196 (OD-AXT5: the assistant always needs the click) | A tool of the Chat page that acts on a machine needs the click of the operator for each call. R-244 (OD-ROU6) took the click away for a routed start | Not decided for a command line: OD-ROU6 was asked before "work" included a command line. OD-RCL5 |
| R-226, R-246 (the applications program) | The hub, an API key and an agent reach Docker only through the applications program, with the rules of a file | On a machine where the user of the command can use the engine, a command line reaches Docker with no rule (Section 7) |
| SF-003 of [security-findings.md](../security-findings.md) | "The hub selects the code that runs on a machine"; the control is the list of repositories at the machine (R-185) | A command line can run with no working tree (RCL-010), and then it names no repository; with a tree, the list applies to the checkout and not to the command. The list is no control for the command. The control must be a setting at the machine (RCL-006) |
| K10 Section 4 | "An `Item` never holds a command, an argument, a path, an address, an image name or an environment variable" | An item of the type `command` holds a program and its arguments. The sentence must name the exception |
| [jobs.md](jobs.md), "Decision" (OD-W10), and the rule of its routing tasks | "There is no 'run this command' type"; "Never: a command line or a session from a routed start" | The two sentences become false for the type `command` and for a routed session |
| JOB-016 of [jobs.md](jobs.md), and PRO-011 and PRO-024 of [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md) | What an agent may start as a job; a native job on a Mac only where its owner set it on | A `command` item of the kind `native` is a native job: the setting of PRO-024 applies to it, with the setting of RCL-006 |
| AXT-021 of [agent-tools-as-extensions.md](agent-tools-as-extensions.md) (R-196) | An install of a tool on a machine always needs the click, in any MCP state | A command line can install a tool with no click (SF-106). OD-RCL15 |
| LOG-007 of the design specification | No credential, token or session content in a log | A command line in the audit entry can hold a secret that no pattern finds (RCL-012; OD-RCL3) |
| R-243 | "no tool takes a command line" | The sentence becomes false |
| ROU-015 of [jobs.md](jobs.md) | A routed session is "a session of the hub … as the user of the daemon" | The two halves name two mechanisms. This document separates them (Sections 5 and 6) and proposes a different user (RCL-008) |

## 3. Two things with one name

"A command line or a session" is two functions. They have different mechanisms in Anima today, and the review must read them apart.

| | A command line | An interactive session |
| --- | --- | --- |
| What it is | One command with its arguments. It runs to its end and gives its output and its exit code | A terminal that stays open: each key goes to the machine, each output comes back at once |
| Who uses it | An agent, a script | A person; an agent only through a tool that drives a terminal |
| Mechanism today | None. K10 forbids it | A session of the hub (design specification Section 6.3): the hub opens SSH to the device with the stored login of the device, through the gateway |
| Proposed mechanism | A work type of K10, `command`, that the daemon claims (Section 5) | The session of the hub, opened on the machine that the placement names (Section 6) |
| Does the queue apply | Yes: the daemon asks, the hub never opens a connection to a machine (K10 Section 2; R-259) | No. ROU-007 says that a session does not wait: it is refused at once when no machine fits |

## 4. Rules for the two (proposed)

- **RCL-001.** A command line or a session on a machine for an agent exists only as one of the two forms of this document. No other tool, command or work type takes a command line.
- **RCL-002.** Routing never widens access (ROU-003). A caller gets a command line or a session only on a machine that its token may use, that takes work, and whose setting of RCL-006 is on.
- **RCL-003.** An agent or an API key gets no more on a machine than the user of RCL-008 or RCL-015 has. Anima adds no right of an administrator.
- **RCL-004.** The limit of ROU-010 applies: at most 30 routed starts for each caller in each hour. A command line and a session each count as one start.
- **RCL-005.** The hub writes one audit entry when a command line or a session starts and one when it ends: the actor kind and the caller, the machine, the reason of the placement, the start, the end and the exit state (ROU-015). What else is kept is RCL-012 and RCL-018.
- **RCL-006.** A machine has one setting, "Command lines for agents", at the machine. The hub cannot set it on. The rules of the switch of K10 Section 3 apply to it (SF-007). With the setting off, the daemon refuses an item of the type `command` (`command_not_permitted`, the one name for this cause) and the hub opens no routed session to the machine. The value after the install is OD-RCL1.
- **RCL-007.** The functions do not exist on a machine of a person who is not the operator before the key of the device exists (task W7 of [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md); R-186).

## 5. The command line: a work type of K10 (proposed text)

- **RCL-008.** A `command` item runs inside the isolation of a job (task W2): in a container with no capability, or, for the kind `native` on a Mac, as the build user (PRO-022). It does not run as the user of the daemon. That user holds the daemon token (K8 Section 2), and on a machine with an engine of a user it can use Docker (R-246). This is different from ROU-015, which says "as the user of the daemon": OD-RCL2.
- **RCL-009.** The item names one program and its arguments as a list. The daemon starts the program with that list and with no shell. A caller that wants a shell names the shell as the program; nothing in Anima reads the text as shell syntax.
- **RCL-010.** The item names a working tree: a repository of a project and one commit (40 hexadecimal characters; a caller that names a branch gets the commit that the hub reads for it), which the job checks out new (PRO-023), or none. A tree of a private repository needs the purpose `repository_token`, which RCL-011 does not give: in this version a `command` item gets a tree only of a public repository (SF-104). With none, the job starts in an empty folder.
- **RCL-011.** A `command` item gets no secret: `secrets` is empty, and the daemon refuses one that names a purpose (`secret_unknown`). It gets no token of Anima (K10 Section 6.3).
- **RCL-012.** The output of the command is the log of the item (K10 Section 7): at most 2 MiB, each line redacted by the daemon and again by the hub, marked as text that is not trusted (SF-010). The command line itself is kept in the audit entry, redacted, with at most 4 KiB. This follows R-120, which keeps the cleaned arguments of each change that runs with no question; it is against the recommendation of OD-ROU9 in [jobs.md](jobs.md), which was "no". OD-RCL3.
- **RCL-013.** The limits of an item apply (`limits.seconds`, memory, disk). The longest time of a `command` item is 3,600 seconds. The network of the job is the network of a job of W2: no loopback address of the machine, no link interface, no private range, unless the owner of the machine set otherwise at the machine.
- **RCL-014.** The result has the exit code of the command as a number. The hub gives the caller the exit code and the log; the reply of the start names the item, and the caller reads the end with a second call. A `command` item waits in the queue as each job does (ROU-007).

The sentences that K10 would get, as a later version of the contract. They are not in the contract.

- Section 1, "Work type": "The list is fixed: `build`, `runner`, the container types, and `command`."
- Section 3, the row "The steps of an item": "For `command` the hub names one program and its arguments. The machine runs them inside the isolation of a job, as a user that holds no token of Anima and no right over the engine of the machine."
- Section 3, a new row: "If the machine takes command lines | The owner of the machine | The setting 'Command lines for agents' at the machine. The hub cannot set it on."
- Section 3, the paragraph "What the fixed list does and does not give", added: "With the setting 'Command lines for agents' on, a person who controls the hub, and each agent and each API key that the hub accepts, can run a program of their choice on the machine, inside the isolation of a job. The isolation is the only limit."
- Section 5, a new cause: "`command_not_permitted` | The item is of the type `command` and the setting at the machine is off".
- A new Section 6.5, `command`:

```json
{ "program": "cargo", "arguments": ["test", "--workspace"], "tree": { "commit": "0123456789abcdef0123456789abcdef01234567" }, "actor": "agent" }
```

  "`program` is at most 256 bytes and `arguments` has at most 256 members of at most 4,096 bytes each. No member is read as shell text. `tree` is null or names a commit of `repository`. `actor` is `user`, `agent` or `hub`, for the log of the machine. `secrets` is empty. The result adds `exitCode`, a whole number."

## 6. The interactive session: a session of the hub (proposed)

- **RCL-015.** A routed session is a session of the hub as the design specification Section 6.3 defines it: the hub opens SSH to the device with the stored login of the device. The user is the user of that login. It is not the user of the daemon, and the daemon takes no part.
- **RCL-016.** The placement names the machine (R-259). Only a machine that has an SSH login in the vault and whose host key is accepted can be named. The hub never accepts a host key for a routed session (INT-003 stays for trust decisions).
- **RCL-017.** A new function of the hub opens a routed session. A login of a person can call it, and an API key that is made for an AI agent can call it. `startSession` stays closed to each key. The session has the idle and the longest time of a session of the operator, and the operator sees it in the list of sessions and can end it.
- **RCL-018.** What is kept of a session: the two audit entries of RCL-005. No key and no output is kept: the recording of sessions is deferred (R-113). OD-RCL4.
- **RCL-019.** A session does not wait in the queue (ROU-007). When no machine fits, the start is refused with the reasons.
- **RCL-020.** How an agent uses a terminal is not in Anima today: an MCP tool gives text and takes text. A session for an agent needs a tool that sends keys and reads the screen, or the agent runs in a pane that the operator opened (AXT-012). This document proposes no such tool: OD-RCL6.

## 7. What is not safe, said plainly

- **RCL-021.** With a command line an agent can do each thing that the user of RCL-008 can do. With a session it can do each thing that the stored login of the device can do. For most devices that login is the account of the operator, often with `sudo`.
- **RCL-022.** R-234 does not hold against a session: on a machine where the stored login can use Docker, the agent runs `docker` and installs, removes and reads what it wants. It holds against a `command` item only where the isolation of the job has no way to the engine (W2: a container with no socket of the engine; a build user that is in no group of the engine).
- **RCL-023.** A prompt injection is the usual way in: text that an agent reads (a web page, an issue, a log line of a job) tells it to run a command. With `allow` by default (R-120) and no click, nothing stands between that text and the machine but the isolation and the setting of RCL-006.
- **RCL-024.** The audit log says who and where, and for a command line what. It does not stop a thing. For a session it does not say what was done.

## 8. On a Mac

- The daemon of a Mac runs as the user of the operator, and with Docker Desktop that user can use the engine (R-246).
- A `command` item of the kind `native` would run as the build user (PRO-022), which is not in the group of the operator. If that user can reach the socket of Docker Desktop is not checked: the record of task W2 says "to check with an administrator" (SF-098). That user does not exist yet: task W2 has it as "needs an administrator on a Mac".
- A `command` item of the kind `container` on a Mac runs in a container of Docker Desktop with no socket of the engine in it.
- A routed session on a Mac is an SSH login as the stored user, with Remote Login on. That user can use Docker, the keychain of the user when it is unlocked, and each file of the user.
- Thus on a Mac the command line is at most as safe as the isolation of W2, which is a proposal that nobody accepted, is not built and is not measured for this (SF-091), and the session is a full login.

## 9. Before any code

1. The security review of this document, by a reviewer who gets the text and no opinion of the session that wrote it. Done for version 1 on 2026-11-19: the report is [command-lines-review-2026-11-19.md](../research/work-route/command-lines-review-2026-11-19.md), and its 24 findings are SF-090 to SF-113 of [security-findings.md](../security-findings.md). A second read comes after the text has his answers.
2. The operator decides each finding that needs him, and OD-RCL1 to OD-RCL16. No finding of severity High stays open when the function ships (as ROU-014 says for routing; OD-RCL16).
3. The text of K10 changes by a ruling, with the exact sentences (CON-001). INT-003, MCP-006, JOB-015 and `AGENTS.md` change in the same ruling.
4. Only then tasks. A `command` item needs W4 (the daemon runs a job in isolation) and thus W2. Nothing that gives an agent a shell is merged before step 2 is done (R-258).

## 10. Open decisions

| Number | Question | Recommendation | State |
| --- | --- | --- | --- |
| OD-RCL1 | The setting "Command lines for agents" at a machine: on or off after the install, and in which form? The review (SF-094): one switch covers two mechanisms; it has the weaker rule of the switch of the daemon, not the file that only the administrator can write (K10 Section 6.4.2); and for a session it is a rule of the hub, because the daemon takes no part | Off after the install; two settings, one for each form; each in a file that only the administrator of the machine can write; and no session to a machine before the machine itself can refuse it. "No question" (R-258) is about a question at each start; a setting that the owner of a machine sets one time is the permission of that machine, as "Takes work" is. Weakness: more steps before the function works | Open |
| OD-RCL2 | As which user does a command line run: the isolation of a job (RCL-008), or the user of the daemon (ROU-015 as written)? The review (SF-092): the user of the daemon gives the daemon token and the socket of the applications program, where the rules for actors do not apply, so R-234 has no effect for each API key | The isolation of a job, and take the other choice off: ROU-015 changes. Weakness: it waits for W2 and W4 | Open |
| OD-RCL3 | Is the command line kept in the audit entry (redacted)? | Yes (R-120). Weakness: an argument can hold a secret that no pattern finds | Open |
| OD-RCL4 | A session keeps no record of what was done (R-113). Is that accepted for a session that an agent opens? | No session for an agent before a record exists; a command line first. Weakness: it does less than "sessions and command lines" | Open |
| OD-RCL5 | Does a command line or a session from the Chat page need the click (CHAT-041), or none (R-244, OD-ROU6)? | The click stays for a command line and a session; OD-ROU6 stays for work of the other types. Weakness: one more exception to "no question" | Open |
| OD-RCL6 | Does Anima get a tool with which an agent drives a terminal (RCL-020)? | No, not now | Open |
| OD-RCL7 | Which form is built first? | The command line (Section 5), after W2 and W4. The session waits for OD-RCL4 | Open |
| OD-RCL8 | Is the interactive session built for an agent at all? The review (SF-090): a routed session is a full login of the stored user for each agent key; from it each setting at the machine can be set on, what the agent leaves stays, and nothing of it is recorded | No: for an agent only the command line. A person with a login can ask for a session on the machine that the placement names. Weakness: it does less than his answer says ("sessions and command lines") | Open |
| OD-RCL9 | Does the command line wait for an isolation that is accepted and measured (SF-091)? The design of task W2 is a proposal that the security reviewer and the operator did not accept; Linux, the build user and the network limits are not measured; task W4 is not started | Yes: no `command` item before W2 is accepted with its measurements and W4 is built; on Linux a container, on a Mac a container first and the build user after its check. Weakness: the function is not near | Open |
| OD-RCL10 | Does an API key need a permission of its own at the hub for a command line (SF-093)? Today each write key would get the function, the MCP state is a file on the computer of the caller, and no text limits a key to devices | Yes: a key gets the function only when the operator set it for that key, with the list of the machines; a key of today does not get it. Weakness: one more setting for each key | Open |
| OD-RCL11 | Is the function open on a hub that the owner of the machine does not control (SF-095)? There the list of R-185 is no control for a command, and the key of the device (W7) proves the machine to the hub, not the hub to the machine | No: only on a machine of the operator at his own hub, until a later ruling. Weakness: nothing for a person who is not the operator | Open |
| OD-RCL12 | Which network does a command have (SF-097)? Proposed in RCL-013: the public internet and no local address. The limits are not measured | The public internet, with the local limits of W2 measured first; "unless the owner set otherwise" is taken out of RCL-013 until it has a text. Weakness: a command can send data out | Open |
| OD-RCL13 | The limits (SF-101): 30 starts in an hour at 3,600 seconds each can hold 30 machines with no end, and "caller" has no definition | A caller is one API key or one login; at most 2 `command` items that are not final for one caller, and 1 for one machine; the 30 in an hour stays. Weakness: slower for an agent that wants many machines | Open |
| OD-RCL14 | The output of a command comes back to an agent that can start the next command (SF-103) | Accept it as the risk of the function, with the mark of text that is not trusted, the limit of OD-RCL13 and the setting of OD-RCL1; a pinned command names its machine so that a machine with a stolen token does not get it. Weakness: the mark does not stop a model | Open |
| OD-RCL15 | The click rules that R-196 made absolute (SF-106): an install of a tool on a machine "always needs the click", "in any MCP state". A command line can install with no click | Say in the ruling that a command line is the exception and that R-196 then limits only the tools of Anima, or keep the click for a command line. The recommendation of this session: keep R-196 and the click of CHAT-041 for a command line (OD-RCL5). Weakness: it is not "no question" | Open |
| OD-RCL16 | The gate (SF-112): "he has decided its findings", or "no finding of severity High is open" as ROU-014 says for routing? | No finding of severity High open, and each other finding decided. Weakness: none known | Open |

## 11. Corrections of the text that wait

The review also asks for corrections that need no decision but that depend on the answers above. They are not made in version 2: SF-096 (a command shares its isolation with `build` and `runner`), SF-099 (what file system a container command has), SF-102 (a queued command that starts when the setting goes on later; a refusal that counts for `WORK_OFF`), SF-105 (a key that speaks the tunnel of a session directly; the start command of an SSH login; the longest time), SF-107 (the lease rules of `command`), SF-108 (who selects the kind; the value `hub` of `actor`), SF-109 (arguments that others can see), SF-110 (the environment, the search path and the input), SF-113 (sentences that are true and protect nothing). Made in version 2: SF-098, the names of SF-102, the working tree of SF-104, the texts of SF-111, the gate of SF-112.

