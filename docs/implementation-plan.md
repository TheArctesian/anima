# Anima Agent Implementation Plan — Phase 1

| Item | Value |
| --- | --- |
| Document | Agent implementation plan |
| Scope | Phase 1 of [design-spec.md](design-spec.md) (Section 9) |
| Version | 0.1 (draft) |
| Date | 2026-11-03 |
| Language | Simplified Technical English |

## 1. Purpose

This plan tells a group of AI agents how to build Phase 1 at the same time, in loops, with a review in each loop. It uses three levels of agents:

1. One **programme orchestrator** controls the full plan.
2. One **package orchestrator** controls each work package.
3. **Builders** and **reviewers** do the work in each package.

The work packages are independent. They can run at the same time because:

- the shared interfaces (contracts) are frozen before the packages start (Section 4);
- each file has one owner package (Section 5);
- each package works in its own git worktree and branch.

## 2. Agent roles and models

| Role | Count | Model | Responsibilities |
| --- | --- | --- | --- |
| Programme orchestrator | 1 | Opus | Writes the contracts. Starts and stops packages in waves. Makes rulings. Merges package branches into the integration branch. Runs the acceptance pass. |
| Package orchestrator | 1 for each active package | Opus | Makes the package plan. Splits it into tasks. Selects builders and reviewers. Runs the loop. Decides when the package is done. |
| Builder, light | As needed | Haiku | Mechanical tasks: configuration files, token JSON, icon tables, fixtures, migrations from a fixed design, documentation. |
| Builder | As needed | Sonnet | Normal feature work: domains, GraphQL parts, Svelte components, tests. |
| Builder, heavy | As needed | Opus | Difficult or high-risk work: protocol code, encryption, concurrency, the scheduler, the motion engine, GraphQL subscriptions. |
| Gate runner | 1 for each package | Haiku | Runs the gates (Section 7) and gives a short result. It does not change code. |
| Reviewer | At least 1 for each task | Sonnet | Correctness, CUPID lenses, spec trace, tests. |
| Reviewer, security | For tasks with a security tag | Opus | Credentials, login, tokens, the tunnel, the network allow list, VPN access rules. |
| Reviewer, interface | For user interface tasks | Sonnet | Measures in a real browser (WebKit and Chromium). Checks motion, layout shift, tokens, accessibility. |

### 2.1 Model rules

- **AGT-001.** Orchestrators always use Opus.
- **AGT-002.** The package orchestrator selects the builder model for each task with this table:

  | Task property | Builder model |
  | --- | --- |
  | The design is fully specified and the change is mechanical | Haiku |
  | Normal feature work with tests | Sonnet |
  | Security, encryption, protocols, concurrency, or a change that crosses a contract | Opus |
  | A Haiku or Sonnet builder failed the same review finding two times | One model higher |

- **AGT-003.** A reviewer must not review its own work. A reviewer must be a different agent from the builder.
- **AGT-004.** A security reviewer is always Opus. A task with the tag `security` needs a security reviewer and a normal reviewer.
- **AGT-005.** A reviewer can use a lower model than the builder only for gate result triage and naming checks.

## 3. The loops

### 3.1 Task loop (inside a package)

```text
         +--------+     +-------+     +------+     +--------+
 task -->|  PLAN  |---->| BUILD |---->| GATE |---->| REVIEW |---> ACCEPT
         +--------+     +-------+     +------+     +--------+
                           ^             |  fail       | findings
                           |             v             v
                           +---------- FIX <-----------+
```

1. **PLAN.** The package orchestrator writes the task: goal, owned files, requirement IDs, contract parts it uses, test plan, builder model.
2. **BUILD.** The builder makes the change on the task branch and commits.
3. **GATE.** The gate runner runs the gates. A failure goes back to the builder with the gate output.
4. **REVIEW.** One or more reviewers examine the diff. Each finding has a CUPID property or a requirement ID, the evidence and a severity (must, should, could).
5. **FIX.** The builder fixes each "must" finding and each "should" finding that the orchestrator accepts. Then the loop goes to GATE again.
6. **ACCEPT.** The package orchestrator merges the task branch into the package branch.

- **LOOP-001.** A task can have a maximum of 5 review rounds. After 5 rounds, the package orchestrator must stop the task and escalate it to the programme orchestrator.
- **LOOP-002.** A reviewer must do these checks, not only read the diff:
  - **Mutation.** Remove a guard or a condition and confirm that a named test fails. If no test fails, that is a finding.
  - **Measurement.** For interface tasks, measure in a real browser (spec Section 5.8).
  - **Interruption.** For motion and sessions, interrupt the action during the test (MV-003).
  - **Spec trace.** Each requirement ID in the task has code and a test.
- **LOOP-003.** Green gates do not prove that a task is correct. A reviewer must not accept a task only because the gates pass.
- **LOOP-004.** A builder must commit before it runs a long test suite. A builder must not finish its turn while a test runs in the background.

### 3.2 Package loop

1. The package orchestrator reads the programme file, the contracts and the spec sections of its package.
2. It writes the package plan in `docs/programme/packages/WP-NN.md`: tasks, order, owned files, models.
3. It runs task loops. Tasks that do not depend on each other run at the same time, with a maximum of 3 builders.
4. When all tasks are accepted, a final reviewer (Opus for security packages, Sonnet for others) reviews the full package diff against the base branch.
5. The package orchestrator reports to the programme orchestrator: done, the changed contract requests (if any), handoff notes and open risks.

### 3.3 Programme loop

1. **Contracts.** The programme orchestrator writes and freezes the contracts (Section 4) with the skeleton package WP-01.
2. **Waves.** It starts the packages of each wave when their dependencies are done (Section 6).
3. **Rulings.** It answers escalations and records each answer as a numbered ruling in `docs/programme/programme.md`. A ruling overrides a package plan.
4. **Integration.** It merges each finished package branch into `phase-1`, runs all gates and does a browser pass on the merged application.
5. **Acceptance.** It runs package WP-17 (Section 5) to check the Phase 1 acceptance criteria.
6. **Landing.** The merge of `phase-1` into `main` needs the confirmation of the operator.

## 4. Contracts (frozen before Wave 1)

| ID | Contract | File | Owner |
| --- | --- | --- | --- |
| K1 | GraphQL schema for Phase 1 (types, queries, mutations, subscriptions) | `contracts/schema.graphql` | Programme orchestrator |
| K2 | Database table ownership and migration number ranges for each domain | `contracts/database.md` | Programme orchestrator |
| K3 | Public Rust interface of each hub domain (function signatures that other domains call) | `contracts/hub-domains.md` | Programme orchestrator |
| K4 | Session tunnel: the `startSession` mutation, the single-use token and the WebSocket path | `contracts/tunnel.md` | Programme orchestrator |
| K5 | Semantic token names (spec Section 5.5.3) | `design/tokens/` (names only) | WP-02 |
| K6 | Synthetic test data: sites, devices and checks (from the mockup data) | `contracts/fixtures/` | Programme orchestrator |
| K7 | Error and ID conventions: typed error for each domain, ID newtypes | `contracts/conventions.md` | Programme orchestrator |

- **CON-001.** After the freeze, only the programme orchestrator can change a contract. A package asks for a change in its report. The programme orchestrator records the change as a ruling and tells all affected packages.
- **CON-002.** The hub must have a test that compares its generated GraphQL schema with `contracts/schema.graphql`.
- **CON-003.** The client packages develop against a mock GraphQL server made from K1 and K6. They do not wait for the hub.
- **CON-004.** Migration numbers use the ranges in K2, for example `devices` 0100–0199 and `checks` 0200–0299. This prevents conflicts between packages.

## 5. Work packages

Each package owns its paths. A package must not change a path that a different package owns. It writes a handoff note in its report.

| ID | Package | Owned paths | Depends on | Builders | Reviewers | Spec IDs |
| --- | --- | --- | --- | --- | --- | --- |
| WP-01 | Repository skeleton and quality gates | Root `Cargo.toml`, `package.json`, `quality/`, CI files, lint configurations, `crates/hub/src/main.rs`, `crates/hub/src/schema.rs` | — | Sonnet, Haiku | Sonnet | CQ-010–013, CQ-040, CQ-043, REPO-001–005 |
| WP-02 | Design tokens and fonts | `design/tokens/`, `apps/client/src/styles/`, `crates/theme/`, token build | WP-01 | Haiku, Sonnet | Sonnet (interface) | TOK-*, UI-010–022 |
| WP-03 | VPN and network | `deploy/headscale/`, VPN setup guide | WP-01 | Sonnet | Opus (security) | VPN-*, VPS-*, NET-005 |
| WP-04 | Deployment | `deploy/compose.yaml`, `deploy/gateway/`, backup scripts | WP-01 | Haiku, Sonnet | Sonnet | GW-001–004, DB-001–007, RA-011 |
| WP-05 | Hub access and logs | `crates/hub/src/access/`, `crates/hub/src/events/`, tracing setup | WP-01, K1–K3 | Opus, Sonnet | Opus (security), Sonnet | SEC-*, NET-001–004, LOG-*, API-003, API-007 |
| WP-06 | Hub devices | `crates/hub/src/devices/` | WP-01, K1–K3 | Sonnet | Sonnet | DEV-010–015, VPN-002 |
| WP-07 | Hub credentials | `crates/hub/src/credentials/` | WP-01, K3 | Opus | Opus (security) | SEC-003–005, CLI-004 |
| WP-08 | Guacamole protocol library | `crates/guac-protocol/` | WP-01 | Opus | Opus | GW-010–011 |
| WP-09 | Hub checks and maintenance | `crates/hub/src/checks/`, `crates/hub/src/maintenance/` | WP-05, WP-06 | Opus (scheduler), Sonnet (check types) | Sonnet | CHK-*, MW-*, DB-008–011 |
| WP-10 | Hub sessions (SSH) | `crates/hub/src/sessions/` | WP-05, WP-06, WP-07, WP-08 | Opus | Opus (security), Sonnet | HUB-002, GW-012–015, RA-001–011, SEC-006–007 |
| WP-11 | Client shell | `apps/client/src/routes/+layout*`, `apps/client/src/lib/hub/`, `lib/icons/`, `lib/motion/` | WP-02, K1 | Opus (motion, subscriptions), Haiku (icon table) | Sonnet (interface) | NAV-*, MOT-001–021, MOT-040–042, NET-004 |
| WP-12 | Client Devices page | `apps/client/src/routes/devices/+page*`, `lib/devices/`, `lib/checks/`, `lib/events/` | WP-11 | Sonnet | Sonnet (interface) | DEV-001–005, LOG-009–010, MOT roles |
| WP-13 | Client device detail and form | `apps/client/src/routes/devices/[id]/`, `lib/forms/` | WP-11 | Sonnet | Sonnet (interface) | DEV-010–022, Section 6.7 |
| WP-14 | Client SSH session tab | `apps/client/src/lib/sessions/` | WP-11, K4 | Opus | Sonnet (interface), Opus (security) | RA-001–008 |
| WP-15 | Client Settings page | `apps/client/src/routes/settings/` | WP-11 | Haiku | Sonnet | SET-001–002 |
| WP-16 | Tauri desktop shell | `apps/desktop/src-tauri/` | WP-11 | Sonnet | Opus (security) | CLI-001–003, REPO-003, packages for macOS and Linux |
| WP-17 | Integration and acceptance | `docs/research/`, acceptance tests | All | Sonnet | Opus | Spec Section 9.1, Section 5.8, CQ-050 |

### 5.1 Shared files

- **PKG-001.** WP-01 owns the workspace files (`Cargo.toml`, `package.json`, `schema.rs`). Another package that needs a change to them writes a handoff note. The programme orchestrator makes the change at integration.
- **PKG-002.** Each hub domain adds its GraphQL part in its own `graphql.rs` (CQ-033). Only WP-01 and the programme orchestrator edit `schema.rs`.
- **PKG-003.** WP-03 changes the VPS only after the operator confirms each change in chat (VPS-005). Until then, WP-03 delivers configuration files and a setup guide only.

## 6. Waves

```text
Wave 0 (serial)    WP-01 skeleton + contracts K1–K7 (programme orchestrator)
                       |
Wave 1 (parallel)  WP-02  WP-03  WP-04  WP-05  WP-06  WP-07  WP-08
                       |                   |      |      |      |
Wave 2 (parallel)  WP-11 ---------------> WP-09  WP-10 <-------+
                       |
Wave 3 (parallel)  WP-12  WP-13  WP-14  WP-15  WP-16
                       |
Wave 4 (serial)    WP-17 integration and acceptance
```

- **WAVE-001.** A package starts when all of its dependencies are accepted. It does not wait for the full wave.
- **WAVE-002.** A maximum of 5 package orchestrators are active at the same time. A maximum of 3 builders and 3 reviewers are active in one package. The operator can change these numbers.
- **WAVE-003.** The critical path is WP-01 → WP-05 → WP-10 → WP-17 (hub) and WP-01 → WP-02 → WP-11 → WP-14 → WP-17 (client). The programme orchestrator gives these packages priority.

## 7. Gates

| Gate | Command | Runs in |
| --- | --- | --- |
| Rust format | `cargo fmt --check` | Each task |
| Rust lint | `cargo clippy --all-targets -- -D warnings` | Each task |
| Rust tests | `cargo test` (database tests use a throwaway PostgreSQL) | Each task |
| Client lint and types | `bun run lint && bun run check` (Stylelint runs inside `lint`) | Each task |
| Client tests | `bun run test` | Each task |
| CUPID limits and baseline | `bun run quality:limits`, `bun run quality:names` and `bun run quality:baseline -- --base phase-1` (the local `phase-1` branch; CQ-010, CQ-011) | Each task |
| Import graph | `bun run quality:imports`: domain `store` imports and cycles (REPO-004) | Each task |
| Tokens | Stylelint raw-value rule (TOK-003) inside `bun run lint`; generated files current (TOK-002) | Each task |
| GraphQL contract | Schema comparison (CON-002) | Each hub task |
| Browser | WebKit and Chromium checks for the interface (spec Section 5.8) | Each interface task |

Run all gates with `bun run gates`. Gates do not run in GitHub Actions (R-005).

## 8. Files and communication

| File | Writer | Readers | Content |
| --- | --- | --- | --- |
| `docs/programme/programme.md` | Programme orchestrator | All agents | Package table, wave state, frozen contracts, numbered rulings |
| `docs/programme/packages/WP-NN.md` | Package orchestrator | Its builders and reviewers | Package plan, tasks, models, review log |
| Package report (message to the programme orchestrator) | Package orchestrator | Programme orchestrator | Status, contract change requests, handoff notes, risks |
| `contracts/` | Programme orchestrator | All agents | Section 4 |

- **COM-001.** Agents tell each other facts with `file:line` references. A reviewer must read the code. It must not trust a builder summary.
- **COM-002.** Before a pull request, search the diff for plan IDs and "round N" text in code comments. Remove them.
- **COM-003.** Branch names: `phase-1` (integration), `wp/NN-slug` (package), `wp/NN-slug/task-slug` (task).

## 9. Operator checkpoints

The agents stop and ask the operator at these points:

1. After Wave 0: approve the contracts K1–K7.
2. Before each change to the VPS or the Hostinger account (WP-03).
3. When a ruling changes a requirement in the design spec.
4. Before the merge of `phase-1` into `main`.

## 10. How to run this plan

This plan runs in Claude Code with these agent definitions in `.claude/agents/`:

| File | Model | Role |
| --- | --- | --- |
| `programme-orchestrator.md` | opus | Section 2 |
| `package-orchestrator.md` | opus | Section 2 |
| `builder-light.md` | haiku | Section 2 |
| `builder.md` | sonnet | Section 2 |
| `builder-heavy.md` | opus | Section 2 |
| `gate-runner.md` | haiku | Section 2 |
| `reviewer.md` | sonnet | Section 2 |
| `reviewer-security.md` | opus | Section 2 |
| `reviewer-interface.md` | sonnet | Section 2 |

In Claude Code, a subagent cannot start other subagents. Thus use one of these two methods:

- **Sessions.** The programme orchestrator runs in the main session. Each package orchestrator runs in its own session with its own worktree. Each package orchestrator starts its builders and reviewers as subagents.
- **Workflow.** One Claude Code workflow script calls the builders and reviewers directly, and the orchestrators run as steps in the script. The operator must ask for a workflow.
