# Operator queue

| Item | Value |
| --- | --- |
| Document | What waits for the operator, in one place (PLN-023) |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | Generated. Do not change the lists by hand |

This document is generated: `bun run quality:register` writes it from the documents that own each fact, and `bun run quality:documents` fails when it does not agree with them. It holds no fact of its own (PLN-016). To take an item off a list, change its owner: decide the decision in its plan, set the state in the header of the document, or write the result of the check in [need-human-testing.md](need-human-testing.md).

- List 1 is in the order of the priorities ([plan/priorities.md](plan/priorities.md)). Start at the top.
- List 2 has each row of a decision table that does not say "Decided". The work items in [work-items/README.md](work-items/README.md), section "Decisions", are in it.
- List 3 has each plan and proposal whose state is Draft, Direction decided or Not checked.
- List 4 has the open entries of [need-human-testing.md](need-human-testing.md). The older checks are in its section "Checks that other documents hold", grouped by machine.
- The business vault (a different, private repository) holds the deadlines of the company. They are not in this list.

<!-- generated from the planning documents: bun run quality:register -->

## 1. In the order of the priorities (13)

| Row | Work | Waits for |
| --- | --- | --- |
| 4.1 | Finish the device manager | Operator: the acceptance run (WP-53); the Linux release and the check that an installed application updates (R7). Agent: nothing |
| 4.16 | One-click install | Operator: the live checks (need-human-testing.md, "One-click") |
| 4.17 | The hosted mode and the shared pool | Operator: approval of HMD-001 to HMD-036, and HM-D4 |
| 4.18 | Update the agent and the hub | Operator: approval of ROL-001 to ROL-052 (only R-122 is approved), and a Linux release for the tasks that need one. Agent: nothing until a ruling names a task |
| 4.2 | Restore a backup and move the coordinator | Operator: approval of FND-006. No task breakdown yet |
| 4.3 | Decisions and approvals for the foundation | Operator: each decision and approval in the row |
| 4.4 | The engine (E0) | Agent: E0-4 T9 (extensions-plan.md Section 5.1). Operator: E0-5 T0, E0-6 T0, the contract ruling for E0-7, and OD-E18 |
| 4.5 | Level 1 model service | Operator: approval of the compute plan, and the compute decisions of row 4.3 |
| 4.7 | Jobs, builds and AI agent runs | Operator: a ruling that changes R-115 (task W1) and the acceptance of the design (task W2). Agent: nothing before them |
| 4.9 | Hosted applications | Operator: approval of the proposal |
| 4.10 | Standby copies of the coordinator database | Operator: approval of the requirements |
| 4.11 | Bulk data on the operator's machines | Operator: approval of the requirements, before E0-5 starts |
| 4.19 | The watchdog | Operator: a tier and approval of the direction (OD-W22) |

## 2. Open decisions (64)

| Decision | Question | Document |
| --- | --- | --- |
| OD-W22 | The watchdog (watchdog.md): approve its direction, and give it a tier. Its own decisions are OD-WDG-1 to OD-WDG-13. | [plan/decisions.md](plan/decisions.md) |
| OD-C1 | Hardware of each compute node | [plans/compute-plan.md](plans/compute-plan.md) |
| OD-C2 | One agent binary or two | [plans/compute-plan.md](plans/compute-plan.md) |
| OD-C3 | Conformance tolerances | [plans/compute-plan.md](plans/compute-plan.md) |
| OD-C4 | Where the block adapters live | [plans/compute-plan.md](plans/compute-plan.md) |
| OD-C5 | Router location | [plans/compute-plan.md](plans/compute-plan.md) |
| OD-C6 | Where conversations are kept | [plans/compute-plan.md](plans/compute-plan.md) |
| OD-C7 | Access to `ai.anima.ing` | [plans/compute-plan.md](plans/compute-plan.md) |
| OD-C8 | Device policy default for client computers | [plans/compute-plan.md](plans/compute-plan.md) |
| OD-E1 | OD-E1: Decide: where Class B data lives | [plans/extensions-plan.md](plans/extensions-plan.md) |
| OD-E4 | OD-E4: Decide: hosted vision model | [plans/extensions-plan.md](plans/extensions-plan.md) |
| OD-E10 | OD-E10: Decide: price source | [plans/extensions-plan.md](plans/extensions-plan.md) |
| OD-E3 | OD-E3: Check SimpleFIN coverage | [plans/extensions-plan.md](plans/extensions-plan.md) |
| OD-E5 | Hub folder grouping (REPO-E-001) | [plans/extensions-plan.md](plans/extensions-plan.md) |
| OD-E7 | Garmin access | [plans/extensions-plan.md](plans/extensions-plan.md) |
| OD-E8 | Obsidian two-way sync | [plans/extensions-plan.md](plans/extensions-plan.md) |
| OD-E11 | Path for a request to a local upstream (MGW-010). A program such as Claude Code has one base address, so it cannot select the path itself. | [plans/extensions-plan.md](plans/extensions-plan.md) |
| OD-ROL-1 | Where an agent gets its files: from its hub, or from the release host | [proposals/agent-and-hub-update.md](proposals/agent-and-hub-update.md) |
| OD-ROL-2 | The signing key: the updater key of UPD-012 for each program, or a second key for the agent and the hub | [proposals/agent-and-hub-update.md](proposals/agent-and-hub-update.md) |
| OD-ROL-3 | The form of the published hub: an image file on the release host, an image in a registry, or a build from source on the machine of the hub | [proposals/agent-and-hub-update.md](proposals/agent-and-hub-update.md) |
| OD-ROL-4 | The order of the build | [proposals/agent-and-hub-update.md](proposals/agent-and-hub-update.md) |
| OD-ROL-5 | The values and the default of the automatic update | [proposals/agent-and-hub-update.md](proposals/agent-and-hub-update.md) |
| OD-ROL-6 | The numbers: a quarter for each step, 10 minutes between steps, 1 hour after the first machine of an automatic rollout, 3 minutes and 5 minutes for the checks | [proposals/agent-and-hub-update.md](proposals/agent-and-hub-update.md) |
| OD-ROL-7 | Who selects the first machine | [proposals/agent-and-hub-update.md](proposals/agent-and-hub-update.md) |
| OD-ROL-9 | Can an API key or an AI agent start, stop or roll back a rollout? | [proposals/agent-and-hub-update.md](proposals/agent-and-hub-update.md) |
| OD-ROL-10 | Does a machine that has "Takes work" off (USE-004) get updates? | [proposals/agent-and-hub-update.md](proposals/agent-and-hub-update.md) |
| OD-ROL-11 | What a rollout does when it stops: only stop, or also roll back each updated machine | [proposals/agent-and-hub-update.md](proposals/agent-and-hub-update.md) |
| OD-ROL-12 | A release whose database change the previous hub cannot read | [proposals/agent-and-hub-update.md](proposals/agent-and-hub-update.md) |
| OD-ROL-13 | On a computer that has the desktop application, who updates the agent: the rollout, or the application when it updates itself | [proposals/agent-and-hub-update.md](proposals/agent-and-hub-update.md) |
| OD-ROL-14 | Is the desktop application part of a rollout? | [proposals/agent-and-hub-update.md](proposals/agent-and-hub-update.md) |
| HM-D8 | What does a hub that sleeps cost, and how fast does it start? | [proposals/hosted-mode.md](proposals/hosted-mode.md) |
| HM-D2 | Where does the usage line sit? | [proposals/hosted-mode.md](proposals/hosted-mode.md) |
| HM-D9 | Which sign-in methods does AuthKit show? | [proposals/hosted-mode.md](proposals/hosted-mode.md) |
| HM-D10 | Do the sign-in pages use a domain of Anima.ing, for example `auth.anima.ing`? | [proposals/hosted-mode.md](proposals/hosted-mode.md) |
| HM-D4 | Is the second factor (HMD-006) required for each account? | [proposals/hosted-mode.md](proposals/hosted-mode.md) |
| HM-D5 | In which region is the first pool host? | [proposals/hosted-mode.md](proposals/hosted-mode.md) |
| HM-D7 | Who reviews the security of the pool and of the grant path before the launch? | [proposals/hosted-mode.md](proposals/hosted-mode.md) |
| OD-WDG-1 | The form of the watchdog: a function of `anima-agent`, a container, or a second program | [proposals/watchdog.md](proposals/watchdog.md) |
| OD-WDG-2 | How the second unit of WDG-040 gets the right to restart on a Linux system installation: it runs as root with a strict sandbox, or it runs as its own user with… | [proposals/watchdog.md](proposals/watchdog.md) |
| OD-WDG-3 | A process with no supervisor | [proposals/watchdog.md](proposals/watchdog.md) |
| OD-WDG-4 | Stage 1 first (watches made at the machine), or wait for the work route and build stage 2 only | [proposals/watchdog.md](proposals/watchdog.md) |
| OD-WDG-5 | When systemd or Docker stopped its own attempts, does the watchdog try again (WDG-024)? | [proposals/watchdog.md](proposals/watchdog.md) |
| OD-WDG-6 | Is a watch a check of a new type (WDG-035), or does it have its own status rules? | [proposals/watchdog.md](proposals/watchdog.md) |
| OD-WDG-7 | Which restarts send a notification by default | [proposals/watchdog.md](proposals/watchdog.md) |
| OD-WDG-8 | Does a report contain the last lines of the output of the program? | [proposals/watchdog.md](proposals/watchdog.md) |
| OD-WDG-9 | Can an API key or an AI agent add, remove or pause a watch? | [proposals/watchdog.md](proposals/watchdog.md) |
| OD-WDG-10 | macOS: watch only the jobs of the user, or also system jobs (LaunchDaemons), which need root | [proposals/watchdog.md](proposals/watchdog.md) |
| OD-WDG-11 | The numbers of WDG-010, WDG-011 and WDG-014 to WDG-016 | [proposals/watchdog.md](proposals/watchdog.md) |
| OD-WDG-13 | A probe to an address that is not on the machine | [proposals/watchdog.md](proposals/watchdog.md) |
| OD-1 | User interface framework | [spec/design-spec.md](spec/design-spec.md) |
| OD-2 | Hub authentication method | [spec/design-spec.md](spec/design-spec.md) |
| OD-3 | Desktop client without a hub | [spec/design-spec.md](spec/design-spec.md) |
| OD-4 | Hub on Proxmox | [spec/design-spec.md](spec/design-spec.md) |
| OD-5 | Time-series storage | [spec/design-spec.md](spec/design-spec.md) |
| OD-7 | Self-signed certificates in the Tauri web view | [spec/design-spec.md](spec/design-spec.md) |
| OD-8 | Spring library | [spec/design-spec.md](spec/design-spec.md) |
| OD-9 | Hub location | [spec/design-spec.md](spec/design-spec.md) |
| OD-10 | Headscale or the Tailscale hosted control server | [spec/design-spec.md](spec/design-spec.md) |
| OD-11 | Gateway location | [spec/design-spec.md](spec/design-spec.md) |
| OD-12 | Build trigger | [spec/design-spec.md](spec/design-spec.md) |
| OD-14 | Compute workers (R-110) | [spec/design-spec.md](spec/design-spec.md) |
| OD-6 | Central log search | [spec/design-spec.md](spec/design-spec.md) |
| OD-15 | Contract change for the actor kinds (Section 3.8.1) | [spec/design-spec.md](spec/design-spec.md) |
| OD-22 | What the website can say about the paid hosted option (SITE-012; work-plan.md OD-W2, other-operators.md) | [spec/design-spec.md](spec/design-spec.md) |

## 3. Documents that wait for an approval (27)

| Document | Kind | State |
| --- | --- | --- |
| [plans/one-click/s12-spec-delta.md](plans/one-click/s12-spec-delta.md) | Proposal: spec delta for packaging | Draft |
| [plans/one-click/s3-spec-delta.md](plans/one-click/s3-spec-delta.md) | Proposal: spec delta for the bundled PostgreSQL | Draft |
| [plans/one-click/s4-spec-delta.md](plans/one-click/s4-spec-delta.md) | Proposal: spec delta for the hub as a background program | Draft |
| [plans/one-click/s6-spec-delta.md](plans/one-click/s6-spec-delta.md) | Proposal: spec delta for the first run | Draft |
| [plans/ui-redesign-direction.md](plans/ui-redesign-direction.md) | Plan: direction of the redesign | Not checked |
| [plans/ui-redesign-plan.md](plans/ui-redesign-plan.md) | Plan | Not checked |
| [proposals/add-this-machine.md](proposals/add-this-machine.md) | Proposal | Draft. Build state not checked |
| [proposals/agent-and-hub-update.md](proposals/agent-and-hub-update.md) | Proposal, with a task table | Direction decided |
| [proposals/agent-runs.md](proposals/agent-runs.md) | Proposal | Draft |
| [proposals/bots.md](proposals/bots.md) | Proposal | Draft |
| [proposals/bulk-data-placement.md](proposals/bulk-data-placement.md) | Proposal | Direction decided |
| [proposals/cloud-machines.md](proposals/cloud-machines.md) | Proposal | Draft |
| [proposals/coordinator-standby.md](proposals/coordinator-standby.md) | Proposal | Direction decided |
| [proposals/cve-watch-and-network-awareness.md](proposals/cve-watch-and-network-awareness.md) | Proposal | Draft |
| [proposals/first-run-setup.md](proposals/first-run-setup.md) | Proposal | Draft. Build state not checked |
| [proposals/hosted-applications.md](proposals/hosted-applications.md) | Proposal | Draft |
| [proposals/hosted-mode.md](proposals/hosted-mode.md) | Proposal | Direction decided |
| [proposals/in-app-enrolment.md](proposals/in-app-enrolment.md) | Proposal | Draft. Build state not checked |
| [proposals/jobs.md](proposals/jobs.md) | Proposal | Direction decided |
| [proposals/machine-profiles.md](proposals/machine-profiles.md) | Proposal | Draft |
| [proposals/new-sources.md](proposals/new-sources.md) | Proposal | Draft |
| [proposals/other-operators.md](proposals/other-operators.md) | Proposal | Direction decided |
| [proposals/power-health.md](proposals/power-health.md) | Proposal | Draft |
| [proposals/shared-pool-hosting.md](proposals/shared-pool-hosting.md) | Proposal | Draft |
| [proposals/shell-prompt.md](proposals/shell-prompt.md) | Proposal | Draft |
| [proposals/ssh-terminal.md](proposals/ssh-terminal.md) | Proposal | Draft. Build state not checked |
| [proposals/watchdog.md](proposals/watchdog.md) | Proposal, with a task table | Draft |

## 4. Checks that only the operator can do (17)

| Check | What |
| --- | --- |
| HT-001 | `anima snapshot` against GitHub |
| HT-002 | the tool inventory from an agent to the command line |
| HT-003 | the tool inventory in an MCP client |
| HT-004 | no window of Apple on a Mac without the developer tools |
| HT-005 | the tool inventory of an agent that runs as a system unit |
| HT-006 | the probes of `claude` and `codex` leave the account alone |
| HT-007 | the pinned Rust version on the Linux computer |
| HT-008 | a macOS release build with the pinned Rust version |
| HT-009 | the hub image builds with Rust 1.100.0 |
| HT-010 | merge a pull request, and a set of pull requests, on GitHub |
| HT-011 | run a workflow again, cancel a run and start a workflow on GitHub |
| HT-012 | set a commit status on GitHub |
| HT-013 | an API key cannot merge, control a run or read a file |
| HT-014 | the issue tools of the MCP server, and a workflow that a comment starts |
| HT-015 | the Work, Code and History tabs of a project in the client |
| HT-016 | a release build on the Linux builder puts the bundle in its own clone |
| HT-017 | "This machine" with the Anima VPN and NetBird together |

<!-- end of the generated queue -->
