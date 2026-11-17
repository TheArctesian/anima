# Anima Work Plan

| Item | Value |
| --- | --- |
| Document | Work plan: the entry point to all planning documents |
| Version | 2.1 |
| Date | 2026-11-17 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | Approved. The operator approved the principles and the priority order on 2026-11-13. Version 1.1 added the plan register and the rules for plans (operator, 2026-11-15). Version 1.2 recorded OD-W13 to OD-W21 (operator, 2026-11-16). Version 1.3 corrected the states in Sections 4 and 9 to agree with the commits on `main`, moved each GitHub issue into the planning documents and added PLN-012 to PLN-017 (operator, 2026-11-16, R-119). Version 2.0 gives each section its own document in [plan/](plan/), with no change to the text (operator, 2026-11-16). Version 2.1 corrected the state of row 4.17, put the rows of tier 1 in order, added the column "Waits for" to Section 4 and added the one-click plan to Section 9 (audit that the operator asked for, 2026-11-17). It also took in the two proposals of the branch `claude/watchdog-autoupdate-proposals`: the watchdog (row 4.19, OD-W22, open) and the update of the agent and the hub (row 4.18, OD-W23). |
| Related | [design-spec.md](design-spec.md), [extensions-plan.md](extensions-plan.md), [compute-plan.md](compute-plan.md), [vpn-plan.md](vpn-plan.md), [programme/handoff.md](programme/handoff.md), [programme/proposals/](programme/proposals/) |

This document obeys the writing rules in Section 0 of the design specification.

This document is the entry point. Start here. Each section below is one document with one job. The section numbers did not change: "work plan Section 9" in a different document means [plan/register.md](plan/register.md).

The work plan contains no build requirements. Each requirement is in the plan or the proposal that owns it (Section 5). GitHub issues are not a store (R-119, PLN-012). The planning documents in this repository are the one source of truth.

## 0. Start here

| You want to | Read |
| --- | --- |
| Know what to do next | Section 4, [plan/priorities.md](plan/priorities.md) |
| Know the state of a plan | Section 9, [plan/register.md](plan/register.md) |
| Find the document that owns a requirement or an identifier | Section 5, [plan/ownership.md](plan/ownership.md) |
| Build something | Section 10, [plan/rules.md](plan/rules.md), PLN-013: read the plan, or change it, or write a proposal. Then build. |
| Find a decision of the operator | Section 7, [plan/decisions.md](plan/decisions.md), and the rulings in [programme/programme.md](programme/programme.md) |
| Find a work item or a defect | [work-items/README.md](work-items/README.md) |
| Find what only a person can test | [need-human-testing.md](need-human-testing.md) |
| Find work that was left for later | [deferred.md](deferred.md) |

Where the documents are:

| Place | Kind | Content |
| --- | --- | --- |
| [work-plan.md](work-plan.md) and [plan/](plan/) | Map | Purpose, priority order, owners, decisions, register, rules |
| [design-spec.md](design-spec.md) | Specification | The device manager. The source of truth for its requirements |
| `*-plan.md` in this folder, [vpn-build.md](vpn-build.md) | Plans | One plan for each subject: extensions, compute, VPN, command line and MCP, mobile, interface |
| [programme/proposals/](programme/proposals/) | Proposals | A draft of a plan. It becomes a plan when the operator approves it with a ruling |
| `../contracts/` | Contracts | The frozen interfaces that the parts share (K1 to K8). Only a ruling changes one |
| [programme/programme.md](programme/programme.md) | Rulings | Each decision that was made, with a number (R, CR) |
| [programme/](programme/) phase files and [programme/packages/](programme/packages/) | Phase plans and logs | Phases 1 to 3: plans, contract changes, acceptance records, one log for each work package |
| [research/one-click/](research/one-click/) | Task breakdowns | The one-click work, S1 to S8 |
| [work-items/](work-items/README.md) | Work items | Engine, extensions, compute, decisions, device manager, defects |
| [need-human-testing.md](need-human-testing.md), [security-findings.md](security-findings.md), [deferred.md](deferred.md) | Lists | Checks that only a person can do. Security findings. Work that was left for later |
| [setup/](setup/), [releases.md](releases.md) | Procedures | How to install, set up and release |
| [research/](research/), `ui-critique-*`, `ui-inventory-*` | Reports | Audits and reviews. They add no requirement |

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

PLN-001 to PLN-017: [plan/rules.md](plan/rules.md).
