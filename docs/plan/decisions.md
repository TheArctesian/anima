# Decisions of the work plan

Part of the work plan: [work-plan.md](../work-plan.md) is the entry point, and its Section 7 is this document. A reference to "work plan Section 7" in a different document means this document.

| Item | Value |
| --- | --- |
| Document | The decisions OD-W1 to OD-W21, each with the words of the operator |
| Version | 2.0 |
| Date | 2026-11-16 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | Approved text of the work plan, version 1.3. Version 2.0 only moved the text to this document (operator, 2026-11-16). |

## 7. Open decisions

Decisions of the other plans are in the "Open decisions" section of each plan, and the work items in [work-items/decisions/](../work-items/README.md) hold the later text for some of them (Section 4, tier 1). These are the decisions of this plan.

| Number | Decision | Result |
| --- | --- | --- |
| OD-W1 | Can Anima change a rented resource (redeploy, purge a cache, restart the VPS)? | Decided (operator, 2026-11-12) |
| OD-W2 | How do other persons use Anima? | Decided (operator, 2026-11-12) |
| OD-W3 | The name of the hub in the interface | Decided (operator, 2026-11-12) |
| OD-W4 | How do commands get to the agent? | Not a decision of this plan. |
| OD-W5 | The runtime for functions that answer a request | Decided (operator, 2026-11-12) |
| OD-W6 | The builder for a job image | Decided (operator, 2026-11-12) |
| OD-W7 | The compute plan says that the agent is optional and that metrics use SSH | Not a decision of this plan. |
| OD-W8 | The position of Use against the extensions in tier 3 | Decided (operator, 2026-11-12) |
| OD-W9 | One home with copies, or a copy on each node that syncs between nodes when the hub is down ([GH-2](../work-items/device-manager/GH-2.md)) | Decided (operator, 2026-11-12) |
| OD-W10 | Can the agent do work that the coordinator gives it? R-115 did not take a command channel to the agent. | Decided (operator, 2026-11-12) |
| OD-W11 | Are projects the front door for rented accounts, and does the builder build each project? | Decided (operator, 2026-11-13) |
| OD-W12 | The proposals that follow OD-W11 | Decided (operator, 2026-11-13) |
| OD-W13 | The build models, and pull requests from forks | Decided (operator, 2026-11-16) |
| OD-W14 | Is each job isolated, whoever wrote the code (PRO-029)? | Decided (operator, 2026-11-16) |
| OD-W15 | Does a job get the least that it needs (PRO-030)? | Decided (operator, 2026-11-16) |
| OD-W16 | What does Anima's own build run for a repository (PRO-032)? | Decided (operator, 2026-11-16) |
| OD-W17 | Which runner mode comes first (PRO-042)? | Decided (operator, 2026-11-16) |
| OD-W18 | A public repository with runners (PRO-044) | Decided (operator, 2026-11-16) |
| OD-W19 | The name of the macOS application: `Anima.app` (design specification) or `Anima.ing.app` (the builder) | Decided (operator, 2026-11-16) |
| OD-W20 | The meaning of "must not be able to build or publish the Anima application" (PRO-028) | Decided (operator, 2026-11-16) |
| OD-W21 | The order of the build work, and the tier of the correction of the release | Decided (operator, 2026-11-16) |

### OD-W1. Can Anima change a rented resource (redeploy, purge a cache, restart the VPS)?

**Decided (operator, 2026-11-12): yes, with the CLI tool of each service, never from the hub.** The hub stays read-only (PRJ-003). First, the client runs the CLI on the operator's computer with `anima run` (SCR-006). When jobs exist, a CLI can also run as a job. It changes PRJ-010 ("changes happen there, not in Anima"). **Exception (operator, 2026-11-13): GitHub.** The hub makes changes on GitHub through the GitHub App of the operator, each change a named function that is in the audit log (PRJ-017, PRJ-027 to PRJ-029). A different rented service can get the same later, with one ruling for each service. Until then this decision applies to it.

### OD-W2. How do other persons use Anima?

**Decided (operator, 2026-11-12): both. A person can host Anima on a service of their choice, and a paid hosted option runs it for them.** [other-operators.md](../programme/proposals/other-operators.md) has the operator's words and what this changes. Nothing is built for it now.

### OD-W3. The name of the hub in the interface

**Decided (operator, 2026-11-12): keep "hub".**

### OD-W4. How do commands get to the agent?

Not a decision of this plan. It is open in [C1](../work-items/compute/C1.md), [SPEC-C](../work-items/compute/SPEC-C.md) and [OD-C2](../work-items/decisions/OD-C2.md). OD-W10 gives its direction: the agent asks the hub for work and listens on no port (USE-005, MET-020).

### OD-W5. The runtime for functions that answer a request

**Decided (operator, 2026-11-12): later, as containers that stay warm.** No WebAssembly runtime. It comes after jobs (JOB-006).

### OD-W6. The builder for a job image

**Decided (operator, 2026-11-12): keep more than one way.** A job image can come from Railpack, from a Dockerfile or from a Nix flake (JOB-002, JOB-003, JOB-008). Test Railpack later.

### OD-W7. The compute plan says that the agent is optional and that metrics use SSH

Not a decision of this plan. [SPEC-C](../work-items/compute/SPEC-C.md) and [OD-C2](../work-items/decisions/OD-C2.md) record that R-084 replaces that text.

### OD-W8. The position of Use against the extensions in tier 3

**Decided (operator, 2026-11-12): jobs and AI agent runs first, then the extensions one at a time, then hosted applications.**

### OD-W9. One home with copies, or a copy on each node that syncs between nodes when the hub is down ([GH-2](../work-items/device-manager/GH-2.md))

**Decided (operator, 2026-11-12): one home, copies by policy.** The section "Decision" of [coordinator-standby.md](../programme/proposals/coordinator-standby.md) has the operator's words. It needs a ruling number in `programme.md`. [GH-2](../work-items/device-manager/GH-2.md) keeps only its phase L0.

### OD-W10. Can the agent do work that the coordinator gives it? R-115 did not take a command channel to the agent.

**Decided (operator, 2026-11-12): yes, one agent, on by default ("it should be on by default").** The section "Decision" of [jobs.md](../programme/proposals/jobs.md) has the limits. It needs a ruling number that changes R-115. It also changes CMP-002 and the recommendation of [OD-C8](../work-items/decisions/OD-C8.md). An agent that is already installed takes work after its update, with no confirmation.

### OD-W11. Are projects the front door for rented accounts, and does the builder build each project?

**Decided (operator, 2026-11-13): yes.** Connect GitHub, import each repository, leave out the ones that the operator selects. Anima proposes resources from signs in the repository. The builder is no longer specific to Anima. The three resource types stay different. A different forge must be possible. [projects-and-builds.md](../programme/proposals/projects-and-builds.md) has the six points.

### OD-W12. The proposals that follow OD-W11

**Decided (operator, 2026-11-13).** A build is the first job type and comes directly after Projects (PRO-009). Anima schedules builds from a webhook of the GitHub App, through the webhook receiver of the engine (WHK-001 to WHK-010), with a slow poll that repairs a lost webhook. Anima can also start a workflow of GitHub Actions (PRO-010, PRO-020, PRO-021). OD-W13 withdrew "a machine is not a runner". The receiver runs on the VPS as a separate program (OD-E15); to open its port needs the confirmation of the operator (VPS-005). A Mac build is a native job that runs as a build user, in a new checkout that is deleted, on a Mac that the operator set on for it (PRO-011, PRO-022 to PRO-024). Keys belong to the project (PRO-012). OD-W13 replaced "no builds of pull requests from forks" (PRO-013). Sign-in with a forge comes later, with the local sign-in as the root (PRO-015, PRO-016). PRJ-017 replaces "read access first" (PRO-014). The hub gives a project secret to one job (JOB-010). The first build is the path to Projects only: the hub folder regroup (E0-1), the sources framework (E0-2), the extension registry (E0-8), then Projects (operator, 2026-11-13).

### OD-W13. The build models, and pull requests from forks

**Decided (operator, 2026-11-16).** (1) Anima has both build models: it schedules its own builds from a push, and it can register an owned machine as a self-hosted runner of GitHub Actions. "Anima must not register a machine as a self-hosted runner" is withdrawn. The goal for the runner is a set-up with one press (PRO-010, PRO-027). (2) Anima does not refuse a pull request from a fork. The owner of the repository sets that in GitHub: "if the user wants to build from fork thats their choice they can harden their git repo in any way we just run it" (PRO-013). (3) Stated again: the installation of a different person builds their projects on their machines and must not be able to build or publish the Anima application. A signing key belongs to a project and reaches a builder for one job only (PRO-008, PRO-012, PRO-028). It needs a ruling number in `programme.md`.

### OD-W14. Is each job isolated, whoever wrote the code (PRO-029)?

**Decided (operator, 2026-11-16): yes.** After OD-W13 the hub cannot know that code is the operator's code. On a public repository the author of a fork selects the runner and the steps, and GitHub says that the owner must supply a clean machine for each job (facts F20, F22).

### OD-W15. Does a job get the least that it needs (PRO-030)?

**Decided (operator, 2026-11-16): yes.** A job from a fork gets no project secret and no signing key unless the operator sets that on. Anima gives a runner job only its registration. This is the default of GitHub (fact F19).

### OD-W16. What does Anima's own build run for a repository (PRO-032)?

**Decided (operator, 2026-11-16): a recipe from a fixed list only.** The desktop release now, the image of a job later. No build language of Anima. A repository that needs other steps uses its workflows on owned machines.

### OD-W17. Which runner mode comes first (PRO-042)?

**Decided (operator, 2026-11-16): warm mode first**, one idle runner for each repository that is set on. It needs no webhook receiver. On-demand mode follows the receiver (WHK, OD-E15).

### OD-W18. A public repository with runners (PRO-044)

**Decided (operator, 2026-11-16).** Anima permits it (OD-W13), shows the recommendation of GitHub and the fork setting of the repository, and asks for one more confirmation. Do not set runners on for `TheArctesian/anima.ing` before tasks W2 and S are accepted. The operator sets its fork rule on GitHub to "all external contributors" first.

### OD-W19. The name of the macOS application: `Anima.app` (design specification) or `Anima.ing.app` (the builder)

**Decided (operator, 2026-11-16): `Anima.ing.app`**, the name that Tauri makes from `productName`. Task R5 changes the design specification Section 3.11 and `carried.rs` to agree. The file names of a release stay `Anima_*` (UPD-010).

### OD-W20. The meaning of "must not be able to build or publish the Anima application" (PRO-028)

**Decided (operator, 2026-11-16).** The source is public under AGPL-3.0-or-later, so each person can compile it. The rule means two things. The product has no release function on a hub that has no keys (PRO-037, PRO-038). The keys stay with the Anima project on the operator's hub (PRO-036).

### OD-W21. The order of the build work, and the tier of the correction of the release

**Decided (operator, 2026-11-16).** Tasks R1 to R7 at tier 0, now, because no release is published. Then the work route (W), then runners in warm mode (B), then Anima's own builds (A). Reason for B before A: B gives each project its builds from the workflows that it has.
