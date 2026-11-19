# Decisions of the work plan

Part of the work plan: [work-plan.md](../work-plan.md) is the entry point, and its Section 7 is this document. A reference to "work plan Section 7" in a different document means this document.

| Item | Value |
| --- | --- |
| Document | The decisions OD-W1 to OD-W30, each with the words of the operator |
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
| OD-W4 | How do commands get to the daemon? | Not a decision of this plan. |
| OD-W5 | The runtime for functions that answer a request | Decided (operator, 2026-11-12) |
| OD-W6 | The builder for a job image | Decided (operator, 2026-11-12) |
| OD-W7 | The compute plan says that the daemon is optional and that metrics use SSH | Not a decision of this plan. |
| OD-W8 | The position of Use against the extensions in tier 3 | Decided (operator, 2026-11-12) |
| OD-W9 | One home with copies, or a copy on each node that syncs between nodes when the hub is down ([GH-2](../work-items/device-manager/GH-2.md)) | Decided (operator, 2026-11-12) |
| OD-W10 | Can the daemon do work that the coordinator gives it? R-115 did not take a command channel to the daemon. | Decided (operator, 2026-11-12) |
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
| OD-W22 | The watchdog ([watchdog.md](../proposals/watchdog.md)): approve its direction, and give it a tier. Its own decisions are OD-WDG-1 to OD-WDG-13. | Decided (R-162, operator, 2026-11-18) |
| OD-W23 | The update of the daemon and the hub ([daemon-and-hub-update-plan.md](../plans/daemon-and-hub-update-plan.md)): its tier. Its own decisions are OD-ROL-1 to OD-ROL-14. | Decided (operator, 2026-11-16) |
| OD-W24 | Can an entry of [need-human-testing.md](../need-human-testing.md) have the state "Deferred", with the reason and the ruling, for a check that can run only against the hub on the VPS or its VPN? The sorting of 2026-11-18 proposes it for fifteen checks ([sort-2026-11-18.md](../research/human-checks/sort-2026-11-18.md)). | Decided (R-169, operator, 2026-11-18: "Approve the sorting"): yes. PLN-014 has the rule. |
| OD-W25 | What is the acceptance run of Phase 3 (WP-53, row 4.1) on a hosted hub? R-169 defers eight of its seventeen checks (P-1 to P-6, P-14, P-16), which can run only against the hub on the VPS or its VPN, and gives new steps to eight more. | Open. No recommendation of the operator's side yet. The choices: (a) the run is the nine checks that stay, done on the hosted hub; (b) the run waits for the self-host path; (c) Phase 3 is accepted by a ruling with the deferred rows named. |
| OD-W26 | What starts a release of Anima with no act of the operator in Anima? Today the operator starts each release (OD-12 and BLD-001 of the design specification, UPD-007, R-107 item 2). [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md), "The automatic release of Anima", has the exact text that changes. The choices: (A) a tag `v<version>` on a commit of `main`; (B) a version file on `main` whose version is more than the published one (the files say 0.1.0 today, so they must then be kept correct); (C) each commit of `main`; (D) no change: one press or one `anima release create`, and each later step is automatic as today. It blocks task R12 and task B8. | Open. Recommendation: (A), the tag. It is one clear mark, GitHub Actions can also start from it, and the refusal of a version that is not more than the published one stays the guard (BLD-002). Each commit (C) is not recommended: each merge would become a signed update of each installed application. |
| OD-W27 | Which path makes the first automatic desktop release? The choices: (A) the short form: the hub starts the release and the desktop builders of today build it (tasks R11 to R13, with U12); 4 tasks, no isolation, no GitHub Actions; (B) model (b): a workflow of GitHub Actions on an owned machine (W1 to W4, B1 to B5, S, B8); 11 tasks; it is what he named; (C) the full model (a): a build from a push as work of the daemon (W1 to W4, A2, A5, S); 7 tasks. The table of the same section compares them. It blocks task R12. | Open. Recommendation: (A) first, then the order of OD-W21 (W, B, A) with no change, so that model (b) follows for the workflows of each repository. If the release itself must be a workflow of GitHub Actions, select (B). |
| OD-W30 | Which hub holds the Anima project and starts its release, when the operator uses the hosted hub each day? A hosted hub cannot publish (R-118), and the release server signs (R-158). The choices: (A) the release server holds it: its builders connect to it, and the operator sees a release with `anima --context` or with a second sign-in; (B) the hosted hub holds the project and asks the release server to sign and publish: a new function between two hubs, and a hub on the pool can then ask for a signature, which is near the option that R-158 did not take. Task R11 finds out if one computer can build for a hub that is not the hub of its sign-in. It blocks task R12 in part. | Open. Recommendation: (A). Decide after the result of task R11. |
| OD-W28 | Can a workflow of GitHub Actions build a release of Anima on an owned machine and give it to the hub? R-114 removed the release workflow, UPD-007 says that there is none, Section 3.10 of the design specification says "GitHub Actions does not build or publish a release", BLD-029 says "no GitHub runner", and R-005 says that GitHub Actions never runs the gates. The choices: (A) yes, with these limits: the workflow runs only for the mark of OD-W26 on `main`, it holds no updater key and no R2 token, only the hub signs and publishes, and the gates run on the owned machine; (B) no: the release of Anima stays a build of Anima (model (a)), and runners are for the other workflows of a repository. It blocks task B8 only. | Open. Recommendation: (B) for now, and decide again when task S is accepted. The release is the one build that ends in a signature that each installed application trusts. |
| OD-W29 | The runner conditions for `TheArctesian/anima.ing`, which is public (OD-W18, PRO-044): no runner before tasks W2 and S are accepted, and the fork rule "all external contributors" first. The choices: (A) no change; (B) permit one runner before S, only for the release workflow of task B8; (C) build the release from a private copy of the repository. It blocks task B8 only. | Open. Recommendation: (A), no change. GitHub says that a public repository must almost never use a self-hosted runner (fact F22 of the plan). |

### OD-W1. Can Anima change a rented resource (redeploy, purge a cache, restart the VPS)?

**Decided (operator, 2026-11-12): yes, with the CLI tool of each service, never from the hub.** The hub stays read-only (PRJ-003). First, the client runs the CLI on the operator's computer with `anima run` (SCR-006). When jobs exist, a CLI can also run as a job. It changes PRJ-010 ("changes happen there, not in Anima"). **Exception (operator, 2026-11-13): GitHub.** The hub makes changes on GitHub through the GitHub App of the operator, each change a named function that is in the audit log (PRJ-017, PRJ-027 to PRJ-029). A different rented service can get the same later, with one ruling for each service. Until then this decision applies to it.

### OD-W2. How do other persons use Anima?

**Decided (operator, 2026-11-12): both. A person can host Anima on a service of their choice, and a paid hosted option runs it for them.** [other-operators.md](../proposals/other-operators.md) has the operator's words and what this changes. Nothing is built for it now.

### OD-W3. The name of the hub in the interface

**Decided (operator, 2026-11-12): keep "hub".**

### OD-W4. How do commands get to the daemon?

Not a decision of this plan. It is open in [C1](../work-items/compute/C1.md), [SPEC-C](../work-items/compute/SPEC-C.md) and [OD-C2](../work-items/decisions/OD-C2.md). OD-W10 gives its direction: the daemon asks the hub for work and listens on no port (USE-005, MET-020).

### OD-W5. The runtime for functions that answer a request

**Decided (operator, 2026-11-12): later, as containers that stay warm.** No WebAssembly runtime. It comes after jobs (JOB-006).

### OD-W6. The builder for a job image

**Decided (operator, 2026-11-12): keep more than one way.** A job image can come from Railpack, from a Dockerfile or from a Nix flake (JOB-002, JOB-003, JOB-008). Test Railpack later.

### OD-W7. The compute plan says that the daemon is optional and that metrics use SSH

Not a decision of this plan. [SPEC-C](../work-items/compute/SPEC-C.md) and [OD-C2](../work-items/decisions/OD-C2.md) record that R-084 replaces that text.

### OD-W8. The position of Use against the extensions in tier 3

**Decided (operator, 2026-11-12): jobs and AI agent runs first, then the extensions one at a time, then hosted applications.**

### OD-W9. One home with copies, or a copy on each node that syncs between nodes when the hub is down ([GH-2](../work-items/device-manager/GH-2.md))

**Decided (operator, 2026-11-12): one home, copies by policy.** The section "Decision" of [coordinator-standby.md](../proposals/coordinator-standby.md) has the operator's words. It needs a ruling number in [rulings.md](../rulings.md). [GH-2](../work-items/device-manager/GH-2.md) keeps only its phase L0.

### OD-W10. Can the daemon do work that the coordinator gives it? R-115 did not take a command channel to the daemon.

**Decided (operator, 2026-11-12): yes, one daemon, on by default ("it should be on by default").** The section "Decision" of [jobs.md](../proposals/jobs.md) has the limits. R-160 (2026-11-18) is the ruling that changes R-115; it confirms the default also for a hosted hub ("A: on everywhere"). It also changes CMP-002 and the recommendation of [OD-C8](../work-items/decisions/OD-C8.md). A daemon that is already installed takes work after its update, with no confirmation.

### OD-W11. Are projects the front door for rented accounts, and does the builder build each project?

**Decided (operator, 2026-11-13): yes.** Connect GitHub, import each repository, leave out the ones that the operator selects. Anima proposes resources from signs in the repository. The builder is no longer specific to Anima. The three resource types stay different. A different forge must be possible. [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md) has the six points.

### OD-W12. The proposals that follow OD-W11

**Decided (operator, 2026-11-13).** A build is the first job type and comes directly after Projects (PRO-009). Anima schedules builds from a webhook of the GitHub App, through the webhook receiver of the engine (WHK-001 to WHK-010), with a slow poll that repairs a lost webhook. Anima can also start a workflow of GitHub Actions (PRO-010, PRO-020, PRO-021). OD-W13 withdrew "a machine is not a runner". The receiver runs on the VPS as a separate program (OD-E15); to open its port needs the confirmation of the operator (VPS-005). A Mac build is a native job that runs as a build user, in a new checkout that is deleted, on a Mac that the operator set on for it (PRO-011, PRO-022 to PRO-024). Keys belong to the project (PRO-012). OD-W13 replaced "no builds of pull requests from forks" (PRO-013). Sign-in with a forge comes later, with the local sign-in as the root (PRO-015, PRO-016). PRJ-017 replaces "read access first" (PRO-014). The hub gives a project secret to one job (JOB-010). The first build is the path to Projects only: the hub folder regroup (E0-1), the sources framework (E0-2), the extension registry (E0-8), then Projects (operator, 2026-11-13).

### OD-W13. The build models, and pull requests from forks

**Decided (operator, 2026-11-16).** (1) Anima has both build models: it schedules its own builds from a push, and it can register an owned machine as a self-hosted runner of GitHub Actions. "Anima must not register a machine as a self-hosted runner" is withdrawn. The goal for the runner is a set-up with one press (PRO-010, PRO-027). (2) Anima does not refuse a pull request from a fork. The owner of the repository sets that in GitHub: "if the user wants to build from fork thats their choice they can harden their git repo in any way we just run it" (PRO-013). (3) Stated again: the installation of a different person builds their projects on their machines and must not be able to build or publish the Anima application. A signing key belongs to a project and reaches a builder for one job only (PRO-008, PRO-012, PRO-028). It needs a ruling number in [rulings.md](../rulings.md).

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

**Decided (operator, 2026-11-16): `Anima.ing.app`**, the name that Tauri makes from `productName`. Task R5 changed the design specification Section 3.11 and `carried.rs` to agree (Done, c1e6226e). The file names of a release stay `Anima_*` (UPD-010).

### OD-W20. The meaning of "must not be able to build or publish the Anima application" (PRO-028)

**Decided (operator, 2026-11-16).** The source is public under AGPL-3.0-or-later, so each person can compile it. The rule means two things. The product has no release function on a hub that has no keys (PRO-037, PRO-038). The keys stay with the Anima project on the operator's hub (PRO-036).

### OD-W21. The order of the build work, and the tier of the correction of the release

**Decided (operator, 2026-11-16).** Tasks R1 to R7 at tier 0, now, because no release was published on that day (the release record in [releases.md](../procedures/releases.md) holds the state now). Then the work route (W), then runners in warm mode (B), then Anima's own builds (A). Reason for B before A: B gives each project its builds from the workflows that it has.

### OD-W22. The watchdog ([watchdog.md](../proposals/watchdog.md)): approve its direction, and give it a tier. Its own decisions are OD-WDG-1 to OD-WDG-13.

**Decided (R-162, operator, 2026-11-18: "A: approve, tier 3").** The direction is approved with the recommendations of the proposal for OD-WDG-1 to OD-WDG-13. The tier is 3, after jobs. The requirements WDG-001 to WDG-041 are not approved one by one, so the document stays a proposal (PLN-018). The recommendation was: approve it as a function of `anima-daemon`, not as a container. Tier 3c, before hosted applications, which can then use its check (HOST-008). Its stage 1 (watches made at the machine) needs no work route and can start after stage B of the update, if the operator wants it earlier. R-160 is the ruling for OD-W10. The change to MET-002 for a restart has no ruling yet.

### OD-W23. The update of the daemon and the hub ([daemon-and-hub-update-plan.md](../plans/daemon-and-hub-update-plan.md)): its tier. Its own decisions are OD-ROL-1 to OD-ROL-14.

**Decided (operator, 2026-11-16): tier 1, directly after the first release.** The operator's words: "thats fine were still pre release". Reason: the work route and the watchdog reach an installed machine only through an update (USE-008). The stages stay a recommendation: the versions show, the machines update, then the hub, after the restore procedure (FND-006) is written. OD-ROL-8 is decided: no Linux machine on ARM is in use. R-139 (operator, 2026-11-17) decided OD-ROL-1 to OD-ROL-14 and deferred the stage of the hub. Open: OD-ROL-15, where the key that signs a release is when the operator uses only a hosted hub.

### OD-W24. Can an entry of the list of human checks have the state "Deferred"?

**Decided (R-169, operator, 2026-11-18: "Approve the sorting"): yes.** The fifteen checks of the sorting are deferred, the restore check HT-024 included; the orchestrating session had recommended to keep that one open. PLN-014 has the rule. The entries change in a later commit (R-169). The text before the decision: the states of an entry are Open, Passed and Failed (PLN-014; [need-human-testing.md](../need-human-testing.md), "How to use this list"). The operator will use only the hosted hub (R-130), and some checks can run only against the hub on the VPS or its VPN. The operator asked for a sorting of the open checks on 2026-11-18 ("Yes, sort them"); [sort-2026-11-18.md](../research/human-checks/sort-2026-11-18.md) is that sorting, and it proposes to put fifteen checks aside. No rule has a state for that. Recommendation: add the state "Deferred". Only a ruling of the operator sets it. The entry gives the reason and the ruling, stays in the list, and goes back to Open when the self-host path is in work again. A task that waits for a deferred check is not Done (PLN-009). Until the operator decides, no entry has this state.

### OD-W25. What is the acceptance run of Phase 3 (WP-53) on a hosted hub?

**Open.** The acceptance run of Phase 3 is the checks P-1 to P-17 of [need-human-testing.md](../need-human-testing.md) (steps: [phase-3-acceptance.md](../plans/phase-3/phase-3-acceptance.md)). Row 4.1 of the priorities waits for it. R-169 defers P-1 to P-6, P-14 and P-16 for the self-host path, gives P-7 to P-13 and P-15 new steps for a hosted hub, and points P-17 to REL-1. The run can thus not end as it is written. The operator did not answer this question on 2026-11-18. The choices are in the table above. Until he decides, row 4.1 stays as it is and no agent sets WP-53.
