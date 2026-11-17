# Anima Work Plan

| Item | Value |
| --- | --- |
| Document | Work plan: the entry point to all planning documents |
| Version | 3.2 |
| Date | 2026-11-17 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | Approved. The operator approved the principles and the priority order on 2026-11-13. Version 1.1 added the plan register and the rules for plans (operator, 2026-11-15). Version 1.2 recorded OD-W13 to OD-W21 (operator, 2026-11-16). Version 1.3 corrected the states in Sections 4 and 9 to agree with the commits on `main`, moved each GitHub issue into the planning documents and added PLN-012 to PLN-017 (operator, 2026-11-16, R-119). Version 2.0 gives each section its own document in [plan/](plan/), with no change to the text (operator, 2026-11-16). Version 2.1 corrected the state of row 4.17, put the rows of tier 1 in order, added the column "Waits for" to Section 4 and added the one-click plan to Section 9 (audit that the operator asked for, 2026-11-17). It also took in the two proposals of the branch `claude/watchdog-autoupdate-proposals`: the watchdog (row 4.19, OD-W22, open) and the update of the daemon and the hub (row 4.18, OD-W23). Version 2.2 adds PLN-018 (the path of an idea) and PLN-019 (mockups), the folder `ideas/`, the index of the mockups, and moves the approved builds proposal to [projects-and-builds-plan.md](plans/projects-and-builds-plan.md) (operator, 2026-11-17). Version 3.0 puts each document in the folder of its stage, archives the finished phases, and generates Section 9 from the header of each plan (PLN-020, operator, 2026-11-17: "lets do all five in that order"). Version 3.1 adds the start of a session and the claims (PLN-022), the mockup check, the idea intake, and version 0.2 of the extensions plan with the Bots proposal in its own document (operator, 2026-11-17). Version 3.2 corrects what a council of four reviewers found: the release record is the one place for the state of a release, the column "Waits for" names exact tasks, PLN-007 says what each state permits, and the operator queue is generated (PLN-023, R-123). |
| Related | [design-spec.md](spec/design-spec.md), [extensions-plan.md](plans/extensions-plan.md), [compute-plan.md](plans/compute-plan.md), [vpn-plan.md](plans/vpn-plan.md), [handoff.md](plans/phase-3/handoff.md), [programme/proposals/](proposals/) |

This document obeys the writing rules in Section 0 of the design specification.

This document is the entry point. Start here. Each section below is one document with one job. The section numbers did not change: "work plan Section 9" in a different document means [plan/register.md](plan/register.md).

The work plan contains no build requirements. Each requirement is in the plan or the proposal that owns it (Section 5). GitHub issues are not a store (R-119, PLN-012). The planning documents in this repository are the one source of truth.

## 0. Start here

| You want to | Read |
| --- | --- |
| Start a session | `AGENTS.md`, "Start of a session": get `main`, find the work, find the task, read the claims, claim the task, build, finish |
| Know what to do next | Section 4, [plan/priorities.md](plan/priorities.md): the first row whose column "Waits for" names a daemon |
| Know what a different session builds now | [claims/README.md](claims/README.md), PLN-022: one file for each task that is taken |
| Know the state of a plan | Section 9, [plan/register.md](plan/register.md) |
| Find the document that owns a requirement or an identifier | Section 5, [plan/ownership.md](plan/ownership.md) |
| Build something | Section 10, [plan/rules.md](plan/rules.md), PLN-013: read the plan, or change it, or write a proposal. Then build. |
| Find a decision of the operator | Section 7, [plan/decisions.md](plan/decisions.md), and the rulings in [rulings.md](rulings.md) |
| Record an idea | [ideas/README.md](ideas/README.md), PLN-018: one folder for each idea. The path is idea, proposal, plan, contract, tasks, check. |
| Change a screen that has a mockup | [mockup/README.md](mockup/README.md), PLN-019: change the mockup in the same commit. |
| Find a work item or a defect | [work-items/README.md](work-items/README.md) |
| See what waits for the operator | [operator-queue.md](operator-queue.md): one generated list, in the order of the priorities (PLN-023) |
| Know what is released | [procedures/releases.md](procedures/releases.md), "Release record" |
| Find what only a person can test | [need-human-testing.md](need-human-testing.md) |
| Find work that was left for later | [deferred.md](deferred.md) |

Where the documents are:

| Place | Kind | Content |
| --- | --- | --- |
| [work-plan.md](work-plan.md) and [plan/](plan/) | Map | Purpose, priority order, owners, decisions, register, rules |
| [ideas/](ideas/README.md) | Stage 1: ideas | One folder for each idea. No requirement. Deleted when the idea becomes a proposal or is dropped |
| [proposals/](proposals/) | Stage 2: proposals | A draft of a plan. When the operator approves it with a ruling, it moves to `plans/` as `<name>-plan.md` (PLN-018) |
| [spec/design-spec.md](spec/design-spec.md) | Stage 3: specification | The device manager. The source of truth for its requirements |
| [plans/](plans/) | Stage 3: plans | One plan for each subject. A subject with task breakdowns has a folder: [plans/one-click/](plans/one-click/), [plans/phase-3/](plans/phase-3/) |
| `../contracts/` | Stage 4: contracts | The frozen interfaces that the parts share (K1 to K8). Only a ruling changes one |
| [work-items/](work-items/README.md) | Stage 5: work items | Engine, extensions, compute, decisions, device manager, defects |
| [need-human-testing.md](need-human-testing.md), [security-findings.md](security-findings.md), [deferred.md](deferred.md) | Lists | Checks that only a person can do. Security findings. Work that was left for later |
| [claims/](claims/README.md) | Claims | One file for each task that a session builds now, with its machine and its coding agent (PLN-022) |
| [rulings.md](rulings.md) | Rulings | The one log of decisions, with one series of numbers (R, CR; PLN-021) |
| [mockup/](mockup/README.md) | Mockups | Static HTML of a layout, with an index that holds the state of each. A mockup agrees with the application (PLN-019) |
| [procedures/](procedures/) | Procedures | How to install, set up and release |
| [research/](research/) | Reports | Audits and measurements. They add no requirement |
| [archive/](archive/) | Finished work | Phase 1, Phase 2, the logs of the work packages, old reviews. Do not build from it |

## 1. Purpose

**Everything that computes for you, or holds data about you, in one place that you own.** The text is in [plan/purpose.md](plan/purpose.md).

## 2. Principles

ETH-001 to ETH-009: [plan/purpose.md](plan/purpose.md).

## 3. Categories

Foundation, See, Reach, Remember, Use: [plan/priorities.md](plan/priorities.md).

## 4. Priority order

The tiers 0 to 4, one row for each block of work, with its work items and its state: [plan/priorities.md](plan/priorities.md).

## 5. Where each requirement lives

One document owns each concern: [plan/ownership.md](plan/ownership.md). Add a requirement to its owner, not to the work plan.

## 6. Not planned

The work that must not be built: [plan/priorities.md](plan/priorities.md).

## 7. Open decisions

OD-W1 to OD-W23, each with the words of the operator: [plan/decisions.md](plan/decisions.md).

## 8. Changes that this plan needs in other documents

Each change, and if it is made: [plan/spec-changes.md](plan/spec-changes.md).

## 9. Plan register

The only list of the plans of Anima, with the state of each: [plan/register.md](plan/register.md). A document that is not in the register is not a plan (PLN-001).

## 10. Rules for plans and progress

PLN-001 to PLN-023: [plan/rules.md](plan/rules.md).
