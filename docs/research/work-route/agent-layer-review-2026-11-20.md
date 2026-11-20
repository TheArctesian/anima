# Record: the security read of the agent layer proposal (2026-11-20)

| Item | Value |
| --- | --- |
| Document | The report of the separate reviewer of [agent-layer.md](../../proposals/agent-layer.md), version 1 |
| Date | 2026-11-20 |
| Language | The words of the reviewer, not changed, apart from two names of a file and of a program flag that are said in words |
| Kind | Record. It adds no requirement (PLN-010). |

The session that wrote the proposal gave a separate reviewer agent the text, the documents that it stands on and the positions of an attacker to take, and no opinion. This is its report. The findings F1 to F18 are the entries SF-NEW-agl01 to SF-NEW-agl18 of [security-findings.md](../../security-findings.md), in the same order. The review of [routed-command-lines.md](../../proposals/routed-command-lines.md) is [command-lines-review-2026-11-19.md](command-lines-review-2026-11-19.md); the second read of that review takes the two texts together.

Reviewer: read-only; no file changed. "NV" = not verified: it rests on my knowledge of Claude Code, Codex, tmux or systemd, not on a file or a run. No code exists for this function, so each finding is about the text.

## 1. Findings

| # | Severity | About | Finding | Scenario | What would close it | Who decides |
| --- | --- | --- | --- | --- | --- | --- |
| F1 | High | AGL-026, OD-AGL7 | OD-AGL7 recommends the unit "on a hub of Anima.ing, where the hub has no SSH login"; AGL-026 makes the hub's SSH terminal the only way to reach a kept session. Each repair gives the vendor a login. RA-008 is a promise of the party that is the attacker. | The hosted hub gets a login "for attach". Vendor or session thief attaches and types `! cmd` in Claude Code (NV): a shell as the person, plain text through the hub (R-196 OD-AXT7). | Decide: no attach through a hosted hub, or state in words that it is a vendor shell. | Owner |
| F2 | High | AGL-025, AGL-028 | Nothing at the machine authenticates "the operator". The request arrives from the hub; no transport, signature, nonce, expiry or count limit is named. The only audit is written by the hub. The transport changes K10 Section 3, MET-002 and MET-013; Section 8 lists none. | A hub controller starts or replays agent programs in each known repository on each machine with the setting on, kills running sessions, and shows "Stopped". | A start signed by a key of the operator's client that the machine checks, single-use, with a log at the machine. | Owner |
| F3 | High | AGL-023 | The interface from daemon to unit is not written. tmux has no restricted protocol: whoever reaches the socket runs any command. The Linux unit has `PrivateTmp=yes` and `ProtectHome=read-only`, so "unit files: no change" is probably false. On a Mac the daemon has no sandbox (`sandbox.rs`; the plist), so "with its protections" is empty there. | A daemon fault or a hub-driven input reaches `tmux new-session '<anything>'` outside the sandbox that AU-5 produced. | A narrow, written interface (one verb, program index, repository index) that the unit checks itself. | Owner |
| F4 | High | AGL-027, AGL-003 | "No AI agent starts, stops or types into a kept session" has no mechanism. Each program of the user can `tmux send-keys`. The `anima` login token is in the key store (CMD-012). Session names are guessable. | An injected agent approves its own permission prompt; runs `anima agents start` as the "signed-in person"; makes `anima-7` with its own program, and the operator attaches to a false prompt. | Say it is a rule for Anima's interfaces only; give the server a private socket; verify the pane's process before a row says "kept". | Owner |
| F5 | Medium | AGL-024 | "By its name" is a search-path lookup in a unit environment. An implementer who uses a login shell brings in aliases (a flag that skips each permission question), `~/.tmux.conf` hooks, the SSH agent and the keychain. Repository configuration (hooks, MCP servers, allow rules; NV) runs at start. | Hub controller picks the repository whose configuration file of Claude Code has a `SessionStart` hook; it runs as the person with no prompt typed. | Absolute path recorded at the machine, fixed environment, `tmux -f /dev/null`, own socket. | Owner |
| F6 | Medium | AGL-011, AGL-025 | "A setting that the hub cannot set" is kept by a user-level daemon, so each program of that user can write it (K10 SF-007 asks for "where no job and no other user can write it"). Where the switch is shown is not said. | An agent sets "Keep agent sessions" on; F2 then applies. | State the store and who can write it; accept or refuse same-user writes in words. | Owner |
| F7 | Medium | AGL-010, AGL-018, AGL-020 | AGL-010 is false as written: a signal also makes rows, causes a push and sends an alert to a third party. No rate limit, no rule for which of 64 rows is dropped, the writer sets the time, and the identifier's alphabet is not repeated from K8. | A script flips Waiting/Working: endless alerts. It writes `UserPromptSubmit` after a real `Notification`: a hidden wait. 64 false rows evict the real ones. A daemon token thief does the same at the hub. | Daemon stamps time, validates identifiers, limits rows and alerts per hour, and prefers rows that match a session file. | Text |
| F8 | Medium | AGL-008 against AGL-015, AGL-019 | The signal drops the folder, so the exclusion can be decided only after the usage reader (15 minutes) reads the session file. Send early and an excluded repository leaks times, state and an alert; send late and the waiting state is useless. | A person starts Claude Code in an excluded client repository; the hosted hub gets the row and the alert first. | Hook resolves the exclusion locally and writes only a verdict, or no row before the verdict. | Text; the correction may raise a choice |
| F9 | Medium | AGL-015 to AGL-018, OD-AGL9 | A hosted hub gets a timeline, accurate to seconds, of when the person prompts, waits and is away, joined by session identifier to model, tokens and repository. "No history" cannot be checked by the person. Hub-side removal is not defined (AXT-052 keeps rows). The recommendation of OD-AGL9 is in no requirement. | The vendor logs each batch. | A requirement that carries OD-AGL9, a hub retention rule, and a row identifier that is not the usage identifier. | Owner |
| F10 | Medium | AGL-012, AGL-013 | Codex `notify` holds one program (NV), so "keep each entry" cannot hold. Not written: absolute path or bare name in the entry; a symlinked config; the mode of the copy (configs hold tokens); home or repository file; comments lost; removal at uninstall. | Backup written `0644`: a second user reads API keys. Bare `anima-daemon`: a planted binary gets each prompt object. | Absolute checked path, no link followed, copy `0600`, home file only, a rule for an existing `notify`. | Text |
| F11 | Medium | AGL-008, AGL-009 | No rule that the hook prints nothing. Claude Code puts hook stdout of some events into the model context (NV). Codex passes its JSON, with message text, as an argument (NV), visible in the process list. `LimitCORE=0` is in the unit, not in the hook process; error tracking (R-241) is not excluded. | A parse error echoes notification text into a crash report; a second user reads Codex text with `ps`. | No output, no log, no core, no crash report; bounded read; record the argument exposure in AL0. | Text |
| F12 | Medium | AGL-020, AGL-021 | The third party gets the machine name (often a person's name), the link (the hub address) and the time of each wait. NOT-007 (template) conflicts with "fixed words"; NOT-003 selects channels per check and this is no check; NOT-005 resend conflicts with "one". R-244 said the needs-you inbox, "not a second alert path". | A public ntfy topic shows when the person works. | Own channel selection, no template, no link for third-party channels. | Owner |
| F13 | Medium | AGL-017 | The alphabet stops injection, not disclosure: `fix/acme-breach-ticket-812` passes whole. Reading `HEAD` is a new read outside the AU-5 limits. | The customer name reaches the hub and each agent that calls `anima_agents_list`. | Option A, or a keyed hash. | Owner (OD-AGL3) |
| F14 | Low | AGL-009 | No size limit, no rule for a link or a pipe in place of the file, no rule for truncation against an append. On Linux the file is in the one writable folder, beside the token. | A pipe blocks each hook past 100 ms; a truncation loses a `Notification`. | Open as `token_file.rs` does; a size cap; rename rotation. | Text |
| F15 | Low | AGL-005 to AGL-007 | `Notification` is not only "waiting" (NV); no event ends Waiting when a permission is answered; "cannot find the process" has no method. "Unknown, active" is a seventh state against AXT-048. | Rows stay Waiting during work; the operator stops trusting them. | AL0 result as a condition of AGL-020. | Text |
| F16 | Low | AGL-014 against AGL-015 | Program version and "hooks installed" leave the device but are not in the "only" list. Reading a version means running a program of the user's path, against CAP-022 and the fixed path in `command.rs`. | Two builds differ. | Add the fields; say how the version is read. | Text |
| F17 | Low | Section 7, Section 10 | The gate AL9 covers kept sessions only and High findings only. AL3 to AL7 have no gate; R-258 asks for all findings decided. | Rows and alerts ship to a hosted hub unreviewed. | One gate before AL3, all findings decided. | Owner |
| F18 | Low | AGL-023 | A user unit ends at logout unless linger is set (NV); linger is a system setting. | "Stays" is false on a server after SSH logout. | State it. | Text |

## 2. Findings of the routed command line review that apply here

- SF-090: an agent with the person's login defeats each machine-side setting, and the key store gives the `anima` login token (F4, F6).
- SF-092: running as the user of the daemon gives the daemon token; the kept agent is that user.
- SF-094: one switch at the machine that is not a control at the machine (F6).
- SF-095: a hub that the machine owner does not control starts a program with no list (F2).
- SF-100: no entry in the log of the machine (F2).
- SF-101: no real limit on starts (F2).
- SF-103: a stolen daemon token gives false results (F7).
- SF-105: a session needs no Anima tool to be used; `tmux` and the CLI suffice (F4).
- SF-106: the click rules of R-196 are bypassed by a self-approved prompt (F4).
- SF-109: arguments visible to a second user (F11).
- SF-110: environment and search path not written (F5).
- SF-111: texts that the change list missed (F2, F12).
- SF-112: the gate is weaker than the ruling (F17).
- SF-113: "no MCP tool for a start" is true and protects nothing (F4).

## 3. Who decides

Operator decision: F1 (attach on a hosted hub, or none), F2 (accept hub-carried starts, or require a signed start), F3 (accept an unsandboxed start primitive beside the daemon), F4 (accepted risk of same-user programs), F5 (may a kept agent use the person's environment and prompt-free configuration), F6 (who may set the switches), F9 (hosted hub on or off; retention), F12 (what a third party may get), F13 (OD-AGL3), F17 (the gate).

Text corrections only: F7, F8 (a design contradiction; the correction may raise a choice), F10, F11, F14, F15, F16, F18. Also the false statements: AGL-010 "can change only the state of a row", AGL-023 "with its protections" on a Mac, and Section 8 "unit files: no change" and its missing K10, MET-002, MET-013, NOT-003 and NOT-007 entries.

## 4. What is sound

- The hook gives its signal through a file, with no listener and no network; MET-020 holds.
- AGL-005: session files never give Working, Waiting or Done, and no alert comes from `files`.
- AGL-011 and OD-AGL4: the daemon and the hub do not write another program's configuration by themselves; off by default.
- AGL-016 and the fixed alert text with no question, path or branch.
- AGL-024 and OD-AGL8: no argument, prompt, path or environment from the hub.
- The tmux server is not a child of the daemon; the unit facts agree with the unit files I read.
- AGL-029: no automatic restart after a reboot.
- AGL-013 respects AIS-024.
- Section 11 and AL0 say honestly that no hook event was recorded and no tmux was run.
- Section 10 already names most of the right subjects; the faults are in the requirements, which do not yet answer them.
