# Record: the security review of a command line or a session for an AI agent (2026-11-19)

| Item | Value |
| --- | --- |
| Document | The report of the separate reviewer of [routed-command-lines.md](../../proposals/routed-command-lines.md), version 1 |
| Date | 2026-11-19 |
| Language | The words of the reviewer, not changed |
| Kind | Record. It adds no requirement (PLN-010). |

The session that wrote the proposal gave a separate reviewer agent the text and the rules of the project, and no opinion. This is its report as it wrote it. The findings F1 to F24 are the entries SF-NEW-rcl01 to SF-NEW-rcl24 of [security-findings.md](../../security-findings.md), in the same order. The paths in the report are from the root of the repository.

Reviewer: an independent reviewer agent. I got the text and the project's rules, and no opinion of the session that wrote the proposal. I changed no file of the repository. No code exists for this function, so each finding is about the text.

## 0. Method, and what "verified" means here

Read in full: the proposal; `contracts/work-route.md` (K10, version 0.4, all sections); `docs/proposals/jobs.md`; `docs/research/work-isolation/w2-isolation.md`; `docs/security-findings.md` lines 1 to 378 (the format, SF-003 to SF-014, the preamble of the container review) and the headings of SF-015 to SF-069.
Read in part: `contracts/daemon-push.md` Sections 1 and 2; `docs/spec/design-spec.md` Section 6.3, INT-003, LOG-005 to LOG-018, MET-013; `docs/plans/cli-mcp-plan.md` MCP-001 to MCP-012, MCP-024 to MCP-026, CMD-010 to CMD-013; `docs/rulings.md` entries R-113, R-120, R-160, R-185, R-186, R-196, R-206, R-231, R-234, R-236, R-243, R-244, R-246, R-NEW-agentshell, R-NEW-placement; `docs/plans/projects-and-builds-plan.md` PRO-011, PRO-022 to PRO-024, PRO-029, PRO-030 and the rows of tasks W0 to W7; CHAT-041 of `compute-plan.md`; AXT-012 and AXT-021 of `agent-tools-as-extensions.md`.
Not read: the bodies of SF-015 to SF-069, K4, the hub code, the daemon code. I ran no program and measured nothing.

In each finding, "Verified" means that I read the quoted sentence in the named file. "Inferred" means that I reasoned from those sentences or from general knowledge of an operating system, SSH or Docker, and did not check it.

The proposal is honest about its main risks (Section 7, Section 8, the proposed K10 sentence "The isolation is the only limit"). Most findings below are not that the text hides a risk, but that a control it names does not exist, does not apply to one of the two mechanisms, or is weaker than the control it replaces.

## 1. Summary table

| # | Title | Severity | Who decides |
| --- | --- | --- | --- |
| F1 | A routed session is a full login of the operator for each agent key, and it defeats each setting at the machine | High | Owner |
| F2 | The isolation that the command line depends on is a proposal: not accepted, not built, not measured on Linux | High | Owner, and text |
| F3 | The option "as the user of the daemon" of OD-RCL2 gives the daemon token and the applications socket | High | Owner |
| F4 | No permission at the hub for a key: each write key gets the function, and the MCP state is not a control | High | Owner |
| F5 | The setting of RCL-006: one switch for two mechanisms, with the weaker protection, and for a session it is not at the machine | High | Owner (OD-RCL1), and text |
| F6 | A hub that the owner of the machine does not control: the list of R-185 is lost, and RCL-007 names the wrong control | Medium | Owner |
| F7 | A command shares its isolation with `build` and `runner`: what it leaves can reach a later job that has a secret | Medium | Text, then owner |
| F8 | The network of a command: the public internet is open, the limits are not measured, and "unless the owner set otherwise" is a setting with no text | Medium | Owner, and text |
| F9 | Section 8 states as fact two things that the W2 record says are not checked | Medium | Text |
| F10 | The text does not say what file system a container command has; the inventory is of the tools of the host | Medium | Text |
| F11 | The audit entry: 4 KiB of a command line of up to 1 MiB, a secret that stays one year, and no entry in the log of the machine | Medium | Owner (OD-RCL3), and text |
| F12 | The limit of 30 starts does not limit: 30 commands of one hour hold each slot, "caller" has no definition, queued work counts later | Medium | Owner, and text |
| F13 | The queue: a command starts when the owner sets the switch on later; a refusal takes the machine out of work; two names for one cause | Medium | Text |
| F14 | The output of a command instructs the agent that has a shell; a stolen daemon token can take commands and give false results | Medium | Owner in part |
| F15 | The working tree: the text says a command has no repository and then gives it one; a private checkout with no secret; source of a private repository can leave | Medium | Text, then owner |
| F16 | A session needs no tool of Anima to be used by a key; the start command of an SSH session; the idle limit does not bind an agent | Medium | Text |
| F17 | A command line removes the click rules that R-196 made absolute (install of Hermes, AXT-021) | Medium | Owner |
| F18 | A command can run two times or continue with no hub: the lease rules for `command` are not written | Low | Text |
| F19 | The hub selects the kind, and `native` is the weaker isolation; `actor` has the value `hub`; PRO-024 is not named | Low | Text |
| F20 | The arguments are visible on the machine and in the lists of the hub; two daemons on one Mac share one build user | Low | Text |
| F21 | The environment, the search path and the input of the command are not written | Low | Text |
| F22 | The texts that Section 2 missed | Low | Text |
| F23 | The gate of Section 9 is weaker than ROU-014 | Note | Owner |
| F24 | "No shell" and "`startSession` stays closed" are true and protect nothing | Note | Text |

## 2. What each attacker gets that it did not have

| Position | Before (accepted texts) | With the proposal as written |
| --- | --- | --- |
| An AI agent that read hostile text | The MCP tools. For a machine: the fixed safe set of R-234; a build of a repository that is in the list of the machine | A program of its choice on each machine whose setting is on, inside an isolation that does not exist yet (F2), with the public internet (F8). With the session form: a login as the operator, often with `sudo` (F1) |
| A holder of a write API key | The same as above, and the limits of INT-003 | The same as the agent, with no MCP state in the way (F4). With an agent key and the session form: each right of the stored login, which includes the login token of the operator in the key store of that machine (F1) |
| A holder of a stolen daemon token | Claim, report, a secret of an item of that device; the environment file of an application (R-231) | Also: the text of each command that the hub gives that device, and a false exit code and a false log for the agent that waits (F14). With the best "free memory" it wins each command that names no machine |
| A person who controls the hub | Code of a repository that is in the list of the machine, inside the isolation (SF-003, R-185). The stored SSH logins (as today) | A program of their choice with no repository and no list, on each machine whose setting is on (F6). For sessions nothing new: the hub holds the logins |
| A job or a command that runs on the machine | Its own isolation | If the isolation is shared with later jobs: files and processes that a later `build` with a project secret meets (F7). On a Mac: the local ports of the operator, if the network rule is not built (F8, F9) |
| A second local user of the machine | Nothing of Anima | The arguments of each command in the list of processes (F20). On a Mac with two accounts and two daemons: one build user for the two (F20) |

## 3. Findings that the owner must decide

### F1. A routed session is a full login of the operator for each agent key, and it defeats each setting at the machine

Severity: High. One call with an API key gives a prompt-injected agent the account of the operator on a machine, and from there each other control of Anima on that machine can be set off.

What it is. RCL-015 makes a routed session "a session of the hub": "the hub opens SSH to the device with the stored login of the device. The user is the user of that login." RCL-017 opens it to "an API key that is made for an AI agent". RCL-021 says the result: "For most devices that login is the account of the operator, often with `sudo`." The text says this plainly. What it does not say is what follows from it:

1. Each setting "at the machine" can be written from the session. K10 Section 3 keeps the switch "where no job and no other user can write it". The stored login is not "an other user": on a user installation it is the user of the daemon (R-206: the daemon is installed "as a background unit of the user, with no administrator password"). With `sudo` it is also the administrator, who alone sets "Manage applications", "All containers", "The hub may delete volumes" (K10 Section 6.4.2). So one session can set on "Takes work", "Command lines for agents", native jobs, the list of repositories of R-185 and each container setting. Inferred from those sentences.
2. The limits of INT-003 fall. INT-003: "An API key must not manage API keys. It must not call `signInBrowser`, `startSession`, `openFiles`, … It must not set or clear a device credential". CMD-012 keeps the login token of the `anima` command "in the operating system key store (service `ing.anima.cli`…)". The proposal's own Section 8 says the stored user can use "the keychain of the user when it is unlocked". A session on the computer where the operator is signed in can thus take the login token, and with it make API keys, call `startSession`, accept a host key and enrol a daemon. Verified sentences; the step "can read the item" is inferred.
3. The daemon token of that machine is readable (K8 Section 2: "a file with mode `0600` owned by the user that runs the daemon", or the keychain), and the socket of the applications program accepts the session's user on an engine of a user (K10 Section 6.4.1: "Each program that runs as the user of the daemon can send a request on the socket with no hub").
4. Persistence has no limit: an authorized key, a scheduled entry, a changed program. The end of the session removes none of them, and RCL-018 keeps no record of what was done.
5. R-234 does not hold (the text says so, RCL-022), and AXT-021 does not hold (F17).

Where. Proposal Section 6 (RCL-015 to RCL-020), RCL-021, RCL-022, Section 8.

Effect, and who can cause it. An AI agent that read hostile text; each holder of an agent key. Effect: the machine, the account of the operator on it, and through the key store the hub as the operator.

Mitigation in the proposal now. RCL-016 (no new host key by the hub); the setting of RCL-006, which for a session is not a control at the machine (F5); OD-RCL4 recommends "No session for an agent before a record exists"; OD-RCL7 recommends the command line first. These are recommendations of open decisions, not requirements. A record of the session (OD-RCL4) would show the damage afterwards; it would stop none of points 1 to 5.

What would correct it. The owner refuses or accepts the session form as a thing of its own, apart from the command line; the question to him should say points 1 to 3 in words. If he keeps it: a separate login for agents on each device (not the login of the operator, no `sudo`, not the user of the daemon), named at the machine by its owner; a setting of its own (F5); never on the computer where the operator is signed in to the hub. If he refuses it: delete Section 6 and write in RCL-001 that a session is not open to a key.

### F2. The isolation that the command line depends on is a proposal: not accepted, not built, not measured on Linux

Severity: High. The proposed contract sentence is "The isolation is the only limit", and that limit does not exist.

What it is. RCL-008 puts a `command` item "inside the isolation of a job (task W2)". The W2 record says of itself: "Status: Partly measured. One Mac is measured. No Linux computer is measured, and no second user of macOS was made… The design in Section 4 is a proposal. The security reviewer and the operator did not accept it." Its Section 5 lists as not shown: each Linux measurement, the build user, "The network limits of a job". Its Section 3.3 says of the one container test: "The network of the job was not limited in this test", and that the test ran inside the virtual machine of Docker Desktop: "A Linux computer has no such second wall." Task W4 is "Not started". All verified.

Two more points:

- W2 was designed for `build` and `runner`, which have a second control outside the isolation: the list of repositories (R-185), and for a fork the approval rule of GitHub. `command` has none (the proposal's Section 2, row SF-003, says so). W2 Section 4.1 itself proposes, for code that is not trusted, "a small virtual machine for each job, not a container. Not measured." A command that a prompt injection wrote is code of that class, and it arrives with no approval of a person.
- The proposal says the dependency only in Section 9 ("A `command` item needs W4 … and thus W2") and in the weakness of OD-RCL2. No requirement RCL-nnn makes it a condition, and RCL-013 states the network rule as a fact of today ("The network of the job is the network of a job of W2").

Where. RCL-008, RCL-013, Section 8, Section 9; W2 record Sections 3.3, 4 and 5.

Effect, and who can cause it. Each caller of the function and a person who controls the hub. If the function is built on a W2 that is weaker than its proposal, the effect is the machine.

Mitigation now. Section 9 step 4; the last line of Section 8 ("as safe as the isolation of W2, which is not built").

What would correct it. A requirement: no `command` type before the W2 design is accepted by the security reviewer and the operator, measured on one Linux machine and one Mac with a real build user, with the network rule measured, and before the tests of W4 pass ("a job cannot read the daemon token or the home folder, and … nothing stays after a stop"). The owner decides if a container on a shared kernel is sufficient for a command of an agent, or if it gets the virtual machine that W2 names for code of a fork.

### F3. The option "as the user of the daemon" of OD-RCL2 gives the daemon token and the applications socket

Severity: High if selected. It is an open choice on the card of the owner, and ROU-015 as written already says it.

What it is. OD-RCL2 asks: "the isolation of a job (RCL-008), or the user of the daemon (ROU-015 as written)?" and gives as the only weakness of the first "it waits for W2 and W4". The second option is the fast one. The proposal names two of its effects (the daemon token; Docker on a Mac). It does not name the third: K10 Section 6.4.1, "Each program that runs as the user of the daemon can send a request on the socket with no hub. It gets what the rules of the machine permit. The rules for actors (Section 6.4.12) are rules of the hub and do not limit such a request. The operator accepts this (R-236)." R-236 was accepted for programs that are already on the machine. With this option each API key becomes such a program on each machine: it can then ask for `application_apply` with a Compose file of its own, `adopted_apply` (which K10 says can make a user "the administrator of the machine" on a system engine, R-235), `docker_prune` and `container_follow`. The table of R-234 has no effect. With the daemon token it can also claim items of that device and get their secrets (SF-006) and the environment file of each application (R-231). K8 Section 1 already says: "A job must thus run as a different user (task W2)." All verified sentences; the chain is inferred.

Where. OD-RCL2; RCL-008; jobs.md ROU-015.

Effect, and who can cause it. An agent, a holder of a write key, a person who controls the hub. Effect: each container work type with no actor rule, the secrets of the machine's items, and on a system engine with an adopted project the administrator.

Mitigation now. The recommendation of OD-RCL2 is the isolation.

What would correct it. Take the second option off the card, or write on it the three effects. Change ROU-015 in the same ruling so that no accepted text says "as the user of the daemon".

### F4. No permission at the hub for a key: each write key gets the function, and the MCP state is not a control

Severity: High. Each key that exists today for a script or an agent becomes a way to run a program on machines, and the only state that the text names can be passed by the key itself.

What it is.

- The proposal names one control at the caller: the MCP state (`allow` by default, R-120). MCP-003 says: "The hub token is the real limit." MCP-012 says the states "are saved in the configuration file" of the computer that runs `anima mcp`. A holder of the key calls the hub directly; an agent with a file tool on that computer can change the file. So `ask` and `off` protect only against an agent that keeps the rules. Verified sentences.
- INT-003 gives a key two properties: "a scope (read or write)" and "a 'made for' value". The proposal adds no third. RCL-017 opens the session to "an API key that is made for an AI agent" and does not name the scope. Section 5 does not say which key may make a `command` item. By the texts, each write key may.
- RCL-002 limits a caller to "a machine that its token may use". I found no text that limits a key to devices (INT-003 as read has none; a search of the specification and of the MCP plan found none). The phrase comes from ROU-003 and has no content: each key may use each machine. Not found is not proof; marked inferred.
- The session is open to a key made for an agent and, by the words, not to a key made for a script. The key kind that reads hostile text gets more than the kind that does not. LOG-015 calls "made for" an attribute for the record of who acted, not a permission.
- The proposal does not say that the start tool has `destructiveHint`. MCP-025 ("Ask before destructive actions") and MCP-026 (the audit of "every destructive call") apply only to a tool with that hint.

Where. Proposal Section 2 (row R-120, MCP-002), RCL-002, RCL-017; INT-003; MCP-003, MCP-012.

Effect, and who can cause it. A holder of each write key, also a key made long ago for a script; an agent. Effect: the function, on each machine whose setting is on.

Mitigation now. The setting of RCL-006 at the machine; the limit of RCL-004 (F12).

What would correct it. The owner decides: a permission of a key, "may start command lines", off for each key that exists and off for a new key, set only by a login of the operator, with an optional list of devices, checked by the hub. It does not break "no question": it is one decision for a key, as the setting is one decision for a machine. In the text: name the scope in RCL-017; give RCL-002 a real rule or delete the phrase; give the tool `destructiveHint`.

### F5. The setting of RCL-006: one switch for two mechanisms, with the weaker protection, and for a session it is not at the machine

Severity: High. It is the one control that the text puts between hostile text and the machine (RCL-023), and for one mechanism it does not exist at the machine, and its default is not decided.

What it is.

1. For a session the daemon "takes no part" (RCL-015). RCL-006 says "the hub opens no routed session to the machine" when the setting is off. So for a session the hub reads and obeys the setting: it is a rule of the hub, as the actor table of K10 Section 6.4.12 is ("The machine cannot see the actor"). The text does not say how the hub learns the value. A device with an SSH login and no daemon (RCL-016 asks only for a login and a host key) has no place for the setting. Inferred from the two requirements.
2. One setting covers a command in an isolation and a full login. An owner who wants the first must permit the second.
3. Its protection is "the rules of the switch of K10 Section 3": kept "where no job and no other user can write it". The container settings have a stronger rule: "Only the administrator of the machine can set them … in one file that only the administrator can write" (K10 Section 6.4.2). `command` gives more than each container type and gets the weaker rule. On a user installation, each program of the user of the daemon can set it on: the daemon itself (K10 Section 6.4.1 names "A fault of the daemon that the hub can reach"), a coding agent that runs on that computer, and a routed session (F1).
4. The default is open (OD-RCL1; recommendation "Off"). The precedent of this project is the other way: USE-004 "takes work by default", USE-008 "On after update", R-160 "A: on everywhere". If the answer is "on", RCL-023 leaves only an isolation that is not built.
5. The setting is not bound to a hub. K10 Section 6.4.2 binds the container settings to "the origin of its hub and its device identifier", because an owner who moves a machine forgets them (SF-039, SF-051). RCL-006 has no such rule.
6. The name, "Command lines for agents", says less than the effect. The proposed K10 sentence is: "a person who controls the hub, and each agent and each API key that the hub accepts, can run a program of their choice on the machine". The screen must say that sentence (the pattern of R-238 and of the adoption screen of K10 Section 6.4.7).
7. The claim has no member for it. For container work the claim got `applications`, and the hub gives no container item when it is off. Without a member, the hub gives `command` items to machines that refuse them (F13).

Where. RCL-006; proposed K10 Section 3 row; OD-RCL1.

Effect, and who can cause it. A person who controls the hub (for a session: no setting stops them, which is as today, since the hub holds the login); a program of the user of the daemon; an owner who reads the name and not the effect.

Mitigation now. The recommendation "Off" of OD-RCL1; "The hub cannot set it on".

What would correct it. Owner: decide OD-RCL1 as "off after the install and after an update". Text: two settings; each in the file that only an administrator writes, on each kind of installation; bound to the hub origin and the device; a member of the claim; the sentence of the effect on the screen; for a session, say in words that the control is a rule of the hub, or move the control to the machine (a login for agents that exists only when the owner made it, F1).

### F6. A hub that the owner of the machine does not control: the list of R-185 is lost, and RCL-007 names the wrong control

Severity: Medium. The text says the effect; the control that the owner got for SF-003 does not carry over, and the condition that should protect a shared machine does not do so.

What it is. SF-003 was decided with "A: allow-list at machine" (R-185): "a machine with no list refuses each item that names a repository". The proposal's Section 2 says: "A command line has no repository. The list is no control for it." R-160 put the work route "on everywhere", also for a hosted hub that "runs on the pool". So with the setting on, a person who controls the hosted hub, or who breaks into it, runs a program of their choice with no repository of GitHub between them and the machine.

RCL-007 says the functions do not exist "on a machine of a person who is not the operator before the key of the device exists (task W7 …; R-186)". Task W7 is: "The key of a device that signs each claim and each request for a secret, so that a daemon token alone gets no secret." That key proves the machine to the hub. It does not prove the hub, or the operator, to the machine. K10 names the other direction only for the applications program ("checks a signature of the hub or of the operator on each request"). A start "with no question" has no signature of the operator by its nature. So RCL-007 protects nobody against the hub, and its case (a machine of a different person at the operator's hub) is not the hosted case (the operator's machine at a hub of a different person). Verified sentences; the conclusion is inferred.

Where. Proposal Section 2 (row SF-003), RCL-007, proposed K10 Section 3 paragraph.

Effect, and who can cause it. A person who controls the hub. Effect: as the sentence of the proposal says.

Mitigation now. The setting; the plain sentence.

What would correct it. The owner decides if `command` exists at all at a hub that he does not run, or only at a hub of his own. Text: say in RCL-007 what the key does and does not do; if a control against the hub is wanted, the machine must hold a key of the operator and check a signature on a standing permission (for example for one API key and a period), which is a new design.

### F8. The network of a command: the public internet is open, the limits are not measured, and "unless the owner set otherwise" is a setting with no text

Severity: Medium. It decides what a command can reach and where data can go, and none of it is built.

What it is.

- RCL-013 takes the network of W2: "The network of a job: the public internet only." (W2 Section 4.1). For a `build` that is needed to get dependencies. For a command of an agent it means: the machine of the operator, from its address, can send to each address of the internet for up to 3,600 seconds an item (scans, mail, mining, an attack on a third party, in the name of the owner's connection), and each byte that the command can read can leave (F15: a private checkout).
- The limits are not measured: W2 Section 5, "The network limits of a job | Linux; on a Mac, a rule of the firewall needs an administrator". On Docker Desktop a container usually reaches services of the host through a gateway name; on a Mac a `native` command as the build user has the network of the computer unless a firewall rule exists. The local ports of the operator are then open to it: a published port of an application (K10 Section 6.4.4 publishes on `127.0.0.1` by default as the protection), a model server, a debug port of a browser or of a runtime, the local port that `anima-link device` "keeps … for the desktop client" (K8 Section 1). A debug port is code as the operator. Inferred from general knowledge; not measured by anyone.
- "unless the owner of the machine set otherwise at the machine" (RCL-013) is a new setting. It has no name, no row in the proposed K10 text, no rule of who writes it and no sentence of its effect. W2 has no such exception.
- W2 also says "not the origin of the hub". RCL-013 leaves it out of its list.

Where. RCL-013; W2 Sections 4.1 and 5.

Effect, and who can cause it. An agent, a key holder, a person who controls the hub. Effect: misuse of the owner's connection; data out; on a Mac, the local services of the operator.

Mitigation now. The rule of RCL-013 as words.

What would correct it. Owner: is the public internet open to a command by default, or closed with a list of hosts at the machine (registries, the forge)? Text: delete the exception or write it as a setting of the administrator with its sentence; add "not the origin of the hub"; make the measured network rule a condition (F2).

### F11. The audit entry: 4 KiB of a command line of up to 1 MiB, a secret that stays one year, and no entry in the log of the machine

Severity: Medium. The record is the only thing the text offers after the fact (RCL-024), and it can be made to show a harmless line.

What it is.

- The proposed Section 6.5: "`arguments` has at most 256 members of at most 4,096 bytes each" — about 1 MiB. RCL-012: "The command line itself is kept in the audit entry, redacted, with at most 4 KiB." A caller puts 4 KiB of harmless text first (`sh`, `-c`, a long comment) and the real command after it. The entry then shows the comment. Verified numbers; the attack is inferred.
- LOG-007: "No log must contain a password, a private key, an access token or the content of a session." K10 Section 7 says of the same redaction: "a best effort". An agent writes tokens into arguments as a habit (`curl -H "Authorization: Bearer …"`, `git clone https://user:token@…`). LOG-006: the hub "must not change or delete an audit log entry before the end of the retention period" (one year, LOG-025). So a secret that no pattern finds stays one year, cannot be removed, and at a hosted hub it is on a server of a different person. OD-RCL3 names this weakness in one line; it does not name the retention or LOG-007, and Section 2 does not list LOG-007 (F22).
- The output (2 MiB) has the same property and is not in an open decision. A command can print what it reads.
- The log of the machine. K10 Section 9 lists what the machine writes for a job ("the id, the type, the kind, the project, the repository, the commit and the recipe…") so that the owner can check the hub (SF-012); the container types got their own list (SF-032). The proposal gives `command` none. The `actor` member is "for the log of the machine", but nothing says that the program and the arguments are written there. Without it, the only record of what ran is at the hub, which a person who controls the hub can leave empty.
- For a session: nothing (RCL-018, RCL-024). The text says so.

Where. RCL-005, RCL-012, RCL-018, OD-RCL3, OD-RCL4; proposed K10 Section 6.5; K10 Sections 7 and 9; LOG-006, LOG-007.

Effect, and who can cause it. An agent or a key holder (hides the command; leaves a secret in the log); a person who controls the hub (reads the secrets; no record at the machine).

Mitigation now. Redaction two times; the 4 KiB limit; OD-RCL3 and OD-RCL4 are open.

What would correct it. Owner: OD-RCL3 with the facts above. Text: the entry keeps the SHA-256 of the whole item and its length in each case, and says "cut" when it is cut; one size for the item and the entry (lower the limit of the arguments to what the entry keeps, for example 8 KiB in all); the log of the machine records the program, the arguments or their SHA-256, the tree and the actor, before the start, where the job and the hub cannot change it; state who can read a stored command line and an output (a read key?).

### F12. The limit of 30 starts does not limit: 30 commands of one hour hold each slot, "caller" has no definition, queued work counts later

Severity: Medium. The limit is the only brake on fan-out, and one caller can still hold each machine all the time.

What it is. RCL-004: "at most 30 routed starts for each caller in each hour." RCL-013: "The longest time of a `command` item is 3,600 seconds." K10 Section 4: "A machine has one slot". So one caller keeps 30 machines busy without end, and no `build` and no `runner` gets a slot there. ROU-010 adds "and at most the free slots of the machines", and "Queued work counts at its start": by those words a caller can put more than 30 items in the queue, and they count only when they start. "Caller" is not defined: an API key, the operator behind the key, or the MCP client. `anima mcp` "uses the same context and token as the command line" (MCP-001), so an agent can act with the login token of the operator; is its limit then shared with the operator? A holder of several keys has 30 for each. A session counts as "one start" (RCL-004) with "the idle and the longest time of a session of the operator"; RA-037 can be "set … off", and I found no longest time of a session in the specification (not found is not proof).

Where. RCL-004, RCL-013; ROU-010; K10 Section 4.

Effect, and who can cause it. An agent in a loop, a key holder. Effect: no build and no runner on the machines; 30 machines run hostile commands in one hour from one injected instruction.

Mitigation now. RCL-004.

What would correct it. Owner: the numbers. A proposal: a limit of commands that run at one time for each key (for example 2) and for each machine (1, already), a lower default time (for example 600 seconds) with the hour only by a setting, a count at the request and not at the start, a limit of queued commands, and a definition of "caller" that sums the keys of one operator. Add a stop of all commands of one key in one act.

### F14. The output of a command instructs the agent that has a shell; a stolen daemon token can take commands and give false results

Severity: Medium. SF-010 accepted that a log carries instructions when the agent could do little; now the reader of the log can start the next command.

What it is.

- RCL-014: "The hub gives the caller the exit code and the log". SF-010's state line: "The risk of instructions in a log stays for each agent that reads logs (MCP-008)." The mark "not trusted" is a request to the model. With a command line, the loop closes: the output of a command on machine A (a web page that it fetched, the output of tests of a repository, a file name) tells the agent to run a command on machine B. RCL-023 names the first step of an injection, not this loop.
- A contradiction in the accepted texts that the proposal leans on: K10 Section 6.4.12 says "No MCP tool returns a log (MCP-004)"; K10 Section 7 says "An MCP tool gives a log only with the mark of text that is not trusted (SF-010)"; the MCP plan names a read tool `anima_job_log`. RCL-014 needs the second reading.
- A stolen daemon token, or a second enrolled device, sends a claim with `types` that name `command` and a large `free.memoryBytes`. ROU-002: among the machines that ask within 20 seconds, "the one with the most free memory gets it". The hub cannot check those numbers. That device then gets each command that names no machine: the program, the arguments (with their secrets, F11) and the tree; and it reports an exit code 0 and a log of its choice. The agent then acts on a false result ("the tests pass") or on injected text. This is SF-006 and SF-011 for a new kind of content. Inferred from K10 Section 4 and ROU-002.

Where. RCL-012, RCL-014; ROU-002; K10 Sections 4, 6.4.12 and 7.

Effect, and who can cause it. The author of text that a command prints; a holder of a stolen daemon token; a second machine at the hub. Effect: commands on other machines; false results; the text of commands.

Mitigation now. The mark of SF-010; two redactions; RCL-007 for a machine of a different person.

What would correct it. Owner: accept the loop, or keep the click for a command that follows the read of a log in one agent turn (hard to enforce), or lower the fan-out (F12). Text: correct the contradiction on logs; a `command` item with no named machine goes only to a device that the operator named for commands at the hub (as the device list of R-186), until W7.

### F17. A command line removes the click rules that R-196 made absolute

Severity: Medium. A first-hand ruling says "always", and this function is a way around it that the table of changed texts does not name.

What it is. AXT-021: "An AI agent must not install Hermes through MCP in any MCP state, `allow` included: the install always needs the click of the operator (R-196, OD-AXT5)." ROU-003: routing "never installs a tool (AXT-021)". With a session the agent installs what it wants as the operator. With a command in the isolation it cannot install on the host, if the isolation holds. Section 2 lists R-196 only for the click of the Chat page (CHAT-041, OD-RCL5). The same applies to each other rule of the kind "only a person does X at a device" where X is a command: INT-003 "must not set or clear a device credential" (a session can change the password or the keys of the login), and MCP-005 in effect (a session can change the host key of the device).

Where. Proposal Section 2; AXT-021; INT-003.

Effect, and who can cause it. An agent, a key holder, through a session. Effect: installs and changes that the operator ruled need his click.

Mitigation now. None for a session. OD-RCL5 recommends the click for the Chat page only.

What would correct it. The owner decides with F1. If the session stays, the ruling must say that OD-AXT5 no longer holds against an agent with a session.

### F23. The gate of Section 9 is weaker than ROU-014

Severity: Note.

What it is. ROU-014: "No routed start ships before the security review RT7 passes with no open finding of severity high." ROU-016 gives a session and a command line "its own part of that review". Section 9 and R-NEW-agentshell say: nothing is merged "before the operator has decided its findings". "Decided" includes "accepted". The ruling itself is "relayed, not first-hand", from the label of a card ("B: agents too, no question"), and its number is a placeholder. For a decision of this weight the owner may want to state it first-hand, with F1 to F5 before him, and to say if a High finding that he accepts still blocks the merge as ROU-014 says.

## 4. Findings that are corrections of the text

### F7. A command shares its isolation with `build` and `runner`: what it leaves can reach a later job that has a secret

Severity: Medium. If one thing survives a command, the next job on that machine can be a build with a project secret.

What it is. RCL-008 uses "the isolation of a job": on a Mac "the build user (PRO-022)", and W2 Section 4.3 says "One Mac has one build user". W2 deletes at each end "the job folder, the temporary folder of that user and the keychain of the job", and ends "each process of the build user". It does not say that the home folder of the build user is made new. The second layer, a sandbox profile that "limits the writes to the job folder", uses a program that its own manual marks "DEPRECATED" (W2 Section 3.1) and was not measured on a build user. If a command can write one file that a later job reads (a start file of a shell, a tool folder, a cache of a compiler, a folder that each user can write), the next `build` of `default_branch` runs it with `project_secret` (K10 Section 6.1). On Linux the same question is the store of the rootless runtime of the one job user and each cache between jobs. Inferred; W2 has no measurement for or against.

A second effect is certain by the text: K10 Section 9, "If one thing cannot be deleted, the machine sets itself `unclean`, claims no item". The owner accepted that for a pull request of a fork, with "the approval rule of GitHub" as a control (R-187). A command needs no approval: each caller can take each machine with the setting on out of work until "the operator at the machine" acts.

Where. RCL-008; W2 Sections 4.1 and 4.3; K10 Sections 6.1 and 9.

Effect, and who can cause it. An agent, a key holder. Effect: a secret of a project or a changed build result on a later job; machines out of work.

Mitigation now. The clean-up of K10 Section 9; the W2 design.

What would correct it. Text: a `command` item never shares a user, a home folder, a cache or a runtime store with a job that can get a secret: a second job user for commands, or a home folder that is made new for each item; a test in W4 that nothing of a command is readable by the next job. Then the owner decides if the `unclean` effect is accepted for commands (it was accepted for a narrower case).

### F9. Section 8 states as fact two things that the W2 record says are not checked

Severity: Medium. The owner will read Section 8 as the state of his Mac.

What it is. Section 8: the build user "is not in the group of the operator and has no way to the socket of Docker Desktop". W2 Section 4.3: "The build user must thus not be in the group of the operator; to check with an administrator", and it explains why the file modes "alone are not a sure wall": the home folder has the mode `drwxr-x---` and "the usual group of a standard user of macOS is that group". W2 Section 3.1 measured that a connection to the socket "Worked" from a sandbox on the user of the operator. No one measured it from a second user. Section 8 also says a container command runs "with no socket of the engine in it" (measured, W2 Section 3.3) but does not say that its network was not limited in that test (F8).

What would correct it. Write each sentence of Section 8 with its state: measured, designed and not measured, or not built.

### F10. The text does not say what file system a container command has; the inventory is of the tools of the host

Severity: Medium. The gap pushes the build toward running on the host, which is the unsafe option of F3.

What it is. The example is `"program": "cargo"`. For the kind `container` nothing says which image the container has. K10 Section 4: an `Item` never holds "an image name". Routing picks a machine because of its tools: ROU-004, "studio: has cargo 1.83 and 22 GB free", from the inventory of the host (ROU-002, CAP-023). A container "with no capability" does not have the tools of the host unless folders of the host are mounted into it, and W2 Section 4.1 says "The job folder is the only path of the computer in the job." So one of these must be true and the text says none: a fixed image of the daemon (then the inventory has no meaning for a command); the caller names an image (a new power, with the weaknesses of SF-031 and SF-041: a pull from each address); folders of the host are mounted (a hole in the isolation, and a way to change the tools of the host if a mount is not read-only); or a command on Linux runs with no container. Inferred from the texts; the texts are verified.

What would correct it. Decide and write it. The safe forms: a fixed list of images of the daemon by digest, named by a code and not by an address; or read-only mounts of a fixed list of tool folders that the owner names at the machine.

### F13. The queue: a command starts when the owner sets the switch on later; a refusal takes the machine out of work; two names for one cause

Severity: Medium.

What it is.

- RCL-014: "A `command` item waits in the queue as each job does (ROU-007)". ROU-007: "the work waits in the queue until a machine fits"; the time after which queued work is refused is open (OD-ROU7; recommendation 24 hours). A command that an agent asked for at 10:00 can start at night, when the conversation is over, the tree has changed and nobody watches. ROU-014 names this for the review ("queued work that starts later than the caller expects"); the proposal gives it no rule. If an item waits for a machine whose setting is off, it starts when the owner sets the setting on: his first act of permission runs the commands that were queued before it.
- RCL-006 has no member of the claim (F5, point 7). The daemon "refuses an item of the type `command`". K10 Section 11: "the device refused or lost 5 items in 1 hour and the hub gives it none for 1 hour (SF-011)". With a pin (ROU-006: "A caller can name a machine"), a caller sends 5 commands to a machine whose setting is off; the machine then gets no build for an hour. Inferred from the three texts.
- RCL-006 says the cause is `not_at_machine`; the proposed Section 5 text adds `command_not_permitted`. One of them.
- The start of a session "is refused at once" (RCL-019) and a command waits. The two words of one tool behave in two ways; the caller must be told which.

What would correct it. A `command` item has a short life in the queue (minutes, a parameter of the caller with a low maximum) and is then final; the hub never queues one for a machine whose claim says the setting is off; a refusal of a `command` item for the setting does not count for `WORK_OFF`; one cause code.

### F15. The working tree: the text says a command has no repository and then gives it one

Severity: Medium.

What it is.

- Section 2 (row SF-003): "A command line has no repository. The list is no control for it." RCL-010: "The item names a working tree: a repository of a project and a commit or a branch". K10 Section 3: "The daemon refuses an item of a repository that is not in the list". The text must say that the list applies when `tree` is not null. As written it reads as an exemption.
- RCL-010 says "a commit or a branch"; the proposed Section 6.5 says "`tree` is null or names a commit". A branch is resolved by whom, and when? With the queue (F13) it can be a different commit than the caller saw.
- RCL-011: "`secrets` is empty". K10 Section 6.1: "A `build` of a private repository gets the `repository_token`". So a command cannot get a private tree, or the daemon gets it by a way that the text does not name.
- If a private tree is given: the command has the public internet (F8). Each holder of a write key can then copy the source of each private repository of each project to an address of its choice. I did not check if a key can read repository content by a different way today; I believe not, since the token of the forge is given only to a machine. Owner decision once the text is clear.
- `origin` (SF-009) and the scratch ref (JOB-017 to JOB-020) are not named for a tree.

What would correct it. Write the four rules: the list of R-185 applies to a tree; a commit only, 40 characters; how a private tree is fetched and that the token never enters the command's isolation; a private tree only with the network closed, or by a decision of the owner.

### F16. A session needs no tool of Anima to be used by a key; the start command; the idle limit

Severity: Medium.

What it is.

- RCL-020: "How an agent uses a terminal is not in Anima today"; OD-RCL6: "No, not now". This reads as if a session were of no use to an agent without a new tool. The design specification Section 6.3 step 4: "The hub sends the stream to the client through the WebSocket tunnel: terminal bytes for SSH". A program with the key speaks that tunnel directly; an agent with a shell on its own computer writes such a program in a minute. OD-RCL6 limits only an agent that keeps to the MCP tools.
- Section 6.3.1 has the setting "Command to run at start | `command` | Login shell". If the new function takes it, or uses the value stored for the device, a "session" is a command line as the operator with no isolation, and Section 5 has no purpose. The text must say that the routed function takes no command and what it does with a stored one.
- RA-037: "Only keystrokes and paste count as input." An agent that sends a key each minute never meets the idle limit, and the limit can be "set … off". RCL-017's "the longest time of a session of the operator": I found no such time.
- RCL-017 says "the operator sees it in the list of sessions and can end it" (RA-034). Good, and it should also say that the list shows the actor kind and the key.
- `openFiles` (the file panel, RA-032) is in the list of INT-003 and is not named in the proposal: is it open in a routed session?

What would correct it. Write each of these. A fixed longest time for a routed session (for example 15 minutes) that no input extends.

### F18. A command can run two times or continue with no hub

Severity: Low.

What it is. K10 Section 7 has one rule for a `build` (it "ends when the time of the lease, 90 seconds, has passed … with no report") and one for a `runner` (it "continues"), and USE-007 with ROU-008 say "the hub routes the work again". The proposal says which applies to `command` nowhere. A command that is not safe to repeat (a deploy script, a migration) then runs a second time on a different machine, or continues for an hour on a machine that the hub can no longer stop. SF-005 was this for builds.

What would correct it. `command` ends at the end of the lease as a `build` does, and the hub makes no second attempt (as for a container item: "The operator asks again").

### F19. The hub selects the kind; `actor` has the value `hub`; PRO-024

Severity: Low.

What it is. K10 Section 3: "An item cannot ask for less isolation." The item names `kind`, and on a Mac `native` is the weaker one (PRO-024: "a native job has no container around it"; PRO-011: "Use a native job only when the work needs macOS directly"). For a command, who selects `native`, and by which rule? RCL-008 does not name the Mac setting of PRO-024 as a second condition. The proposed Section 6.5 gives `actor` the value `hub`; K10 Section 6.4.12 says "The hub makes no container item with no actor in this version". A command that the hub starts by itself has no purpose in the proposal.

What would correct it. `native` for a command only when the caller asks for it and the two settings are on; delete the value `hub`.

### F20. The arguments are visible on the machine and in the lists of the hub; two daemons share one build user

Severity: Low.

What it is. K10 Section 8 forbids a secret on a "command line that a different user of the machine can read". The arguments of a command are in the list of processes of the machine for its whole run; on most systems each local user can read them. Inferred, general knowledge. The command line is also in the item, so it is in the list of what runs (PRO-052) and in each read of the job: the text does not say if a read key sees it. K10 Section 6.4.1: "A computer can have a daemon for each of two accounts (R-206, R-208)"; W2: "One Mac has one build user, so it runs one native job at a time." Two daemons do not share a slot, so two accounts and two hubs can run commands as one user at one time and read each other's job folders.

What would correct it. Say on the screen and in the tool description that an argument is not a place for a secret; state who reads a stored command line; one build user for each daemon account, or a lock for the build user across daemons.

### F21. The environment, the search path and the input of the command are not written

Severity: Low.

What it is. Section 6.5 has no environment member, which is good. It does not say what the environment is. K10 Section 6.4.1 has the model sentence for `docker`: "an environment that holds only what the program sets". A daemon that passes its own environment to the job passes what its unit holds. `program` is "at most 256 bytes": a name that is found on which search path, or a path, absolute or in the tree? The standard input is not named (it must be closed). `exitCode` is a number that the job selects; give it a range (0 to 255) so that it stays a "fixed codes and numbers" member in the sense of K10 Section 7.

### F22. The texts that Section 2 missed

Severity: Low. Each is verified by reading the sentence.

| Text | The sentence | Why it is changed |
| --- | --- | --- |
| K10 Section 4, last line of `Item` | "An `Item` never holds a command, an argument, a path, an address, an image name or an environment variable." | The proposed Section 6.5 puts a program and arguments in `parameters`. The table names Sections 3 and 6 only, and the proposed sentences name Sections 1, 3, 5 and 6.5 |
| K10 Section 6, the version 0.3 line | "There is no type that runs a command which the hub names (USE-006, JOB-015)." | The table quotes only the version 0.4 sentence |
| K10 Section 3, the row of the repositories; Section 7 (what the log of each type is; the lease rules); Section 9 (the log of the machine); Section 4 (the member of the claim) | — | F5, F11, F15, F18 |
| LOG-007 | "No log must contain a password, a private key, an access token or the content of a session." | RCL-012 keeps the command line and the output. ROU-015 cites LOG-007 for "no command text, no output" |
| jobs.md, "Decision" | "There is no 'run this command' type." and "The hub still cannot name a command." These are the limits that R-160 says "stay" | The table names R-160, USE-006 and JOB-015, not this list |
| jobs.md, "Rules for each task" | "Never: a command line or a session from a routed start" | A rule for each agent that builds RT3 to RT6 |
| jobs.md JOB-016 | "The steps come from the configuration of the project (PRO-009), not from the caller." | The caller names the steps |
| jobs.md ROU-003, AXT-021 (R-196, OD-AXT5) | "never installs a tool"; "in any MCP state, `allow` included" | F17 |
| R-243 | "no tool takes a command line" | — |
| R-115, the item that R-160 did not replace | "Not taken: … anything that changes a device (Section 8)" | A session and a command change a device. I read this item only inside the entry R-113/R-115 of `rulings.md`; check its reach |
| PRO-011, PRO-024 | F19 | — |
| R-236, R-231 | Accepted for "each program that runs as the user of the daemon" and for a stolen token | Their reach grows with F3 and F1 |
| INT-003, the parts after `startSession` | `openFiles`; "must not set or clear a device credential" | F1, F16 |
| MET-013 of the design specification | "It allows only two calls" | Already false since K10; not caused by this proposal |

Also: R-NEW-agentshell lists what it amends and does not list CHAT-041, LOG-007 or AXT-021.

### F24. "No shell" and "`startSession` stays closed" are true and protect nothing

Severity: Note.

RCL-009 ("The daemon starts the program with that list and with no shell") stops an injection through the daemon's own handling; it is correct and should stay. It limits no caller, as the same requirement says. RCL-017 and the row of INT-003 say "`startSession` itself is not opened to a key". A new function that opens the same session for a key changes INT-003 in effect; the ruling should say "INT-003 is changed", not that it stays.

## 5. What I found sound

- The proposal separates the two mechanisms (Section 3) and says the worst case of each in plain words (RCL-021 to RCL-024, Section 8, the proposed K10 paragraph).
- A list of arguments and no shell text; no environment member; no secret and no token of Anima for a `command` item (RCL-011); a refusal of an item that names a purpose.
- The hub never accepts a host key for a routed session (RCL-016).
- The recommendations of the open decisions go to the safe side each time: the setting off (OD-RCL1), the isolation (OD-RCL2), no session before a record (OD-RCL4), the click on the Chat page (OD-RCL5), the command line first (OD-RCL7).
- Section 9 puts this review and the decision of the owner before code.

## 6. Counts

High: 5 (F1 to F5). Medium: 12 (F6 to F17). Low: 5 (F18 to F22). Note: 2 (F23, F24).

For the owner: F1, F2, F3, F4, F5, F6, F8, F11, F12, F14, F17 and F23, and F7 and F15 after their text is corrected. Corrections of the text only: F9, F10, F13, F16, F18 to F22, F24.
