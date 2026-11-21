# Record: how Claude Code and Codex can be driven by their own programs (task TH10, 2026-11-20)

| Field | Value |
|---|---|
| Document | TH10 record: driving Claude Code and Codex with the vendors' own command-line programs |
| Date | 2026-11-20 |
| Versions measured | `claude` 2.1.311 (/opt/homebrew/bin/claude), `codex` codex-cli 0.170.0 (/opt/homebrew/bin/codex), macOS 27.3.0, arm64 |
| Kind | Record. It adds no requirement (PLN-010). |

How to read this record. Every statement marked "measured" comes from a run on this Mac on 2026-11-20. There were 16 agent runs: 12 of `claude`, 2 of `codex exec`, 2 of `codex app-server`. Each ran alone, with a time limit, in the scratch folder `work/`. `<th10>` below means the folder `<th10>`. Samples are trimmed. Account fields are written as "[left out]".

## 1. Summary

| Capability | Claude Code via `claude -p` stream JSON | Codex via `exec` | Codex via `app-server` |
|---|---|---|---|
| Start a turn | Yes. Prompt as argument, or a JSON line on stdin. | Yes. Prompt as argument. | Yes. `thread/start` then `turn/start`. |
| Stream text | Yes. `stream_event` with `text_delta`, needs `--include-partial-messages`. | No. Only whole items (`item.completed`). | Yes. `item/agentMessage/delta`. |
| Several turns in one process | Yes. With `--input-format stream-json`; 4 turns measured in one process. | No. One turn, then exit. | Yes. 2 turns per process measured. |
| Steer while a turn runs | Partly. A message sent mid-turn is queued and runs as the next turn. Injection into a running turn was not seen. | No. | Yes. `turn/steer` was accepted during a running command. Its effect on the answer was not seen (turn was then interrupted). |
| Interrupt | Yes. SIGINT, or `control_request` `interrupt` on stdin. The command's processes were gone after both. | Not measured. | Partly. `turn/interrupt` ends the turn with status `interrupted`, but the `sleep` process was still alive 2 seconds later. |
| Resume | Yes. `--resume <id>` in a new process, also after an interrupt. | Yes. `codex exec resume <id>`. | Yes. `thread/resume` in a second process. |
| Approvals for a command | Yes. MCP tool or stdio control request. Read-only commands such as `echo` never ask. | No. It never asks. | Yes. `item/commandExecution/requestApproval`. |
| Approvals for a file change | Yes. Same two ways (tool name `Write`). | No. It never asks; the read-only sandbox just refuses. | Yes. `item/fileChange/requestApproval`. |
| Questions | Yes, on the stdio way: `can_use_tool` for `AskUserQuestion` (measured with one environment variable set). Not measured on the MCP way. | No. | Not measured. The schema has `item/tool/requestUserInput`. |
| Fork | Yes. `--resume <id> --fork-session` gives a new session ID with the old history. | Not measured. `codex exec fork` exists in help. | Yes. `thread/fork` returns a new thread with `forkedFromId`. |
| Model and effort choice | Yes. `--model sonnet` and `--effort low` accepted. | Partly. `-m` exists in help; not measured. | Partly. `effort: "low"` on `turn/start` accepted; `model` field exists; not measured. |
| Project settings answer before the approval tool | Partly. Project **deny** rules answer first. Project **allow** rules are ignored in an untrusted folder. Flag `--settings` allow rules answer first. `--setting-sources user` turns project settings off. | Not measured. | Not measured. |
| Claim 1: `--permission-prompt-tool stdio` gives control requests | Yes. `control_request` / `can_use_tool` on stdout; `control_response` on stdin. `initialize` and `interrupt` also work. | Does not apply. | Does not apply. |
| Claim 2: `CLAUDE_CODE_EMIT_SESSION_STATE_EVENTS=1` | Yes. `system` / `session_state_changed` with `state` = `running`, `requires_action`, `idle`. | Does not apply. | Does not apply. |
| Claim 3: `--session-id <uuid>` | Yes. The ID is used, the session file has that name, `--resume` continues it. | Does not apply. | Does not apply. |
| Claim 4: which notification ends a turn | Does not apply. | Does not apply. | Yes. `turn/completed` with `turn.status`. `completed` and `interrupted` were seen. `failed` is in the schema, not seen. |

## 2. Claude Code

Common setup. Every run used `cwd = <th10>/work`. The test driver removed all environment variables that start with `CLAUDE`, `ANTHROPIC_`, `MCP_`, `CODEX`, `OPENAI` before the start, so the child used the owner's normal login. All runs but the first used `--model sonnet --permission-mode manual --strict-mcp-config`.

Why `--permission-mode manual` is given: in run c1 (no flag) the `init` event said `"permissionMode": "auto"`. That comes from the owner's user settings. With `--permission-mode manual` the `init` event says `"permissionMode": "default"`. A runner must pass the mode itself and not trust the default.

### 2.1 One-shot stream JSON (run c1)

Command:

```
claude -p --output-format stream-json --verbose --model sonnet --strict-mcp-config "Read the file README.txt and tell me in one short sentence what it says."
```

Result: exit code 0 after about 3 seconds. One JSON object per line on stdout. Order of events:

| # | `type` / `subtype` | Main fields |
|---|---|---|
| 1 | `system` / `init` | `session_id`, `cwd`, `tools`, `mcp_servers`, `model`, `permissionMode`, `claude_code_version`, `agents`, `skills`, `plugins`, `capabilities`, `uuid`, `apiKeySource` [left out] |
| 2 | `assistant` | `message.content[]` with `{"type":"tool_use","id":"toolu_…","name":"Read","input":{…}}`, `session_id`, `uuid`, `parent_tool_use_id`, `request_id` |
| 3 | `rate_limit_event` | `rate_limit_info` [left out] |
| 4 | `user` | `message.content[]` with `{"type":"tool_result","tool_use_id":"toolu_…","content":"…"}`, `tool_use_result`, `tool_result_meta` |
| 5 | `assistant` | `message.content[]` with `{"type":"text","text":"README.txt just says \"th10 test folder\"."}` |
| 6 | `result` / `success` | `is_error:false`, `num_turns:2`, `result:"…"`, `stop_reason:"end_turn"`, `terminal_reason:"completed"`, `session_id`, `permission_denials:[]`, `duration_ms`, `total_cost_usd`, `usage` |

The end of a turn is the `result` event. The session ID is the field `session_id`; it is on every event, first on `system`/`init`.

Other `system` subtypes seen in later runs: `status` (`"status":"requesting"`), `thinking_tokens`, `task_started` and `task_notification` (for a long Bash command), `session_state_changed` (only with the variable in 2b.2).

### 2.2 Streaming input, several messages in one process (run c2)

Command:

```
claude -p --input-format stream-json --output-format stream-json --verbose --include-partial-messages --replay-user-messages --model sonnet --permission-mode manual --strict-mcp-config
```

Lines accepted on stdin (both forms worked):

```
{"type":"user","message":{"role":"user","content":"What is 2+2? Do not use tools. Answer with the number only."}}
{"type":"user","message":{"role":"user","content":[{"type":"text","text":"Multiply that by 3. Do not use tools. Number only."}]}}
```

Measured:

- Four user messages went to one live process. Each got its own `result` event (`result_index` 0, 1, 2, 3). Answers: `4`, `12`, the numbers 1 to 60, `Paris`. The second answer shows the context is kept.
- A new `system`/`init` event comes at the start of every turn, with the same `session_id`.
- With `--replay-user-messages` each user message is sent back as a `user` event with `"isReplay": true`.
- The fourth message was sent while turn 3 was still writing. It did not change turn 3. It ran as turn 4 right after the `result` of turn 3. So a message during a turn is queued. This was a text-only turn; a turn with tool calls was not tried.
- The process stays alive between turns. It ended with exit code 0 about 0.6 seconds after stdin was closed.

### 2.3 Partial messages (run c2)

Yes. With `--include-partial-messages` these come before the full `assistant` event:

```
{"type":"stream_event","event":{"type":"message_start",…}}
{"type":"stream_event","event":{"type":"content_block_start","content_block":{"type":"text","text":""}}}
{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"1\n2\n3\n4"}}}
…
{"type":"stream_event","event":{"type":"content_block_stop"}}
{"type":"stream_event","event":{"type":"message_delta","delta":{"stop_reason":"end_turn",…}}}
{"type":"stream_event","event":{"type":"message_stop"}}
```

### 2.4 Resume (runs c3 and c11)

```
claude -p --output-format stream-json --verbose --model sonnet --permission-mode manual --strict-mcp-config --resume 9d0c1c50-7b03-4550-8a13-b1b5f2a42228 "What is the secret word for this chat, …"
```

Result: the `init` and `result` events carry the same session ID as the first process. The answer named the secret word (`KIWI`) given in the first process. So a second process continues the conversation. This session had been stopped by SIGINT before (see 2.7), so resume also works after an interrupt.

Fork (run c11): `--resume 2529a82d-… --fork-session --effort low`. The `init` event had a new `session_id` (`e246bf78-…`), and the answer (`d1`) came from the old history.

### 2.5 Approvals through an MCP tool (run c4)

Files: the server is `<th10>/scripts/perm_mcp.py` (Python, stdio, one tool `approve`). The config is `<th10>/mcp.json`. `--strict-mcp-config` exists and was used.

```
claude -p --output-format stream-json --verbose --model sonnet --permission-mode manual --session-id <uuid> --mcp-config <th10>/mcp.json --strict-mcp-config --permission-prompt-tool mcp__th10perm__approve "Do these three steps … 1) Bash tool: echo hello  2) Bash tool: mkdir d1  3) Write tool: create the file note.txt containing the word ok. …"
```

What the tool received (MCP `tools/call` params, measured):

```
{"name":"approve","arguments":{"tool_name":"Bash","input":{"command":"mkdir d1","description":"Create directory d1"},"tool_use_id":"toolu_01C8m8…"},"_meta":{"claudecode/toolUseId":"toolu_01C8m8…","progressToken":2}}
{"name":"approve","arguments":{"tool_name":"Write","input":{"file_path":"<th10>/work/note.txt","content":"ok"},"tool_use_id":"toolu_01GWzu…"},"_meta":{…}}
```

So the request has the tool name, the full input and the tool-use ID. It has no separate request ID and no suggestions.

| Step | Was the tool called? | Outcome |
|---|---|---|
| `echo hello` | No | Ran at once. `tool_result_meta`: `{"permission_decision":{"decision":"accept","source":"config","reason_type":"subcommandResults"}}`. Claude Code treats it as safe by itself. |
| `mkdir d1` | Yes | Reply allow. Command ran. `permission_decision`: `{"decision":"accept","source":"user_temporary"}`. |
| Write `note.txt` | Yes | Reply deny after an 8 second sleep. File was not written. |

Reply format. The MCP result must be one text block whose text is a JSON string:

```
allow: {"content":[{"type":"text","text":"{\"behavior\":\"allow\",\"updatedInput\":{\"command\":\"mkdir d1\",\"description\":\"Create directory d1\"}}"}]}
deny:  {"content":[{"type":"text","text":"{\"behavior\":\"deny\",\"message\":\"TH10 test: denied by the approval tool\"}"}]}
```

After allow the tool runs and a normal `user`/`tool_result` follows. After deny the agent gets a `tool_result` with `"is_error": true` and the deny message as content; `tool_result_meta` says `"non_execution_kind":"permission-rule"`, `"permission_decision":{"decision":"reject","source":"user_reject"}`. The agent went on, wrote its final text and did not retry. The turn ended with `result`/`success`. (In run c8 a config denial was also listed in `permission_denials` of the `result` event.)

A late reply blocks the turn. The `Write` tool_use came at 3.7 s. The tool slept 8 s. No event came until the `tool_result` at 11.7 s. A reply that never comes was not tried; the turn waited for the whole delay that was tried.

### 2.6 Does configuration answer before the approval tool? (runs c6, c7, c8)

All three runs had the MCP approval tool set to allow everything.

| Run | Setup | `echo hello` | `mkdir` | Other |
|---|---|---|---|---|
| c6 | `work/.claude/settings.json` = `{"permissions":{"allow":["Bash(echo:*)","Bash(mkdir:*)"]}}` | Tool not called (`source: config`, same as without any rule) | Tool **was** called | stderr warning, see below |
| c7 | Same file, plus `--setting-sources user` | Tool not called | Tool was called | No stderr warning |
| c8 | Project file = allow `Bash(echo:*)`, deny `Bash(mkdir:*)`; plus `--settings '{"permissions":{"allow":["Bash(touch:*)"]}}'` | not asked | **Denied without calling the tool**: `"Permission to use Bash with command mkdir d8 has been denied."`, `"decision":"reject","source":"config"` | `touch t8.txt` ran without calling the tool (`source: config`). `rmdir d1` (no rule) called the tool. |

The stderr line in c6:

```
Ignoring 2 permissions.allow entries from .claude/settings.json: this workspace has not been trusted. Run Claude Code interactively here once and accept the trust dialog, or set projects["<th10>/work"].hasTrustDialogAccepted: true in <home>/.claude.json.
```

What this shows:

- In `-p` mode, project **allow** rules are ignored when the folder has not been trusted. Only a stderr line says so. The trust flag is in a home-folder file, so it was not changed in this task. The trusted case was therefore not measured.
- Project **deny** rules are applied even in an untrusted folder, and they answer before the approval tool.
- Allow rules from the `--settings` flag answer before the approval tool.
- Built-in safe commands (`echo`) never reach the approval tool, with or without rules.
- `--setting-sources user` loads only user settings: the trust warning is gone. Because allow rules were already ignored, the visible result did not change. The effect of this flag on a project deny rule was not measured.

What `claude --help` says:

| Flag | Help text (short) |
|---|---|
| `--permission-mode <mode>` | Choices: `acceptEdits`, `auto`, `bypassPermissions`, `manual`, `dontAsk`, `plan`. |
| `--permission-prompts <target>` | Who answers prompts with `--print`: `host` (the SDK host or `--permission-prompt-tool`) or `none` (anything that would prompt is denied). Default `host`. |
| `--allowedTools`, `--allowed-tools <tools...>` | Comma or space-separated list of tool names to allow (e.g. `"Bash(git *) Edit"`). |
| `--disallowedTools`, `--disallowed-tools <tools...>` | Same list form, to deny. |
| `--add-dir <directories...>` | Additional directories to allow tool access to. |
| `--model <model>` | Alias (`fable`, `opus`, `sonnet`) or full model name. |
| `--effort <level>` | `low`, `medium`, `high`, `xhigh`, `max`. |
| `--fork-session` | When resuming, create a new session ID (use with `--resume` or `--continue`). |
| `--session-id <uuid>` | Use a specific session ID (must be a valid UUID). |
| `--setting-sources <sources>` | Comma-separated list of setting sources to load (`user`, `project`, `local`). |
| `--settings <file-or-json>` | Extra settings from a file or a JSON string. |
| `--strict-mcp-config` | Only use MCP servers from `--mcp-config`. |

`--permission-prompt-tool` itself is not listed in `claude --help`. It is only named inside the text of `--permission-prompts`. It works (2.5 and 2b.1).

### 2.7 Interrupt (runs c9, c9b, c9c, c10)

A plain `sleep 30` cannot be used: Claude Code refused it itself (`"Blocked: standalone sleep 30. …"`), before any approval. The test command was `python3 -c "import time; time.sleep(30)"`, allowed through the approval tool.

(a) SIGINT to the `claude` process, 5 seconds into the command (run c9c):

```
{"type":"system","subtype":"task_started","task_id":"…","tool_use_id":"toolu_…","description":"Sleep for 30 seconds","task_type":"local_bash","is_backgrounded":false}
   — SIGINT —
{"type":"system","subtype":"task_notification","task_id":"…","tool_use_id":"toolu_…","status":"stopped"}
{"type":"user","message":{"content":[{"type":"tool_result","is_error":true,"content":"The user doesn't want to proceed with this tool use. …"}]},"tool_use_result":"User rejected tool use","tool_result_meta":[{"non_execution_kind":"user-rejected"}]}
{"type":"user","message":{"content":[{"type":"text","text":"[Request interrupted by user for tool use]"}]}}
{"type":"result","subtype":"error_during_execution","is_error":true,"result":null,"terminal_reason":"aborted_tools","stop_reason":"tool_use"}
```

- The process then exits with code **0** about 0.6 seconds later. The exit code does not show the interrupt; the `result` event does.
- Before SIGINT the process tree was: `claude` → the Python MCP server (same process group) and `/bin/zsh -c …` → `python3` (its own process group). One second after SIGINT none of them was left.
- The session was resumable (2.4). One thing to know: the saved history says the tool use "was rejected", although the command had run for 5 seconds. On resume the model said the command "never executed".
- SIGINT sent just as the approval was answered (run c9b) gave the same events with `terminal_reason: "aborted_tools"`. SIGINT while the model was still answering (run c9) gave `"[Request interrupted by user]"` and `terminal_reason: "aborted_streaming"`.

(b) Control message on stdin (run c10): see 2b.1. Same events as SIGINT, but the process stays alive and takes the next message.

### 2.8 Questions from the agent (run c10)

Measured once, on the stdio way, with `CLAUDE_CODE_ENABLE_ASK_USER_QUESTION_TOOL=1` set. In the earlier runs (variable removed by the driver) the `init` tool list did not contain `AskUserQuestion`. Whether the variable is what adds the tool was not tested apart.

The question arrives as an approval request:

```
{"type":"control_request","request_id":"bd9a39bb-…","request":{"subtype":"can_use_tool","tool_name":"AskUserQuestion","display_name":"AskUserQuestion","input":{"questions":[{"question":"Do you prefer red or blue?","header":"Color","options":[{"label":"Red","description":"Choose red"},{"label":"Blue","description":"Choose blue"}],"multiSelect":false}]},"tool_use_id":"toolu_…","requires_user_interaction":true}}
```

Reply that worked (answers are added to the input, keyed by the question text):

```
{"type":"control_response","response":{"subtype":"success","request_id":"bd9a39bb-…","response":{"behavior":"allow","updatedInput":{"questions":[…],"answers":{"Do you prefer red or blue?":"Red"}}}}}
```

The agent then got `"Your questions have been answered: \"Do you prefer red or blue?\"=\"Red\". …"` and answered "You chose red." Questions through the MCP approval tool: not measured.

## 2b. Claims of other tools, tested

### 2b.1 `--permission-prompt-tool stdio` (run c10) — the claim holds

Command (no MCP server at all):

```
claude -p --input-format stream-json --output-format stream-json --verbose --model sonnet --permission-mode manual --strict-mcp-config --permission-prompt-tool stdio
```

`initialize` exists. Sent on stdin, answered in 0.2 s:

```
in:  {"type":"control_request","request_id":"init-1","request":{"subtype":"initialize","hooks":null}}
out: {"type":"control_response","response":{"subtype":"success","request_id":"init-1","response":{"commands":[…102],"agents":[…],"models":[…],"output_style":"default","pid":63679,"claude_code_version":"…","current_permission_mode":"default","session_state":"idle","capabilities":["ui_surface_v1","mcp_call_result_not_for_model_v1"],"account":"[left out]",…},"pending_permission_requests":[],"pending_user_dialog_requests":[]}}
```

Each entry of `models` has `value`, `resolvedModel`, `displayName`, `supportsEffort`, `supportedEffortLevels`, `supportsAdaptiveThinking`, `supportsAutoMode`.

Approval request on stdout (Bash, then Write):

```
{"type":"control_request","request_id":"b01d8ead-…","request":{"subtype":"can_use_tool","tool_name":"Bash","display_name":"Bash","input":{"command":"mkdir d10","description":"Create directory d10"},"description":"Create directory d10","permission_suggestions":[{"type":"addRules","rules":[{"toolName":"Bash","ruleContent":"mkdir d10 *"}],"behavior":"allow","destination":"localSettings"},{"type":"addDirectories","directories":["<th10>/work"],"destination":"session"},{"type":"setMode","mode":"acceptEdits","destination":"session"}],"blocked_path":"<th10>/work/d10","tool_use_id":"toolu_01Va2D…"}}
{"type":"control_request","request_id":"c6ef02d6-…","request":{"subtype":"can_use_tool","tool_name":"Write","display_name":"Write","input":{"file_path":"<th10>/work/note10.txt","content":"ok"},"description":"note10.txt","permission_suggestions":[{"type":"setMode","mode":"acceptEdits","destination":"session"}],"tool_use_id":"toolu_015iww…"}}
```

Exact reply lines on stdin that worked:

```
allow: {"type":"control_response","response":{"subtype":"success","request_id":"b01d8ead-…","response":{"behavior":"allow","updatedInput":{"command":"mkdir d10","description":"Create directory d10"}}}}
deny:  {"type":"control_response","response":{"subtype":"success","request_id":"c6ef02d6-…","response":{"behavior":"deny","message":"TH10 test: denied on stdin"}}}
```

After allow: `mkdir d10` ran (`"source":"user_temporary"`). After deny: `tool_result` with `is_error:true` and the message; the file was not written; the turn ended with `result`/`success` and the denial listed in `permission_denials`.

`interrupt` exists. Sent 6 seconds into the Python sleep:

```
in:  {"type":"control_request","request_id":"int-1","request":{"subtype":"interrupt"}}
out: {"type":"control_response","response":{"subtype":"success","request_id":"int-1","response":{"still_queued":[]}}}
out: {"type":"system","subtype":"task_notification",…,"status":"stopped"}
out: {"type":"user",… tool_result "The user doesn't want to proceed with this tool use. …"}
out: {"type":"user",… "[Request interrupted by user for tool use]"}
out: {"type":"result","subtype":"error_during_execution","terminal_reason":"aborted_tools",…}
```

One second later only the `claude` process was left; the `zsh` and `python3` children were gone. The same process then ran a third turn. So this interrupt stops the turn and keeps the process.

Comparison of the two ways:

| Point | MCP tool (`mcp__server__tool`) | `stdio` control requests |
|---|---|---|
| Extra parts | A stdio MCP server program, a config file, `--mcp-config` | None. Same pipe as the events. Needs `--input-format stream-json`. |
| Request ID | Only `tool_use_id` (and the MCP call ID) | Own `request_id`, plus `tool_use_id` |
| Content | `tool_name`, `input`, `tool_use_id` | Same, plus `display_name`, `description`, `permission_suggestions`, `blocked_path`, `decision_reason` |
| "Allow for the session" help | None in the request | `permission_suggestions` (`addRules`, `addDirectories`, `setMode`, each with a `destination`). Sending a suggestion back was not tried. |
| Questions | Not measured | Yes (2.8) |
| Interrupt, initialize | Not part of it (use SIGINT) | Yes, on the same pipe |
| Listed in `claude --help` | The flag is named only inside `--permission-prompts` | The word `stdio` is not in the help at all |

The stdio way is simpler and gives more. Its risk is that it is not described in `--help`.

### 2b.2 `CLAUDE_CODE_EMIT_SESSION_STATE_EVENTS=1` (run c10) — the claim holds

With the variable set, 14 extra events came in a three-turn run. Without it (run c2, same input format) none came.

```
{"type":"system","subtype":"session_state_changed","state":"running","uuid":"…","session_id":"…"}
{"type":"system","subtype":"session_state_changed","state":"requires_action","uuid":"…","session_id":"…"}
{"type":"system","subtype":"session_state_changed","state":"idle","uuid":"…","session_id":"…"}
```

Order seen: `running` when a user message is taken; `requires_action` just before each `can_use_tool` request; `running` again after the reply; `idle` right after each `result` event, also after an interrupted turn. Fields: `type`, `subtype`, `state`, `uuid`, `session_id`. Note: run c10 also had the stdio approval way and the question variable set; the comparison run c2 had neither.

### 2b.3 `--session-id <uuid>` (runs c4, c9c, c3, c11) — the claim holds

- The ID was made with `uuidgen` (lower case) before the start. The `init` and `result` events carried exactly that ID.
- A file named `<that uuid>.jsonl` appeared in the Claude projects folder for `work/` under the home folder. Its content was not read.
- `--resume <that uuid>` continued it (c3). `--resume <that uuid> --fork-session` made a new ID from it (c11).

### 2b.4 Codex `app-server`: end of a turn — the claim holds

The end of a turn is the notification `turn/completed`. Its `params.turn.status` tells the outcome. Seen: `"completed"` (4 times) and `"interrupted"` (2 times). The schema lists `completed`, `interrupted`, `failed`, `inProgress`, and a `turn.error` object (`message`, `codexErrorInfo`, `additionalDetails`) "associated with a failed or interrupted turn". No failed turn was produced, so `failed` was not seen; `error` was `null` on the interrupted turns. A `thread/status/changed` with `{"type":"idle"}` comes just before `turn/completed`. Samples are in 3.2.

## 3. Codex

Note for all Codex runs: the owner's Codex configuration starts its own MCP servers (seen as child processes and as `rmcp` lines on stderr, one of them an authorization error from a remote MCP host). This comes from the configuration, not from the test prompts. A runner should expect this start-up noise.

### 3.1 `codex exec --json` (runs x1, x2)

```
codex exec --json -s read-only -C <th10>/work "The secret word for this chat is MANGO. Run the command `echo hello` and tell me what it printed. Then try to create the file x1.txt containing ok with a shell command and tell me in one line whether that worked."
```

Events on stdout, one JSON object per line (all of them):

```
{"type":"thread.started","thread_id":"01a1fb67-41a8-7f71-b1fd-6ef43663491c"}
{"type":"turn.started"}
{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"I’ll run `echo hello`, then try to write …"}}
{"type":"item.started","item":{"id":"item_1","type":"command_execution","command":"/bin/zsh -lc 'pwd -P'","aggregated_output":"","exit_code":null,"status":"in_progress"}}
{"type":"item.completed","item":{"id":"item_1","type":"command_execution","command":"/bin/zsh -lc 'pwd -P'","aggregated_output":"<th10>/work\n","exit_code":0,"status":"completed"}}
{"type":"item.started","item":{"id":"item_2","type":"command_execution","command":"/bin/zsh -lc 'echo hello'",…}}
{"type":"item.completed","item":{"id":"item_2","type":"command_execution",…,"aggregated_output":"hello\n","exit_code":0,"status":"completed"}}
{"type":"item.completed","item":{"id":"item_3","type":"agent_message","text":"`echo hello` printed `hello`; creating `x1.txt` containing `ok` failed: operation not permitted."}}
{"type":"turn.completed","usage":{"input_tokens":49603,"output_tokens":131,…}}
```

- The session ID is `thread_id` in `thread.started`. The end is `turn.completed`, then exit code 0 about 2 seconds later.
- There are no text deltas. A message comes whole in `item.completed`.
- There was no approval event and no question event. The read-only sandbox refused the write and the agent just reported it. `codex exec --help` has no `--ask-for-approval` flag (the top-level `codex --help` has it, with `on-request` and `never`). So exec mode never asks.
- stdin: with a prompt argument and stdin closed, stderr says `Reading additional input from stdin...` and the run goes on.

Resume:

```
codex exec resume --json -c 'sandbox_mode="read-only"' 01a1fb67-41a8-7f71-b1fd-6ef43663491c "What is the secret word for this chat? Answer with one word. Do not run any command."
```

Result: `thread.started` with the same `thread_id`, `turn.started`, `item.completed` (`agent_message`, text `MANGO`), `turn.completed`. So it continues. `codex exec resume` has no `-s` flag; the sandbox was set with `-c sandbox_mode=…`.

### 3.2 `codex app-server` (runs x3, x4)

How the method names were found: `codex app-server generate-json-schema --out <dir>` writes the schema. It lists 109 client requests, 10 server requests, 86 server notifications and 1 client notification (`initialized`). The names used below are from that schema and were then seen on the wire.

Transport: `codex app-server` (default `stdio://`), one JSON-RPC object per line on stdin and stdout. Client program: `<th10>/scripts/appserver.py`.

Sequence that worked (run x3):

```
→ {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientInfo":{"name":"th10-probe","version":"0.1"}}}
← {"id":1,"result":{"userAgent":"[left out]","codexHome":"<home>/.codex","platformFamily":"unix","platformOs":"macos"}}
→ {"jsonrpc":"2.0","method":"initialized"}
→ {"jsonrpc":"2.0","id":2,"method":"thread/start","params":{"cwd":"<th10>/work","sandbox":"workspace-write","approvalPolicy":"untrusted","approvalsReviewer":"user"}}
← {"id":2,"result":{"thread":{"id":"01a1fb68-8d7f-7300-adac-d57f64010dbd","sessionId":"…","forkedFromId":null,"ephemeral":false,"path":"…","cwd":"…",…},"model":"gpt-6.1-sol","modelProvider":"openai","approvalPolicy":"untrusted","approvalsReviewer":"user","sandbox":{"type":"workspaceWrite","writableRoots":[],"networkAccess":false,…},"reasoningEffort":"medium",…}}
← {"method":"thread/started","params":{"thread":{…}}}
→ {"jsonrpc":"2.0","id":3,"method":"turn/start","params":{"threadId":"01a1fb68-…","input":[{"type":"text","text":"… 1) mkdir x3a 2) mkdir x3b 3) create x3.txt with apply_patch …"}],"effort":"low"}}
← {"id":3,"result":{"turn":{"id":"01a1fb68-8e21-…","status":"inProgress","items":[],"error":null,…}}}
```

Notifications seen during a turn (name and count in run x3): `thread/status/changed` 14, `turn/started` 2, `item/started` 12, `item/completed` 11, `item/agentMessage/delta` 68, `turn/diff/updated` 3, `serverRequest/resolved` 5, `thread/tokenUsage/updated` 7, `turn/completed` 2, `mcpServer/startupStatus/updated` 12, `remoteControl/status/changed` 1, `account/updated` and `account/rateLimits/updated` (params [left out]). In run x4 also `thread/goal/cleared`. No account method was called.

Item types seen in `item/started` / `item/completed`: `userMessage`, `agentMessage` (with `phase`: `commentary` or `final_answer`), `commandExecution` (`command`, `cwd`, `status`, `exitCode`, `aggregatedOutput`, `processId`), `fileChange` (`changes[]` with `path`, `kind`, `diff`).

Text delta:

```
{"method":"item/agentMessage/delta","params":{"threadId":"…","turnId":"…","itemId":"msg_…","delta":"I"}}
```

Approval for a command. A server→client request (it has an `id` and needs a reply):

```
← {"method":"item/commandExecution/requestApproval","id":1,"params":{"kind":"command","threadId":"…","turnId":"…","itemId":"exec-fb66914f-…","startedAtMs":1795219302057,"environmentId":"local","command":"/bin/zsh -lc 'mkdir x3a'","cwd":"<th10>/work","commandActions":[{"type":"unknown","command":"mkdir x3a"}],"proposedExecpolicyAmendment":["mkdir","x3a"],"availableDecisions":["accept",{"acceptWithExecpolicyAmendment":{"execpolicy_amendment":["mkdir","x3a"]}},"cancel"]}}
→ {"jsonrpc":"2.0","id":1,"result":{"decision":"decline"}}
← {"method":"serverRequest/resolved","params":{"threadId":"…","requestId":1}}
← {"method":"item/completed","params":{"item":{"type":"commandExecution",…,"status":"declined","exitCode":null}}}
```

- While it waits: `thread/status/changed` with `{"type":"active","activeFlags":["waitingOnApproval"]}`.
- `{"decision":"accept"}` runs the command: `item/completed` with `"status":"completed","exitCode":0`. Folder `x3b` was created.
- `{"decision":"decline"}` gives `"status":"declined"`. The turn goes on; the agent wrote "`mkdir x3a` was refused; no retry." `decline` worked although `availableDecisions` did not list it. The schema lists `accept`, `acceptForSession`, `acceptWithExecpolicyAmendment`, `applyNetworkPolicyAmendment`, `decline`, `cancel`.
- With policy `untrusted` even `pwd -P` asked for approval (4 command approvals in run x3: `pwd -P`, `mkdir x3a`, `mkdir x3b`, `sleep 30`). `git rev-parse --show-toplevel` did not ask.

Approval for a file change:

```
← {"method":"item/started","params":{"item":{"type":"fileChange","id":"exec-ff556b19-…","changes":[{"path":"<th10>/work/x3.txt","kind":{"type":"add"},"diff":"ok\n"}],"status":"inProgress"}}}
← {"method":"item/fileChange/requestApproval","id":3,"params":{"threadId":"…","turnId":"…","itemId":"exec-ff556b19-…","startedAtMs":…,"reason":null,"grantRoot":null}}
→ {"jsonrpc":"2.0","id":3,"result":{"decision":"accept"}}
← {"method":"item/completed","params":{"item":{"type":"fileChange",…,"status":"completed"}}}
← {"method":"turn/diff/updated","params":{"turnId":"…","diff":"diff --git a/x3.txt b/x3.txt\nnew file mode 100644 …"}}
```

The request itself has no diff. The client must join it by `itemId` with the `fileChange` item that came just before. Decline for a file change was not tried; the schema lists `accept`, `acceptForSession`, `decline`, `cancel`.

End of a turn:

```
← {"method":"thread/status/changed","params":{"threadId":"…","status":{"type":"idle"}}}
← {"method":"turn/completed","params":{"threadId":"…","turn":{"id":"…","items":[{"type":"agentMessage","text":"1) `mkdir x3a` was refused; no retry.\n2) `mkdir x3b` succeeded.\n3) Created `x3.txt` …","phase":"final_answer"}],"itemsView":"summary","status":"completed","error":null,"startedAt":…,"completedAt":…,"durationMs":11974}}}
```

Steer (run x3, during `sleep 30`):

```
→ {"jsonrpc":"2.0","id":5,"method":"turn/steer","params":{"threadId":"…","expectedTurnId":"01a1fb68-bd4a-…","input":[{"type":"text","text":"When the command ends, also say the word PEAR."}]}}
← {"id":5,"result":{"turnId":"01a1fb68-bd4a-…"}}
← item/started and item/completed for a new `userMessage` item with that text, in the same turn
```

The steer was accepted into the running turn. The turn was interrupted 3 seconds later, so the effect on the answer was not seen.

Interrupt (runs x3 and x4):

```
→ {"jsonrpc":"2.0","id":6,"method":"turn/interrupt","params":{"threadId":"…","turnId":"01a1fb69-86d9-…"}}
← {"id":6,"result":{}}
← {"method":"turn/completed","params":{"threadId":"…","turn":{"id":"01a1fb69-86d9-…","items":[],"itemsView":"notLoaded","status":"interrupted","error":null,"durationMs":5401}}}
```

- The reply and `turn/completed` came within 0.02 s.
- No `item/completed` came for the running `commandExecution` item. A client must close open items itself when the turn ends.
- Run x4 looked for the process. Before the interrupt `sleep 31` was a direct child of `codex app-server`. Two seconds after `turn/completed` (`interrupted`) **the same `sleep 31` process was still alive**. It was gone after the app-server ended. So `turn/interrupt` ends the turn for the client but did not kill the running command in this measurement.

Resume and fork in a second process (run x4):

```
→ {"jsonrpc":"2.0","id":2,"method":"thread/resume","params":{"threadId":"01a1fb68-8d7f-…","cwd":"<th10>/work","sandbox":"read-only","approvalPolicy":"untrusted","excludeTurns":true}}
← {"id":2,"result":{"thread":{"id":"01a1fb68-8d7f-…",…},…,"initialTurnsPage":…,"turnsBackwardsCursor":…,"itemsBackwardsCursor":…}}
→ turn/start "What is the secret word for this chat, and which directory did you create? …"
← item/completed agentMessage "The secret word is PLUM, and I created `x3b`."
← turn/completed, status "completed"
→ {"jsonrpc":"2.0","id":4,"method":"thread/fork","params":{"threadId":"01a1fb68-8d7f-…","ephemeral":true,"cwd":"<th10>/work","sandbox":"read-only","approvalPolicy":"untrusted","excludeTurns":true}}
← {"id":4,"result":{"thread":{"id":"01a1fb69-862c-…","forkedFromId":"01a1fb68-8d7f-…","ephemeral":true,…}}}
← {"method":"thread/started","params":{"thread":{"id":"01a1fb69-862c-…",…}}}
```

Resume continues the thread. Fork returns a new thread ID. No turn was run on the fork.

Questions: the schema has the server request `item/tool/requestUserInput` (params `questions[]` with `id`, `header`, `question`, `options[]`, plus `isBlocking`; reply `{"answers":{<id>:{"answers":[…]}}}`), marked experimental. It was not triggered.

### 3.3 Process tree and session files (both tools)

| Case | Measured |
|---|---|
| `claude -p`, normal end | Exit 0. No process left in its group. The MCP approval server (same process group) ended with it. |
| `claude -p`, SIGINT mid-command | Exit 0. `zsh` and `python3` children (own process group) gone within 1 s. Nothing left. |
| `claude -p`, stdin `interrupt` | Children gone within 1 s. `claude` stays for the next turn. Exit 0 after stdin close. |
| `codex exec`, normal end | Exit 0. No process left. |
| `codex app-server`, normal | While alive it has helper children from the owner's configuration (two or three `node_repl`, one `npm exec … mcp` with a `node` child). After stdin close: exit 0 within 0.1 s, helpers gone within 1.5 s. |
| `codex app-server`, `turn/interrupt` | The command (`sleep 31`) was still running 2 s after the turn ended. Gone after the server ended. |
| Claude session files | A new `<session id>.jsonl` appeared for each of the 11 sessions in the Claude projects folder for `work/` under the home folder (11 files for 12 runs; the resume run reused one). Content not read. |
| Codex session files | `~/.codex/sessions/2026/11/20/rollout-<time>-<thread id>.jsonl` appeared for the exec thread and for the app-server thread. Content not read. |

## 4. What this means for the adapters

Can `claude -p` match what T3 Code gets from the SDK? From these measurements, yes for the parts tested, if the adapter uses the stream JSON mode in both directions with the stdio control channel:

```
claude -p --input-format stream-json --output-format stream-json --verbose --include-partial-messages --permission-prompt-tool stdio --permission-mode manual --session-id <uuid> | --resume <uuid>
```

- One long-lived process per thread gives: many turns, text deltas, approvals with request IDs and suggestions, agent questions, interrupt without losing the process, state events, resume and fork.
- The MCP approval tool also works but needs an extra server program and gives less. It is the fallback if the stdio way goes away, because the stdio word is not in `--help`.
- Points an adapter must handle itself:
  - Pass `--permission-mode` always. The owner's default here was `auto`.
  - Safe commands (`echo`) and config rules are decided before any approval request. Project deny rules apply; project allow rules are silently dropped in an untrusted folder (only a stderr line). `tool_result_meta.permission_decision.source` (`config`, `user_temporary`, `user_reject`) tells who decided.
  - A message sent during a turn is queued, not injected. Real steering is: interrupt, then send.
  - The exit code is 0 after an interrupt. Read the `result` event (`subtype`, `terminal_reason`).
  - After an interrupt the history says the tool was rejected, even if it ran for a while.
  - The driver must remove inherited `CLAUDE*` variables if it is itself started from Claude Code, or the child may behave like a nested session. (Not measured apart; the driver always removed them.)
- Not shown here: whether the SDK uses exactly this channel. That is a claim from reading, not from this record.

Is `codex exec` enough, or is `app-server` needed? `codex exec` is enough only for an unattended one-turn run: start, whole-item events, end, resume by ID. It has no text deltas, no approvals, no questions, no second turn in the same process, no steer. For a supervised thread with approvals, streaming text, steer, interrupt and fork, `app-server` is needed. With `app-server` the adapter must handle:

- Server→client requests need a JSON-RPC reply with the same `id`; file-change requests must be joined to the `fileChange` item by `itemId`.
- `turn/completed` with `turn.status` is the end of a turn. Open items get no `item/completed` after an interrupt.
- `turn/interrupt` did not stop the running command here. The adapter may need its own clean-up, or must accept that a command runs on until the server ends. This needs a second look before it is relied on.
- With `untrusted` even `pwd` asks. `on-request` was not measured.
- The server sends account and rate-limit notifications without being asked. The adapter should drop them.

## 5. Not measured, and what went wrong

Not measured:

- A reply to an approval that never comes. Only an 8 second delay was tried; the turn waited.
- Steering inside a Claude turn that uses tools (only a text-only turn was tried).
- Project allow rules in a trusted folder. Trust is a home-folder setting and was not changed.
- The effect of `--setting-sources user` on a project deny rule.
- Sending a `permission_suggestions` entry back ("allow for the session") on the stdio way.
- Agent questions through the MCP approval tool. Agent questions without `CLAUDE_CODE_ENABLE_ASK_USER_QUESTION_TOOL=1`.
- Codex: interrupt of `codex exec`; `codex exec fork`; `-m` and effort for exec; `on-request` policy; decline of a file change; `acceptForSession`; a failed turn; `item/tool/requestUserInput`; a turn on a forked thread; the visible effect of `turn/steer`.
- `--allowedTools`, `--disallowedTools`, `--add-dir` were read from help only.

What went wrong:

- Run c9: Claude Code refused a plain `sleep 30` by itself. The SIGINT then hit the model while it was answering. The run was repeated with a Python sleep.
- Run c9b: a timing fault in the test driver sent SIGINT 0.1 s after the tool call, at the moment of approval, not mid-command. The driver was fixed and the run repeated as c9c. This cost two extra runs.
- Run x3: the list of prepared answers was off by one, because `pwd -P` asked for approval first. So `mkdir x3a` was declined and `mkdir x3b` accepted, the other way round from the plan. Both answers were still measured.
- Run x3: the process list taken during `sleep 30` did not show the `sleep` process among the server's children. Run x4 searched all processes and found it as a direct child. Why x3 missed it is not known.
- The first run showed the owner's default permission mode is `auto`. Only run c1 (a file read) ran in that mode; all later runs used `manual`.
- Codex started the owner's configured MCP servers in every run. One of them made a network request that failed with an authorization error. The test prompts themselves used no network.

## 6. Clean-up

- Processes: after the last run, `ps` showed no `/opt/homebrew/bin/claude` or `/opt/homebrew/bin/codex` process, no `perm_mcp.py`, no `sleep 30` / `sleep 31` and no Python sleep. Every run's process group was checked at its end and was empty.
- Two `codex app-server` processes of the Codex desktop daemon are running (`…/app-server-daemon/…/codex app-server daemon` and `… --listen unix:// … --managed-daemon`). They were started on 2026-11-19, before this task, and were not touched.
- No account or login method was called. No credential file was read. No settings file in the home folder was changed. No bypass mode was used.
- Files left in `<th10>`: `th10-record.md` (this record); `scripts/` (`drive.py`, `perm_mcp.py`, `appserver.py`, `show.py`, `showx.py`); `mcp.json`, `rules.json`, the run specs `c*.json` and `x*.json`; `out/` (one `.jsonl` log per run, the approval tool logs `perm_c*.log`, and `out/schema/`, the generated app-server schema, 4.7 MB). Account-like fields in the logs were replaced by "[left out]".
- Files left in `<th10>/work`: `README.txt`, `d6/`, `d7/`, `d10/`, `x3b/`, `t8.txt`, `x3.txt`. The test file `work/.claude/settings.json` was removed.
- Files left under the home folder, written by the programs as usual: 11 Claude session files for the `work/` folder and 2 Codex rollout files dated 2026-11-20 (thread IDs `01a1fb67-41a8-…` and `01a1fb68-8d7f-…`). They were not removed.
