# T3 Code: feature and architecture inventory

Source: repository `pingdotgg/t3code`, shallow clone at commit
`6f5293fbdca476cddbc37466450983ba1dd2ada2` (committed 2026-11-20 12:38:15 -0800).
Read only. Nothing was installed, built or run. All paths are relative to the repository root.

Purpose: a reference for Anima (Rust hub, per-machine daemon, Tauri/Svelte client), which will
copy T3 Code's behaviour natively.

Each item gives:

- **Sees/does**: what the user sees or does.
- **Underneath**: how it works, with file and module names.
- **Tag**: `must` = the core thread experience; `should` = daily users expect it;
  `later` = niche, or other providers.

Wording: "verified" means I read the code that does it. "Doc says" means only the docs say it.
"Uncertain" means I could not confirm it from source.

---

## 0. Big picture (read this first)

- **The product.** T3 Code is a "control plane for coding agents" (`README.md`). It does not
  contain an agent of its own. It drives agents that are already installed and signed in on the
  machine: Claude Code, Codex, Cursor, OpenCode, Grok Build, Antigravity, Kiro, Pi, Muse Code, fx,
  and any ACP agent. `docs/user/faq.md` says this plainly ("T3 Code is not a harness").
- **The parts.**
  - `apps/server`: a Node server written in TypeScript with the Effect library. Node
    `^22.16 || ^23.11 || >=24.10`, from `apps/server/package.json`. It owns provider processes,
    terminals (PTYs), Git, files and the database.
  - `apps/web`: a React + Vite client using TanStack Router, Tiptap and zustand.
  - `apps/desktop`: an Electron 44 shell. It bundles the server and loads the web UI.
  - `apps/mobile`: an Expo 58 / React Native 0.88 phone app.
  - `apps/marketing`: the website.
  - `packages/client-runtime`: connection, retry and state logic shared by web, desktop and mobile.
  - `packages/contracts`: every wire schema (Effect `Schema`).
- **Ownership rule** (`docs/internals/overview.md`): execution stays on the machine that holds
  the workspace (the "environment"). Clients only send authenticated RPC. A remote client never
  uses its own filesystem or credentials in place of the environment's.
- **Data flow:**
  native provider protocol → adapter / normaliser → V2 orchestration events, committed to SQLite
  → projections → WebSocket streams → clients (`docs/orchestration-v2/README.md`, "Conceptual
  Layers").
- **Main concepts** (`docs/orchestration-v2/core-graph-and-data-model.md`):

  | Concept | Meaning |
  | --- | --- |
  | Project | A folder or repository. |
  | AppThread | The conversation the user sees. |
  | Run | One counted user-visible turn. |
  | RunAttempt | One provider try at a run. Retries and steer-restarts add attempts. |
  | ExecutionNode | A tree of work inside a run: `root_turn`, `assistant_message`, `reasoning`, `plan`, `todo_list`, `tool_call`, `approval_request`, `user_input_request`, `subagent`, `hook`, `system`. |
  | ProviderSession | A live or resumable provider process. |
  | ProviderThread | The provider's own conversation handle. |
  | ProviderTurn | The provider's own turn handle. |
  | RuntimeRequest | A provider callback that needs a user answer. |
  | Checkpoint | Saved workspace state. |
  | ContextTransfer / ContextHandoff | How context moves for forks and provider switches. |

---

## 1. Projects and threads

### 1.1 Projects
- **Sees/does.** Add a project in four ways:
  - an existing folder;
  - **New project** (makes a Git repo in `~/.t3/projects` with a README, an icon and a first
    commit);
  - clone from GitHub, GitLab or another host;
  - **Publish Repository**.

  Projects can be pinned and archived. Each has a name, an icon (emoji, monogram or image), and
  per-project setting overrides.
- **Underneath.**
  - Server: `apps/server/src/project/`, `apps/server/src/orchestration-v2/ProjectCommands.ts`,
    `ProjectStore.ts`.
  - Contracts: `packages/contracts/src/project.ts`, `projectClone.ts`.
  - RPCs (in `packages/contracts/src/rpc.ts` `WS_METHODS`): `projects.createNew`,
    `projects.mutate`, `projectClone.start/cancel/retry`, `sourceControl.cloneRepository`,
    `sourceControl.publishRepository`.
  - The clone runs in the background. The user can type the first prompt while it runs, and
    sending waits for the files (doc: `docs/user/source-control.md`).
- **Tag:** add folder = must; new project / clone / publish = should; icons, pin, archive = later.

### 1.2 Threads without a project ("scratch")
- **Sees/does.** Start a thread with no project (`mod+alt+n`). It works in its own folder,
  `~/.t3/scratch/<date>-<first-words>-<id>`. **Turn into project** converts the folder into a
  project in place.
- **Underneath.** RPCs `projects.ensureScratch` and `projects.createFromThread`; command
  `thread.project.adopt-folder` (`packages/contracts/src/orchestrationV2.ts`). Doc:
  `docs/user/thread-sidebar.md`.
- **Tag:** later.

### 1.3 Creating a thread
- **Sees/does.**
  - **New thread** (`mod+n` / `chat.new`) opens a draft. It asks for a project if there are
    several. `mod+shift+n` (`chat.newLocal`) skips that question.
  - The draft carries over the model, effort and permission mode. Branch and workspace mode
    (current checkout or **New worktree**) come from defaults.
  - Choose a base branch when making a worktree.
  - `mod+enter` in a draft sends it and opens a new draft ("start in background").
  - Shift-click several models to send one prompt to several models. Each gets its own thread
    and worktree.
- **Underneath.**
  - RPC `orchestration.launchThread` takes `OrchestrationV2ThreadLaunchInput`, which has a
    workspace strategy:
    - `root` (the project checkout);
    - `existing_worktree`;
    - `worktree` (with `baseRef`, `startFromOrigin` and an optional `branch`).
  - `apps/server/src/orchestration-v2/ThreadLaunchService.ts` creates the worktree (section 6),
    then dispatches `thread.create` and the first `message.dispatch`.
  - A draft is a client-side idea until the first send. The `draftThreadRoute` keybinding context
    stays true until then (`docs/user/keybindings.md`).
- **Tag:** must (create and first send); should (worktree choice); later (multi-model fan-out,
  background start).

### 1.4 Sidebar grouping and order
- **Sees/does.** The sidebar has these sections:
  - **Pinned**;
  - **Active** (the user arranges it);
  - **Working** (beta, on by default): threads busy or waiting on background work;
  - **Snoozed** shelf;
  - **Settled** shelf.

  Drag rows to reorder, pin, unpin, settle or wake. An optional "boring sidebar" (Labs) groups
  threads in a folder per project. Each project-group folder shows a count and a "needs you" dot.
  Undo toast for 5 seconds (`mod+z`).
- **Underneath.**
  - Order is saved on the server through commands `thread.pin.reorder`, `thread.active.reorder`,
    `thread.pin`, `thread.unpin`. The shell fields are `pinOrderKey` and `activeOrderKey`
    (`OrchestrationV2ThreadShell`).
  - Client: `apps/web/src/components/Sidebar.tsx`, `Sidebar.drag.ts`, `Sidebar.logic.ts`.
  - Settings: `sidebarWorkingShelfEnabled`, `boringSidebarEnabled`, `sidebarProjectGroupingMode`,
    `sidebarProjectSortOrder`, `sidebarThreadSortOrder` (`packages/contracts/src/settings.ts`).
- **Tag:** list of threads per project = must; pinned + settled = should; drag and drop, Working
  shelf, boring sidebar = later.

### 1.5 Thread states
- **Sees/does.** Status dots and labels:
  - idle, working, waiting (approval or question), failed;
  - **Limited** (stopped on a usage limit);
  - interrupted;
  - Goal (a running `/goal`).
- **Underneath.**
  - `OrchestrationV2ShellThreadStatus` = `"idle"` or a `OrchestrationV2RunStatus`. Run statuses:
    `preparing | queued | starting | running | waiting | completed | interrupted | failed |
    cancelled | rolled_back` (`packages/contracts/src/orchestrationV2.ts` ~line 682).
  - The shell row also carries `pendingRuntimeRequest`, `lastError`, `lastErrorClass`,
    `usageLimitResetAt`, `pendingBackgroundTasks`, `goal`, `limitRecovery`, `snoozedUntil`,
    `settledAt`, `pinnedAt`, `tags`, `lastVisitedAt` and more (`OrchestrationV2ThreadShell`).
- **Tag:** must (idle / running / waiting-for-you / failed); should (unread / visited watermark);
  later (Limited, Goal).

### 1.6 Pin, snooze, settle, archive, delete
- **Sees/does.**
  - **Pin** keeps a thread on top.
  - **Snooze** hides a thread until a wake time. The presets can be changed. It comes back early
    for an approval, a new question, a failure, or your own message.
  - **Settle** moves finished work to the Settled shelf. Auto-settle happens after 7 days idle by
    default, on PR merge, and optionally on PR close.
  - **Archive** hides a thread. **Delete** removes it.
  - **Auto-settle behaviour → Never** exempts one thread.
  - Settling closes terminals that sit at an idle prompt and can run "on settle" scripts.
- **Underneath.**
  - Commands: `thread.pin`, `thread.unpin`, `thread.snooze`, `thread.unsnooze`, `thread.settle`,
    `thread.unsettle`, `thread.auto-settle`, `thread.auto-settle.set`, `thread.archive`,
    `thread.unarchive`, `thread.delete`, `thread.mark-unread`, `thread.visit`, `thread.tag.set`
    (`OrchestrationV2Command`).
  - Settlement is decided on the server, with no client needed:
    `apps/server/src/orchestration-v2/ThreadSettlementService.ts` and `PullRequestSyncReactor.ts`.
    The guarded `thread.auto-settle` rejects newer activity and live work
    (`docs/internals/overview.md`).
  - Deletion: `ThreadDeletion.ts`.
  - Settings: `sidebarAutoSettleAfterDays`, `sidebarAutoSettleOnMerge`,
    `sidebarAutoSettleKeepPullRequests`, `snoozePresets`.
- **Tag:** archive / delete = must; pin = should; settle / auto-settle / snooze = later.

### 1.7 Titles
- **Sees/does.** The first message gives a placeholder title, then a model writes a title.
  **Regenerate title** is in the thread menu and is also the `thread.regenerateTitle` command.
  Custom title instructions can be set globally or per project. Rename also works.
- **Underneath.**
  - `apps/server/src/orchestration-v2/placeholderThreadTitle.ts`
  - `ThreadTitleRegenerationService.ts`
  - `apps/server/src/textGeneration/TextGeneration.ts`, using the provider chosen in
    `textGenerationModelSelection`.
  - Prompts: `packages/provider-core/src/server/textGenerationPrompts.ts` and
    `threadTitleContext.ts`.
  - Commands: `thread.metadata.update`, `thread.title.regeneration.complete`.
  - Some providers are told the title natively (`applyThreadTitle`; Codex `thread/name/set`).
- **Tag:** must (auto title); should (rename / regenerate); later (title instructions).

### 1.8 Forks, side chats, subagents, merge-back
- **Sees/does.**
  - **Fork** a thread from a run or checkpoint.
  - **Side chat** (`/side` or `/btw`): ask about a thread without adding to it. It opens in the
    right panel, and **Add to chat** links it back.
  - Subagents (provider-native or app-owned `delegate_task`) appear as cards and child threads.
  - **Merge back** a child thread.
- **Underneath.**
  - Commands: `thread.fork`, `thread.merge_back`, `delegated_task.request`, `subagent.stop`.
  - Services: `ThreadForkService.ts`, `ContextHandoffService.ts`, `ProviderSwitchService.ts`.
  - Docs: `docs/orchestration-v2/thread-lineage-and-context-transfer.md`, `feature-lifecycles.md`.
  - A fork records app lineage first. At the fork's first run, it uses the provider's own fork if
    it can (Codex `thread/fork`, Claude SDK `forkSession`), otherwise a portable summary
    ("handoff").
- **Tag:** later (fork, side chat, subagent cards); should (show subagent work at least read-only
  for Claude and Codex).

---

## 2. The chat thread

### 2.1 How the thread is rendered
- **Sees/does.** One timeline. In order:
  - user messages;
  - assistant text;
  - collapsed "work log" groups of tool calls;
  - plan and todo cards;
  - approval and question panels;
  - checkpoint and diff rows;
  - errors and system notices;
  - compaction, handoff and fork markers.
- **Underneath.**
  - The server builds an ordered `turnItems` projection (`OrchestrationV2TurnItem`). The client
    does not merge messages, plans and activities itself (core-graph doc, "Turn Item Projection").
  - Item types (verified in `packages/contracts/src/orchestrationV2.ts` ~line 1762):
    `notification`, `user_message`, `assistant_message`, `reasoning`, `proposed_plan`,
    `todo_list`, `user_input_request`, `file_change`, `command_execution`, `file_search`,
    `web_search`, `approval_request`, `checkpoint`, `run_interrupt_request`,
    `run_interrupt_result`, `system_notice`, `error`, `compaction`, `handoff`, `fork`,
    `thread_created`, `secret_request`, `link_request`, `subagent`, `dynamic_tool`.
  - Web: `apps/web/src/components/chat/MessagesTimeline.tsx`, `ThreadTimeline.tsx`,
    `TimelineWorkGroupRows.tsx`, `TimelineWorkEntryRow.tsx`, `WorkLog.tsx`,
    `V2ItemInspector.tsx`.
  - The `toolCallVisibility` client setting has values `showAll | hideSuccessful | hideAll | zen`.
- **Tag:** must.

### 2.2 User messages
- **Sees/does.**
  - Rich Markdown composer (Tiptap), with plain mode available.
  - Up to 120,000 characters. Pastes of 32 KiB or more become a text-file attachment.
  - Inline "chips" for files, terminal excerpts, PRs, threads, quotes and model mentions.
  - Prompt history: Up / Down in an empty composer.
  - **Edit from here** rewinds the thread (section 6.3).
- **Underneath.**
  - Command `message.dispatch` (`OrchestrationV2Command`). Fields: `commandId`, `threadId`,
    `messageId`, `text`, `context`, `attachments`, `modelSelection`, `dispatchMode`,
    `deliveryIntent`, `freshContext`, `sourcePlanRef` and others.
  - Limits in `packages/contracts/src/chatAttachment.ts`: `PROVIDER_SEND_TURN_MAX_INPUT_CHARS`
    = 120000.
  - Intake: `apps/server/src/orchestration-v2/ThreadMessageIntake.ts`.
  - Composer: `apps/web/src/components/ComposerPromptEditorTiptap.tsx`,
    `chat/ChatComposer.tsx`, `chat/composerSubmission.ts`.
- **Tag:** must (text send); should (chips, history); later (rich-text editing details).

### 2.3 Assistant text and streaming
- **Sees/does.** Assistant replies appear as Markdown with code blocks, tables, Mermaid and math.
- **Underneath (important).**
  - Adapters emit `turn_item.updated` / `message.updated` with `streaming: true` while text
    arrives.
  - The server **does not stream token by token to clients by default.** The server setting
    `responseStreamingMode` is `"turn" | "paragraph"` and defaults to `"paragraph"`
    (`packages/contracts/src/settings.ts` lines 1144 and 1394).
    - `paragraph`: "Each paragraph or code block appears as soon as it is complete."
    - `turn`: "Text appears once the agent finishes its turn."
    - These strings come from `apps/web/src/components/settings/SettingsPanels.tsx` line 140.
  - Paragraph boundaries come from a Markdown-aware scanner in
    `apps/server/src/orchestration-v2/assistantStreaming.ts`. It is used by `RunExecutionService.ts`
    (~line 1117).
  - Provider text deltas are batched in `Adapters/ProviderTextDeltaCoalescer.ts`. Live events are
    batched in `ThreadLiveEventCoalescer.ts`.
  - Command output deltas are different: `command_output.delta` events are **not saved**. They go
    only to clients showing that row (`ProviderAdapter.ts` comment), through RPC
    `orchestration.subscribeCommandOutput` (`CommandOutputHub.ts`).
- **Tag:** must (assistant text, at least by paragraph); should (live command output).

### 2.4 Reasoning
- **Sees/does.** A reasoning or "thinking" item, collapsible.
- **Underneath.**
  - Turn item `reasoning` (`text`, `streaming`).
  - Codex sources: `item/reasoning/textDelta` and `item/reasoning/summaryTextDelta`. The adapter
    sends `summary: "auto"` unless the user's Codex config sets `model_reasoning_summary`
    (`CodexAdapterV2.ts` ~line 880).
  - Claude sources: SDK `thinking` with `{ type: "adaptive", display: "summarized" }` plus the
    `showThinkingSummaries` setting, when summaries are requested (`ClaudeAdapterV2.ts`
    ~line 2397).
- **Tag:** should.

### 2.5 Tool calls, by kind
The normalised item types are listed below. Codex maps native item types. Claude maps by tool
name in `classifyClaudeNativeTool` (`ClaudeAdapterV2.ts` ~lines 3687–3713, verified):

| Kind | Turn item | Claude tools | Codex source | Tag |
| --- | --- | --- | --- | --- |
| Shell command | `command_execution` (`input`, `output`, `exitCode`, `outputIndicatesFailure`, `monitor`, `completionWake`) | `Bash` | `commandExecution` items plus `item/commandExecution/outputDelta` | must |
| File edit | `file_change` (`fileName`, `additions`, `deletions`, `diffStr`, `oldStr`, `newStr`, `changes[]`) | `Edit`, `MultiEdit`, `Write`, `NotebookEdit` | `fileChange` items | must |
| File read | `dynamic_tool` (no special type) | `Read`, `LS` | Codex reads show as command executions, or are not shown (uncertain) | should |
| Search | `dynamic_tool` for Claude `Glob` / `Grep`; there is a `file_search` type (`pattern`, `results[]`), but the Claude mapping does not use it | `Glob`, `Grep` | uncertain | should |
| Web | `web_search` (`patterns[]`, `results[]`) | `WebSearch`, `WebFetch` | `webSearch` items | should |
| MCP / other | `dynamic_tool` (`toolName`, `input`, `output`, `viewedImagePath`) | any `mcp__*` and unknown tools; also `Task`, `Agent`, `TodoWrite`, `ToolSearch`, `Monitor`, `SendMessage`, `TaskStop` | `mcpToolCall` / `dynamicToolCall` | must (a generic JSON view) |
| Subagent | `subagent` card + child thread | `Agent` / `Task` | collab agent items | later |

- The UI shows a summary per call. Expanding it shows the full command, input and output. Long
  output keeps only the latest part. Output withheld on the wire (`outputOmitted`) is fetched
  with `orchestration.getTurnItem`.
- `docs/user/activity-log.md` covers the UI. Code: `TimelineWorkEntryRow.tsx`,
  `ShellCommandBlock.tsx`, `CommandOutputPanel.tsx`; MCP display
  `packages/provider-core/src/server/mcpToolPresentation.ts`; Codex display
  `apps/server/src/provider/CodexToolPresentation.ts`.

### 2.6 Approvals
- **Sees/does.** When the agent wants to run a command, change files, read outside the allowed
  area, or answer an MCP elicitation, an approval row appears in the timeline and the composer.
  - Default buttons: **Approve**, **Decline**, and a "more" menu with **Always allow this
    session** and **Cancel**.
  - Providers can offer their own options with labels and warnings.
  - With Claude, typing a reply while an approval waits declines it and sends the text as feedback
    to the agent that asked (`docs/user/permission-modes.md`).
  - Subagent approvals show in the parent thread with the subagent's name.
- **What the request contains** (turn item `approval_request`, verified):
  - `requestId`;
  - `requestKind`: one of `command | file-read | file-change | mcp-elicitation | permission`
    (`packages/contracts/src/providerPolicy.ts` `ProviderRequestKind`);
  - optional `prompt`, `reason`, `appName`;
  - `fullAccessOverride`: `ask_rule | provider_check`. This explains why an approval appeared
    even in Full access;
  - `options[]` of `{decision, label, warning?}`.

  The command or patch details are on the matching tool item (`command_execution` /
  `file_change`).
- **The choices:** `ProviderApprovalDecision` = `accept | acceptForSession | acceptAlways |
  decline | cancel`, with optional `feedback` text.
- **What happens:**
  1. The adapter receives the native callback. It records a `RuntimeRequest` (status `pending`,
     `responseCapability` `live` bound to the provider session id) and an `approval_request`
     item. The run goes to `waiting`.
  2. The client sends command `runtime-request.respond` with `{threadId, requestId, decision?,
     feedback?, answers?, attachmentsByQuestionId?}`.
  3. The orchestrator saves the resolution first. Then the effect worker calls
     `adapter.respondToRuntimeRequest` (`RuntimeRequestService.ts`, `EffectWorker.ts`).
- **Codex mapping** (`CodexAdapterV2.ts` ~lines 623–668):
  - Exec / patch approvals: `accept` → `approved`; `acceptForSession` and `acceptAlways` →
    `approved_for_session`; `decline` → `denied` with a rejection message; `cancel` → `abort`.
  - `item/permissions/requestApproval`: the permissions are granted with `scope: "turn"` or
    `"session"`.
  - Native requests handled: `item/commandExecution/requestApproval`,
    `item/fileChange/requestApproval`, `item/permissions/requestApproval`,
    `mcpServer/elicitation/request`.
- **Claude mapping** (`permissionResultFromDecision`, `ClaudeAdapterV2.ts` ~line 4453):
  - The approval arrives through the SDK `canUseTool` callback.
  - `accept` → `{behavior:"allow", updatedInput}`.
  - `acceptForSession` → the same, plus `updatedPermissions` (the SDK's suggestions, or an
    `addRules` allow rule for that tool, with `destination:"session"`).
  - `decline` → `{behavior:"deny", message}`. The feedback text is included when given.
  - `cancel` → deny with `interrupt:true`.
  - MCP elicitations get the 4 options Cancel / Decline / Always allow this session / Approve.
- **After restart:** a live approval cannot be answered once its process is gone. It is
  `not_resumable` or expired. Codex async questions (`responseCapability: {type:"message"}`) do
  survive (section 2.7).
- **Code:** `apps/web/src/components/chat/ComposerPendingApprovalActions.tsx`,
  `TimelinePendingApprovalRow.tsx`, `ChildThreadApprovals.tsx`.
- **Tag:** must.

### 2.7 Questions from the agent (user input requests)
- **Sees/does.** A question panel with choices and/or free text. Files can be attached to
  answers. A question can be dismissed.
- **Underneath.**
  - Turn item `user_input_request` (`questions[]`, `questionAnswer`, `responseMode`).
  - Answered with `runtime-request.respond` (`answers`). Dismissed with
    `thread.user-input.dismiss`.
  - Codex `item/tool/requestUserInput`. Codex async questions arrive as notifications and are
    answered **as a new user message**, so they survive turn end, process exit and server restart
    (`docs/internals/providers.md`, "Protocol traps").
  - Claude `AskUserQuestion` goes through `canUseTool`.
  - UI: `ComposerPendingUserInputPanel.tsx`. Doc: `docs/user/question-attachments.md`.
- **Tag:** must (Claude AskUserQuestion, Codex requestUserInput); later (attachments in answers).

### 2.8 Plans and todo lists
- **Sees/does.**
  - **Plan mode** (interaction mode `plan`) makes the agent propose a plan. It shows as a
    "Proposed plan" card with collapse, copy and "save to workspace".
  - Todo or "Tasks" lists show as a row with steps (`pending | running | completed`).
- **Underneath.**
  - `ProviderInteractionMode` = `default | plan` (`providerPolicy.ts`). Set with command
    `thread.interaction-mode.set`. It is toggled in the composer traits picker
    (`TraitsPicker.tsx`).
  - Codex: `collaborationMode: {mode, settings}` on every `turn/start`; plan text from
    `item/plan/delta`; todo list from `turn/plan/updated`.
  - Claude: SDK `permissionMode: "plan"`. `ExitPlanMode` becomes a proposed plan. `TodoWrite`
    becomes a todo list. The doc says Claude task lists need `CLAUDE_CODE_ENABLE_TODO_TOOLS=1` in
    the instance environment (`docs/user/providers-claude.md`).
  - Turn items `proposed_plan` (`markdown`, `streaming`) and `todo_list` (`steps[]`,
    `explanation`).
  - A follow-up that implements a plan carries `sourcePlanRef`. The exact button label in the UI
    is uncertain.
  - UI: `ProposedPlanCard.tsx`.
- **Tag:** must (todo list display); should (plan mode + proposed plan).

### 2.9 Diffs per turn
- **Sees/does.**
  - Each completed turn gets a checkpoint row listing changed files with +/- counts.
  - **Open diff** shows the turn's diff. The Diff panel (`mod+d`) can show a turn, the whole
    thread, the branch against base, or single commits.
  - Find in diff with `mod+f`.
- **Underneath.** See section 6.2 (Git-ref checkpoints). Diffs come from comparing checkpoints:
  RPCs `orchestration.getTurnDiff` and `orchestration.getFullThreadDiff`;
  `apps/server/src/checkpointing/CheckpointDiffQuery.ts`, `Diffs.ts`.
- **Tag:** must.

### 2.9a Comments on a line of a diff (added 2026-11-20, after a research report named the file)
- **Sees/does.** The person selects lines of a diff and writes a comment; the comment goes to the agent with the file, the range of lines and the text of the diff.
- **Underneath.** `apps/web/src/reviewCommentContext.ts` (`ReviewCommentContextSchema`: `filePath`, `rangeLabel`, `text`, `diff`, an optional `selection` and `pullRequest`). The file was opened and its schema read; how the comment is shown was not read.
- **Tag:** should.

### 2.10 Errors
- **Sees/does.** Error rows with a class and message. Banners for provider status. A **Retry** or
  **Resume** button.
- **Underneath.**
  - Turn item `error` (`failure: OrchestrationV2ProviderFailure`, optional `retry`).
  - `OrchestrationV2ProviderFailureClass` (~line 1597).
  - Safe user-facing text: `apps/server/src/orchestration-v2/UserFacingErrors.ts`.
  - `turn.terminal` with `status:"failed"` carries `failure`, `retry` and `threadDisposition`
    (`reusable | broken`).
  - UI: `ThreadErrorBanner.tsx`, `ProviderStatusBanner.tsx`.
- **Tag:** must.

### 2.11 Token and usage display
- **Sees/does.**
  - A context-window meter in the thread details card (off by default; **Settings → Appearance →
    Thread details → Context window**).
  - A **Usage** page (`mod+u`) with tokens, cache, estimated cost and limits per provider.
  - Usage-limit banners with reset time.
- **Underneath.**
  - Per-turn `OrchestrationV2ProviderTurnTokenUsage`: `usedTokens`, `maxTokens`, `inputTokens`,
    `cachedInputTokens`, `outputTokens`, `reasoningOutputTokens`, `promptCacheTtlMs`,
    `autoCompactThreshold`, `promptCacheExpiresAt`.
  - Codex: `thread/tokenUsage/updated` and `account/rateLimits/updated` / `read`.
  - Claude: `apps/server/src/provider/ClaudeTurnTokenUsage.ts`.
  - The Usage page scans each provider's own history files: `apps/server/src/provider/Drivers/*Usage.ts`,
    `packages/contracts/src/usage.ts`, RPC `server.getUsageSummary`.
  - UI: `ThreadDetailsUsage.tsx`, `ComposerUsageLimits.tsx`.
- **Tag:** should (context meter); later (Usage page, cost estimates).

### 2.12 Other thread items and actions
- **Compaction:** `/compact`, the `compaction` item with token counts before and after, and idle
  compaction for Claude (`IdleCompaction.ts`). Tag: should.
- **`/clear`:** a fresh session in the same thread (`freshContext`). Tag: should.
- **Goals:** `/goal` for Codex, Claude and OpenCode 2.2.5+. Codex `thread/goal/*`. Tag: later.
- **Suggested next prompt** (Labs): `prompt_suggestion.updated`. Tag: later.
- **Find in thread:** `chat.find`, `orchestration.searchThread`. Tag: should.
- **Quote / cite an assistant response:** `AssistantCitationChip.tsx`. Tag: later.
- **HTML renders:** sandboxed pages from agents (`docs/user/html-renders.md`,
  `apps/server/src/htmlRender/`). Tag: later.
- **Secret requests:** an agent asks for a secret in a private card (`secret_request` item). Tag:
  later.

---

## 3. Provider adapters

### 3.1 The common adapter interface
- **File:** `packages/provider-core/src/server/ProviderAdapter.ts` (836 lines, verified).
- A session runtime (`ProviderAdapterV2SessionRuntime`) exposes:
  - an `events` stream;
  - `ensureThread`, `resumeThread`, `injectHistory?`, `startTurn`, `resumeTurn?`,
    `compactThread?`, `steerTurn`, `applyModelSelection?`, `applyThreadTitle?`, `stopTask?`,
    `awaitBackgroundTasks?`, `interruptTurn`, `stopSubagent?`, `unloadThread?`,
    `respondToRuntimeRequest`, `readThreadSnapshot`, `uploadFeedback?`, `mcpApps?`,
    `rollbackThread`, `forkThread`;
  - `canResumeAfterRestart`, `hasPendingBackgroundWork`.
- **Turn input** (`ProviderAdapterV2TurnInput`): `appThread`, `threadId`, `runId`, `runOrdinal`,
  `attemptId`, `rootNodeId`, `providerThread`, `message` (`text` + `attachments`),
  `modelSelection`, `runtimePolicy`, `appContext`.
- **Runtime policy** (`ProviderAdapterV2RuntimePolicy`): `runtimeMode`, `interactionMode`, `cwd`,
  optional raw `approvalPolicy` / `sandboxPolicy`, and `maxRuntimeMode` (an admin ceiling).
- **The normalised event model** (`ProviderAdapterV2Event`, verified):
  - `app_thread.created`
  - `provider_session.updated`
  - `provider_thread.updated`
  - `provider_turn.updated`
  - `node.updated`
  - `subagent.updated`
  - `message.updated`
  - `turn_item.updated`
  - `steer.updated` (`pending | completed | cancelled`)
  - `background_work.updated`
  - `command_output.delta` (not saved)
  - `prompt_suggestion.updated` (not saved)
  - `runtime_request.updated`
  - `plan.updated`
  - `turn.terminal`: `status` `completed | interrupted | cancelled`, or `failed` with `failure`
    and `retry`. Always `threadDisposition` `reusable | broken`.
- **Provider capabilities** (`OrchestrationV2ProviderCapabilities`, ~line 405) are grouped as
  sessions, threads, turns, streaming, tools, approvals, planning, subagents, context,
  checkpointing, identity and runtimePolicy. Examples: `supportsActiveSteering`,
  `supportsInterrupt`, `supportsNativeResume`, `canRollbackThread`,
  `providerCanRollbackConversation`, `streamsAssistantText`, `supportsCommandApproval`. Features
  follow these flags, not the provider's name (`docs/orchestration-v2/provider-capability-system.md`).
- A provider "driver kind" names an integration. An "instance" is one configuration plus account.
  Work routes by instance (`docs/internals/providers.md`).
  - Registry: `apps/server/src/provider/ProviderInstanceRegistry.ts`, `builtInDrivers.ts`.
  - Adapter registry: `apps/server/src/orchestration-v2/ProviderAdapterRegistry.ts`.

### 3.2 Permission ("runtime") modes
- **Sees/does.** Four modes are chosen in the composer (`mod+shift+a`):
  - **Supervised** (`approval-required`)
  - **Auto-accept edits** (`auto-accept-edits`)
  - **Auto** (`auto`)
  - **Full access** (`full-access`)

  The default for new threads is **Full access** (`DEFAULT_RUNTIME_MODE`, `providerPolicy.ts`
  line 32). The default can be set per environment or project. An admin policy can cap it.
- **Underneath.**
  - `RuntimeMode` in `packages/contracts/src/providerPolicy.ts`. Set with command
    `thread.runtime-mode.set`. Saved on the AppThread.
  - Policy resolution: `apps/server/src/orchestration-v2/RuntimePolicy.ts`.
  - Managed ceiling: `docs/user/managed-policy.md`, `apps/server/src/policy/`.
- **Tag:** must (at least Supervised and Full access); should (Auto-accept edits); later (Auto).

### 3.3 Codex: app-server JSON-RPC over stdio (verified)
- **Launch.** `codex app-server -c thread_unload_delay_secs=0 [user launch args]`
  (`apps/server/src/provider/codexLaunchArgs.ts`).
  - Client library: `packages/effect-codex-app-server` (`client.ts`, `protocol.ts`, `rpc.ts`).
    Its schemas are generated from Codex in `src/_generated/schema.gen.ts`.
  - Initialise opts out of `turn/diff/updated`, because T3 makes diffs from checkpoints
    (`docs/internals/providers.md`).
  - T3's MCP server is injected in each `thread/start`, `thread/resume` and `thread/fork` config,
    with a bearer header (`docs/orchestration-v2/orchestrator-mcp-server.md`).
- **Methods T3 sends** (found in `apps/server/src/orchestration-v2/Adapters/CodexAdapterV2.ts`):
  - `thread/start`, `thread/resume`, `thread/read`, `thread/fork`, `thread/archive`,
    `thread/unarchive`, `thread/unsubscribe`, `thread/name/set`;
  - `thread/settings/update`, `thread/compact/start`, `thread/sample`;
  - `thread/goal/set|get|clear`;
  - `thread/backgroundTerminals/list|terminate`;
  - `turn/start`, `turn/steer`, `turn/interrupt`;
  - `config/read`, `mcpServerStatus/list`, `mcpServer/tool/call`, `mcpServer/resource/read`;
  - `account/rateLimits/read`, `feedback/upload`;
  - `thread/revert` (in `apps/server/src/provider/CodexThreadRevert.ts`).
- **Notifications T3 handles:**
  - `turn/started`, `turn/completed` (this one decides that a turn ended), `turn/plan/updated`;
  - `item/started`, `item/completed`, `item/agentMessage/delta`, `item/reasoning/textDelta`,
    `item/reasoning/summaryTextDelta`, `item/plan/delta`, `item/commandExecution/outputDelta`;
  - `thread/tokenUsage/updated`, `thread/settings/updated`, `thread/closed`,
    `thread/goal/updated|cleared`;
  - `account/rateLimits/updated`, `model/rerouted`, `mcpServer/startupStatus/updated`,
    `hook/completed`.
- **Requests from Codex** (they need a reply): `item/commandExecution/requestApproval`,
  `item/fileChange/requestApproval`, `item/permissions/requestApproval`,
  `item/tool/requestUserInput`, `mcpServer/elicitation/request`.
- **Turn start** (`buildCodexTurnStartParams`, ~line 805) sends `turn/start` with:
  `threadId`, `input` (text plus images as `data:` URLs), `cwd`, `model`, `summary`,
  `approvalsReviewer`, `approvalPolicy`, `sandboxPolicy`, `effort`, `serviceTier`,
  `cyberAccessProgram`, `collaborationMode {mode, settings{model, reasoning_effort}}`, and
  `additionalContext` (T3 context, and app notes marked `untrusted`).
- **Mode mapping** (`codexRuntimeModeTurnDefaults`, ~line 746):

  | T3 mode | approvalPolicy | approvalsReviewer | sandboxPolicy |
  | --- | --- | --- | --- |
  | approval-required | `untrusted` | `user` | `{type:"readOnly"}` |
  | auto-accept-edits | `on-request` | `user` | `{type:"workspaceWrite"}` |
  | auto | `on-request` | `auto_review` | `{type:"workspaceWrite"}` |
  | full-access | granular: only `mcp_elicitations: true`, all else false | `user` | `{type:"dangerFullAccess"}` |

- **Interrupt.** `turn/interrupt` answers first. The interrupted end state arrives later as
  `turn/completed` (`docs/orchestration-v2/README.md`, probe facts).
- **Steer.** `turn/steer`.
- **Model / effort / speed.** `reasoningEffort` and `serviceTier` options. Fast mode maps to the
  service tier `fast` (`apps/server/src/codexModelOptions.ts`). These are sent per turn, or with
  `thread/settings/update`.
- **Accounts.** Codex homes, "shadow homes" for several accounts, and managed ChatGPT sign-in
  (`apps/server/src/provider/CodexChatGpt*.ts`, `CodexManaged*.ts`,
  `Drivers/CodexHomeLayout.ts`).
- **Tag:** must (if Anima supports Codex).

### 3.4 Claude Code: Anthropic **Claude Agent SDK** (TypeScript), not a hand-written CLI wrapper (verified)
- **Dependency.** `@anthropic-ai/claude-agent-sdk` `^0.3.321` (`apps/server/package.json`
  line 39). The adapter imports `query` and `forkSession` from it (`ClaudeAdapterV2.ts` lines
  43, 67, 1672).
  - The SDK is pointed at the user's installed `claude` binary through
    `pathToClaudeCodeExecutable` (`settings.binaryPath`). T3 supplies its own
    `spawnClaudeCodeProcess`.
  - How the SDK talks to that binary (believed to be stream-json control messages) lives inside
    the SDK, not in this repository. Uncertain.
- **Session shape.** One long-lived `query()` per provider session, with streaming input.
  `ClaudeAgentSdkQuerySession` (~line 723) exposes:
  - `messages` (a stream of `SDKMessage`);
  - `offer(SDKUserMessage)`, used for new turns and steers;
  - `setModel`, `applyFlagSettings`, `setPermissionMode`, `renameSession`, `stopTask`,
    `backgroundTasks`, `mcpServerStatus`, `readMcpResource`, `callMcpTool`,
    `listPermissionRules`;
  - interrupt calls `queryRuntime.interrupt()` (~line 1971);
  - MCP servers are registered after boot with `setMcpServers` (~line 1706).
- **Options passed** (~line 2369):
  - `model` (the compiled API model id);
  - `tools`, `permissionMode`, `includePartialMessages: true` (deltas arrive as `stream_event`
    messages);
  - `promptSuggestions`, `effort`;
  - `resume` or `sessionId`, `resumeSessionAt`;
  - `allowedTools` / `disallowedTools`, `canUseTool`, `allowDangerouslySkipPermissions`;
  - `thinking: {type:"adaptive", display:"summarized"}` when summaries are wanted;
  - `settings` (can include `env`, for example `CLAUDE_CODE_AUTO_COMPACT_WINDOW` and
    `ENABLE_TOOL_SEARCH`);
  - `onUserDialog`, `onElicitation`, `env`, `mcpServers`, `systemPrompt: {type:"preset", …}`;
  - `title`.
- **Mode mapping** (`permissionModeForClaudeRuntimePolicy`, ~line 3424):
  - plan interaction → `"plan"`;
  - `approval-required` → `"default"`;
  - `auto-accept-edits` → `"acceptEdits"`;
  - `auto` → `"auto"`;
  - `full-access` → `"bypassPermissions"`, plus `allowDangerouslySkipPermissions: true`.

  If Claude settings disable bypass, it falls back to `default` or `auto` (~line 3496).
  `canUseTool` is installed in every mode, for AskUserQuestion and ask rules.
- **Turn end.** An SDK `result` message (~lines 2725, 4713).
- **Rollback.** Uses `resumeSessionAt` to cut the conversation at a message UUID (~line 14687).
- **Model options.** `effort`, `thinking`, `fastMode`, `contextWindow` and `ultracode` option ids
  (`apps/server/src/provider/ClaudeModelCatalog.ts`, `apps/server/src/claudeModelOptions.ts`).
  Several accounts use separate `CLAUDE_CONFIG_DIR` paths (`docs/user/providers-claude.md`).
- **Tag:** must.

### 3.5 Other providers (all `later` for a Claude- and Codex-first emulation)

| Provider | Protocol, as found | Files |
| --- | --- | --- |
| Cursor | `@cursor/sdk` 1.4.1 | `packages/provider-cursor/src/server/` |
| OpenCode | `@opencode-ai/sdk` (1.x: one T3-managed server per thread) and `@opencode/client` 2.0.28 (2.x: one server per instance) | `packages/provider-opencode/src/server/`, `v2/` |
| Pi | the user's `pi` in RPC mode | `packages/provider-pi/src/server/rpc.ts`, `mcpInjection.ts` |
| Kiro | ACP: `kiro acp --agent-engine=v3 --auth-method=cli` | `packages/provider-kiro/src/server/acpSupport.ts` |
| Grok Build | ACP: `agent stdio` with permission flags | `packages/provider-grok/src/server/acpSupport.ts` |
| fx | ACP (`acp`) | `packages/provider-fx/src/server/acpSupport.ts` |
| Antigravity | ACP runtime | `apps/server/src/orchestration-v2/Adapters/AntigravityAdapterV2.ts`, `apps/server/src/provider/acp/AntigravityAcpSupport.ts` |
| Muse Code | `@muse-code/sdk` 1.4.6 | `packages/provider-muse/src/server/` |
| Generic ACP / ACP Registry | ACP v1 and v2-preview | `packages/provider-acp/`, `packages/provider-acp-registry/`, `packages/effect-acp/` |

### 3.6 Turn lifecycle (all providers)
1. The client sends `message.dispatch` with a `dispatchMode`:
   - `start_immediately`
   - `queue_after_active`
   - `steer_active {targetRunId}`
   - `restart_active {targetRunId}`
   - `defer_start`
2. The orchestrator writes the events and an outbox effect in one transaction.
3. The effect worker opens or resumes the provider session (`ProviderSessionManager.ts`). It then
   calls `startTurn` (`ProviderTurnStartService.ts`, `RunExecutionService.ts`).
4. The adapter streams events. `ProviderEventIngestor.ts` and `ProviderCommitBatch.ts` turn them
   into saved events.
5. `turn.terminal` ends the provider turn. `RunFinalizationService.ts` then captures the
   checkpoint and settles the run.
6. Interrupt: the `run.interrupt` command → `ProviderTurnControlService.ts` →
   `adapter.interruptTurn`. If the provider does not confirm in time, the item is marked
   `unresponsiveAt` and the UI offers **Force stop** (`orchestration.forceStopThread`,
   `thread.interrupt.unresponsive`).
- **Tag:** must.

---

## 4. Server, event store, streaming to clients

### 4.1 Commands, events, projections
- **Commands:** about 70 `OrchestrationV2Command` types in
  `packages/contracts/src/orchestrationV2.ts` (~line 3137). The full list:
  - thread state: `thread.create`, `thread.stop`, `thread.archive`, `thread.unarchive`,
    `thread.delete`, `thread.settle`, `thread.auto-settle`, `thread.unsettle`, `thread.snooze`,
    `thread.unsnooze`, `thread.auto-settle.set`, `thread.pin`, `thread.unpin`, `thread.tag.set`,
    `thread.pin.reorder`, `thread.active.reorder`, `thread.visit`, `thread.mark-unread`,
    `thread.metadata.update`;
  - links: `thread.pull-request.link/unlink/watch/sync`, `thread.pull-request-link.sync`,
    `thread.issue.link/unlink`;
  - titles and modes: `thread.title.regeneration.complete`, `thread.runtime-mode.set`,
    `thread.interaction-mode.set`, `thread.model-selection.set`;
  - sessions and messages: `provider-session.detach`, `message.dispatch`,
    `prepared-run.release/progress/fail/retry`, `notification.delivery.accept`;
  - runs and queue: `subagent.stop`, `run.interrupt`, `queued-message.promote-to-steer`,
    `queue.resume`, `queued-run.start/reorder/cancel/edit`;
  - requests: `runtime-request.respond`, `thread.user-input.dismiss`;
  - rollback and lineage: `checkpoint.rollback`, `checkpoint.rollback.fail`, `thread.fork`,
    `thread.merge_back`, `delegated_task.*`, `thread.created.record`, `provider.switch`;
  - guests: `thread.guest-share.*`, `thread.guest-message.*`;
  - other: `thread.project.adopt-folder`, `thread.pull-request-watch.*`,
    `runtime-request.delivery.fail`, `thread.interrupt.unresponsive`,
    `thread.background-work.settle`, `secret_request.record`, `link_request.record`.
- **Domain events** (`OrchestrationV2DomainEvent`, ~line 2037):
  - thread: `thread.created`, `thread.pull-request-link-updated`, `thread.visit-recorded`;
  - run: `run.created`, `run.updated`, `run.background-work-cancelled`,
    `run-attempt.created`, `run-attempt.updated`;
  - graph: `node.updated`, `subagent.updated`;
  - provider: `provider-session.detached`, `provider-thread.updated`, `provider-turn.updated`,
    `runtime-request.updated`;
  - content: `message.updated`, `turn-item.updated`, `plan.updated`;
  - checkpoints: `checkpoint-scope.created`, `checkpoint.captured`,
    `checkpoint.rollback-requested`;
  - context: `context-handoff.updated`, `context-transfer.created`, `context-transfer.updated`.
- **Pipeline** (`docs/internals/overview.md`, "Durable intent and side effects"):
  - `apps/server/src/orchestration-v2/Orchestrator.ts` runs commands one at a time. It decides
    events and does no I/O.
  - `EventSink.ts` commits, **in one SQLite transaction**: the events, the saved projections, the
    command receipt, and the outbox effects.
  - Subscribers get events after the commit.
  - `EffectWorker.ts` / `EffectOutbox.ts` do the side effects (provider calls, checkpoints) and
    feed results back.
  - A command acknowledgement means "intent committed", not "work done".
  - Command ids are idempotent through receipts (`CommandReceiptStore.ts`, table
    `orchestration_v2_command_receipts`).
- **Projection tables:** `orchestration_v2_projection_threads`, `_runs`, `_run_attempts`,
  `_nodes`, `_messages`, `_turn_items`, `_plans`, `_checkpoints`, `_checkpoint_scopes`,
  `_provider_sessions`, `_provider_threads`, `_provider_turns`, `_runtime_requests`,
  `_subagents`, `_context_handoffs`, `_context_transfers`, and others. Events are in
  `orchestration_v2_events`. The outbox is in `orchestration_v2_effect_outbox`. Migrations live in
  `apps/server/src/persistence/Migrations/` (130 files).
- **Tag:** must (the pattern); the exact schema = reference only.

### 4.2 Persistence technology
- **SQLite** through Node's built-in `node:sqlite` (`@t3tools/shared/nodeSqliteClient`), with
  `PRAGMA journal_mode = WAL` and `synchronous = NORMAL` and a 128 MiB WAL limit
  (`apps/server/src/persistence/Sqlite.ts`, verified).
- **Files** (`apps/server/src/config.ts` `deriveServerPaths`):
  - The database is `<baseDir>/userdata/statev2.sqlite`. Docs give baseDir as `~/.t3`; it can be
    overridden with `T3CODE_HOME`.
  - Also in `userdata`: `settings.json`, `keybindings.json`, `attachments/`, `logs/`
    (`provider/events.log`, `terminals/`), `secrets/`.
  - Worktrees go in `<baseDir>/worktrees`. Caches go in `<baseDir>/caches`.
- **Raw provider frames** go to rotating NDJSON logs, not the database
  (`apps/server/src/provider/EventNdjsonLogger.ts`, `docs/internals/providers.md`,
  "Provider diagnostics"). There is a 64 KiB budget per payload.
- **Retention / replay:** the event log keeps about a day of history.
  `ProjectionMaintenance.ts` deletes older events that the projections already cover. So
  **projections cannot be rebuilt from the log**; the projections are the source of truth. It
  keeps the newest event per stream, `thread.created`, `running` run updates and prompt anchors.
  A client whose cursor falls below the pruned point resumes from a snapshot.
- **Tag:** must (an event log + projections + idempotent commands); the pruning details = should.

### 4.3 Client transport
- **WebSocket** at path `/ws` (`apps/server/src/ws.ts` ~line 3643). It uses **Effect RPC**
  (`effect/rpc` `RpcServer` / `RpcGroup`) with **JSON serialisation** (`RpcSerialization.layerJson`;
  `ws.ts` line 3716 and `packages/client-runtime/src/rpc/session.ts` line 287).
- Method names are in `WS_METHODS` (`packages/contracts/src/rpc.ts`). The orchestration ones are:
  - `orchestration.dispatchCommand`, `orchestration.launchThread`;
  - `orchestration.subscribeShell` (sidebar), `orchestration.subscribeThread` (detail),
    `orchestration.subscribeCommandOutput`, `orchestration.subscribePromptSuggestion`;
  - `orchestration.getThreadProjection`, `getTurnDiff`, `getFullThreadDiff`, `getTurnItem`;
  - `orchestration.searchThreads` / `searchThread`, `orchestration.forceStopThread`, and others.

  Other method groups: `terminal.*`, `vcs.*`, `git.*`, `review.*`, `projects.*`, `server.*`,
  `pullRequests.*`, `issues.*`, `provider.*`, `preview.*`, `device.*`, `scheduledTasks.*`.
- **Snapshot-plus-cursor.**
  - `subscribeThread {threadId, afterSequence?, …}` returns `{kind:"snapshot", snapshotSequence,
    projection, historyCursor, hasMoreHistory}`, then `{kind:"event"}` items with sequence greater
    than the snapshot, then `{kind:"synchronized"}`.
  - The shell stream items are `synchronized | snapshot | project.updated | project.removed |
    thread.updated | thread.removed`.
  - Every committed event has a store-assigned increasing `sequence` (core-graph doc).
- **HTTP** (`apps/server/src/http.ts`) is used for the first shell snapshot (so it loads before the
  socket), attachment uploads (signed upload URLs from `attachments.createUploadUrl`), asset and
  media serving, auth, and `/mcp`.
- **Reconnect** (`docs/internals/connection-runtime.md`):
  - `packages/client-runtime/src/connection/supervisor.ts` is the one owner of retries. It uses
    jittered exponential backoff capped at 5 minutes.
  - On foreground or network change it probes the socket. Probe timeouts are 3 s on LAN and 15 s
    elsewhere.
  - Subscriptions resume with `afterSequence`, or take a new snapshot.
  - Mutations are **not** replayed automatically.
  - The thread cache plus cursor is kept for 5 idle minutes (`packages/client-runtime/src/state/threads.ts`).
- **Tag:** must (stream + resume from cursor); should (HTTP shell prefetch, adaptive probes).

---

## 5. Sessions that outlive the window; resume; server restart

- **Server outlives the UI.**
  - The server is its own process. The desktop app starts a bundled one, and closing the last
    desktop window stops that local server (`docs/user/remote-access.md`).
  - `t3 service install` runs it as a per-user background service (`docs/user/background-service.md`,
    `apps/server/src/serviceLauncher.ts`). Agents then keep running with no client connected.
  - Tag: must (in Anima the daemon plays this part).
- **Provider sessions are saved state.**
  - `ProviderSession`, `ProviderThread` and the native resume refs (Codex thread id, Claude
    session id) are stored. The live process handle is not.
  - After a crash, restart or idle release, the session is recreated through
    `ProviderSessionManager` (core-graph doc, "ProviderSession").
  - Codex resumes with `thread/resume`. Claude resumes with `resume: <sessionId>` (and
    `resumeSessionAt`).
  - Tag: must.
- **After a server restart.**
  - `ProviderRuntimeRecoveryService.ts`, `RestartContinuation.ts` and `RestartBackgroundNote.ts`
    rebuild the state of interrupted runs.
  - A prompt the provider never accepted stays in the held queue for the user, or is sent again
    when an agent or scheduler sent it.
  - The setting **Continue threads after restarts** (`continueThreadsAfterServerUpdate`, on by
    default) resumes supported active threads after an update, crash or reboot.
  - It does **not** resume threads that wait on approval, answer or secret, or that are archived,
    snoozed or settled. It stops after 3 crash recoveries in a row (`docs/user/updating.md`).
  - Live approvals are lost. Codex async questions survive.
  - Queued messages are kept and **held** until **Resume** (`docs/user/composer.md`).
  - Tag: should.
- **Idle release.** Sessions are unloaded when idle (`provider-session.detach`; the Codex launch
  arg `thread_unload_delay_secs=0` because T3 owns unloading). Tag: should.
- **Usage limits.** "Resume at reset" schedules a continuation (`UsageLimitRecoveryWorker.ts`,
  setting `autoResumeLimitedThreads`). Tag: later.

---

## 6. Git

### 6.1 Worktree per thread
- **When created.** When the user picks **New worktree**, or the project default
  `defaultThreadEnvMode` is `worktree`, it is made at thread launch, before the first turn, by
  `apps/server/src/orchestration-v2/ThreadLaunchService.ts`. Default `ThreadEnvMode` values are
  `local | worktree` (`packages/contracts/src/environment.ts`). A new project with no threads yet
  uses its folder even when the default is worktree (doc).
- **Base branch.**
  - Picked in the composer (`mod+shift+g`, `WorktreeBaseBranchPicker.tsx`).
  - With `newWorktreesStartFromOrigin` it fetches and uses `refs/remotes/<remote>/<branch>`.
  - There is a fallback base when the ref is missing (`ThreadLaunchService.ts` ~lines 650–720).
- **Naming.**
  - It starts with a temporary branch `t3/<8 hex>` (`buildTemporaryWorktreeBranchName`,
    `packages/shared/src/git.ts`; prefix `WORKTREE_BRANCH_PREFIX = "t3"`).
  - It is renamed after a model writes a name from the first message (`generateBranchName`).
  - `branchNamingMode` can be `static` (prefix, default `t3/`), semantic (model picks `feat/`,
    `fix/`), or `custom` (instructions).
  - The folder is `<worktreesDir>/<repoName>/<branch prefix up to 23 bytes>-<16 hex of
    sha256(gitCommonDir + branch)>`, with `-2`, `-3`… if taken (`createWorktree`,
    `apps/server/src/vcs/GitVcsDriverCore.ts` ~lines 5335–5375). It runs `git -c
    checkout.workers=N worktree add [--no-checkout] [--lock --reason …] -- <path> <ref>`.
  - Submodules are set up recursively by default (`worktreeSubmodules`). Sparse patterns come
    from `t3.json` `worktreeSparsePatternsPath`. There is an optional APFS copy-on-write checkout
    cache on macOS (`apps/server/src/vcs/worktreeCheckoutCache.ts`).
  - Progress stages (fetch, checkout, submodules, setup scripts) stream to the UI
    (`subscribeWorktreeSetup`, `WorktreeSetupCard.tsx`).
- **Cleanup.**
  - Off by default. Policies (`StorageCleanupSettings`): `worktreeAfterDays`,
    `worktreeSettledAfterDays`, `worktreeOnMerge`, `worktreeOnDelete`, `worktreeUnchanged`, and
    `worktreeKeepWhen` (protect uncommitted changes, any local files, or edited tracked files).
  - Runs at startup, on change and hourly (`apps/server/src/storageCleanup.ts`,
    `apps/server/src/vcs/worktreeRemoval.ts`, which uses `git worktree remove --force --force`).
  - Only T3-managed worktrees are touched. Branches and history are kept.
- **Reuse.** **New thread in this worktree**, `mod+shift+l` (previous worktree). RPCs
  `vcs.createWorktree` / `vcs.removeWorktree`.
- **Tag:** should (worktree per thread with base branch and auto name); later (cleanup policies,
  CoW cache, sparse).

### 6.2 Checkpoints
- **What.** After each root run, the workspace state is captured as a **hidden Git commit pointed
  to by a private ref**. This uses neither stash nor commits on the user's branch.
- **Ref name:** `refs/t3/orchestration-v2/checkpoints/<base64url(scopeKey)>/ordinal/<n>`
  (`apps/server/src/orchestration-v2/CheckpointService.ts` lines 28 and 164). Older refs use
  `refs/t3/checkpoints/…` (`storageCleanup.ts`).
- **Capture** (`apps/server/src/vcs/GitVcsDriver.ts` `checkpoints.captureCheckpoint`, ~line 1113,
  verified):
  1. Use a scratch index (`GIT_INDEX_FILE`) seeded with `read-tree --reset HEAD`.
  2. `git add -A` (or `-u`) the workspace.
  3. `write-tree`, then `commit-tree`, with author and committer "T3 Code
     <t3code@users.noreply.github.com>".
  4. `update-ref <checkpointRef> <commit>`.
  5. Objects go to a private object directory with alternates, so cleanup cannot remove another
     writer's objects.
  6. Sparse and skip-worktree cases are handled.
- **Services:** `CheckpointCaptureService.ts`, `CheckpointService.ts`,
  `apps/server/src/checkpointing/CheckpointStore.ts`. Captures run after the turn ends and do not
  extend the turn's recorded duration (overview doc).
- **Diff.** Two checkpoint refs are compared (`diffCheckpoints`, patch or numstat).
  `listAuthoredPaths` leaves out files that Git brought in by pull, merge or rebase during the
  turn (`docs/user/source-control.md`, "Review turn changes").
- **Tag:** must (per-turn diff) and should (restore).

### 6.3 Undo / revert ("Edit from here")
- **Sees/does.** **Edit from here** under a sent message rewinds the conversation to before it.
  Two choices:
  - **Revert and keep changes**: conversation only.
  - **Revert files too**: restores the checkpoint.

  The prompt goes back into the composer. File restore is offered only for threads in a worktree.
  It is refused if another thread or session uses an overlapping directory.
- **Underneath.**
  - Command `checkpoint.rollback` → `CheckpointRollbackService.ts`, with a safety check in
    `CheckpointRestoreSafety.ts`.
  - Restore runs `git restore --source <commit> --worktree --staged -- .`, then `git clean -fd`,
    repeated until stable (`GitVcsDriver.ts` ~lines 1843–2000).
  - The provider conversation is rolled back too: Codex `thread/revert {threadId, beforeTurnId}`;
    Claude via `resumeSessionAt`.
  - A provider that cannot roll back its conversation (Antigravity) must refuse **before** files
    change (`docs/internals/providers.md`).
  - Runs become `rolled_back`.
- **Tag:** should.

### 6.4 Commit, push, pull request, merge
- **Sees/does.** Git actions in the thread:
  - commit (a model writes the message);
  - push;
  - create PR (a model writes title and body from the diff and the thread);
  - commit + push + PR in one step.

  Also a full Pull requests page (review, comment, request reviewers, merge, auto-merge, GitHub
  stacks), linked PRs per thread, PR watching (`watch_pull_request` checks every 2 minutes and
  wakes the agent), and an Issues page.
- **Underneath.**
  - RPC `git.runStackedAction` with `GitStackedAction` = `commit | push | create_pr |
    commit_push | commit_push_pr` and phases `branch | commit | push | pr`
    (`packages/contracts/src/git.ts`).
  - Code: `apps/server/src/git/GitWorkflowService.ts`, `GitManager.ts`,
    `linkCreatedPullRequest.ts`.
  - Forge drivers: `packages/source-control-core` (interfaces `SourceControlProvider`,
    `PullRequestProvider`, `IssueTracker`) and `packages/source-control-github`.
  - GitHub talks to the REST and GraphQL API directly. The token comes from settings, then
    `GH_TOKEN`, then `gh auth` (`GitHubCredentials.ts`, `GitHubRest.ts`, `gitHubGraphQlClient.ts`).
  - Other forges: GitLab (`glab`), Forgejo/Gitea (`fj`/`tea`), Bitbucket, Azure DevOps, GitCafe,
    Cursor Origin (`packages/source-control-*`).
  - Auto PR linking when an agent runs `gh pr create` (`PullRequestSyncReactor.ts`,
    `pullRequestWatch.ts`).
- **Tag:** should (commit, push, create PR on GitHub); later (PR review page, merge, stacks,
  watches, other forges, issues).

### 6.5 Branch status and pull
- Live branch status (`subscribeVcsStatus`, `subscribeVcsBranch`). Optional auto-fetch
  (`automaticGitFetchInterval`). `vcs.pull`, auto-pull (`defaultAutoPull`). Branch switching and
  creation (`vcs.switchRef`, `vcs.createRef`). Tag: should.

---

## 7. Terminals, scripts and actions

- **Integrated terminal per thread.**
  - Toggle with `mod+j`. New terminal `mod+n` and split `mod+d` / `mod+shift+d` while a terminal
    is focused. Close with `mod+w`.
  - Terminals are keyed by `threadId` + `terminalId` (`packages/contracts/src/terminal.ts`).
  - The **server owns the PTYs** through **`node-pty`** (`apps/server/package.json` line 70;
    `apps/server/src/terminal/Manager.ts`).
  - Scrollback is kept on the server: 5,000 lines and 8 MiB per terminal, saved to
    `logs/terminals`. Clients attach and replay. Terminal queries and replies are stripped from
    saved history.
  - Rendering: web and Android use `libghostty-vt` (WASM on the web,
    `apps/web/src/terminal/ghostty/core.ts`); iOS uses GhosttyKit
    (`docs/internals/terminal-runtime.md`).
  - Shell from `T3CODE_TERMINAL_SHELL`, or the platform default (`docs/user/terminal.md`).
  - RPCs: `terminal.open/attach/observe/write/resize/clear/close`, `subscribeTerminalMetadata`.
    Events: `started | output | exited | closed | error | cleared | restarted | activity`.
  - Terminal excerpts can be attached to a prompt as chips.
  - Tag: should (a terminal per thread); later (splits, saved scrollback across restarts).
- **Project scripts ("actions").**
  - Per-project commands with a name, an icon and a key binding (`script.{id}.run`).
  - Fields (`ProjectScript`): `id`, `name`, `command`, `icon`, `runOnWorktreeCreate`,
    `runOnSettle`, `stopOnSettle`, `async`, `previewUrl`, `autoOpenPreview`.
  - Can be imported from a repo `t3.json` (`packages/contracts/src/t3ProjectFile.ts`).
  - Run in a thread terminal. Setup scripts run on worktree creation.
  - Settings: `defaultProjectScripts`, `projectScriptOverrides`.
  - Tag: should (setup script on worktree create); later (other script hooks).
- **Local servers panel.** The thread details card lists local servers started by the thread's
  terminals or by the Claude agent, with **Stop** (`subscribeDiscoveredLocalServers`,
  `preview.stopLocalServer`, `ThreadServersPanel.tsx`). Tag: later.
- **Preview browser / devices.** An embedded browser (`preview.*`), iOS and Android device panels
  (`device.*`), SnapShot screen capture. Tag: later.

---

## 8. Keyboard model

`mod` = Cmd on macOS, Ctrl elsewhere. Rules are `{key, command, when?}` in
`~/.t3/userdata/keybindings.json`. Later rules win. Defaults are added on startup without
overwriting the user's rules (`apps/server/src/keybindings.ts`). Default list: `DEFAULT_KEYBINDINGS`
in `packages/shared/src/keybindings.ts` (verified). Command ids: `packages/contracts/src/keybindings.ts`.

**When-context keys:** `terminalFocus`, `terminalOpen`, `previewFocus`, `previewOpen`,
`rightPanelOpen`, `modelPickerOpen`, `usagePageOpen`, `composerFocus`, `draftThreadRoute`,
`turnRunning`, `editableFocus`, `isWeb`, `isDesktop`. Operators: `!`, `&&`, `||` and parentheses.

**Default bindings** (from source):

| Key | Command | When |
| --- | --- | --- |
| mod+b | sidebar.toggle | |
| f6 / mod+f6 | workspace.focusNext | |
| shift+f6 / mod+shift+f6 | workspace.focusPrevious | |
| mod+[ / mod+] | navigation.back / navigation.forward | !terminalFocus |
| mod+j | terminal.toggle | |
| mod+alt+b | rightPanel.toggle | |
| mod+shift+b | threadPanel.toggle | !terminalFocus |
| mod+t | rightPanel.new | rightPanelOpen && !terminalFocus |
| mod+d | terminal.split | terminalFocus |
| mod+shift+d | terminal.splitVertical | terminalFocus |
| mod+shift+d | composer.dictation | !terminalFocus |
| mod+n | terminal.new | terminalFocus |
| mod+w | terminal.close | terminalFocus |
| mod+w | rightPanel.close | !terminalFocus |
| mod+shift+t | view.reopenClosed | |
| mod+d | diff.toggle | !terminalFocus |
| mod+shift+j | preview.toggle | |
| mod+r / mod+l | preview.refresh / preview.focusUrl | previewFocus |
| mod+= / mod++ / mod+- / mod+0 | preview.zoomIn / zoomIn / zoomOut / resetZoom | previewFocus |
| mod+f | preview.find | previewFocus |
| mod+k | commandPalette.toggle | !terminalFocus |
| mod+p | filePicker.toggle | !terminalFocus |
| mod+shift+f | projectSearch.toggle | !terminalFocus |
| mod+u | usage.open | !terminalFocus |
| mod+alt+a | theme.select | !terminalFocus |
| mod+alt+shift+a | appearance.cycle | !terminalFocus |
| mod+alt+shift+t | themeEditor.toggle | |
| mod+s | composer.stash | !terminalFocus |
| mod+shift+enter | thread.steerQueuedMessage | !terminalFocus |
| alt+arrowup | thread.editQueuedMessage | composerFocus |
| mod+enter | composer.sendAlternate (opposite of queue/steer) | composerFocus && turnRunning |
| mod+enter, mod+alt+enter | composer.sendBackground | composerFocus && draftThreadRoute |
| mod+alt+enter | composer.sendAndNewThread | composerFocus && !draftThreadRoute |
| mod+n / mod+shift+o | chat.new | !terminalFocus |
| mod+shift+n | chat.newLocal | !terminalFocus |
| mod+alt+n | chat.newWithoutProject | !terminalFocus |
| mod+f | chat.find | !terminalFocus && !previewFocus |
| mod+shift+m | modelPicker.toggle | !terminalFocus |
| mod+shift+h | composer.host | !terminalFocus |
| mod+shift+e | composer.effort | !terminalFocus |
| mod+shift+a | composer.mode (permissions) | !terminalFocus |
| mod+shift+x | composer.workspace | !terminalFocus |
| mod+shift+g | composer.branch | !terminalFocus |
| mod+shift+l | composer.previousWorktree | !terminalFocus |
| mod+shift+k | pullRequest.copyNumber | !terminalFocus |
| mod+shift+arrowup / arrowdown | modelPicker.previousProvider / nextProvider | modelPickerOpen |
| mod+o | editor.openFavorite | |
| mod+shift+[ / mod+shift+] | thread.previous / thread.next | |
| mod+shift+c | thread.copyReference | !terminalFocus |
| mod+shift+s | thread.settle | !terminalFocus |
| mod+shift+p | thread.pin | !terminalFocus |
| mod+z | thread.undo | !terminalFocus && !editableFocus (doc: also right after an action in a text field) |
| c / t / l | usage.cost / usage.tokens / usage.limits | usagePageOpen |
| mod+shift+1..4 | usage.period.day / week / month / quarter | usagePageOpen |
| mod+1..9 | jump to thread N | !modelPickerOpen && isDesktop |
| mod+1..9 | jump to model N | modelPickerOpen && isDesktop |

- **Unbound by default:** `thread.stop`, `thread.regenerateTitle`, `composer.cycleHost`,
  `composer.increaseEffort`, `composer.decreaseEffort`, `rightPanel.toggleMaximized`, and
  `script.{id}.run`.
- **Composer keys** that are not rebindable, or are rebindable only through settings
  (`docs/user/keybindings.md`, `docs/user/composer.md`):
  - Enter sends; Shift+Enter makes a newline. The `sendShortcut` setting is `enter |
    mod-enter-multiline | mod-enter`.
  - Up / Down in an empty composer recalls earlier prompts.
  - `mod+shift+v` pastes large text without turning it into an attachment.
  - Esc cancels dictation or dismisses a suggestion. Tab accepts a suggestion.
  - Inside the model picker: Left / Right / Up / Down navigation.
  - `mod+f` in a diff (not configurable).
  - Desktop quit uses Cmd/Ctrl+Q with a 1.2 s hold or double press (`confirmQuit`).
- **Command palette** (`mod+k`):
  - Searches commands, projects and threads across environments: title, branch, PR, project, then
    "In messages" full-text matches.
  - Ranking uses how often and how recently a thread was opened.
  - `>` shows actions only.
  - Actions include New project, Add project, Link pull request, Restart agent session, Pin
    project, New thread in…, Open issues.
  - Code: `apps/web/src/components/CommandPalette*.tsx`, `commandPaletteStore.ts`,
    `packages/shared/src/paletteSearch.ts`; RPC `orchestration.searchThreads`.
- **Tag:** must (send, newline, stop, new thread, palette basics); should (most of the table);
  later (preview, usage, theme keys).

---

## 9. Settings (schema: `packages/contracts/src/settings.ts`)

Settings come in two layers:

- **Client settings** (`ClientSettingsSchema`, line 412) stay on each device.
- **Server settings** (`ServerSettings`, line 1376) are stored in `userdata/settings.json` on each
  environment. They can be overridden per project (`ProjectSettingsOverrides`, line 1221). The
  order is: project override → environment → `t3.json` → built-in default
  (`docs/user/project-settings.md`).
- The settings UI can edit "All environments" at once as a bulk edit (`docs/internals/overview.md`).

**Client settings** (all fields, grouped):

- Notifications: `notificationMode` (`off | notifications | sound | notifications-and-sound`;
  default `off`), `inAppNotificationsEnabled`.
- Layout and appearance: `chatWidth` (`comfortable | wide | full`), `effortDisplay`
  (`label | bars`), `appearanceContrast`, `panelAnimationDurationMs`, `glassOpacity`.
- Fonts: `fontSizeInterface`, `fontSizePrompt`, `fontSizeCode`, `fontSizeTerminal`,
  `fontFamilyCode`, `fontFamilyComposer`, `fontFamilySans`, `fontFamilyTerminal`,
  `fontSmoothing`.
- Diffs: `diffColorScheme`, `diffLayout` (`stacked | split`), `diffFilesCollapsed`,
  `diffIgnoreWhitespace`, `wordWrap`, `mathRenderingEnabled`.
- Load balancing: `loadBalancingEnabled`, `loadBalancingWeights`.
- Embedded browser: `browserDefaultViewport`, `browserDefaultZoomFactor`,
  `browserDefaultAppearance`, `browserRecording*`, `browserLinkTarget`, `browserSearchEngine`,
  `browserCustomSearchUrl`, `browserAutoShowFloatingPreview`, `browserProfiles`,
  `browserDefaultProfileId`.
- Confirmations: `confirmQuit`, `confirmThreadArchive`, `confirmTerminalClose`,
  `confirmThreadDelete`, `confirmThreadUnpin`.
- Environments: `environmentIdentificationMode`.
- Onboarding and composer: `onboardingCompletedAt`, `persistComposerContextStrip`, `favorites`,
  `providerModelPreferences`, `composerRichTextEnabled`, `sendShortcut`, `followUpBehavior`
  (`queue | steer`), `showSkillsInSlashMenu`.
- Pull requests: `pullRequestMergeMethodOverrides`, `pullRequestLinksOpenInApp`.
- Panels and timeline: `sideThreadOpenTarget` (`panel | main`), `proactivePanelsEnabled`,
  `toolCallVisibility`.
- Accessibility: `screenReaderAnnouncements`, `screenReaderWindowTitle`.
- Thread details: `threadDetailsUsageEnabled`, `threadDetailsItems`.
- Sidebar: `boringSidebarEnabled`, `boringSidebarMultilineRows`, `sidebarWorkingShelfEnabled`,
  `sidebarProjectGroupingMode`, `sidebarProjectGroupingOverrides`, `sidebarProjectSortOrder`,
  `sidebarThreadSortOrder`, `timestampFormat`.
- Voice: `voiceMicrophone(s)`, `voiceShortcutMode`, `speech*` (language, translate, custom words,
  silence filtering, streaming, filler removal, post-processing prompt, correction word),
  `language`.
- SnapShot: `snapShot*` (enabled, accessibility, shortcut, sound, flash, animations).
- Cost rates: `inputCostPerMillionTokens`, `outputCostPerMillionTokens`,
  `cacheReadCostPerMillionTokens`, `cacheWriteCostPerMillionTokens`.

**Server settings** (all fields, grouped):

- Model defaults: `favorites`, `providerModelPreferences`, `defaultModelSelection`,
  `fallbackModelSelection`, `defaultRuntimeMode`.
- Worktrees and storage: `worktreeCleanup`, `storageCleanup`, `worktreesDirectory`,
  `previousWorktreesDirectories`, `fastWorktreeCheckout`, `defaultThreadEnvMode`,
  `newWorktreesStartFromOrigin`, `worktreeSubmodules`.
- Streaming and maintenance: `responseStreamingMode` (`turn | paragraph`, default `paragraph`),
  `enableProviderUpdateChecks`, `continueThreadsAfterServerUpdate`, `keepAwakeWhileAgentsWork`.
- Access and devices: `requireConnectDeviceApproval`, `enableAgentBrowserAccess`,
  `projectAgentBrowserAccessOverrides`, `enablePreviewTunnel`, `enableAgentDeviceAccess`,
  `enableDeviceSupport`, `deviceOnboardingCompleted`, `deviceHosts`, `easDeviceHosts`,
  `simlockDeviceHosts`, `simlockDeviceHostsEnabled`.
- Speech: `cloudSpeechCredentials`, `speechModelId`, `speechAcceleration`,
  `speechModelUnloadTimeout`, `speechProjectCustomWords`, `speechPostProcessingModelSelection`.
- Projects: `defaultAutoPull`, `projectAutoPullOverrides`, `defaultProjectScripts`,
  `projectScriptOverrides`, `projectSettingsOverrides`, `projectSettingsFolded`,
  `addProjectBaseDirectory`, `ghqCloneLayout`.
- Thread behaviour: `sidebarAutoSettleAfterDays`, `sidebarAutoSettleOnMerge`,
  `sidebarAutoSettleKeepPullRequests`, `compactIdleThreads`, `idleCompaction`,
  `snoozeLimitedThreads`, `autoResumeLimitedThreads`, `snoozePresets`, `hiddenThreadTags`,
  `hideTaggedThreads`, `guestSharingEnabled`, `linkedEnvironmentsEnabled`,
  `promptSuggestionsEnabled`.
- Background: `backgroundActivity`, `backgroundActivityProfile`, `automaticGitFetchInterval`,
  `providerHealthRefreshInterval`.
- Theme and identity: `defaultTheme`, `defaultAppearance`, `environmentIcon`.
- Text generation: `textGenerationModelSelection`, `threadTitleInstructions`.
- Branch naming: `branchNamingMode`, `branchNamePrefix`, `branchNameInstructions`.
- Source control: `removeAgentCreditsOnMerge`, `sourceControlWritingStyle`,
  `sourceControlWriterModelSelection`, `reviewGuidesEnabled`, `reviewGuideModelSelection`,
  `reviewGuideInstructions`, `pullRequestMergeMethod`, `sourceControlHosts`,
  `offeredSourceControlProviders`.
- Providers and integrations: `providerInstances`, `observability`, `usageLimitSources`,
  `textGenerators`, `secretProviders`.
- Usage: `usageHistoryRetentionDays`, `usagePriceOverrides`, `usageModelAliases`,
  `usageModelFilter`.

**Provider instance settings** (`makeProviderSettingsSchema`, ~lines 640–900): `enabled`,
`binaryPath`, `homePath` (`CODEX_HOME` / `CLAUDE_CONFIG_DIR`), `shadowHomePath`, `launchArgs`,
environment variables (values can be marked sensitive or be secret references), `customModels`,
`autoCompactWindow`, `idleCompaction`, `authMethod`, `apiKey`, `setupMode`.

**Keybindings** are a separate file (section 8).

**Tag:** must (default model, default permission mode, provider binary path and environment,
send shortcut, follow-up behaviour, worktree on/off); should (notifications, branch naming,
streaming mode, diff layout, auto-settle); later (the rest).

---

## 10. Model, effort and provider pickers; fast mode

- **Sees/does.** One combined picker (`mod+shift+m`). It has a provider list on the left
  (instances, so several accounts each show) and model search. Right arrow opens the selected
  model's effort levels.
  - Effort picker `mod+shift+e`. Permissions picker `mod+shift+a`. Host (machine) picker
    `mod+shift+h`.
  - Speed / fast toggle (Codex "Fast" / "Ultrafast" service tiers; Claude `fastMode`).
  - Claude extras: thinking on/off, context window, `ultracode`. Codex extra: Cyber Access
    program.
  - Custom models can be added per provider instance.
  - Changing the model mid-thread works. Changing provider mid-thread makes a "handoff" item.
- **Underneath.**
  - `ModelSelection = {instanceId, model, options?}` (`packages/contracts/src/modelSelection.ts`).
    `options` is a list of option-id/value pairs, described by each model's
    `optionDescriptors` from the provider catalog.
  - Catalogs: `apps/server/src/provider/ClaudeModelCatalog.ts`, `codexModelCatalog.ts`,
    `model-manifest.json`, `ModelManifest.ts`; `docs/internals/model-manifest.md`.
  - Commands: `thread.model-selection.set`, `provider.switch` (`ProviderSwitchService.ts`,
    `ContextHandoffService.ts`, `docs/user/portable-handoffs.md`).
  - In-session model switch: Codex per turn; Claude `setModel`, though thinking changes need a new
    process (`ClaudeAdapterV2.ts` ~line 2147).
  - UI: `apps/web/src/components/chat/ProviderModelPicker.tsx`, `ModelPickerContent.tsx`,
    `TraitsPicker.tsx`, `TraitsSpeed.tsx`, `ComposerModelEffort.tsx`, `EffortGlyph.tsx`.
  - Leaving effort or service tier unset uses the provider's own config.
- **Tag:** must (model + effort for Claude and Codex); should (fast mode, several accounts per
  provider); later (provider switch mid-thread with handoff, multi-model send).

---

## 11. Attachments, drag and drop, paste

- **Sees/does.**
  - Up to 100 files per message. Each image up to 10 MiB, with 80 MiB of images in total. Other
    files up to 50 MiB.
  - Drag or paste images into the composer. HEIC is converted to JPEG. Large images are resized.
  - Drag files onto a sidebar thread row to attach them to that thread.
  - Paste of 32 KiB or more of text becomes a text attachment.
  - Images also get an inline chip. Files exist only as chips.
  - Thread drag-and-drop into the composer makes a thread reference, or a transcript file from
    another machine.
  - Uploads start right away and must finish before sending.
- **Underneath.**
  - Limits: `packages/contracts/src/chatAttachment.ts` (`PROVIDER_SEND_TURN_MAX_*`). Attachment
    types are `image` and `file`; there is also a pasted-text source.
  - Upload: RPC `attachments.createUploadUrl`, then HTTP upload (`apps/server/src/http.ts`).
    Storage in `userdata/attachments` (`apps/server/src/attachmentStore.ts`). Files are
    **outside the workspace** and are claimed per thread (`AttachmentClaims.ts`).
  - "A path in the prompt does not grant filesystem access" (`docs/internals/providers.md`).
  - Delivery to providers:
    - Codex: images as base64 `data:` URL input items (`CodexAdapterV2.ts` ~line 3740).
    - Claude: base64 `image` blocks and PDFs as `document` blocks (`ClaudeAdapterV2.ts` ~lines
      3261 and 3317).
    - Other files: paths added to the text (`packages/provider-core/src/server/attachmentPrompt.ts`).
  - Client: `apps/web/src/components/chat/useComposerAttachmentIntake.ts`,
    `ComposerAttachmentStrip.tsx`, `workspaceFileDrop.ts`, `folderDrop.ts`, `textPaste`
    (`packages/client-runtime/src/textPaste.ts`).
- **Tag:** must (paste / drag images); should (file attachments, large paste → file); later
  (thread drops, HEIC, share sheet).

---

## 12. Queued messages, steering, interrupts, stop

- **Follow-up behaviour.** While a turn runs, sending either **queues** the message (a new turn
  after this one) or **steers** (injects into the running turn). The default is set with
  `followUpBehavior`. `mod+enter` does the opposite for one message.
- **Queue.**
  - Queued messages are stored **on the server** as queued runs (`dispatchMode`
    `queue_after_active`). They show as dashed messages at the end of the thread.
  - Users can **Send now**, edit (pencil, `alt+up`), remove, reorder, and promote one to a steer
    (`mod+shift+enter`).
  - Commands: `queued-run.start/reorder/cancel/edit`, `queued-message.promote-to-steer`,
    `queue.resume`. Ordering logic: `QueuedRunOrder.ts`.
  - After a stop or restart the queue is **held** until **Resume** or a new message.
  - UI: `QueuedRunsControl.tsx`, `queuedMessageEdit.ts`.
- **Steer.**
  - `dispatchMode: steer_active` calls `adapter.steerTurn`.
  - Codex uses `turn/steer`.
  - Claude: the message is offered into the live query. It waits until the current tool calls
    finish, and shows "unread" (`steer.updated` `pending`) until read. If Claude drops it, it is
    sent as the next turn (`docs/user/providers-claude.md`).
  - Providers without steering use `restart_active` (interrupt and start a new attempt under the
    same run; capability `supportsSteeringByInterruptRestart`).
- **Interrupt / Stop.**
  - The composer shows **Interrupt** while the agent works and the draft is empty.
  - `thread.stop` (unbound by default) and command `run.interrupt` / `thread.stop`.
  - Stop also stops delegated subagents.
  - If the provider does not confirm, **Force stop** appears (`orchestration.forceStopThread`).
  - Stopping shows a notice listing any unread steers.
- **Resume.** **Resume** in an empty composer continues an interrupted or limited turn natively
  where supported (`resumeTurn`, `manualContinuationOfRunId`).
- **Tag:** must (stop / interrupt, queue at least client-side); should (server-side queue, steer
  for Claude and Codex, edit / reorder queue); later (force stop, resume continuation).

---

## 13. Remote access, mobile, pairing

- **Environments.**
  - Each server is an "environment" with an id (`userdata/environment-id`).
  - Clients can connect to several at once. Threads from all of them show together. A host picker
    chooses where a new thread runs. Optional load balancing across machines.
  - `packages/client-runtime/src/connection/registry.ts`, `docs/internals/remote.md`.
  - Tag: should (Anima's hub already plays this part).
- **Pairing over LAN / private network.**
  - `t3 serve --lan` or `t3 up --lan`, then `t3 pair` (a QR code or `t3code://pair` link).
  - A one-time pairing link is exchanged for a session. The DPoP-bound token lasts 1 hour and
    renews the session to 30 days on use.
  - Code: `apps/server/src/cli/pair.ts`, `apps/server/src/auth/*`,
    `apps/server/src/persistence/AuthPairingLinks.ts`, `AuthSessions.ts`;
    `docs/internals/environment-auth.md`.
  - Default port 3773 (`apps/server/src/config.ts` line 30); `T3CODE_PORT`, `T3CODE_HOST`.
  - Tag: should.
- **Tailscale.**
  - `t3 serve --tailscale-serve` or `t3 pair --tailscale` sets up `tailscale serve --https`
    (default port 443) and gives a `https://machine.tailnet.ts.net/` link.
  - `packages/tailscale/src/tailscale.ts` wraps `tailscale status --json` and `serve`.
  - Tag: later.
- **SSH (desktop-managed).**
  - Desktop **Add environment → SSH**: it downloads the server into `~/.t3/runtime` on the remote
    host, starts or reuses it, and opens a port forward. It needs `ExitOnForwardFailure=yes`,
    `ServerAliveInterval=15` and `BatchMode`, and can reuse a `ControlMaster` connection.
  - Code: `packages/ssh/src/tunnel.ts`, `command.ts`, `auth.ts`; `apps/desktop/src/ssh/`.
  - Tag: later.
- **T3 Connect** (hosted relay plus Cloudflare tunnel via `cloudflared`; the relay does not proxy
  sessions according to `docs/user/faq.md`). Supports device approval codes and webhooks.
  `apps/server/src/relay/`, `docs/internals/t3-connect.md`. Tag: later (Anima has its own hub).
- **Mobile app.**
  - `apps/mobile` (Expo / React Native). Same threads, approvals and questions.
  - Push notifications, iOS Live Activities and widgets via T3 Connect
    (`docs/user/mobile-notifications.md`).
  - Offline queueing of drafts and messages.
  - Tag: later.
- **Hosted web app.** `app.t3.codes` connects directly to an HTTPS endpoint. Tag: later.
- **Linked environments / peer links.** One server reads another's threads (`apps/server/src/peer/`).
  Tag: later.
- **Guest links.** Read-only, HMAC-signed thread sharing with proposed messages
  (`apps/server/src/guest/GuestShareService.ts`). Tag: later.

---

## 14. Everything else a user can do

| Feature | Underneath | Tag |
| --- | --- | --- |
| Thread search (palette) and find in thread | `ThreadSearch.ts`, `messageSearchIndex.ts`, `orchestration.searchThreads` / `searchThread` | should |
| File picker (`mod+p`) and project content search (`mod+shift+f`) | `projects.searchEntries`, `projects.searchContents`, `apps/server/src/workspace/` | should |
| File viewer and editor in the right panel (Markdown, HTML, PDF, ZIP, images) | `projects.readFile` / `writeFile`, `apps/web/src/components/files/` | should |
| Open in external editor (`mod+o`) | `shell.openInEditor`, `OpenInPicker.tsx` | should |
| Desktop / sound notifications when a thread finishes or needs you | client setting `notificationMode`, `ThreadNotificationCoordinator.tsx` | should |
| Commands and skills: `/` menu, `$skill` | `ComposerCommandMenu.tsx`, `composerSlashCommandSearch.ts`, `Drivers/ClaudeSkills.ts` | should |
| Restart agent session (reload skills, MCP) | palette action, `provider-session.detach` | should |
| Prompt stash (`mod+s`) | client draft store, `useComposerStash.ts` | later |
| Voice dictation | `docs/user/voice-input.md`, `apps/server/src/speech*`, `packages/client-runtime/src/voice-input` | later |
| Usage page | section 2.11 | later |
| Issue tracker (GitHub, GitLab, Azure, Forgejo, Linear) | `issues.*`, `issueTrackers.*`, `packages/issue-tracker-linear` | later |
| Scheduled tasks and webhook automations | `scheduledTasks.*`, `apps/server/src/scheduledTasks/`, `docs/user/project-settings.md` | later |
| Secret providers (1Password) and agent secret requests | `packages/secret-provider-*`, `secrets.answerRequest`, `docs/user/secrets*.md` | later |
| Outside agents over MCP (`/mcp`, OAuth) | `apps/server/src/mcp/`, `auth/McpOAuth.ts`, `docs/user/outside-agents.md` | later |
| Orchestrator MCP tools given to the agents themselves (delegate_task, threads, PR linking, preview, devices) | `docs/orchestration-v2/orchestrator-mcp-server.md`, `McpSessionRegistry.ts` | later |
| Import Claude / Codex / Pi history (welcome page) | `agentSessions.scan` / `import`, `docs/user/welcome-page.md` | later |
| Prompt links (`t3code://` drafts) | `docs/user/prompt-links.md` | later |
| Thread reference copy (`mod+shift+c`) | `threadReference.ts` | later |
| Guest sharing | section 13 | later |
| Export | **No thread export / download feature found** in docs. Only "copy message as Markdown", "copy thread link" and transcript-file drops. | n/a |
| Triage (an agent investigates T3 problems) | `server.prepareTriage`, `docs/user/triage.md` | later |
| Appearance, themes, theme editor | `docs/user/appearance.md`, `apps/server/src/environmentTheme.ts` | later |
| Embedded browser, browser profiles, cookie import | `preview.*`, `apps/desktop/src/preview`, `docs/user/browser-import.md` | later |
| Device panel (iOS Simulator / Android Emulator) | `device.*`, `docs/user/devices.md` | later |
| Managed policy (admin caps) | `docs/user/managed-policy.md`, `apps/server/src/policy/` | later |
| Telemetry (opt-out `T3CODE_TELEMETRY_ENABLED=false`) | `docs/user/telemetry.md` | n/a |
| Diagnostics / process and resource telemetry | `server.getProcessDiagnostics`, `resourceTelemetry` | later |
| CLI `t3` (serve, up, pair, service, update, auth, thread, doctor) | `apps/server/src/cli/` | should (Anima equivalent) |

---

## 15. Security model

- **Server auth** (`docs/internals/environment-auth.md`, `apps/server/src/auth/`):
  - The environment issues its own sessions.
  - Pairing links give scoped grants. Scopes are `access:read|write`, `diagnostics:read`,
    `filesystem:read|write`, `orchestration:read|operate`, `preview:operate`, `providers:manage`,
    `relay:read|write`, `review:write`, `settings:write`, `source-control:write`,
    `terminal:read|operate` (`packages/contracts/src/auth.ts`).
  - Browser cookies, bearer tokens and **DPoP** proof-bound tokens all map to one session model
    (`dpop.ts`, `SessionStore.ts`).
  - WebSocket upgrades use short-lived tickets. A DPoP ticket needs a fresh proof in the `dpop`
    query parameter.
  - **Every RPC declares a required scope**, checked by `RpcScopeAuthorization` middleware
    (`apps/server/src/auth/RpcAuthorization.ts`). Authenticating the socket does not authorise
    every method.
  - The default pairing grant is effectively full power. The doc says `terminal:operate` and
    `orchestration:operate` already amount to shell access as the server user.
- **Local MCP for agents:** a per-provider-session bearer credential scoped to environment,
  thread, instance and session. It expires when idle and on release, and is never saved
  (`McpSessionRegistry.ts`). Outside MCP clients use OAuth with PKCE and get either read-only
  access or a runtime-mode ceiling (`McpOAuth.ts`, `mcp/McpToolAccess.ts`).
- **How approvals are bound:**
  - Each approval is a `RuntimeRequest` with app id `requestId`, tied to its `nodeId`,
    `providerTurnId` and `nativeRequestRef`. Its `responseCapability` is
    `{type:"live", providerSessionId}`.
  - It can be answered only while that exact provider session is live. Otherwise it is
    `not_resumable` / expired, or `{type:"message"}` for Codex async questions.
  - The answer is saved before delivery. The same command id returns the same receipt and never
    answers twice (`docs/internals/providers.md`).
  - The native option ids are kept ("a display label is not necessarily a valid reply").
- **What the agent process inherits:**
  - **Environment:** the server's `process.env`, with the provider instance's configured
    variables merged on top. Secret references are resolved; `~` is expanded
    (`packages/provider-core/src/server/instanceEnvironment.ts`).
  - Antigravity removes ambient Google credentials (`docs/internals/providers.md`).
  - Shells started by Codex carry `T3CODE_THREAD_ID` (`CodexAdapterV2.ts` ~line 1831).
  - **cwd:** the thread's worktree path, or the project root, or the scratch folder
    (`runtimePolicy.cwd`).
  - Accounts: Claude uses `CLAUDE_CONFIG_DIR`; Codex uses `CODEX_HOME` and shadow homes.
- **Sandboxing:**
  - T3 adds **no sandbox of its own**. It passes the permission mode to the provider's own
    sandbox and approvals.
  - Codex: `readOnly`, `workspaceWrite` or `dangerFullAccess` sandbox policies (section 3.3).
  - Claude: permission modes only.
  - The doc on Antigravity helpers says it directly: "Prompt instructions and tool denial do not
    create a native sandbox" (`docs/internals/providers.md`).
  - Default mode is **Full access**.
- **Filesystem RPC boundary:**
  - `filesystem:read` can read any file the server account can read, including absolute paths
    outside the project.
  - Writes follow workspace path rules (`apps/server/src/workspace/WorkspaceFileSystem.ts`).
  - Signed asset URLs are bearer credentials for one file identity
    (`apps/server/src/assets/AssetAccess.ts`).
- **Webhook and app-context data** is wrapped and marked untrusted (Codex `additionalContext`
  `kind:"untrusted"`; webhook `<webhook-data-…>` tags).
- **Tag:** must (scoped auth between client and daemon, approvals bound to live sessions, cwd and
  env handling); should (DPoP-style key binding); later (MCP OAuth).

---

## Could not verify (only running the app would show)

- How the UI actually looks and behaves: animations, exact labels on plan buttons ("Implement
  plan" or similar), how the work log groups and summarises calls, the exact wording of
  approval rows.
- Whether paragraph streaming feels "live" in practice. Also whether any client renders
  token-level deltas despite `responseStreamingMode` (the setting only offers `turn` and
  `paragraph`).
- How the Claude Agent SDK talks to the `claude` binary. That code is inside the SDK package, not
  this repository.
- How Codex shows file reads and searches. I found no Codex mapping to `file_search`. Reads may
  appear as `commandExecution` items.
- The real latency and behaviour of steering, interrupt, force stop and restart recovery.
- Whether `Read`, `Glob` and `Grep` get special rendering on the client, despite being typed
  `dynamic_tool` on the server.
- Exact defaults for some settings that I did not check one by one (most default values were not
  read).
- The mobile app's behaviour in detail. I read docs only, not `apps/mobile` source.
- `docs/orchestration-v2/*` says it describes a "target architecture". The code clearly
  implements most of it (all the files named above exist), but small gaps between the doc and the
  code are possible.

## Licence

- `LICENSE` is MIT. The copyright line, quoted: "Copyright (c) 2026 T3 Tools Inc."
- MIT allows copying, changing and redistributing the code, including in commercial or closed
  products, **as long as the copyright notice and the licence text are kept** with substantial
  portions.
- **Third-party material listed in `third-party-licenses.config.json`** (with SPDX texts in
  `third-party-licenses/spdx/`):
  - MIT: GitHub gemoji alias data; "opencode Office viewers" (`apps/web/src/components/files/office/LICENSE.opencode`);
    Ionic keyboard assist; libghostty-vt; GhosttyKit; Symbols Nerd Font Mono
    (`apps/web/src/terminal/ghostty/fonts/SymbolsNerdFontMono-Regular.woff2`); agent-device;
    react-native-uitextview; the Expo text editor implementation; SIMD Everywhere; RaTeX;
    micromark-extension-llm-math.
  - **CC0-1.0** sound effects: "Notification Sound 1" and "Notification Sound 3" by
    deadrobotmusic, "Folder_Whoosh.wav" by BaggoNotes, "Contarex camera shutter.wav" by Tonik1105.
  - **Apache-2.0:** MesloLGS NF font (`apps/mobile/modules/t3-terminal/android/.../fonts/MesloLGS-NF-*.ttf`);
    the wasm2c runtime (WABT).
  - **MIT AND Apache-2.0 AND BSD-3-Clause:** expo-device-hub.
  - **OFL-1.1:** KaTeX mathematical fonts.
- **Brand assets.** `assets/` (`dev/`, `nightly/`, `prod/`) holds T3 Code app icons and logos
  (`assets/prod/logo.svg`, `t3-black-*` PNGs). The licence and README have no separate
  trademark clause. Even so, copying the T3 name and logo into another product is not advisable;
  MIT covers copyright, not trademarks.
- `.repos/alchemy-effect` and `.repos/effect` are vendored or reference copies of other projects,
  under their own licences (not checked).
- npm dependencies have their own licences. Notably `@anthropic-ai/claude-agent-sdk` is
  Anthropic's licence, not MIT (not checked here).

## Commit

`6f5293fbdca476cddbc37466450983ba1dd2ada2` (2026-11-20 12:38:15 -0800), repository
`pingdotgg/t3code`.
