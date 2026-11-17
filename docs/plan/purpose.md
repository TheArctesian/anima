# Purpose and principles

Part of the work plan: [work-plan.md](../work-plan.md) is the entry point, and its Sections 1 and 2 are this document. A reference to "work plan Sections 1 and 2" in a different document means this document.

| Item | Value |
| --- | --- |
| Document | What Anima is for, and the principles ETH-001 to ETH-009 |
| Version | 2.0 |
| Date | 2026-11-16 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | Approved text of the work plan, version 1.3. Version 2.0 only moved the text to this document (operator, 2026-11-16). |

## 1. Purpose

**Everything that computes for you, or holds data about you, in one place that you own.**

Anima joins three types of resource:

| Type | Examples | What Anima does |
| --- | --- | --- |
| Owned | The computers at home and at the offices | Operates them |
| Rented | Hostinger, Vercel, Cloudflare, Neon | Oversees them |
| Held about you | WHOOP, Garmin, Apple Health, Spotify, a bank | Brings a copy home |

The three types stay different. A project can run on owned compute and on rented compute. A service that holds data about the operator is a source only: nothing runs there, and a project never lists it as its hosting (OD-W11, PRO-006).

Anima is for the operator first. Later, other persons can run their own Anima on a service of their choice, or pay for a hosted one. Each person has their own coordinator and their own data (OD-W2, [other-operators.md](../proposals/other-operators.md)).

## 2. Principles

The coordinator is the hub of the design specification. The interface, the specification and the code all say "hub" (OD-W3).

- **ETH-001. One home.** Each person has one coordinator.
- **ETH-002. The coordinator is portable.** It can run on a VPS, at home, or on a host that a service operates. No code must assume one provider.
- **ETH-003. One home, copies by policy.** Each set of data has one home. A policy places its copies on other machines. The operator does not select the machines. The live database has at least two standby copies. Bulk data has at least two copies of each encrypted piece.
- **ETH-004. The operator confirms a new home.** Anima finds a failure and selects the best copy. A copy becomes the home only after the operator confirms. Anima must not elect a new home alone.
- **ETH-005. Spread the work, not the state.** Work that holds no data can run on any machine. Only the home accepts writes. A machine that has no key can hold only encrypted pieces.
- **ETH-006. Own first.** When owned and rented resources can do the same work, Anima shows the owned resource first.
- **ETH-007. Control follows ownership.** Anima can change an owned resource: a machine with `anima-daemon` takes work from the coordinator by default, and the operator can set a machine off (USE-004). For a rented resource, the hub only reads; an action uses the CLI tool of the service, on the operator's computer or in a job (OD-W1). Anima only reads a resource that is held about the operator.
- **ETH-008. Private by construction.** The coordinator is not on the public internet (SEC-008). A source uses read-only access unless ETH-007 permits more.
- **ETH-009. Build for use.** Build only for the services and devices that the operator uses (EXT-000, CMP-001). A different person must be able to add a source without a change to the engine.
