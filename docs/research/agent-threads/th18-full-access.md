# Record: Full access of Claude Code and Codex in a trusted and in a not trusted folder (task TH18, 2026-11-20)

| Field | Value |
|---|---|
| Document | TH18 record: the default mode of a new thread (Full access), measured with the vendor programs |
| Date | 2026-11-20 |
| Versions measured | `claude` 2.1.311 (/opt/homebrew/bin/claude), `codex` codex-cli 0.170.0 (/opt/homebrew/bin/codex), macOS 27.3.0, arm64 |
| Kind | Record. It adds no requirement (PLN-010). It answers point H17 of the third security read and point 12 of Section 6.2.1 of [agent-threads.md](../../proposals/agent-threads.md). |

How to read this record. Each statement marked "measured" comes from a run on this Mac on 2026-11-20. There were 8 agent runs: 6 of `claude` and 2 good runs of `codex app-server` (plus 2 runs of `codex` that stopped at the start, which are a result too, see 3.2). The driver programs are the TH10 drivers ([th10-driving-claude-code-and-codex.md](th10-driving-claude-code-and-codex.md)), changed only to take the folder from a variable. The scripts and the raw logs are in the notes folder of the session that made this record (`th18/`), with the account fields replaced by "[left out]".

No vendor file was changed. The trust of a folder is in a file of the vendor program in the home folder (`~/.claude.json` for Claude Code, `~/.codex/config.toml` for Codex). This task did not write those files. To measure a trusted folder, it used a path that the person had trusted before and that did not exist on the disk any more: it made the folder for the run and removed it after the run. Only the names of the trusted paths were read from the two files, with a filter that printed nothing else.

## 1. Summary

| Question | Claude Code (`--permission-mode bypassPermissions`) | Codex (`thread/start` with `sandbox: danger-full-access`, `approvalPolicy: never`) |
|---|---|---|
| Does a tool call ask for an approval? | No. No `can_use_tool` request came for `Bash` or `Write`, in both folders. | No. No server request came, in both folders. |
| Does a question of the agent still come? | Yes. `AskUserQuestion` came as a `can_use_tool` control request on the pipe, in both folders, and the answer reached the agent. | Not measured (no question tool was used by the agent). |
| Does a project deny rule still hold? | Yes. `Bash(mkdir:*)` in `.claude/settings.json` was refused with `"source":"config"`, in a trusted and in a not trusted folder. | Does not apply (Codex has no deny rule of this form). |
| Does a project allow rule hold? | In Full access every call is allowed, so it does not show. In `manual` mode (extra runs): yes in the trusted folder, no in the not trusted folder. | Does not apply. |
| Does a project hook run? | Yes. The `PreToolUse` hook of `.claude/settings.json` ran for each `Bash` call (also the refused one), in both folders. | Not measured. In a not trusted folder Codex says that project hooks are off (3.2). |
| Is the project configuration read? | Yes, in both folders (the deny rule and the hook). Only the allow rules wait for trust. | Only in a trusted folder. In a not trusted folder Codex sends the notice `configWarning` and leaves it out. |
| Do the parameters of the runner win over the project configuration? | Not measured (the project file had no mode). | Yes. A project file with `approval_policy = "on-request"` and `sandbox_mode = "read-only"` did not change the answer of `thread/start` (`approvalPolicy: never`, `sandbox: dangerFullAccess`). |
| Can the agent write outside the folder? | Yes. `touch` of a file in a different folder ran with no question. | Yes. `touch` of a file in a different folder ran. |
| Can the agent use the network? | Not tried. | Yes. `curl https://example.com` gave `200`. |
| What does the program ask at a first start in a folder? | Nothing on the pipe. A not trusted folder gives one line on standard error (2.2). There was no trust question and no question about the bypass mode. | Nothing on the pipe in the good runs. A not trusted folder gives the notice `configWarning` and a line on standard error (3.2). |
| Does the trust of a parent folder count? | No. A folder below the home folder, which the person trusted, was not trusted (2.2). | Not measured. |

## 2. Claude Code

### 2.1 The command

Each run used the command line of the adapter that THR-020 names, with Full access:

```
claude -p --input-format stream-json --output-format stream-json --verbose --include-partial-messages
  --model sonnet --strict-mcp-config --permission-prompt-tool stdio --permission-mode bypassPermissions
```

The driver removed each variable whose name starts with `CLAUDE`, `ANTHROPIC_`, `MCP_`, `CODEX` or `OPENAI`, and then set `CLAUDE_CODE_EMIT_SESSION_STATE_EVENTS=1` and `CLAUDE_CODE_ENABLE_ASK_USER_QUESTION_TOOL=1`. The prompt went on standard input as one `user` line. Each folder held `README.txt` and `.claude/settings.json`:

```
{"permissions":{"allow":["Bash(touch:*)"],"deny":["Bash(mkdir:*)"]},
 "hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"echo pre-hook >> hook.log"}]}]}}
```

The prompt asked for five steps: `mkdir d1`; `touch t1.txt`; write `note.txt`; `touch` of a file outside the folder; a question with `AskUserQuestion`. The driver allowed each control request and gave the first choice as the answer.

### 2.2 The folders

| Run | Folder | Trusted? | Measured |
|---|---|---|---|
| c-full-trusted | A new folder below the home folder. The home folder is trusted in `~/.claude.json`. | No, as the result showed | Standard error: `Ignoring 1 permissions.allow entry from .claude/settings.json: this workspace has not been trusted. …` |
| c-full-trusted2 | A path that the person trusted before, made for the run and removed after it | Yes | No line on standard error |
| c-full-untrusted | A new folder in `/private/tmp` | No | The same line on standard error as c-full-trusted |

So the trust of Claude Code is for the exact folder (a folder that is not in a git repository was used). A runner cannot assume that a folder below a trusted folder is trusted.

### 2.3 What happened in Full access (c-full-trusted2 and c-full-untrusted: the same in both)

| Step | Approval request? | Result | `tool_result_meta.permission_decision` |
|---|---|---|---|
| `mkdir d1` | No | Refused: `Permission to use Bash with command mkdir d1 has been denied.` A `system`/`permission_denied` event came first. The refusal is in `permission_denials` of the `result` event. | `{"decision":"reject","source":"config","reason_type":"subcommandResults"}` |
| `touch t1.txt` | No | Ran | `{"decision":"accept","source":"config","reason_type":"mode"}` |
| Write `note.txt` | No | Written | `{"decision":"accept","source":"config","reason_type":"mode"}` |
| `touch` outside the folder | No | Ran; the file was made | `{"decision":"accept","source":"config","reason_type":"mode"}` |
| `AskUserQuestion` | Yes: `control_request` `can_use_tool`, `"requires_user_interaction": true` | The answer reached the agent | `{"decision":"accept","source":"user_temporary"}` |

- The `init` event said `"permissionMode":"bypassPermissions"`.
- The state events came as in TH10: `running`, `requires_action` before the question, `running` after the answer, `idle` after the `result` event.
- `hook.log` held three lines in each folder: the hook ran for each of the three `Bash` calls, also for the refused `mkdir`.
- The turn ended with `result`/`success`, `terminal_reason: completed`, about 7 to 8 seconds after the prompt. The process ended with exit code 0 about 0.6 seconds after standard input was closed. No process of its group was left.
- The `reason_type` `mode` tells that the mode allowed the call; a runner can show "allowed by Full access" from it.

### 2.4 The same folders in `manual` mode (c-manual-trusted and c-manual-untrusted)

These two runs measure the allow rule of a project, which TH10 could not measure in a trusted folder.

| Step | Trusted folder | Not trusted folder |
|---|---|---|
| `mkdir d1` | Refused by the deny rule, no request | Refused by the deny rule, no request |
| `touch t1.txt` | No request; `"source":"config","reason_type":"subcommandResults"`: the allow rule answered | A `can_use_tool` request came; the allow rule was ignored (the line on standard error) |
| Write `note.txt` | Request | Request |
| `touch` outside the folder | Request, although the allow rule names `touch`: a path outside the folder is asked | Request |
| `AskUserQuestion` | Request | Request |

## 3. Codex

### 3.1 The command

`codex app-server`, started in the folder of the thread, with the methods of TH10. The driver sent:

```
→ initialize, initialized
→ {"method":"thread/start","params":{"cwd":"<folder>","sandbox":"danger-full-access","approvalPolicy":"never","approvalsReviewer":"user"}}
→ turn/start with four steps: mkdir d1; create note.txt with apply_patch; touch a file outside the folder; curl https://example.com
```

Each folder held `README.txt` and `.codex/config.toml`. The trusted folder was a path that is in `~/.codex/config.toml` with `trust_level = "trusted"`, made for the run and removed after it. The not trusted folder was a new folder in `/private/tmp`.

### 3.2 The first two runs: a project file that Codex does not accept

The first project file had `approval_policy = "untrusted"` (the value that TH10 used as a parameter) and `sandbox_mode = "read-only"`.

- Trusted folder (x-full-trusted): `codex app-server` wrote `Error: approval_policy = "untrusted" is no longer supported; remove this setting` to standard error 0.2 seconds after the start and ended with exit code 1. It did not answer `initialize`.
- Not trusted folder (x-full-untrusted): the server started, sent `configWarning` (`Project-local config, hooks, and exec policies are disabled in the following folders until the project is trusted, but skills still load.`), and answered `thread/start` with the error `-32600` `failed to load configuration: approval_policy = "untrusted" is no longer supported; remove this setting`.

What this shows: Codex reads the project file of a trusted folder when the server starts, and a value that it does not accept stops the server. In the not trusted folder it still read the file at `thread/start` and refused the start, although it said that the project configuration is off. So a project file can stop a thread in each folder, and the runner must show the text of standard error and of the error answer when the server ends or refuses at the start.

### 3.3 The good runs (x-full-trusted2 and x-full-untrusted2)

The project file had `approval_policy = "on-request"` and `sandbox_mode = "read-only"`.

| Point | Trusted folder | Not trusted folder |
|---|---|---|
| `configWarning` | No | Yes, as in 3.2 |
| Answer of `thread/start` | `approvalPolicy: never`, `sandbox: {"type":"dangerFullAccess"}` | The same |
| Server requests (approvals, questions) | None | None |
| `mkdir d1` | Ran (`exitCode` 0) | Ran |
| `note.txt` by `apply_patch` | Written (`fileChange`, `completed`) | Written |
| `touch` outside the folder | Ran; the file was made | Ran; the file was made |
| `curl https://example.com` | `200` (`item/commandExecution/outputDelta` with `"200"`) | `200` |
| End of the turn | `turn/completed`, `status: completed`, after about 13 seconds | The same, after about 13 seconds |

- The parameters of `thread/start` won over the project file in both folders.
- Codex sent `account/updated` and `account/rateLimits/updated` without a request, as in TH10. Their content is left out of this record.
- At its start the server ran `git ls-remote https://github.com/…` as a child (seen in the process list of x-full-untrusted), and the MCP servers of the person's Codex configuration made a request that failed with an authorization error (`rmcp::transport::worker … AuthRequired`). This is the configuration of the person, not the prompt.
- The helper programs that the server started (`node_repl` of the ChatGPT application) had process groups of their own, not the group of the server. A stop that sends a signal to the group of the server only does not reach them; the walk of the process tree of THR-049 does.
- After standard input was closed the server ended with exit code 0 within 0.1 seconds, and no process of the tree was left after 1.5 seconds.

## 4. What this means for the runner

- Full access is `--permission-mode bypassPermissions` for Claude Code and `sandbox: danger-full-access` with `approvalPolicy: never` on `thread/start` for Codex. Neither asks for a tool call. Both write outside the folder of the thread, and Codex used the network. "Anima stops nothing that the agent does with the tools of the machine" (THR-033) is the measured fact.
- A question of the agent still comes in Full access for Claude Code, so the composer must take it (THR-117, TH15).
- The project settings of Claude Code partly apply in each folder: deny rules and hooks always; allow rules only in a trusted folder. Codex reads the project file only in a trusted folder, but a bad project file stopped the start in both. The notice of THR-046 can say this from the facts: the standard error line of Claude Code and the `configWarning` of Codex tell the runner that the folder is not trusted.
- The runner writes no trust state and no file of the vendor program. A not trusted folder works in Full access with no question, so the runner does not need trust to start. What the person loses in a not trusted folder is only the project allow rules of Claude Code (no effect in Full access) and the project configuration of Codex.
- The closed list of flags of the Claude Code adapter (H17): `-p`, `--input-format stream-json`, `--output-format stream-json`, `--verbose`, `--include-partial-messages`, `--permission-prompt-tool stdio`, `--permission-mode <mode>`, `--model <alias>`, `--effort <level>`, `--session-id <uuid>` or `--resume <uuid>`, `--fork-session`. The runner never passes `--settings`, `--setting-sources`, `--allowedTools`, `--disallowedTools`, `--add-dir`, `--mcp-config` or `--strict-mcp-config` (the last two until THR-127 is built: with `--strict-mcp-config` the MCP servers of the person are off, which THR-091 does not permit).
- The parameters of the Codex adapter for Full access are on `thread/start` and `thread/resume`; the server is started with no flag but `app-server`.
- The runner must show standard error and an error answer at a start: a project file stopped the Codex server at its start (3.2).
- The stop of THR-049 must walk the process tree: the helpers of Codex are in their own groups.

## 5. Not measured

- A question of Codex (`item/tool/requestUserInput`) in Full access.
- A hook of Codex in a trusted folder.
- Full access on Linux (`pop-os`).
- A folder inside a git repository whose root the person trusted (the trust of Claude Code by the root of a repository).
- The network for Claude Code in Full access.
- Whether `bypassPermissions` asks a question the first time in an interactive terminal; only `-p` was used, and it asked nothing.

## 6. Clean-up

- The two trusted folders were removed after their runs. The files that the runs wrote outside the folders are in the scratch folder of the session.
- No `claude` or `codex` process of the runs was left (each driver checked the tree at its end).
- The vendor programs wrote their own session files under the home folder as usual (6 Claude Code sessions, 2 Codex threads). They were not removed.
