---
kind: Plan, with the task table for builds and runners
state: In progress
evidence: It was a proposal until 2026-11-17 (PLN-018). PRO-031 to PRO-053 have no ruling, except the requirements that OD-W14 to OD-W21 name. Decided (OD-W11, OD-W12, OD-W13). Connect GitHub is on `main`. Version 2 (2026-11-16): OD-W14 to OD-W21 are decided; its status line names the requirements that still have no ruling. Its "Tasks" holds the state of each task: P0 and R1 to R6 are done, R7 is in progress, and each other task is not started. Version 2.3 (2026-11-18) adds the path to an automatic release of Anima, with the rows R11 to R13 and B8. Version 2.4 (2026-11-18) records R-170 and R-171: the first automatic release goes by the runner path, and a workflow on a runner of the operator can build a release; it adds PRO-054 to PRO-058 and task B9, which have a direction and no approval. OD-W26, OD-W29, OD-W30, OD-W31 and OD-W32 are open.
---

# Plan: projects first (import from GitHub, the builder as a job, sign-in)

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this plan (PLN-005).

Version: 2.4. Date: 2026-11-18. Version 2.4 records R-170 (OD-W27: the runner path) and R-171 (OD-W28: a workflow on a runner of the operator can build a release), puts the chain in one table as it stands after them, and adds PRO-054 to PRO-058 with task B9 (the set-up for a coding agent and the text to copy) and the open decisions OD-W31 and OD-W32. Version 2.3 adds the section "The automatic release of Anima" and the task rows R11 to R13 and B8; it changes no requirement, and its open decisions are OD-W26 to OD-W30. Version 2.2 (2026-11-17) adds the results of task P0 and its decisions Q1 to Q4 (operator, 2026-11-17); it changes no requirement. Version 2.1 only moved this document from `programme/proposals/projects-and-builds.md` (PLN-018, operator, 2026-11-17).
From: a side chat with the operator on 2026-11-13, sent to the planning session. Version 2 adds
the rulings of 2026-11-16 (OD-W13) and the plan for the two build models.
Status: the header of this document holds the state (PLN-020). History: In progress. The operator decided the six points in "Decision" (OD-W11), the points in
"Decision on the proposals" (OD-W12) and the three points in "Decision of 2026-11-16" (OD-W13).
The operator decided OD-W14 to OD-W21 on 2026-11-16: PRO-029, PRO-030, PRO-032, PRO-037, PRO-038,
PRO-044, the first mode of PRO-042 and the order of the tasks. The other requirements from
PRO-031 to PRO-053 are proposals with no ruling. Connect GitHub and the Work, Code and History
tabs are on `main`. No build function of this proposal is built.
Map: [work-plan.md](../work-plan.md), categories See and Use, decisions OD-W11 to OD-W21.
Identifiers: PRO-001 to PRO-058. The prefix `PRJ` belongs to the extensions plan and is not
changed here.

## Problem

Projects ([XP-02](../work-items/extensions/XP-02.md)) is one extension in a list. But the rented accounts of the operator
(Cloudflare, Vercel, Hostinger, Neon) only have a meaning through the project that runs on them.
And the release builder (design specification BLD-001 to BLD-082) builds only the Anima desktop
application. The operator wants one front door: connect GitHub, get each repository as a project,
see where it runs, and build it on the machines that the operator owns.

## Decision (OD-W11, operator, 2026-11-13)

| Number | Decided |
| --- | --- |
| 1 | Projects are the front door for rented accounts. The model is the operator's HQ application: one project for each repository, with an Operations area (infrastructure, state of each service, cost). Anima takes only that part, not the company parts. |
| 2 | When the operator connects GitHub, Anima imports each repository. The operator can then leave out the repositories that they select. |
| 3 | Anima reads each repository for signs of where it runs, and proposes resources. The operator confirms. The operator signs in to a provider only when a project needs it. |
| 4 | The builder must not be specific to the Anima application. It becomes a builder for each GitHub project of the operator. Later, the installation of a different person builds their applications on their machines, and must not be able to build this application. |
| 5 | Three types stay different: compute that you own (operate), compute that you rent (oversee), and data that is held about you (read-only sources). |
| 6 | A different forge (GitLab, Bitbucket) must be possible later. The forge is a source behind a small boundary. |

Requirements that follow directly from the decision:

- **PRO-001.** One project for each repository. A project can have more than one repository
  (PRJ-001 keeps this).
- **PRO-002.** The import takes each repository that the connection can read. The operator can
  leave out a repository, and Anima must not import it again.
- **PRO-003.** The hub must look for these signs in a repository, and for the deployment records
  of the forge:

  | Sign | Proposed resource |
  | --- | --- |
  | `wrangler.toml` | Cloudflare (Workers or Pages) |
  | `vercel.json`, or a Vercel deployment record | Vercel |
  | `Dockerfile`, a compose file | A server |
  | `tauri.conf.json` | A desktop application that needs a builder |

- **PRO-004.** A sign is a proposal. The project page must mark the resource "found in the
  repository" until the provider account confirms it or the operator confirms it.
- **PRO-005.** Anima must ask for a provider sign-in only when a confirmed project uses that
  provider.
- **PRO-006.** The three types of the work plan (Section 1) apply to a project:
  - A project can run on owned compute and on rented compute.
  - A service that holds data about the operator (WHOOP, Garmin, Spotify, Screen Time) is a
    source for Remember. It is never a resource. A project must not list one as its hosting.
  - A job on an owned machine can read the local copy that Anima holds. It must not call the
    service.
- **PRO-007.** The forge is a source of the engine (SRC). Code that is specific to GitHub must be
  behind the source boundary (K-E1), so that a second forge adds a source and changes nothing
  else (ETH-009).
- **PRO-008.** A builder builds for the projects of its own hub only. It must take work only from
  the hub that enrolled it (BLD-021, BLD-022 keep this). The operator stated this again on
  2026-11-16 (OD-W13, PRO-028).

## Decision on the proposals (OD-W12, operator, 2026-11-13)

The operator decided each point in the planning session. PRO-010 and PRO-014 have new text; the
other identifiers keep their text. OD-W13 (2026-11-16) changed PRO-010, PRO-013 and PRO-020
again: the text below is the text of OD-W13.

- **PRO-009. A build is a job.** Decided. The builder becomes the first job type of
  [jobs.md](../proposals/jobs.md). The order is: Projects (See), then builds as jobs (Use), then the other
  extensions. Fine-tuning ([C7](../work-items/compute/C7.md)) is the second job type. This is the order of the work only:
  Projects stays an extension that is off in a new installation (EXT-005, OD-E13).
- **PRO-010. Anima has two build models.** Decided (OD-W13, operator, 2026-11-16; it replaces
  the text of 2026-11-13).
  - Model (a): Anima schedules a build. The GitHub App tells the hub about a push with a
    webhook. The webhook goes to the webhook receiver of the engine. The hub gets it over a
    connection that the hub opens, so the hub stays off the public internet (WHK-001 to WHK-010,
    SEC-008; operator, 2026-11-13: "I would prefer the webhook version"). The hub places the
    build job on an owned machine. The result goes back to the commit as a status.
  - Model (b): Anima registers an owned machine as a self-hosted runner of GitHub Actions
    (PRO-027). Then the workflows of the repository run on the machines of the operator.
  - Withdrawn: "Anima must not register a machine as a self-hosted runner of GitHub Actions",
    and its reason. Model (a) is still the path that works for a different forge (PRO-007).
- **PRO-020.** For a repository that has workflows, Anima can start a workflow by hand and can
  start a workflow run again. Each is a named GitHub function (PRJ-027, OD-E14). The run uses
  the runner that the workflow names: a runner of GitHub, or an owned machine (PRO-027). Anima
  shows its state (Section 4.2 of the extensions plan). (Changed by OD-W13: the first text said
  that the run stays on the runners of GitHub.)
- **PRO-021.** The hub must not start a second build for a commit that it has built. A webhook
  is a signal to read, not the record: a slow poll must find a push whose webhook was lost. The
  project page must show the time of the last webhook and of the last poll.
- **PRO-011. Native jobs on a Mac come early.** Decided. A job has two kinds: a container job
  (the default) and a native job on a Mac. Use a native job only when the work needs macOS
  directly, for example a macOS or iOS build.
- **PRO-022.** A native job must run as a macOS user that is made for builds. This user must not
  be the login user of the operator, must not be an administrator, and must not be able to read
  the home folder of the operator or the token of `anima-daemon`.
- **PRO-023.** A native job must run in a new checkout. The machine must delete the checkout,
  the keychain of the job and the secrets of the job when the job ends, also when it fails or is
  stopped.
- **PRO-024.** The operator must set a Mac on for native jobs. A Mac that takes container work
  (USE-004) does not take native jobs until then, because a native job has no container around
  it.
- **PRO-012. Keys belong to the project.** Decided. A signing key and a secret belong to a
  project, not to a builder. A builder gets them for one job (JOB-010, SCR-009). The operator
  stated this again on 2026-11-16 (OD-W13).
- **PRO-013. The owner of the repository decides about forks.** Decided (OD-W13, operator,
  2026-11-16; it replaces "Never build a pull request from a fork"). Anima must not refuse a
  pull request from a fork. The owner of the repository sets in GitHub if pull requests from
  forks run. Anima runs what it is sent. The operator's words: "if the user wants to build from
  fork thats their choice they can harden their git repo in any way we just run it". A build
  still runs the code of the repository on an owned machine: PRO-029 and PRO-030 are the
  limits.
- **PRO-014. Read-only sources stay read-only.** The first text ("read first, write later") is
  replaced by PRJ-017: the GitHub App of the operator has read and write permission from the
  start, and each change is a named function (PRJ-027). This part stays: a sign-in to a source
  of data that is held about the operator is read-only always (ETH-007).
- **PRO-015. Two different things with GitHub.** Decided. "Connect GitHub" makes a source.
  "Sign in with GitHub" is identity. They are separate.
- **PRO-016. Forge sign-in is an extra method, later.** Decided. GitHub, GitLab and Bitbucket
  can become sign-in methods, mainly for the hosted option
  ([other-operators.md](../proposals/other-operators.md)). The local sign-in stays the root, so that a forge
  that is down or an account that is locked never stops access to the owned machines.

## Decision of 2026-11-16 (OD-W13, operator)

| Number | Decided |
| --- | --- |
| 1 | Anima has both build models. Model (a): Anima schedules a build from a push and writes a commit status. Model (b): Anima registers an owned machine as a self-hosted runner of GitHub Actions, so the workflows of a repository run on the operator's machines. The sentence "Anima must not register a machine as a self-hosted runner" is withdrawn. The goal for model (b) is a set-up with one press. |
| 2 | Anima does not refuse a pull request from a fork. The owner of the repository sets in GitHub if fork pull requests run. Anima runs what it is sent. |
| 3 | Not changed, and stated again: the installation of a different person builds their projects on their machines. It must not be able to build or publish the Anima application. A signing key belongs to a project and reaches a builder for one job only. |

PRO-010, PRO-013 and PRO-020 have the new text above. These requirements follow directly:

- **PRO-027.** Anima can register an owned machine as a self-hosted runner of GitHub Actions for
  a repository of a project. The operator sets this on for each project. The set-up must need
  the fewest steps that GitHub permits; "One press" below gives them.
- **PRO-028.** An installation must build only the projects of its own hub, on the machines of
  that hub (PRO-008). An installation of a different person must have no function that builds or
  publishes a release of the Anima application (OD-W11 decision 4). The keys of that release
  belong to the Anima project on the operator's hub (PRO-012).

A fact that limits PRO-028: the source of Anima is public (`TheArctesian/anima.ing`, licence
AGPL-3.0-or-later). Each person can compile it. Anima can keep only two things from a different
person: the function in the product, and the keys. The keys are the updater key, the token of
the release bucket and the Developer ID identity (R-116, R-117). The operator confirmed this
meaning (OD-W20, 2026-11-16): the rule is PRO-037 and PRO-038, and the keys stay with the Anima
project on the operator's hub.

## Limits for each job (OD-W14, OD-W15, operator, 2026-11-16)

The session of 2026-11-16 proposed these two. The operator accepted them the same day.

- **PRO-029. Each job is isolated, whoever wrote the code.** Decided (OD-W14). A container job runs in a
  new container that the machine deletes. A native job on a Mac runs as the build user, in a new
  checkout that the machine deletes (PRO-022 to PRO-024). This applies to model (a) and to
  model (b), to the operator's branches and to a fork. Reason: after OD-W13 the hub cannot know
  that the code is the operator's code.
- **PRO-030. A job gets the least that it needs.** Decided (OD-W15). In model (a), a job that a pull
  request from a fork starts gets no secret of the project and no signing key, unless the
  operator sets that on for the project. This is the default of GitHub (fact F19). In model (b),
  Anima gives a runner only its registration (PRO-040). GitHub gives that job the secrets of
  the workflow, by the rules of GitHub.

## Builds in the client (operator, 2026-11-13)

The operator's words, from a side chat: "we can embed it in to the UI somehow on the bittom I like
just above the settings maybe for build running and live sessions".

- **PRO-025.** When builds are jobs of a project, the client must show the builds of a project on
  the project page, and the release of Anima must start from the Anima project. Then the
  navigation bar must not have a "Builds" entry. Until then the Builds view stays, because it is
  the only way to release Anima (BLD-001, BLD-080, UPD-007).
- **PRO-026.** The navigation bar must have an area at the bottom, above Settings, that shows
  what runs now: each build that runs and each session that is open. The area must show nothing
  when nothing runs.

## The builder today

Read in the code of `main` at commit `db625835` on 2026-11-15. Nothing is changed.

| Part | Where | What it does |
| --- | --- | --- |
| Machine | `apps/desktop/src-tauri/src/builder/` | Fixed steps for one repository. The URL is a constant in `plan.rs`. A step can start only seven named programs. |
| Client loop | `apps/client/src/lib/builds/` | The desktop application claims a job and runs four steps: checkout, gates, bundle, upload. |
| Hub | `crates/hub/src/fleet/builds/` | Releases, jobs, leases, builders, upload addresses, checks, the signature and the feed. |
| Start | The Builds view, `anima release create`, the MCP tool `anima_release_create` | A release of the Anima application, and nothing else. |

Each desktop application shows the "Builds" entry (BLD-080). `create_release` of the hub does not
check that the hub can publish. Thus the installation of a different person can queue a release of
Anima, run its gates on a machine, and never publish it.

Defects, each verified in the code. No defect is corrected.

| Defect | What the code does | Verified by |
| --- | --- | --- |
| D1. An old file can be uploaded | `stabilize_names` in `artifact.rs` stops when the stable `Anima_*` file exists. The checkout keeps `target/` (`git clean --exclude=/target/`). A second attempt at one version makes a new `Anima.ing_*` file and then uploads the file of the first attempt. | Reading. No test shows it. |
| D2. Two names for the macOS application | Tauri writes `Anima.ing.app`, from `productName`. The design specification Section 3.11 and `local_hub/carried.rs` say `Anima.app`. The updater file `Anima.app.tar.gz` holds `Anima.ing.app`. | Reading. `tauri-plugin-updater` 2.14.1 drops the first folder of the archive and replaces the content of the installed application, so an installed `Anima.app` keeps its name. No real update has proved this. |
| D3. The release does not carry the local hub | `local_hub/carried.rs` looks for `anima-local-hub` and a database folder in the application. The builder puts only `anima` there, and `mac_binaries` refuses a third program. | Reading. Task T9 of [s4-local-hub-tasks.md](one-click/s4-local-hub-tasks.md) owns the correction. |
| D4. The tool check has no Go | `builder/tools.rs` checks git, bun, Rust, cargo-nextest, PostgreSQL and the Tauri packages. The gates also run six Go gates for `link/`. A builder with no Go claims a job and fails at the gates. | Reading. |
| D5. The clone address had the old name (corrected: task R1, commit `e3058276`) | On 2026-11-15, `plan.rs` and BLD-040 said `TheArctesian/anima.git`. The repository is `TheArctesian/anima.ing`. GitHub redirects the old name. The redirect stops if a repository takes the old name. `plan.rs` and BLD-040 now say `TheArctesian/anima.ing.git`. | `gh repo view` on 2026-11-15. |
| D6. No release was published on 2026-11-15 (changed on 2026-11-17: the release record in [releases.md](../procedures/releases.md) holds the state) | Each feed at `releases.anima.ing` answered 404 on 2026-11-15: `linux-x86_64`, `darwin-aarch64`, `darwin-x86_64`. The macOS feeds give 0.2.0 since 2026-11-17 ([s12-packaging-tasks.md](one-click/s12-packaging-tasks.md) Section 6.4); the `linux-x86_64` feed is not checked here. OD-16 to OD-18 of the design specification are not verified. | `curl`. |
| D7. No pinned toolchain | `main` has no `rust-toolchain.toml` and no `flake.nix`. The tool check now names a missing Rust target for the universal macOS build, so that builder does not claim. | Reading. It replaces the fact of 2026-11-13 in this section. |
| D8. A hub that cannot publish accepts a release | See the paragraph above the table. The hub fails the job only after the build, with the reason that publishing is off. | Reading `fleet/builds.rs` and `fleet/builds/publishing.rs`. |

## Plan A: the general builder

PRO-032, PRO-037 and PRO-038 are decided (OD-W16, OD-W20; operator, 2026-11-16). The other
requirements of this section are proposals with no ruling.

- **PRO-031. A build belongs to a project.** A build is one item of work. It names the project,
  the repository, the commit, the recipe and the trigger. The trigger is the operator or a push.
- **PRO-032. A build runs a recipe from a fixed list.** Anima must not have a build language of
  its own. The first recipe is the desktop release: the steps that the builder has today, with
  the project as a parameter. The second recipe makes the image of a job or of a hosted
  application (JOB-002, JOB-003, JOB-008). Build it when a job needs it. A repository that needs
  other steps uses its workflows on owned machines (PRO-027). Decided (OD-W16).
- **PRO-033. The daemon does the build.** `anima-daemon` gets a build through the work route
  (USE-005, USE-006). The builder in the desktop application (BLD-020 to BLD-029) stays only for
  the release of Anima, until the daemon can do that release. Then the desktop builder goes.
  Reason: the desktop builder runs as the operator, with the operator's keychain (PRO-022).
- **PRO-034. The repository is a parameter.** The address and the branch come from the project.
  The code must hold no name and no address of the operator (ETH-002). For a private repository,
  the hub gives the machine a token for one job. The token must be for that repository only,
  with read access to its content only (fact F26, JOB-010).
- **PRO-035. A job does not trust the files of a different job.** A build starts from a new
  checkout (PRO-023, PRO-029). A cache is permitted only for one project. A job must not read a
  cache that a job from a fork wrote. A release build must not read a cache that a different
  build wrote. This also removes defect D1.
- **PRO-036. The release of Anima is one project.** It is the desktop release recipe of the
  project of the Anima repository. Its keys are secrets of that project (PRO-012): the updater
  key, the token of the release bucket, the Developer ID identity and the notary credentials.
  Until the Secrets extension exists (SCR-001, SCR-009), the keys stay where R-116 and R-117 put
  them. A release still starts by hand (BLD-001, OD-12 of the design specification). Decided
  (OD-W20): the keys stay with the Anima project on the operator's hub. The other sentences are
  proposals.
- **PRO-037. A hub with no keys has no release.** Decided (OD-W20). The hub must refuse a release when it cannot
  publish. The refusal must give a typed reason. The hub must tell the client if it can publish.
- **PRO-038. What a different person sees.** Decided (OD-W20). When the hub cannot publish, the client must show
  no "Builds" entry and no "Build releases on this computer" row. The "Updates" row stays: that
  installation still gets the releases of Anima from the public feed. The builds of a project
  are on its project page (PRO-025). A project with no build must show the two ways to get one:
  a recipe that Anima found (PRO-003), or its workflows on owned machines (PRO-027).

The release of Anima thus has three states, in this order:

1. Today: the Builds view and the desktop builder. Group R of "Tasks" corrects the defects.
2. Interim: the same builder, started from the Anima project page. The hub refuses when it has
   no keys (PRO-037, PRO-038). The "Builds" entry goes (PRO-025).
3. End: a native job of `anima-daemon` on a Mac, and a container job on Linux, with the keys as
   project secrets (PRO-033, PRO-036).

## Plan B: owned machines as runners of GitHub Actions

### Facts of GitHub

Read in the documentation of GitHub on 2026-11-15. `D` is `https://docs.github.com/en`. "Not
stated" means that no page says it; task B1 must then prove it with the operator's account.

| Fact | Content | Source |
| --- | --- | --- |
| F1 | A just-in-time runner comes from `POST /repos/{owner}/{repo}/actions/runners/generate-jitconfig`, or the same path under `/orgs/{org}`. The body has `name`, `runner_group_id` and `labels`. The answer has `encoded_jit_config`. The runner program starts with `./run.sh --jitconfig <config>`. The runner does one job at most. Then GitHub removes it. | `D/rest/actions/self-hosted-runners`, `D/actions/reference/security/secure-use` |
| F2 | The repository path needs the repository permission "Administration", write. The organisation path needs the organisation permission "Self-hosted runners", write. An installation token of a GitHub App is accepted. | `D/rest/authentication/permissions-required-for-github-apps` |
| F3 | "Administration", write, also permits these: delete the repository, change its settings, change branch protection and rule sets, add a collaborator, add a deploy key, change the Actions permissions. | The same page |
| F4 | The App that Anima makes has "Administration", "Actions", "Workflows", "Commit statuses", "Checks", "Contents", "Pull requests" and "Variables", write (PRJ-039). It has no organisation permission and no event, and its webhook is off. | `crates/providers/github/src/registration/manifest.rs` |
| F5 | The documentation names three levels for a runner: repository, organisation, enterprise. It has no path for a personal account. Runner groups are for organisations and enterprises. Not stated in one sentence. The group number 1 is only in examples. | `D/actions/how-tos/manage-runners/self-hosted-runners/add-runners`, `D/rest/actions/self-hosted-runner-groups` |
| F6 | GitHub removes a runner that was offline for 14 days, and an ephemeral runner after 1 day. A job can run for 5 days. A job can wait in the queue for 24 hours. The life of a JIT configuration that no runner used: not stated. | `D/actions/how-tos/manage-runners/self-hosted-runners/remove-runners`, `D/actions/reference/limits` |
| F7 | A runner must be current. The reference page says 63 days after a new version. The changelog of 2026-06-12 says 30 days. A runner below 2.329.0 cannot register since 2026-09-29. `GET /repos/{owner}/{repo}/actions/runners/deprecations/{version}` gives the dates for a version. | `D/actions/reference/runners/self-hosted-runners`, `github.blog/changelog/2026-06-12-…minimum-version-enforcement…`, `github.blog/changelog/2026-09-28-…enforcement-date-has-moved` |
| F8 | The runner program has the MIT licence. GitHub publishes it for Linux (x64, arm64, arm) and macOS (x64, arm64). It refuses to run as root. | `github.com/actions/runner`: `LICENSE`, `config.sh` |
| F9 | A self-hosted runner costs nothing at GitHub. GitHub announced a charge for private repositories on 2025-12-16 and then postponed it. No new date is published. | `D/billing/concepts/product-billing/github-actions`, `github.blog/changelog/2025-12-16-…` |
| F10 | The event `workflow_job` has the actions `queued`, `in_progress`, `completed` and `waiting`. An App needs "Actions", read. The payload has `labels`, `run_id`, `head_sha`, `status`, `conclusion`, `runner_id` and `runner_name`. The runner fields are null while the job is queued. That the `queued` payload has the `runs-on` labels: in the schema, not stated in a sentence. | `D/webhooks/webhook-events-and-payloads#workflow_job` |
| F11 | Without a webhook: `GET /repos/{owner}/{repo}/actions/runs?status=queued`, then `GET …/actions/runs/{run_id}/jobs`. Each needs "Actions", read. | `D/rest/actions/workflow-runs`, `D/rest/actions/workflow-jobs` |
| F12 | `runs-on` selects a runner by labels. A runner must have each label of the list. A runner that the configuration script adds gets `self-hosted`, its operating system and its architecture. That a JIT runner gets these: not stated. The example of F1 gives them in `labels`. | `D/actions/reference/workflows-and-actions/workflow-syntax`, `D/actions/how-tos/manage-runners/self-hosted-runners/use-in-a-workflow` |
| F13 | `runs-on` can read the context `vars`. The documentation shows `runs-on: ${{ vars.RUNNER }}`. A variable that is not set gives an empty text. That a workflow of a fork pull request gets `vars`: not stated. | `D/actions/reference/workflows-and-actions/contexts`, `D/actions/how-tos/write-workflows/choose-what-workflows-do/use-variables` |
| F14 | To change a file in `.github/workflows` an App needs "Contents", write, and "Workflows", write. To open a pull request it needs "Pull requests", write. | `D/rest/repos/contents`, `D/apps/creating-github-apps/registering-a-github-app/choosing-permissions-for-a-github-app` |
| F15 | A manifest gives `default_permissions`, `default_events` and `hook_attributes` when the App is registered. The documentation describes no later use of a manifest. | `D/apps/sharing-github-apps/registering-a-github-app-from-a-manifest` |
| F16 | The documentation gives only the settings page as the way to change the permissions of an App. The REST reference has no call for it. GitHub then sends an email to the owner of each installation. The installation keeps its old permissions until the owner approves. The event `installation` with the action `new_permissions_accepted` reports the approval. That a new event alone needs approval: not stated. | `D/apps/maintaining-github-apps/modifying-a-github-app-registration`, `D/apps/using-github-apps/approving-updated-permissions-for-a-github-app` |
| F17 | An App has one webhook. `PATCH /app/hook/config` with the App JWT sets its address and its secret. It has no field for events. Each delivery has the header `X-Hub-Signature-256`. A delivery fails after 10 seconds. GitHub does not send a failed delivery again; `POST /app/hook/deliveries/{delivery_id}/attempts` does. | `D/rest/apps/webhooks`, `D/webhooks/using-webhooks/handling-failed-webhook-deliveries` |
| F18 | Public repository, default: a first-time contributor needs approval before a workflow runs. A contributor with a merged commit is not first-time. The owner can select "all external contributors". | `D/repositories/managing-your-repositorys-settings-and-features/enabling-features-for-your-repository/managing-github-actions-settings-for-a-repository` |
| F19 | A workflow of a fork pull request gets no secret. Its `GITHUB_TOKEN` can only read. | `D/actions/reference/workflows-and-actions/events-that-trigger-workflows` |
| F20 | For `pull_request`, GitHub runs the workflow file of the merge commit of the pull request. The author of the fork controls that file. Thus the author can name the labels of a self-hosted runner and each step. | `D/actions/reference/security/securely-using-pull_request_target` |
| F21 | Private repository: a workflow of a fork pull request runs only when the owner enables it. Three more settings send a write token, send secrets, or require approval. | The page of F18; `D/rest/actions/permissions` |
| F22 | GitHub recommends self-hosted runners only for private repositories. It says that a public repository must almost never use one. It gives the same warning for a private repository that persons can fork. A JIT runner limits the registration to one job. It does not clean the machine: GitHub says that the owner must supply a clean environment for each job. | `D/actions/how-tos/manage-runners/self-hosted-runners/add-runners`, `D/actions/reference/security/secure-use` |
| F23 | No setting keeps fork pull requests off the runners of a repository. Runner groups can, and they are for organisations. | `D/actions/how-tos/manage-runners/self-hosted-runners/manage-access` |
| F24 | An App can read the fork settings of a repository: `GET …/actions/permissions/fork-pr-contributor-approval` and `GET …/actions/permissions/fork-pr-workflows-private-repos`. Each needs "Administration", read. | `D/rest/actions/permissions` |
| F25 | `pull_request_target` runs the workflow file of the default branch (since 2025-12-08). GitHub blocks this event by default in public repositories from 2026-12-02. | `github.blog/changelog/2026-11-02-pull_request_target-blocked-by-default-december-2/` |
| F26 | An installation token can be made for selected repositories and for a part of the permissions of the App. It lives one hour. | `D/rest/apps/apps#create-an-installation-access-token-for-an-app` |
| F27 | GitHub also has an API for "runner scale sets", with the client `actions/scaleset` (public preview). Its documentation says an App that an organisation owns. This plan does not use it. | `github.com/actions/scaleset`, `D/actions/tutorials/use-actions-runner-controller/authenticate-to-the-api` |

What the facts mean for the operator:

- The App of the operator can register a runner for a repository today, with no new permission
  (F2, F4). The cost of that permission is in F3, and the operator accepted it with PRJ-017.
- The repositories of the operator belong to a personal account. Thus each repository needs its
  own runner (F5). One runner cannot serve all repositories.
- `TheArctesian/anima.ing` is public. GitHub says that a public repository must almost never use
  a self-hosted runner (F22). The author of a fork selects the runner and the steps (F20). The
  approval rule of GitHub is the only gate before the code runs (F18). OD-W13 leaves that
  setting to the owner. Thus isolation on the machine (PRO-029) is the control that Anima has.

### One press

For one repository of a personal account, with the App that Anima made:

| Step | Who | Can Anima remove it? |
| --- | --- | --- |
| Check the permission of the App, select the machines, get the runner program | Anima | It is automatic. |
| Register a runner for one job, start it in isolation, register the next one | Anima | It is automatic (F1). |
| "Run the workflows of this repository on my machines" | The operator, one press in Anima | No. It is the decision. |
| Change `runs-on` in each workflow file, so that it names the labels of Anima | A commit in the repository. Anima can open the pull request (F14). A person merges it. | No. GitHub selects a runner only by the labels in the file (F12). Anima can merge with its merge function, after a confirmation (PRJ-029). |
| Set the fork rule of the repository | The owner, on GitHub | Anima shows the setting (F24). OD-W13 leaves the decision to the owner. |
| Set a Mac on for native jobs, and make its build user | The operator, one time for each Mac | No. To make a macOS user needs an administrator. PRO-024 requires the setting. |
| Subscribe the App to `workflow_job` and set its webhook on | The operator, one time, on the settings page of the App | Not for an App that exists (F16, F17). A new App can get the event from its manifest (F15). The warm mode of PRO-042 does not need this step. |
| Approve a new permission | The owner of each installation, by email | Only an organisation needs a new permission (F2). Not removable (F16). |

Thus "one press" is true for the registration and for the machines. One merge for each
repository stays. With a variable in `runs-on` (F13), that merge is necessary one time only:
later, on and off are one press. Task B1 must prove the variable first.

### Requirements

PRO-044 and the first mode of PRO-042 are decided (OD-W17, OD-W18; operator, 2026-11-16). The
other requirements of this section are proposals with no ruling.

- **PRO-039. One runner for one job.** Anima must register only just-in-time runners (F1). A
  runner does one job. Then the machine deletes the runner, its work folder and its
  configuration (PRO-029). Anima must not keep a runner that did a job.
- **PRO-040. The registration is a secret for one job.** The hub makes the JIT configuration and
  gives it to one machine (JOB-010). It must not be in a log, in the database as plain text, in
  the output of a command or in an MCP tool (MCP-004). Anima gives a runner nothing else: no
  project secret and no signing key (PRO-030).
- **PRO-041. Anima names each label.** The labels are `self-hosted`, `anima`, the operating
  system and the architecture. The hub sends each of them in `labels`, because F12 does not
  state the default.
- **PRO-042. Two modes.** Warm: Anima keeps one idle runner for each repository that is set on.
  GitHub gives it the next job. This mode needs no webhook. On demand: the hub registers a
  runner when `workflow_job` says that a job waits (F10). This mode needs the webhook receiver
  (WHK-001 to WHK-010) and the event. A slow poll repairs a lost delivery (F11, WHK-006).
  Decided (OD-W17): warm mode comes first. On-demand mode follows the receiver.
- **PRO-043. The runner program.** The machine gets the version that the hub names, from the
  releases of `actions/runner`, and checks its SHA-256. The program must not run as root (F8).
  Each job starts from a clean copy of the program. The hub must keep the version inside the
  shorter limit of F7, 30 days, and must read the dates of F7 from GitHub. Anima does not put
  the program in its own release.
- **PRO-044. The set-up says what is true.** Before the press, the client must show: public or
  private, the fork setting that GitHub reports (F24), and the machines that will run the jobs.
  For a public repository it must show the recommendation of GitHub (F22) and ask for one more
  confirmation. Decided (OD-W18). The operator also decided two conditions for
  `TheArctesian/anima.ing`: no runner before tasks W2 and S are accepted, and its fork rule on
  GitHub set to "all external contributors" first. The operator sets that rule.
- **PRO-045. The change of `runs-on`.** Anima can open a pull request that changes `runs-on`.
  It changes only a line of the plain form `runs-on: <one label>`. It must not parse YAML
  (ruling on PRJ-035, 2026-11-14). For each other form it shows the line that the operator must
  write. The pull request is a named GitHub function (PRJ-027, PRJ-028).
- **PRO-046. Off is one action.** When the operator sets runners off for a project, Anima must
  remove its idle runners and must register no new one. A workflow that still names the labels
  then waits 24 hours and GitHub cancels it (F6). The client must say this and must offer the
  change back.
- **PRO-047. GitHub is the scheduler in model (b).** GitHub decides what runs and when. GitHub
  holds the log and the secrets of the workflow. Anima decides the machine and the isolation.
  Anima shows the state of the run in the Work tab (PRJ-031) and makes no second copy of the
  log.
- **PRO-048. Organisations come later.** A runner for an organisation needs a permission that
  the App does not have (F2, F4). Add it only when the operator uses an organisation, because
  each installation must then approve it (F16).
- **PRO-049. The forge boundary.** The calls of F1, F11 and F24 are code of the GitHub provider
  (PRO-007). Each change is a named function with its audit entry (PRJ-027, PRJ-028). A
  different forge adds its own runner functions and changes nothing else.

### The set-up for a coding agent, and the text to copy (R-170)

The operator's words (2026-11-18, R-170): "building this into the project pannel such that it will be easy for an agent to use the MCP and CLI to set up github actions for the project. Also on the UI front building all of this out we can then create a copy and paste thing where other people can just copy a prompt on the UI to set up deployments with anima as the runner a bit like my nix optimisation skill". R-170 gives the direction of PRO-054 to PRO-058. No ruling approves their text one by one (PLN-007). Task B9 builds them, after B4 and B5.

- **PRO-054. The set-up is in the project panel.** Proposed. The page of a project has one section for runners. It shows the steps of "One press" above, in their order, with the state of each step and the name of who does it: Anima, the person in Anima, the person on GitHub, or a merge in the repository. It shows the facts of PRO-044 before the press. The person does each step of Anima there. A second place for this set-up must not exist.
- **PRO-055. Each step is a function of the hub.** Proposed. Each step that Anima does has its hub API, its `anima` command and its MCP tool, in that order (CMD-030, PRO-053), so that a coding agent can do the set-up for a project with no screen. The steps: read the state of the set-up, set runners on and off, open the pull request of PRO-045, read the fork settings (F24), list the runners, and read the runs of a workflow (PRJ-031). A coding agent writes a workflow file in its own checkout of the repository; Anima needs no function for that.
- **PRO-056. One read gives the whole set-up.** Proposed. One query, `anima runner setup <project>` and one read tool give: each step, its state, the next step, and who must do it. When the next step belongs to the person, the answer says so in a sentence and gives no command for it. An agent thus needs no knowledge of the order.
- **PRO-057. What only the person does.** Proposed. No MCP tool does these steps, and the command line asks for a session of the operator (L) and a confirmation: the confirmation for a public repository (PRO-044), the merge of the pull request that changes `runs-on` (MCP-005: a merge is a trust decision), the fork rule on GitHub, the setting of a Mac for native jobs and its build user (PRO-024), and each change of a secret or a variable of the repository on GitHub. A tool that an agent calls for such a step answers with the sentence that the person must do it, and where.
- **PRO-058. The text to copy.** Proposed. The section of PRO-054 shows a text and a "Copy" control. A person puts the text into a coding agent that has the checkout of the repository and the MCP server `anima mcp`, or the `anima` command. `anima runner prompt <project>` prints the same text. The text has one source in the repository of Anima, with a version; the client and the command do not hold two copies.
  - The text must hold: the name of the project and of its repository; the goal in one sentence; the rule that the agent reads the rules of the repository first and changes nothing before it has a list of the workflows and their `runs-on` lines; the tools and commands of PRO-055 and PRO-056 by name; the labels of PRO-041, or the variable of fact F13 when task B1 has proved it; the steps that the agent must leave to the person (PRO-057) and the sentence that it then says; the conditions on which it must stop and tell the person (the repository is public, a workflow uses `pull_request_target`, a workflow needs Docker and task W2 has not said how, the hub refuses a step); and the check at the end: one run of a workflow that shows a machine of the person as its runner.
  - The text must not hold: a secret, a token, a key, a JIT configuration, an API key, a password, a one-time link, or an address that carries one (MCP-004, PRO-040); an instruction to read or to print a secret file; a command line for a device (MCP-006); an instruction to change the MCP permissions or to approve for the person (MCP-013); an instruction to merge, to push to the default branch or to change the fork rule; text that comes from GitHub other than the two names, which are cleaned and cut as MCP-008 says; and the address of a hub of a different person.
  - The text is the same for each person, with the two names as the only values. A test must fail when the text holds a value that matches the secret patterns of the redaction crate, and when the text names a tool or a command that does not exist.
  - The form follows the skills that the operator writes for coding agents: read the rules of the project, make a list with no change, propose, change only what the person approved, check. If the text is later also published as a skill file is an open point.

### What model (b) needs from the webhook receiver

The receiver is planned and not built (WHK-001 to WHK-010, OD-E15). The webhook of the App is
off, and the App has no event (F4, PRJ-019).

| Function | Without the receiver | With the receiver |
| --- | --- | --- |
| Model (b), warm mode | Works. The runner program asks GitHub for work. | The same. |
| Model (b), on demand | Only with the poll of F11, which is slow and uses the request budget. | Works, with `workflow_job`. |
| Model (a), a build from a push | Only with the slow poll of PRO-021. | Works, with `push`. |
| The state of a run in the client | The hub reads GitHub when the page is open (PRJ-032). | The same, and sooner. |

To open the port on the VPS needs the confirmation of the operator (VPS-005).

## Plan C: the parts that A and B share

| Part | Model (a): a build of Anima | Model (b): a runner job | The shared unit |
| --- | --- | --- | --- |
| Signal | `push`, or the operator | `workflow_job`, or a warm runner | The webhook receiver and the slow poll (WHK-006, PRO-021) |
| The item in the hub | A build (PRO-031) | A runner for one job (PRO-039) | One queue with a lease (PRO-050) |
| The machine | Placement by the hub | Placement by the hub | One pure function (USE-003, PRO-051) |
| The program on the machine | `anima-daemon`, work type `build` | `anima-daemon`, work type `runner` | The work route (USE-005, USE-006) |
| Isolation | Container, or the build user on a Mac | The same | PRO-029, PRO-022 to PRO-024 |
| The code | The machine makes the checkout (PRO-034) | The workflow makes the checkout | Not shared |
| A secret for one job | A project secret, a repository token (JOB-010) | The JIT configuration (PRO-040) | One delivery and one deletion (JOB-010) |
| The log | The machine sends it to the hub (BLD-048, BLD-049) | GitHub holds it (PRO-047) | The view, not the store |
| The result | A commit status under `anima/` (OD-E14) | The check of GitHub | Not shared |
| Stop | The hub tells the machine | Cancel the run on GitHub (PRJ-034); the machine stops the runner | The stop control of JOB-007 |
| "What runs now" | A row for each build | A row for each runner job | One list from the hub (PRO-026, PRO-052) |
| Audit | Each item of work (LOG-005) | The same | The audit log, with the actor kind (LOG-012) |

- **PRO-050. One queue for work.** Proposed. The hub keeps builds and runner jobs in one queue.
  The rules of the build queue stay: an atomic claim, a lease, a report, three attempts (BLD-003
  to BLD-010). The queue comes from `fleet/builds/` and is not written a second time.
- **PRO-051. One placement function.** Proposed. Its inputs are the work type, the operating
  system, the architecture, container or native, and the policy of each machine (USE-004,
  USE-009, PRO-024). It takes the time as a parameter (CQ-021).
- **PRO-052. One list of what runs.** Proposed. The area of PRO-026 reads one list from the hub:
  each item of work that runs and each open session. The slot `nav/running/` exists in the
  client and has no data path for work.
- **PRO-053. The order of CMD-030.** Proposed. Each function gets its hub API, its `anima`
  command, its MCP tool and then its screen. An MCP tool must not return a JIT configuration, a
  repository token or a key (MCP-004). A tool that makes a change is off until the operator sets
  it on (MCP-002).

| Function | `anima` command | MCP tool |
| --- | --- | --- |
| The builds of a project | `build list`, `build log` | Read tools |
| Start or stop a build | `build start` L, `build stop` L | Change tools, off by default |
| The runners of a project | `runner list` | Read tool |
| Set runners on or off | `runner on` L, `runner off` L | Change tools, off by default |
| The pull request of PRO-045 | `runner pull-request` L | Change tool, off by default |
| The state of the set-up (PRO-056) | `runner setup` | Read tool |
| The text to copy (PRO-058) | `runner prompt` | None: the text is for a person to copy; an agent that has the tools needs the read tool of PRO-056 |
| The release of Anima | `release …` and `builder …` stay, until state 3 of Plan A | As today |

`L` means that the command asks for a session of the operator, as in Section 6 of the CLI and MCP
plan. The names are proposals: that plan owns them. "Off by default" in this table is the text of 2026-11-16; R-120 made each tool `allow` by default (MCP-002, MCP-024). Task B4 must say which default the runner tools get, and PRO-057 names the steps that no tool does.

## The automatic release of Anima (version 2.3, 2026-11-18)

This section is a plan of the path. Version 2.3 approved nothing. On 2026-11-18 the operator selected the path: R-170 (OD-W27: the runner path, model (b)) and R-171 (OD-W28: a workflow on a runner of the operator can build a release). The comparison and the recommendation below are the text of version 2.3 and stay as the record of what he selected from. "The chain" is the order as it stands after the two rulings. Open: OD-W26, OD-W29, OD-W30, OD-W31 and OD-W32 in [decisions.md](../plan/decisions.md). The task rows are in "Tasks" (R11 to R13, B8, B9).

### The goal, in the words of the operator

The operator, 2026-11-18, in the orchestrating session (relayed, not first-hand in the session that records it): "but we should think through this more, I do want releases to be automatic I just want to be able to run it via anima for this project. Which requires me installing a live version of the app that automaticly updates for the UI or the CLI my hubs being linked and then me being able to configure github actions to build the apps. Then I am ready to build is the possible today?" The orchestrating session answered that it is not possible today, and proposed: publish 0.2.2 by hand through the Builds view, and plan the automatic path apart. His answer: "Yes, both". This section is the second half. It does not change the steps of the release of 0.2.2.

The goal of this section: the first release of Anima that starts with no act of the operator in Anima, for the project of the Anima repository, run through Anima.

He named three parts.

| Part | What exists today | What is absent | Owner of the work |
| --- | --- | --- | --- |
| (a) An installed application that updates itself: the graphical client | The desktop application checks the feed at start and each 4 hours, and installs an update with no question (UPD-001 to UPD-005). macOS 0.2.1 is published. | No person saw an installed application update (P-17 and REL-1 of [need-human-testing.md](../need-human-testing.md) are Open). No Linux release exists. | Task R7 |
| (a) The same for the command line | The `anima` program inside the application updates with the application. | `anima` alone, by Homebrew and by Cargo (R-151), is not built: tasks R8a to R8d. OD-24 and OD-26 of the design specification are not ruled. With Homebrew and Cargo the person runs the upgrade command: that update is not automatic, and no document plans one that is. | Tasks R8a to R8d |
| (b) His hubs linked | The hub on the VPS can publish. A hosted hub cannot: it holds no key, and `createRelease` is refused there (R-118, PRO-037). | The release server of R-158, which is the one hub that can publish after the move (task U12 of [daemon-and-hub-update-plan.md](daemon-and-hub-update-plan.md)). How a computer of the operator builds for the release server while its application is signed in to the hosted hub is not verified (task R11, decision OD-W30). | Tasks U12 and R11 |
| (c) Builds on his machines, started from GitHub Actions or from a push | The desktop builder: while the application runs with "Build releases on this computer" on, it claims each job of its platform, runs the gates, makes the bundle, and on a Mac signs and notarizes it (BLD-020 to BLD-029, BLD-040 to BLD-047, BLD-083 to BLD-085). The hub checks, signs and publishes (BLD-045, BLD-060 to BLD-066). | The start. A release starts only when the operator presses "Release" or runs `anima release create` (BLD-001, UPD-007, OD-12). Nothing reads a push. No machine is a runner. | This section |

Thus one step of a desktop release is not automatic today: the start. Each step after the start runs with no act of the operator, when a builder of each target is in operation.

### Two build models, and a short form of the first one

PRO-010 gives two models. The operator named GitHub Actions, which is model (b). The table gives each model in full. The column "(a) short" is model (a) with the builder that exists: the hub sees a new version on `main` and queues the release, and the desktop builders do the rest as today. It is the "later version" that OD-12 of the design specification already names.

| | (a) short: the hub starts the release, the desktop builders build | (a) full: a build from a push, as work of the daemon | (b): a workflow of GitHub Actions on an owned machine |
| --- | --- | --- | --- |
| What starts a release | The hub reads the repository at a slow interval (PRO-021) and finds a new version mark on `main` (OD-W26) | The same signal; later the webhook receiver gives it sooner (WHK-001 to WHK-010) | A workflow file in the repository, on the same mark (`on: push`). GitHub is the scheduler (PRO-047) |
| Who builds | The desktop application of each builder computer, as the operator, with his keychain | `anima-daemon`, in a container on Linux and as the build user on a Mac (PRO-022, PRO-029, PRO-033) | A runner for one job, which `anima-daemon` starts in the same isolation (PRO-039) |
| Who checks, signs and publishes | The hub that holds the key, as today (R-117, R-158) | The same | The same. The workflow must give the bundle to that hub: a new function (task B8). The updater key does not go to GitHub (R-114, R-117) |
| Tasks, in order | U12; R11; R12; R13. With R7 for Linux | W1; W2; W3; W4; A2; A5; S. Then A4 removes the old builder | W1; W2; W3; W4; B1; B2; B3; B4; B5; S; B8 |
| Count of tasks that are not started | 4 | 7 | 11 |
| What it needs from the operator | The release server and the move of the key (U12). The decisions OD-W26, OD-W27 and OD-W30. A Mac and a Linux computer whose application stays open. No new account and no new secret | The acceptance of the contract (W1) and of the isolation design (W2). A build user on the Mac, made one time by an administrator (PRO-024). A signing identity that a job can use for one job: not designed (W2). The acceptance of the security review (S) | Each item of (a) full. The GitHub App that exists (it has the permission, fact F2). One merge of the workflow file and of `runs-on` (PRO-045). The fork rule of the repository, set by him on GitHub (OD-W18). The decisions OD-W28 and OD-W29. Task B1 on his account |
| Security position for the public repository | The builder refuses a commit that is not an ancestor of `origin/main` (BLD-041). The code of a fork cannot reach a builder. The build runs as the operator with his keychain; that is the state of today, and PRO-033 names it as the reason to replace the builder later. New risk: each mark on `main` becomes a signed update of each installed application with no person between. The protection of `main` and of the mark is then the only gate | Each job is isolated (PRO-029). A job of a fork gets no secret (PRO-030). The release recipe builds only an ancestor of `main` | GitHub says that a public repository must almost never use a self-hosted runner (F22). The author of a fork pull request selects the runner and the steps (F20). The approval rule of GitHub and the isolation on the machine are the two controls (PRO-029, PRO-044). OD-W18: no runner for `TheArctesian/anima.ing` before W2 and S are accepted, and the fork rule "all external contributors" first. A signed release from a runner needs a rule that the release workflow runs only for the mark on `main` and never for a pull request: not designed (B8) |
| macOS | Fits now: the builder signs, notarizes and staples (BLD-083 to BLD-085) | Fits after W2 gives the build user a signing identity for one job | The same as (a) full |
| Linux | Fits when task R7 has published the first Linux release | Fits: a container job | Fits: a container job |
| Android APK (task R10) | Needs one more builder step that calls the script of M5-5, and the release key in the key store of that computer (RLS-010, RLS-011) | The script of M5-5 as a recipe step | The script of M5-5 as a workflow step |
| iOS | Not in the automatic release. R-158: the script on his Mac (M5-4), and the upload to TestFlight is his act | The same | The same |
| What it does not give | No isolation. No build of a different project. It keeps the Builds view and the desktop builder longer | It does not run the workflows of a repository | It needs GitHub for each release, and it is not the path for a different forge (PRO-007) |

Recommendation (of the session that wrote this section; not a decision): build the short form first, because it reaches the first automatic desktop release with 4 tasks and changes no security position of a machine. Keep the order of OD-W21 for the rest: W, then B, then A. Model (b) stays the path for "configure GitHub Actions to build the apps", and it comes after W2 and S by the operator's own condition (OD-W18). When model (b) or the full model (a) can make the release, the start rule of R12 stays and only the builder changes. If the operator wants the release itself to be a workflow of GitHub Actions from the start, the short form is not built and the chain is the column (b): that is choice B of OD-W27.

Result (R-170, operator, 2026-11-18): choice B. The recommendation above is not taken. The short form is not built, and the chain is the column (b).

### The text that an automatic release changes

Each row is a decision of the operator. R-171 changed the rows that name it as "Changed"; each other text is not changed.

| Text now | Where | The change | Decision |
| --- | --- | --- | --- |
| "Phase 2: the operator starts each release (Section 3.10). A later version can let the hub poll `main` and queue a release for each new commit." | OD-12, design specification Section 14 | The second sentence becomes the rule, with the mark of OD-W26 in place of "each new commit" | OD-W26 |
| "The operator must start a release from the Builds view with "Release <version>"." | BLD-001 | Add: the hub also starts a release when it finds the version mark on `main`, if the operator set that on for the hub | OD-W26 |
| "A release must start only from the Builds view (BLD-001). There is no GitHub release workflow (changed by R-107; `release.yml` removed by R-114)." | UPD-007 | The first sentence gets the same addition. The second sentence changes only with model (b) | First sentence: OD-W26, open. Second sentence: changed (R-171) |
| "(2) The operator starts each release from the app; the hub does not poll GitHub (OD-12)." | R-107, item 2 | A new ruling replaces this item | OD-W26 |
| "A release still starts by hand (BLD-001, OD-12 of the design specification)." | PRO-036 of this plan | The same addition | OD-W26 |
| "the GitHub release workflow is removed now" and "UPD-007 says there is no GitHub workflow" | R-114 | Model (b) adds a workflow file again. It is not the old `release.yml`: it holds no updater key, and it gives the bundle to the hub. The reason of R-114 was the secret `TAURI_SIGNING_PRIVATE_KEY`; that secret does not come back | Changed (R-171) |
| "GitHub Actions does not build or publish a release (R-005, R-107)." | Design specification Section 3.10, first paragraph | With model (b): "A workflow of GitHub Actions can build a release on a machine of the operator. Only the hub signs and publishes." | Changed (R-171), in the design specification |
| "It needs no open port and no GitHub runner." | BLD-029 | With model (b), the last four words go | Changed (R-171), as a note: the sentence stays true for the desktop builder, and a runner for one job is a second kind of builder |
| "GitHub Actions runs only deployment, never gates or tests." | R-005, CQ-010 | With model (b) the builder step of the workflow runs `bun run gates` on the owned machine (BLD-042). The operator decides if R-005 permits that | Not changed by R-171. OD-W32, open |
| "no runner before tasks W2 and S are accepted, and its fork rule on GitHub set to "all external contributors" first" | PRO-044 (OD-W18) | No change is proposed. The other choices are in OD-W29 | OD-W29 |
| "Publishing is not set up on this hub." for each hub with no key | R-118 | No change is proposed: a hosted hub does not publish | OD-W30 |
| "It connects out to the hub over the VPN." | BLD-029 | The release server is not on the VPN of the VPS. Task U12 writes how a builder connects to it | Task U12, no new decision |

### The chain

The order as it stands after R-170 and R-171, from today to the first release of Anima that a workflow builds on a machine of the operator. A task of a different plan is only named here; its row is in its owner (PLN-016). The column "State" of "Tasks" holds the state of each task; this table does not copy it. "Accepts" names who must say yes before the task is Done or before the next task can start. "Agent now" says if an agent can take the task today, with no new answer of the operator.

| Order | Task | Content, in short | Needs | Check | Accepts | Agent now |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | R11 | Can one computer build for a hub that is not the hub of its sign-in | — | The result names the files and the test | Nobody. It is a record; it informs OD-W30 | Yes |
| 2 | W1 | The contract of the work route, and its security review before code (R-160) | — | Each finding of the review has a state in [security-findings.md](../security-findings.md) | The operator, with a ruling (CON-001): a contract is not a contract before that | Yes: the text and the review. No code |
| 3 | W2 | The isolation design, with prototypes | — | A record with measured results on one Linux machine and one Mac | The security reviewer and the operator | In part: the design and the prototype on a Mac. The Linux part needs a Linux computer that the operator names |
| 4 | B1 | The facts of GitHub that the documentation does not state | The account of the operator and a private test repository | Each result is in "Facts of GitHub" | The operator does it; an agent writes the results that he sends | No. An agent wrote the steps; the account is his |
| 5 | W3 | The hub: one queue, the placement function, the log store | W1 accepted | The gates. The release through the Builds view still works | The merge of the orchestrating session | No, until W1 is accepted |
| 6 | W4 | The daemon: a container job and a native job on a Mac, with deletion at the end | W1 and W2 accepted | Tests: a job cannot read the daemon token or the home folder; nothing stays after a stop | The merge. Task S reviews it later | No |
| 7 | W5 | "Takes work" for each machine, "native jobs" for each Mac | W3 | A machine that is set off at the machine refuses work | The merge; a human check for the setting on a real Mac | No |
| 8 | B2 | The GitHub provider: JIT configuration, remove and list runners, queued jobs, fork settings | A ruling for the new `Change` values of K-E1 Section 13 (CON-001) | A test with a recorded answer for each function; the JIT configuration is in no log | The operator, for the contract change. Then the merge | No, until the contract change has its ruling |
| 9 | B3 | The daemon, work type `runner` | W4, B1 | A workflow of the test repository runs on a Linux machine and on a Mac; nothing stays | The operator, with a human check on his machines | No |
| 10 | B4 | The hub: runners on and off, warm mode, the audit entries; `anima runner`; MCP tools | W3, B2 | Two jobs in sequence each get a new runner; with runners off GitHub lists none | The operator, with a human check against GitHub | No |
| 11 | B5 | The set-up in the client, and the pull request that changes `runs-on` | B4 | One press and one merge make a workflow run on an owned machine, on a private test repository | The operator, with a human check | No |
| 12 | B9 | The set-up for a coding agent and the text to copy (PRO-054 to PRO-058) | B4, B5; the approval of PRO-054 to PRO-058 | A coding agent with the text and the MCP server brings a private test repository to the merge of the person; the text holds no secret (a test) | The operator: the approval of the requirements, then a human check | No |
| 13 | S | The security review of builds: the work route, a native job, a fork job on a public repository | W4, B3 (and A2, when it exists) | A test for each claim | The security reviewer. Only the operator sets a finding to Accepted | No |
| 14 | U12 | The release server signs and publishes (R-158); owner: [daemon-and-hub-update-plan.md](daemon-and-hub-update-plan.md) | The operator: the server, the key, the token | Its row there | The operator | In part: an agent writes the procedure |
| 15 | B8 | The release of Anima as a workflow on a runner of the operator | OD-W26, OD-W29, OD-W31, OD-W32 and OD-W30; B5; S; W2 and S accepted (OD-W18); U12, or the VPS hub while it signs | A release that the operator starts makes one workflow run on his machine, and the hub publishes its files. A pull request from a fork cannot start the workflow and cannot reach a signing identity: a test for each claim | The operator | No |
| 16 | R13 | The first release that starts with no act of the operator in Anima. A human check | B8; the ruling of OD-W26; a runner machine of each target in operation | Its row | The operator | No |

Not on this path: W6 (the list of what runs), B6 (on-demand mode), B7 (organisations), group A (the builds of Anima's own), R12 in the form of the short form, and the tasks R7 to R10 and R8a to R8d, which stay in their own order. R7 stays the task of the first Linux release.

Until OD-W26 is decided, a release by this path still starts with an act of the operator (BLD-001, OD-12): he starts the release in Anima, the hub starts the release workflow (PRO-020), a runner on his machine builds, and the hub checks, signs and publishes. That is rows 1 to 15. Row 16 needs the answer to OD-W26.

The security review of the work route has two places, and both stay: before code, with the contract of W1 (R-160; [jobs.md](../proposals/jobs.md), "Needs"); and at the end, task S, for a native job and for a fork job on a public repository.

### How a runner job gets a signed release (OD-W31, open)

R-117 and R-158 stay: only the hub that holds the updater key signs, and no builder holds the key. R-171 lets a runner job build. This section proposes how the files of that job reach the hub. It decides nothing; OD-W31 in [decisions.md](../plan/decisions.md) holds the question.

Facts that limit the design:

- In model (b) Anima gives a runner only its registration (PRO-030, PRO-040). GitHub gives the job its secrets, by the rules of GitHub (PRO-047).
- A warm runner takes the next job whose `runs-on` has its labels (PRO-042). The hub does not select the job. A job of a pull request from a fork can be that job (F20).
- The desktop builder proves what it is with a builder token that the operator enrolled (BLD-021, BLD-022), and the hub gives it upload addresses for the files of its job only (BLD-060, BLD-061).

| Choice | How it works | For | Against |
| --- | --- | --- | --- |
| A. The job hands its files to the hub and proves what it is with the identity token of GitHub | The release workflow asks GitHub for the identity token of its job (OpenID Connect). A step sends that token to the hub with the request for upload addresses. The hub checks the signature of GitHub and the fields: the repository is the repository of the Anima project, the workflow file is the release workflow on the default branch, the event is not a pull request, and the commit is the commit of the release and an ancestor of `origin/main` (BLD-041). Then the path is the path of today: upload addresses for this job only, the checks of BLD-064 and BLD-065, the signature of the hub, the feed | No stored secret in the repository, in the job or on the machine. R-117 does not change. The checks of the hub stay the last gate. A job of a fork gets a token whose fields say what it is, and the hub refuses it | The hub must read the public keys of GitHub. The fields of the token on a self-hosted runner must be proved (task B1 gets this fact). A new hub function and a new contract text |
| B. The daemon hands the files over | The job writes its files to one output folder. After the job ends, `anima-daemon` uploads them as the holder of the work item. The hub asks GitHub which job that runner did (F10, F11) and accepts the files only for the release workflow on the mark | The job gets no credential and makes no request to the hub | The proof is a second read of GitHub by the name of the runner, after the fact. The daemon must take files out of the isolation of a job whose code it does not trust. More new code on the machine |
| C. The key is a secret for one job | The hub gives the updater key to the job (JOB-010), or the key is a secret of the repository, and the job signs | The smallest change to the workflow | It reverses R-117 and the reason of R-114. Code of the repository, its build scripts and its packages run beside the key that each installed application trusts (BLD-044). Not recommended |

Recommendation: A. It keeps one place that signs, it adds no secret, and a job that is not the release workflow cannot get an upload address. With A, the hub that signs must accept a request from the machine that runs the job: the same network question as for a builder of today (task R11, OD-W30, task U12).

The Developer ID identity of the Mac is a different question: the macOS bundle is signed on the Mac (BLD-083), and a runner job as the build user has no keychain of the operator (PRO-022). Task W2 designs a signing identity for one job. Until W2 is accepted, a runner job cannot make the macOS release.

### What stays an act of the operator

- The accounts and the keys: the Apple Developer account, the Developer ID certificate and the notary profile in the keychain of the Mac (R-116); the updater key and its copy (UPD-012); the R2 token. An agent reads none of them.
- The first move of the updater key and of the token to the release server (task U12, R-158). It is done one time.
- A builder of each target must be in operation: a Mac with its keychain open, and a Linux computer. With the desktop builder, the application must be open (BLD-026).
- "Restart to update" in each installed application (UPD-003): an update is installed with no question, and it is in use after the restart that the person selects.
- `brew upgrade` or `cargo install` for `anima` alone (R-151).
- iOS: the upload to TestFlight and each step in App Store Connect (R-158; RLS-010 of [release-four-platforms.md](../proposals/release-four-platforms.md)). The rules of the store must be read at the source before a decision.
- Android: the release key, made one time by the operator, and the choice of a store, which is not decided.
- The version mark itself (OD-W26): a person or an agent puts it on `main`. That act is in the repository and not in Anima.
- A release that fails: the operator reads the log. Task B8 sends the failure to the notification channels.

[release-four-platforms.md](../proposals/release-four-platforms.md), Section 9, says what "automatic" means for each of the four targets.

## Tasks

The column "State" of each table holds the state of each task. Change the state in the commit that does the work (PLN-008). The
order is the order of the groups: R, then W, then B, then A (OD-W21, operator, 2026-11-16). S is
the last task of W, B and A.

Group R: correct the release of Anima. It needs no other group. It is tier 0 of the work plan
(OD-W21).

| Task | Content | Needs | Done when | State | Commits |
| --- | --- | --- | --- | --- | --- |
| P0 | Compare PRO-005, PRO-006 and PRO-007 with the code on `main`. Write below this table, for each requirement, if it is built, with the file and the test that show it. Build nothing. | — | The three results are in this plan, and row 4.6 of the work plan agrees with them. | Done. The three results are in "Results of task P0" below these tables. PRO-005: not built. PRO-006: partly built. PRO-007: built differently. The operator decided the differences Q1 to Q4 on 2026-11-17, each as recommended. | Branch `claude/projects-p0-d18079` |
| R1 | The clone address comes from one configuration value with the new name (D5). Change BLD-040. | — | A test proves that the builder clones the configured address. | Done | `e3058276`, on `main` (branch `builds/release-corrections`) |
| R2 | Delete the bundle folder before the bundle step (D1). | — | A test builds one version two times and uploads the file of the second attempt. | Done | `e3058276`, on `main` (branch `builds/release-corrections`) |
| R3 | Add Go to the tool check (D4, BLD-028). | — | A builder with no Go names Go and does not claim. | Done | `e3058276`, on `main` (branch `builds/release-corrections`) |
| R4 | The hub refuses a release when it cannot publish. The client hides the Builds entry and the builder row then (D8, PRO-037, PRO-038). | — | A hub with no updater key refuses `createRelease` with a typed reason. The command and the MCP tool give the reason. The client shows no Builds entry. | Done | `e3058276`, on `main` (branch `builds/release-corrections`) |
| R5 | One name for the macOS application: `Anima.ing.app` (D2; OD-W19, operator, 2026-11-16). | — | The design specification Section 3.11, `carried.rs` and the builder say `Anima.ing.app`. The file names of a release stay `Anima_*` (UPD-010). | Done. Section 3.11 and `carried.rs` say `Anima.ing.app`; the builder already made `Anima.ing.app` and its tests now use the name. | c1e6226e on branch `claude/watchdog-autoupdate-proposals` |
| R6 | The release carries the local hub (D3). | T9 of the S4 tasks; S3 T11 | As T9 of [s4-local-hub-tasks.md](one-click/s4-local-hub-tasks.md) states. This row only points to it. | Done, as T9 of the S4 tasks: the release 0.2.0 carries the programs and the tree. | `f24f4c54`, `aec3c400` |
| R7 | The first release. | R1 to R5; the operator | Each feed answers 200. An installed application updates on macOS and on Linux (D6, OD-16 to OD-18). The operator does this. | In progress. The release record holds what is published and what is verified: [releases.md](../procedures/releases.md). For macOS 0.2.2 the hub on the VPS is not deployed again (operator, 2026-11-18, "C: check first"; a fact, not a ruling). The orchestrating session reported this check; it is a static check and was not run against the hub: each operation of the release path of `3dc5beae` is in the schema of the deployed hub `408bbbab`, and the Builds code of the hub and the upload code of the builder are the same in the two commits. Outside the release path, an application of `3dc5beae` cannot use these functions with that hub: the daemon panel, Agents > Usage, the hosted sign-in and the text search of the event log; and the `anima-daemon` in the bundle cannot enrol with it. | `43291afc`, `b361eb5f` |
| R8a | The crate `anima-cli` is publishable (INS-011, INS-012; R-151). One PR. Details: [Tasks R8a to R8d](#tasks-r8a-to-r8d-the-command-line-alone-by-homebrew-and-cargo). | — | `cargo package --list -p anima-cli` lists the schema and no test file. A check fails when the crate version is not the release version. Nothing is published. | Not started. Planned only. | — |
| R8b | The release publishes a standalone `anima` file for each target, with its SHA-256 in the release record (INS-010; OD-24). One PR. | R7; R8a; the operator (OD-24) | macOS universal has the file and its SHA-256 in the record. The Linux x86_64 file waits for the Linux release of R7. Linux aarch64 is not planned. | Not started. Planned only. Blocked until OD-24 is ruled. | — |
| R8c | The Homebrew formula and its tap (INS-010; OD-24, OD-26). One PR, in the tap repository and one line in this repository. | R8b; the operator (OD-24, OD-26) | `brew install` of the formula installs `anima` on a clean Mac, and `anima --version` prints the release version. The Linux check waits for R8b. | Not started. Planned only. Blocked until OD-24 and OD-26 are ruled. | — |
| R8d | The operator publishes the first formula and the first crate version (INS-012; OD-24, OD-26). A human task, with an entry in [need-human-testing.md](../need-human-testing.md). | R8a; R8b; R8c | `cargo install anima-cli --locked` and `brew install` print the release version. The operator sets the entry to Passed. | Not started. Planned only. The agent publishes nothing. | — |
| R9 | The bucket keeps the 3 newest published versions of each target (BLD-086). The code, its migration and its tests are on the branch `builds/keep-three-versions`, which is not merged. That branch is older than the move of the documents: it changes `docs/design-spec.md`, `docs/programme/programme.md` and `docs/releases.md`, which are now `docs/spec/design-spec.md`, `docs/rulings.md` and `docs/procedures/releases.md`. Bring it to `main`: merge `origin/main` into it, put BLD-086 in Section 3.10 of the design specification, give the migration a number that is free, and give its ruling the next free number (the branch calls it R-118, which a different ruling has; PLN-021). The branch gives the operator's words of 2026-11-12 for the ruling; they are not on `main`. Source of this row: [release-four-platforms.md](../proposals/release-four-platforms.md), RLS-007. | — | `bun run gates` passes. The pruning tests of the branch pass. After the next release of a target that has 4 versions, the bucket holds 3. | Not started. Built on the branch, not merged. | — |
| R10 | The release has a target for the Android application. The hub checks the size, the SHA-256 and the signing certificate of the APK, writes it under `<target>/<version>/`, copies the stable name and writes the feed of the target (RLS-006, RLS-012 of [release-four-platforms.md](../proposals/release-four-platforms.md); as BLD-060 to BLD-066). It needs new BLD requirements in the design specification Section 3.10 first. Hub API, then `anima`, then MCP (CMD-030). | R-158 names it (OD-REL-4) and approves RLS-006 and RLS-012. The new BLD requirements go in the design specification in the first commit of the task (PLN-013). The check of a real APK needs task M5-5 of [mobile-plan.md](mobile-plan.md); the hub side can be built before it, with a test APK that the test signs | An APK that a different key signed is refused. An APK of the release key is at its stable name, and its feed gives the version. | Not started | — |
| R11 | Find out if one computer can build for a hub that is not the hub of its sign-in: the desktop application as a builder for the release server (R-158) while the application is signed in to the hosted hub, and the same for `anima-daemon` (PRO-008, BLD-021 to BLD-024, BLD-029). Read the code, then try it with two local hubs. Build nothing. Write the result in this plan, below "The automatic release of Anima", and in the row OD-W30 of [decisions.md](../plan/decisions.md). | — | The result names the files and the test that show it. If the answer is no, the result names the smallest change, as a new task row. | Not started | — |
| R12 | The start rule of a release. The hub that can publish reads the Anima repository at a slow interval and, when it finds the version mark that OD-W26 decides on a commit that is an ancestor of `origin/main` (BLD-041) with a version that is more than each published version (BLD-002), it queues the release for each target that has a builder. A setting of the hub sets the rule on; it is off by default. The hub does not start a release two times for one mark (PRO-021). It writes the start to the audit log with the actor kind of the hub (BLD-082, LOG-012), and it sends a failed release to the notification channels. New BLD requirements go in the design specification Section 3.10 in the first commit (PLN-013), with the changes of BLD-001, UPD-007 and OD-12 that the ruling gives. Hub API, then `anima release`, then MCP, then the Settings row (CMD-030). | The rulings of OD-W26 and OD-W27. R11. | A test: a mark on a commit of `main` makes one release, and a second read makes no second one. A mark on a commit that is not on `main` makes none. A version that is not more than the published one makes none. With the setting off, nothing starts. | Not started. Not on the path of R-170: OD-W27 selected the runner path, where a workflow file reads the mark and GitHub is the scheduler (PRO-047). The row stays, because no ruling cancels it and OD-W26 can still select a start rule of the hub. Do not build it. | — |
| R13 | The first release that starts with no act of the operator. A human check, with an entry in [need-human-testing.md](../need-human-testing.md) that the commit of B8 adds. | B8 (R-170; it was R12); the ruling of OD-W26; U12; R7 for Linux; a runner machine of each target in operation | The operator puts the mark on `main` and does nothing in Anima. Each feed gives the new version. An installed application updates. The operator sets the entry to Passed. | Not started | — |

## Tasks R8a to R8d: the command line alone by Homebrew and Cargo

Owner: INS-010 to INS-013 in [design-spec.md](../spec/design-spec.md) Section 3.11, and R-151. The operator ruled "ya homebrew and cargo are good cuz the cli can operate alone" (relayed, to be confirmed first-hand). The standalone file, the checksum, the publishing step and the tap are not ruled (OD-24, OD-26). Each task is one pull request. A coding agent builds R8a, R8b and R8c only after the operator rules the open decisions that the row names. Nothing is published before R8d.

Current state, checked on `cd0bbf58`: `crates/anima-cli/Cargo.toml` has no `description`, `repository`, `readme`, `rust-version` or `include`. It gets `publish = false` and the version `0.1.0` from the workspace (`Cargo.toml`, `[workspace.package]`). About 150 `graphql_client` derives in `crates/anima-cli/src/` name `../../contracts/schema.graphql`, a file outside the package. The `build.rs` sets the printed version only when `ANIMA_VERSION` is set. The releases 0.2.0 and 0.2.1 are for macOS only and carry the program as `anima-agent` ([releases.md](../procedures/releases.md)). The Linux feed is not published.

### R8a: make the crate publishable

- **Why.** `cargo install anima-cli` fails today: the schema is outside the package, and the metadata is missing (INS-011).
- **Needs.** Nothing. It changes no contract: K1 to K7 stay as they are.
- **Change.** (1) Add `description`, `repository`, `readme`, `rust-version`, `keywords` and `include` to `crates/anima-cli/Cargo.toml`, with `publish = true` for this package only. `include` lists `src/`, `build.rs`, `release_version.rs`, the schema copy and the readme, and no test. (2) Put one copy of `contracts/schema.graphql` in the package and point every derive at it. The copy is made by one script, and a gate fails when it differs from K1. K1 stays the only source (PLN-016). A symlink is an alternative only if `cargo package --list` then holds the file content. (3) Add a check that the version of `anima-cli` is the release version. (4) Change the "Install" section of `crates/anima-cli/README.md` to name Cargo and Homebrew. (5) On a clean Linux computer, find out if the build needs libdbus and pkg-config for `keyring`. Write the result in the README.
- **Acceptance (pass or fail).** (a) `cargo package --list -p anima-cli --locked` lists the schema copy, `build.rs` and `release_version.rs`, and lists no file of `tests/`. Pass or fail. (b) `cargo package -p anima-cli --locked` builds the unpacked crate. Pass or fail. (c) The copy of the schema equals `contracts/schema.graphql`, and a changed copy fails the gate. (d) The version check fails when the crate version and the release version differ. (e) `cargo publish --dry-run -p anima-cli --locked` ends without error. (f) The CMD-031 coverage test still passes.
- **Check commands.** `cargo fmt --check`; `cargo clippy -p anima-cli --all-targets -- -D warnings`; `cargo nextest run -p anima-cli`; `cargo package --list -p anima-cli --locked`; `cargo publish --dry-run -p anima-cli --locked`; `bun run gates`.
- **Out of scope.** The real publish. A new command. A change to K1 to K7. The daemon (OD-23). The sign-in of a hosted hub (OD-25).
- **Always / Ask / Never.** Always: keep one source for the schema. Ask: before any change under `contracts/`. Never: run `cargo publish` without `--dry-run`.
- **Done.** One commit on `main` that sets the state of R8a with the commit (PLN-008, PLN-009).
- **Claude Code prompt.** Follow AGENTS.md "Start of a session". Claim and build task R8a of docs/plans/projects-and-builds-plan.md: make the crate `anima-cli` publishable. Read INS-011, INS-012, R-151 and the section "Tasks R8a to R8d". Change only what R8a lists. Run the check commands of R8a, then `bun run gates`. Publish nothing.

### R8b: a standalone `anima` file for each target

- **Why.** The formula needs a file that holds only `anima`. Today `anima` is inside the disk image and the AppImage (INS-010).
- **Needs.** R7 for the Linux file; R8a; the ruling of OD-24.
- **Change.** The builder builds `anima` for each target with `ANIMA_VERSION` set, and uploads it as a file with a stable name. The hub checks the file and writes its SHA-256 in the release record (ROL-003). The desktop bundle does not change (INS-002).
- **Acceptance (pass or fail).** (a) A macOS release holds one universal `anima` file, and the record holds its SHA-256. (b) `anima --version` of that file prints the release version. (c) The SHA-256 of the downloaded file equals the record. (d) The Linux x86_64 file has the same checks. It is accepted only after R7 publishes the Linux feed. (e) No Linux aarch64 file is made: that target is out of scope. (f) The disk image and the AppImage are unchanged.
- **Check commands.** `cargo nextest run -p anima-hub`; `cargo nextest run --manifest-path apps/desktop/src-tauri/Cargo.toml builder`; `shasum -a 256 anima` against the record; `./anima --version`; `bun run gates`.
- **Out of scope.** The formula. The crate. A Windows file. The daemon and the hub programs.
- **Always / Ask / Never.** Always: build from the commit of the release. Ask: before any change to the signing of the updater file. Never: sign or upload from an agent session (UPD-012).
- **Done.** One commit on `main` with the state of R8b. The Linux part stays "Not verified" until R7 publishes Linux.
- **Claude Code prompt.** Follow AGENTS.md "Start of a session". Claim and build task R8b of docs/plans/projects-and-builds-plan.md: the standalone `anima` file for each target with its SHA-256. Read INS-010, ROL-003, R-151 and OD-24. Change only what R8b lists. Run the check commands of R8b, then `bun run gates`.

### R8c: the formula and the tap

- **Why.** `brew install` is the first channel of the ruling (INS-010).
- **Needs.** R8b; the rulings of OD-24 and OD-26.
- **Change.** The tap repository `homebrew-anima` holds one formula `anima`. The formula installs the standalone file of R8b for the platform and checks its SHA-256. Its test runs `anima --version`. This repository gets one line that names the tap in [releases.md](../procedures/releases.md).
- **Acceptance (pass or fail).** (a) `brew audit --strict --online anima` passes in the tap. (b) The formula has a `url` of the standalone file and a `sha256` line equal to the record, and no `depends_on "rust"`. (c) `brew test anima` prints the release version. (d) The formula installs no desktop application and no daemon. (e) The Linux check waits for R8b.
- **Check commands.** `brew audit --strict --online anima`; `brew install thearctesian/anima/anima`; `brew test anima`; `anima --version`; `bun run quality:documents`.
- **Out of scope.** A request to Homebrew's own repository. A cask. Publishing the tap (R8d).
- **Always / Ask / Never.** Always: keep the SHA-256 in the formula equal to the record. Ask: before any change to the name of the formula. Never: push the tap; the operator does it (R8d).
- **Done.** One commit in the tap, and one on `main` with the state of R8c.
- **Claude Code prompt.** Follow AGENTS.md "Start of a session". Claim and build task R8c of docs/plans/projects-and-builds-plan.md: the Homebrew formula and the tap, locally. Read INS-010, OD-24, OD-26 and R-151. Push nothing.

### R8d: the operator publishes the first version

- **Why.** A published crate version cannot be deleted, so the operator does the first publish (INS-011).
- **Needs.** R8a, R8b, R8c; the release record of the signed release.
- **Change.** None in code. The operator runs the steps of the entry in [need-human-testing.md](../need-human-testing.md). The commit that builds R8a adds that entry, with the next free number `HT-nnn` (PLN-014).
- **Acceptance (pass or fail).** (a) `cargo install anima-cli --locked` installs `anima`, and `anima --version` prints the release version. (b) `brew install thearctesian/anima/anima` installs the same version. (c) `anima --version` of both equals the version of the disk image (INS-012). (d) The operator sets the entry to Passed.
- **Check commands.** `cargo publish -p anima-cli --locked` (the operator); `cargo install anima-cli --locked`; `brew install thearctesian/anima/anima`; `anima --version`.
- **Out of scope.** Any agent action that publishes.
- **Always / Ask / Never.** Always: publish after the signed release exists. Ask: the operator, for each step. Never: an agent runs `cargo publish` or pushes the tap.
- **Done.** The entry is Passed, and the state of R8d names its date.

## Results of task P0 (2026-11-17)

The comparison is with `main` at `0e7dff86`. It changed no code and no requirement. "Built" here
means that the code and its test are on `main`; it does not mean that a person checked it (PLN-009).

### PRO-005: not built

- The hub knows one provider: GitHub. Its registry of sources holds one declaration
  (`crates/hub/src/lib.rs:273`). `anima source connect` has one subcommand, `github`
  (`crates/anima-cli/src/sources/connect.rs:30`).
- A confirmation changes only the state of the resource and writes the audit entry
  (`crates/hub/src/work/projects/graphql.rs:247`, `crates/hub/src/work/projects/store/choices.rs:68`).
  It starts no sign-in and proposes none. The command (`crates/anima-cli/src/projects/confirm.graphql`),
  the MCP tool `anima_project_resource_confirm` (`crates/anima-cli/src/mcp/tools/projects.rs:137`) and the
  client (`apps/client/src/lib/projects/ResourceRow.svelte:27`: a Confirm button and nothing after it) do
  the same.
- Thus Anima never asks for a sign-in to Cloudflare, Vercel or a different hosting provider. The
  "only" part of the requirement is true, because no such sign-in exists. The part "ask when a
  confirmed project uses the provider" is not built. No test names PRO-005.
- One sign-in is asked before a project exists: GitHub (`apps/client/src/lib/projects/ProjectsScreen.svelte:52`).
  The import needs it (PRO-002). See Q1.

### PRO-006: partly built

- Part 1, a project on owned and on rented compute: partly built. One project can hold one resource
  of each kind at the same time (`crates/hub/migrations/2200_projects_schema.sql:75`), so it can show
  `SERVER` and `CLOUDFLARE` together. But a resource does not record if the compute is owned or
  rented, and a `SERVER` resource has no link to a device of the operator
  (`crates/hub/migrations/2200_projects_schema.sql:66` to 78). See Q2.
- Part 2, a service that holds data about the operator is never a resource: built as written. Only
  four hosting kinds exist: in the hub (`crates/hub/src/work/projects/model.rs:16`), in the table
  (`crates/hub/migrations/2200_projects_schema.sql:69`), in the command
  (`crates/anima-cli/src/projects/resource.rs:29`) and in the client
  (`apps/client/src/lib/projects/words.ts:41`). Each sign proposes hosting
  (`crates/hub/src/work/projects/proposals.rs:11`). Test: `pro_006_only_hosting_kinds_exist`
  (`crates/hub/src/work/projects/tests/proposals.rs:39`).
- Part 3, a job on an owned machine reads the local copy and does not call the service: not built.
  No job function is built (groups W and A are not started), and no task names this part. See Q3.

### PRO-007: built differently

- Built as written: the code that talks to GitHub and reads its answers is in the provider crate
  `crates/providers/github`, and GitHub is one declaration in the source registry of the engine
  (`crates/hub/src/lib.rs:273`). The engine holds no GitHub name. The names of the contract are
  neutral (`contracts/schema.graphql:1976`).
- Different: the Projects domain of the hub is specific to GitHub. Thirty files (26 of them not tests) of
  `crates/hub/src/work/projects/` import `provider_github`: the model
  (`model.rs:10`), the intake (`intake.rs:8`), and each live read and change of pull requests,
  issues, workflows, history and code (for example `pull_requests/graphql.rs:5`, `issues/graphql.rs:5`,
  `workflows/graphql.rs:5`, `history/graphql.rs:5`, `code/graphql.rs:8`). The store has one fixed forge
  (`store.rs:19`), the table allows only `github` (`crates/hub/migrations/2200_projects_schema.sql:33`),
  and the address of a repository is made from `https://github.com/` (`graphql/views.rs:23`).
- The contract K-E1 permits a part of this: an extension holds the intake of its source (K-E1 Section 3),
  and Section 13 defines the live reads as reads of GitHub.
- The client shows the GitHub mark for each repository (`apps/client/src/lib/projects/RepositoryRow.svelte:18`,
  `detail/Overview.svelte:23`). The command and the MCP tool have GitHub only in their help text.
- Thus a second forge needs a new source and also a change to the Projects domain, to a migration
  and to the client. That is not "changes nothing else". No test names PRO-007. See Q4.

### Decisions of task P0

| Decision | Question | Recommendation | Result |
| --- | --- | --- | --- |
| Q1 | PRO-005 is not built, because GitHub is the only provider with a sign-in. Does it get a task now? And does the sign-in to the forge, which Anima asks before a project exists, agree with PRO-005? | No task now. Keep PRO-005 as it is. The first plan that adds a hosting provider as a source (Cloudflare or Vercel) gets the task: the project page proposes the sign-in when a confirmed resource of that kind has no account. Record that PRO-005 is about hosting providers and that the forge is the exception, because the import needs it. | Decided (operator, 2026-11-17): as recommended. |
| Q2 | PRO-006, part 1: a resource does not record if its compute is owned or rented, and a `SERVER` resource has no link to a device. Must a resource hold this? | Yes, but not now. Add it with the Operations area of the project page (OD-W11, number 1): a `SERVER` resource can name a device of the operator, and each kind says owned or rented. Give it a task in that plan. | Decided (operator, 2026-11-17): as recommended. |
| Q3 | PRO-006, part 3 (a job reads the local copy and does not call the service) has no task. Which task owns it? | Task W1: the contract of the work route states it, and task W4 proves it with a test. No code before W1. | Decided (operator, 2026-11-17): as recommended. |
| Q4 | PRO-007: a second forge changes the Projects domain of the hub, one migration and the client, not only the source registry. Change the code now, or accept this until a second forge is planned? | Accept it now (ETH-009: build for what the operator uses; no second forge is in use). Do not change the code. Record that PRO-007 is met at the engine and in the names of the contract, and that the Projects domain becomes neutral in the plan that adds the second forge. | Decided (operator, 2026-11-17): as recommended. |

Group W: the work route. [jobs.md](../proposals/jobs.md) owns its requirements; the tasks are here so that one
table has the order.

| Task | Content | Needs | Done when | State | Commits |
| --- | --- | --- | --- | --- | --- |
| W1 | The contract of the work route: claim, report, log, stop, a secret for one job, the work types `build` and `runner`. It states that a job reads the local copy that Anima holds and does not call the service (PRO-006, Q3). | R-160 (2026-11-18) is the ruling that changes R-115 ([jobs.md](../proposals/jobs.md), "Needs"); this task no longer waits for a ruling. The contract and the security review of the work route come before code | The programme accepts the contract. | Not started | — |
| W2 | The isolation design, with prototypes: the container runtime on Linux, a workflow that needs Docker, the build user on a Mac, a signing identity for one job (PRO-029). | — | A record with measured results on one Linux machine and one Mac. The security reviewer and the operator accept it. | Not started | — |
| W3 | The hub: one queue for work, the placement function, the log store (PRO-050, PRO-051). Hub API, then `anima`, then MCP. | W1 | The gates pass. The release of Anima still works through the old path. | Not started | — |
| W4 | The daemon: do a container job and a native job on a Mac, and delete each thing at the end (PRO-029). A test proves that a job reads the local copy and does not call the service (PRO-006, Q3). | W1, W2 | A test proves that a job cannot read the daemon token or the home folder, and that nothing stays after a stop. | Not started | — |
| W5 | "Takes work" for each machine, and "native jobs" for each Mac (USE-004, USE-008, PRO-024). | W3 | A machine that is set off at the machine refuses work from the hub. | Not started | — |
| W6 | The list of what runs, in the hub and in the client (PRO-026, PRO-052). | W3 | The area shows a build and a session, and shows nothing when nothing runs. | Not started | — |

Group B: runners.

| Task | Content | Needs | Done when | State | Commits |
| --- | --- | --- | --- | --- | --- |
| B1 | Prove the facts that the documentation does not state, on a test repository of the operator: the labels of a JIT runner (F12), the life of an unused JIT configuration (F6), `vars` in `runs-on` for a branch and for a fork (F13), the memory of one idle runner. | The operator's account | Each result is in the table "Facts of GitHub". | Not started | — |
| B2 | The GitHub provider: make a JIT configuration, remove a runner, list runners, read queued jobs, read the fork settings (PRO-049). | A contract change for the new `Change` values (K-E1 Section 13) | Each function has a test with a recorded answer. The JIT configuration is not in a log or in `Debug` output. | Not started | — |
| B3 | The daemon, work type `runner`: get the program, check it, run it in isolation for one job, delete it (PRO-039, PRO-043). | W4, B1 | A workflow of the test repository runs on a Linux machine and on a Mac. Nothing stays on the machine. | Not started | — |
| B4 | The hub: runners on and off for a project, warm mode, the next runner after each job, the audit entries (PRO-042, PRO-046). Hub API, `anima runner`, MCP tools (PRO-053). | W3, B2 | With runners on, two jobs in sequence each get a new runner. With runners off, GitHub lists no runner of Anima. | Not started | — |
| B5 | The set-up in the client, and the pull request that changes `runs-on` (PRO-044, PRO-045). | B4 | From a project page, one press and one merge make a workflow run on an owned machine. Use a private test repository: `TheArctesian/anima.ing` gets no runner before W2 and S are accepted (OD-W18). | Not started | — |
| B6 | On-demand mode (PRO-042). | The webhook receiver (WHK); B4 | With no idle runner, a queued job gets a runner. A lost delivery is repaired by the poll. | Not started | — |
| B7 | Runners for an organisation (PRO-048). | A need of the operator; a new permission | Parked. | Not started | — |
| B9 | The set-up for a coding agent, and the text to copy (PRO-054 to PRO-058; R-170). The section for runners on the project page with each step and its state; the query, the command `anima runner setup` and its read tool; the refusals of PRO-057 with their sentences; the text of PRO-058 from one source, with `anima runner prompt` and the "Copy" control; the test of the text. Hub API, then `anima`, then MCP, then the client (CMD-030). The mockup of the project page changes in the same commit (PLN-019). | B4, B5. The approval of PRO-054 to PRO-058 (R-170 gives the direction only) | A coding agent that gets only the text and the MCP server brings a private test repository to the merge that the person makes, and says which steps the person must do. The test of the text passes. An entry in [need-human-testing.md](../need-human-testing.md), which the operator sets. | Not started | — |
| B8 | The release of Anima as a workflow of GitHub Actions on an owned machine (model (b)). A workflow file that runs only for the version mark of OD-W26 on `main` and never for a pull request; a hub function that takes the bundle of a runner job, with the checks of BLD-060 to BLD-066 before the hub signs; the signing identity of the Mac for one job (W2). The updater key and the R2 token do not go to GitHub (R-114, R-117). It needs new BLD requirements in the design specification first, and the changes of UPD-007, BLD-029 and Section 3.10 that the ruling gives. | R-171 decided OD-W28. The rulings of OD-W29, OD-W31 and OD-W32; OD-W30 after task R11. B5. S. OD-W18: no runner for `TheArctesian/anima.ing` before W2 and S are accepted. The hub that signs: task U12, or the VPS hub while it signs (R-158). OD-W26 only for the start with no act of the operator: until it is decided, the operator starts the release in Anima and the hub starts the workflow (PRO-020, BLD-001). | A release that the operator starts makes one workflow run on an owned machine, and the hub publishes its files. With the ruling of OD-W26, a mark on `main` does the same with no act in Anima (task R13). A pull request from a fork cannot start the workflow and cannot get the signing identity: a test for each claim. | Not started. R-170 and R-171 put it on the path. Blocked until OD-W29, OD-W31 and OD-W32 are decided and its tasks are done. | — |

Group A: builds of Anima's own.

| Task | Content | Needs | Done when | State | Commits |
| --- | --- | --- | --- | --- | --- |
| A1 | The interim state of the release: start it from the Anima project page; remove the Builds entry (PRO-025, PRO-038). | R4; the Projects extension is on | A release starts from the project page and from `anima release create`. | Not started | — |
| A2 | A build as an item of work of a project, with the desktop release as its first recipe (PRO-031, PRO-032, PRO-034, PRO-035). Hub API, `anima build`, MCP, then the project page. | W3, W4 | The Anima release runs as work of `anima-daemon`, from a new checkout, on Linux and on a Mac. | Not started | — |
| A3 | The keys of the release as project secrets (PRO-036). | The Secrets extension (XP-04); A2 | The hub opens the key of the project for one release. No key is in the configuration of the hub. | Not started | — |
| A4 | Remove the desktop builder and the Builds view. Change BLD-020 to BLD-029 and BLD-080 in the design specification (PRO-033). | A2, R7 | One release is published through A2 and installs. Then the old path is deleted. | Not started | — |
| A5 | A build from a push, with a commit status (PRO-010 model (a), PRO-021). | A2; the webhook receiver for the fast path | A push to a project with a recipe makes one build and one status. A second delivery makes no second build. | Not started | — |
| A6 | The image recipe (PRO-032). | A job or a hosted application that needs an image | Parked. | Not started | — |

| Task | Content | Needs | Done when | State | Commits |
| --- | --- | --- | --- | --- | --- |
| S | The security review of builds: the work route, a native job, a fork job on a public repository. | W4, B3, A2 | The security reviewer accepts each, with a test for each claim. | Not started | — |

## Conflicts with text that exists

| Text | Says | This proposal |
| --- | --- | --- |
| PRJ-001 | "The operator must accept each project before Anima adds it." | The import takes each repository and the operator leaves some out (decision 2). PRJ-001 needs new wording. |
| JOB-001, USE-001 | A job is a container image. Jobs run in containers. | Decided (PRO-011): a job is a container job or a native job on a Mac. JOB-004 comes early and is not only Nix: Nix does not supply Xcode or the macOS SDK. PRO-022 to PRO-024 give the limits of a native job. [jobs.md](../proposals/jobs.md) has the new text. |
| [jobs.md](../proposals/jobs.md), "Fine-tuning becomes the first job type" | C7 is first | Decided: the build is first (PRO-009). |
| BLD-001 to BLD-082 | The builder builds the Anima desktop application, started from the Builds view | The same machinery, with a project as a parameter. The identifiers stay; the release of Anima becomes one project. |
| BLD-080, BLD-001, UPD-007, design specification Section 5.3 | A "Builds" navigation entry; a release starts from the Builds view | PRO-025 and PRO-026 replace these when builds are jobs of a project. The design specification is not changed yet. |
| Design specification, sign-in (SEC-002, NAV-009, the `access` domain) | One sign-in: user name, password, authenticator code | Not changed now. PRO-016 adds methods later and keeps this one as the root. |
| Work plan tier 3b | "Projects comes early in that order" | Projects moves before the other extensions and gets its own row. |
| BLD-029, design specification Section 3.10 | A builder "needs no open port and no GitHub runner". "GitHub Actions does not build or publish a release." | OD-W13 permits a machine to be a runner (PRO-027). R-171 (2026-11-18): a workflow on a runner of the operator can build a release of Anima, and only the hub signs and publishes. The design specification has the new text of Section 3.10, UPD-007 and BLD-029. |
| R-005 | "GitHub Actions runs only deployment, never gates or tests." | Not changed (R-171 says so). OD-W32 asks if the release workflow runs the gates on the machine of the operator. |
| BLD-040 | The clone address `TheArctesian/anima.git` | The repository is `TheArctesian/anima.ing` (D5). Corrected in `e3058276` (task R1); the address becomes a parameter (PRO-034). |
| BLD-028 | The tool list has no Go | The gates need Go (D4). |
| Design specification Section 3.11 | `Anima.app` | The builder makes `Anima.ing.app` (D2). Decided (OD-W19): `Anima.ing.app`. Changed by task R5. |
| PRJ-019, `contracts/extensions/sources.md` | The App has no event and its webhook is off | Model (a) needs `push`. On-demand runners need `workflow_job` (PRO-042). The manifest of a new App can name them (F15). An App that exists needs a step by the operator (F16). |
| `contracts/extensions/sources.md` Section 1 | "23 repository permissions: `metadata` read and 22 others write" | The manifest has 24: `metadata` read and 23 write (`manifest.rs`). The text is wrong by one. Not changed here. |
| OD-E14 of the extensions plan | The first GitHub functions that make a change | PRO-049 adds: make a JIT configuration, remove a runner, open the pull request of PRO-045. Each needs the operator's word, as OD-E14 says. |
| [cli-mcp-plan.md](cli-mcp-plan.md) Sections 6 and 7 | `release` and `builder` commands | PRO-053 proposes `build` and `runner`. That plan owns the names. |

## Rules that already apply

- **PRO-017.** Each function is built in this order: the hub API, the `anima` command, the MCP
  tool, the graphical client (CMD-030, [cli-mcp-plan.md](cli-mcp-plan.md) on the branch
  `claude/anima-cli`). Projects and builds must obey it. An MCP tool must not take or return a
  signing key or a provider token (MCP-004).
- **PRO-018.** The hub only reads a rented service (PRJ-003). An action uses the CLI tool of the
  service (OD-W1). GitHub is the exception: the hub makes changes there through the GitHub App
  (PRJ-017; operator, 2026-11-13). A different service gets the same only with its own ruling.
- **PRO-019.** A machine with `anima-daemon` takes work by default, and the operator can set a
  machine off (OD-W10, USE-004). A build is such work.

## Open points

- The area of PRO-026: does an entry open its build or its session (assumed, not said by the
  operator), and its order with Reminders, which the extensions plan Section 3.3 also puts at
  the bottom above Settings.
- The Builds view is in a redesign on the branch `claude/stoic-blackburn-a88d9b`. The operator
  decides if that work moves to the build section of the project page.
- The design of the build user on a Mac (PRO-022): how the daemon starts a process as that user,
  and how Xcode gets a signing identity for one job from a keychain that is deleted after it
  (PRO-023). The Developer ID and the updater key have rulings (R-116, R-117); read them before
  the design. Task W2 makes this design.
- A builder gets a project secret from the hub for one job (JOB-010; operator, 2026-11-13). The
  design of the release and of the deletion on the machine is not made. Task W1 makes it.
- The operator decided OD-W14 to OD-W21 on 2026-11-16. Section 7 of the work plan has each one.
- A workflow that needs Docker (a `container:` job, a Docker action) needs a Docker service
  inside the isolation. The Docker socket of the machine must not go into a job. Task W2 decides
  the way.
- The cost of warm mode: one idle runner for each repository (F5). Task B1 measures it.
- The facts F5, F6, F10, F12, F13 and F16 each have a part that the documentation does not
  state. Task B1 proves them.
- The receiver of the webhook is one address for each source account (WHK-001). The App has one
  webhook (F17). They agree; no change is necessary.
- The webhook receiver runs on the VPS, as a separate small program with one public port
  (OD-E15, operator, 2026-11-13: "on the VPS"; WHK-001 to WHK-010). It changes SITE-006 ("the
  website is the only public part of Anima"). To open the port needs the confirmation of the
  operator at that time (VPS-005). Until the receiver exists, the slow poll is the only path.
- The first GitHub functions that make a change are decided (OD-E14 of the extensions plan).
- The cost data of decision 1 ("cost"): which providers give cost through a read-only API. Not
  verified.
- The facts of GitHub in Plan B are verified against its documentation (2026-11-15). Nothing
  else here is verified against the documentation of a provider (as in
  [new-sources.md](../proposals/new-sources.md)).

## Needs

- A ruling number in `docs/rulings.md` for OD-W11, for OD-W12, for OD-W13 and for OD-W14 to OD-W21.
- Rulings for the requirements that are still proposals: PRO-031, PRO-033 to PRO-035, PRO-039 to
  PRO-041, PRO-043 and PRO-045 to PRO-053.
- A change to PRJ-001 and to Section 4.2 of the extensions plan (with [XP-02](../work-items/extensions/XP-02.md) and [SPEC](../work-items/extensions/SPEC.md)): the import
  takes each repository and the operator leaves some out.
- A security review of builds before the design is final: they run repository code on owned
  machines and use signing keys. A native job (PRO-022 to PRO-024) is the primary subject.
- A checklist on [XP-02](../work-items/extensions/XP-02.md) for the import, and one work item for builds as jobs in
  `docs/work-items/` (PLN-012), when the operator wants to start.
