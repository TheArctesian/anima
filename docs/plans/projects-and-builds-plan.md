---
kind: Plan, with the task table for builds and runners
state: In progress
evidence: It was a proposal until 2026-11-17 (PLN-018). PRO-031 to PRO-053 have no ruling, except the requirements that OD-W14 to OD-W21 name. Decided (OD-W11, OD-W12, OD-W13). Connect GitHub is on `main`. Version 2 (2026-11-16): OD-W14 to OD-W21 are decided; its status line names the requirements that still have no ruling. Its "Tasks" holds the state of each task: P0 and R1 to R6 are done, R7 is in progress, and each other task is not started. Version 2.3 (2026-11-18) adds the path to an automatic release of Anima, with the rows R11 to R13 and B8. Version 2.4 (2026-11-18) records R-170 and R-171: the first automatic release goes by the runner path, and a workflow on a runner of the operator can build a release; it adds PRO-054 to PRO-058 and task B9, which have a direction and no approval. OD-W26, OD-W29 and OD-W32 are open. Version 2.5 (2026-11-18) records R-172 to R-175: the link gets a Unix socket for the daemon before any runner work (task W0), the job of the release workflow signs with the updater key as a secret of GitHub (OD-W31; R-173 amends R-117 and R-171), the hosted hub of the operator holds the Anima project (OD-W30), and the operator runs applications that are built on his machines until the new path can publish. OD-W33 to OD-W37 are open for what follows from R-173.
---

# Plan: projects first (import from GitHub, the builder as a job, sign-in)

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this plan (PLN-005).

Version: 2.14. Date: 2026-11-19. Version 2.14 records targets 1 and 2 of the shorter release as built (the release calls the workflow `gates`, runs the Mac tests in a job of its own, and does not gate a content that passed on `dev`; not run on GitHub). Version 2.13 records what task B8i built (the jobs `ios`, `ios-upload` and `ios-record` of the release workflow, behind `ANIMA_IOS_BUILD` and `ANIMA_PUBLIC_TESTFLIGHT`; not run, and the archive script is task M5-4 of the mobile plan), and the plan for a shorter release in "The trial of 2026-11-18, and the time of the gates". Version 2.12 records R-217 to R-219 (the standalone file, the tap of the operator, the crate with each release) and what task B8h built with them: the jobs `cli`, `cli-publish`, `tap` and `crate` of the release workflow, behind the switch `ANIMA_PUBLIC_CLI`. Version 2.11 records the compile cache of the job `gates` and the job `bucket` of the release workflow, and what is left of task B8h when its three decisions are answered. Version 2.10 records what task R8a built (the crate `anima-cli` packages and builds from its own files; its part (3) waits for OD-W47), adds `anima` alone to the map of the release (task B8h, first part), and names what blocks the rest of B8h: OD-24 and OD-26 are not ruled. Version 2.9 records R-198 to R-204 (each push to `main` releases and daily work goes to `dev`; no limit of releases in a period; tests and comments release; the version counts the first line of `main` only; a local build can share a number; `anima` alone and then iOS join next; the unused secrets stay), says what task B8g built in the section "A release for each change of `main`", and adds the rows B8h and B8i. Version 2.8 records the answers of the evening of 2026-11-18 (R-182 to R-184, R-190 to R-195: no approval, the version from the number of commits, a release for each change of `main`, the gates in the workflow, the workflow publishes, Linux, no release server, the key on the VPS stays), adds the section "A release for each change of `main`" as a design with its questions, and the rows B8f and B8g. Version 2.7 records R-185 to R-188 (SF-003: the list at the machine; SF-006: the device list now and the device key later, task W7; SF-013: accepted; task W2 is measured on a Linux machine of the operator, OD-W39), the two facts of the pool of 2026-11-18, and the notes on what the release session decided. Version 2.6 records R-179 (the release workflow is written now, for the runners of GitHub) and R-180 (the keys are secrets of GitHub for now), and adds the section "The release workflow on the runners of GitHub" with the rows B8a to B8e. Version 2.5 records R-172 (SF-004: the daemon talks to the link over a Unix socket before any runner work; task W0), R-173 (OD-W31: the job signs, and the updater key is a secret of GitHub; he first selected that the hub signs, and withdrew it; R-173 amends R-117 and R-171; OD-W33 to OD-W37 are open for what follows), R-174 (OD-W30: the hosted hub of the operator holds the Anima project) and R-175 (he runs applications that are built on his machines; no public release until the new path can publish). Version 2.4 records R-170 (OD-W27: the runner path) and R-171 (OD-W28: a workflow on a runner of the operator can build a release), puts the chain in one table as it stands after them, and adds PRO-054 to PRO-058 with task B9 (the set-up for a coding agent and the text to copy) and the open decisions OD-W31 and OD-W32. Version 2.3 adds the section "The automatic release of Anima" and the task rows R11 to R13 and B8; it changes no requirement, and its open decisions are OD-W26 to OD-W30. Version 2.2 (2026-11-17) adds the results of task P0 and its decisions Q1 to Q4 (operator, 2026-11-17); it changes no requirement. Version 2.1 only moved this document from `programme/proposals/projects-and-builds.md` (PLN-018, operator, 2026-11-17).
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

This section is a plan of the path. Version 2.3 approved nothing. On 2026-11-18 the operator selected the path: R-170 (OD-W27: the runner path, model (b)) and R-171 (OD-W28: a workflow on a runner of the operator can build a release). The comparison and the recommendation below are the text of version 2.3 and stay as the record of what he selected from. "The chain" is the order as it stands after the two rulings. R-173 and R-174 decided OD-W31 and OD-W30. Open: OD-W26, OD-W29, OD-W32 and OD-W33 to OD-W37 in [decisions.md](../plan/decisions.md). The task rows are in "Tasks" (R11 to R13, B8, B9).

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

### The state on 2026-11-18, after the release of macOS 0.2.2 was put aside

R-175 (operator, 2026-11-18: "Yes, local builds") is the direction for now.

- The operator did not publish macOS 0.2.2 by the Builds path: "Skip 0.2.2 for now". He said of the hub on the VPS: "I am no longer using the hub. That hub should be repointed to the hosted one or just removed."
- He runs applications that are built on his own machines. There is no public release until the new path can publish. The published version of macOS stays 0.2.1.
- The server of the VPS stays as it is and is not used. Its hub is still the only place that holds the updater key (R-117), and R-158 is not changed: it signs until the release server has published one release. Thus by the Builds path no release is signed, unless he uses that hub one more time. R-173 gives a second way to a signature: the job of the release workflow signs with the copy of the key that is a secret of GitHub (R-116). That way needs the chain below. If the release server of R-158 is still built is OD-W34: do not build task U12 of [daemon-and-hub-update-plan.md](daemon-and-hub-update-plan.md) before that answer.
- His hosted hub holds the Anima project and its runners (R-174). It needs an application that is newer than 0.2.1, thus an application that he builds on his machine. Two facts of the pool of 2026-11-18, as the orchestrating session relayed them (the pool is in the private repository; no agent of this repository saw them): the image of a hosted hub is now `anima-hub-2ba73005`; and the operator deleted his hub `h6opknk7w29jw56jqd0kc` on 2026-11-18 (the pool answered 200). The pool makes a new hub at his next sign-in, which he makes from the local build 0.2.90. He has not reported that sign-in. Thus on that day he has no hosted hub, and each text that names the hub `h6opknk7w29jw56jqd0kc` (the result of HT-022 in [need-human-testing.md](../need-human-testing.md)) names a hub that is gone.
- A defect was seen with the 0.2.2 build on his Mac: [DEF-001](../work-items/defects/DEF-001.md), the application replaces itself with a published version that is lower. It must be corrected for an application that is built on a computer before he can use one each day (R-175). The correction is a change of its own, not a part of this plan.

### What the release session decided on 2026-11-18, and what here depends on it

The session of the branch `release/github-workflow` records these answers of the operator with their rulings. This plan section does not decide them and does not quote them: it names the rows that must be read again when those rulings are on `main`.

| Answer, as relayed | Rows of this plan that depend on it |
| --- | --- |
| No separate release server (OD-W34) | Row 14 of the chain and task U12 of [daemon-and-hub-update-plan.md](daemon-and-hub-update-plan.md) go; OD-W35 and OD-W37 lose one choice; the sentence "R-158 is not changed" in the section above must be read again |
| The second release path is Anima as a self-hosted runner (R-170 stands) | None changes. The chain W0, W1 to W4, B1 to B5, S, B8 stays the path |
| The release workflow has no approval step, and its trigger becomes "each change of `main` releases the affected applications" | OD-W26 and task R13 (what starts a release); task R12; the section "What stays an act of the operator"; task B8 and B8c (the workflow file runs for a tag today); OD-W29 and OD-W36 weigh more, because a runner of the operator then signs with no act of his; task B9 (the text to copy must not describe a start by hand) |
| A version comes from the count of commits | Task B8c (`trial-bundle.sh` takes a version of three numbers today); the version of a local build (0.2.90 was selected to stay above 0.2.1 and below the next public version); ROL-012 of the update plan (a machine refuses a lower version) |

The rulings of those answers: R-182 (no approval), R-183 (the version), R-184 (what starts a release) and R-193 (no release server; a machine of Anima as a self-hosted runner). The section "After the rulings of the evening of 2026-11-18" says what each changes in the workflow.

### The chain

The order as it stands after R-170 and R-171, from today to the first release of Anima that a workflow builds on a machine of the operator. A task of a different plan is only named here; its row is in its owner (PLN-016). The column "State" of "Tasks" holds the state of each task; this table does not copy it. "Accepts" names who must say yes before the task is Done or before the next task can start. "Agent now" says if an agent can take the task today, with no new answer of the operator.

| Order | Task | Content, in short | Needs | Check | Accepts | Agent now |
| --- | --- | --- | --- | --- | --- | --- |
| 0 | W0 | The daemon talks to the link over a Unix socket that only it can open, in place of the local port (R-172, SF-004) | — | Its row | The operator, with a human check on a machine that reaches a hosted hub. The ruling for the change of K8 exists (R-172) | Built on the branch `daemon/w0-link-socket` (2026-11-18). It waits for the check HT-037, and for HM-D16 before it serves a hosted hub |
| 1 | R11 | Can one computer build for a hub that is not the hub of its sign-in | — | The result names the files and the test | Nobody. It is a record; it informs OD-W30 | Done on 2026-11-18; see "The result of task R11" |
| 2 | W1 | The contract of the work route, and its security review before code (R-160) | — | Each finding of the review has a state in [security-findings.md](../security-findings.md) | The operator, with a ruling (CON-001): a contract is not a contract before that | Done: the operator accepted version 0.3 on 2026-11-19 (R-209). Task W3 can start |
| 3 | W2 | The isolation design, with prototypes | — | A record with measured results on one Linux machine and one Mac | The security reviewer and the operator | The design and the prototype on a Mac are written (2026-11-18). The Linux measurements are made on a machine of his own (R-188); he has not named it (OD-W39). The rest also needs an administrator on a Mac |
| 4 | B1 | The facts of GitHub that the documentation does not state | The account of the operator and a private test repository | Each result is in "Facts of GitHub" | The operator does it; an agent writes the results that he sends | No. An agent wrote the steps; the account is his |
| 5 | W3 | The hub: one queue, the placement function, the log store | W1 accepted | The gates. The release through the Builds view still works | The merge of the orchestrating session | No, until W1 is accepted |
| 6 | W4 | The daemon: a container job and a native job on a Mac, with deletion at the end | W1 and W2 accepted; W0 for a machine on the link | Tests: a job cannot read the daemon token or the home folder; nothing stays after a stop | The merge. Task S reviews it later | No |
| 7 | W5 | "Takes work" for each machine, "native jobs" for each Mac | W3 | A machine that is set off at the machine refuses work | The merge; a human check for the setting on a real Mac | No |
| 8 | B2 | The GitHub provider: JIT configuration, remove and list runners, queued jobs, fork settings | A ruling for the new `Change` values of K-E1 Section 13 (CON-001) | A test with a recorded answer for each function; the JIT configuration is in no log | The operator, for the contract change. Then the merge | No, until the contract change has its ruling |
| 9 | B3 | The daemon, work type `runner` | W4, B1 | A workflow of the test repository runs on a Linux machine and on a Mac; nothing stays | The operator, with a human check on his machines | No |
| 10 | B4 | The hub: runners on and off, warm mode, the audit entries; `anima runner`; MCP tools | W3, B2 | Two jobs in sequence each get a new runner; with runners off GitHub lists none | The operator, with a human check against GitHub | No |
| 11 | B5 | The set-up in the client, and the pull request that changes `runs-on` | B4 | One press and one merge make a workflow run on an owned machine, on a private test repository | The operator, with a human check | No |
| 12 | B9 | The set-up for a coding agent and the text to copy (PRO-054 to PRO-058) | B4, B5; the approval of PRO-054 to PRO-058 | A coding agent with the text and the MCP server brings a private test repository to the merge of the person; the text holds no secret (a test) | The operator: the approval of the requirements, then a human check | No |
| 13 | S | The security review of builds: the work route, a native job, a fork job on a public repository | W4, B3 (and A2, when it exists) | A test for each claim | The security reviewer. Only the operator sets a finding to Accepted | No |
| 14 | U12 | The release server (R-158); owner: [daemon-and-hub-update-plan.md](daemon-and-hub-update-plan.md). With R-173 the job signs, so what this server is for is open (OD-W34) | OD-W34 | Its row there | The operator | No, until OD-W34 is decided |
| 15 | B8 | The release of Anima as a workflow on a runner of the operator | OD-W26, OD-W29, OD-W32, OD-W33, OD-W35 and OD-W36 (R-173 and R-174 decided OD-W31 and OD-W30); W0; B5; S; W2 and S accepted (OD-W18) | A release that the operator starts makes one workflow run on his machine; its job signs (R-173), and the files are published as OD-W33 decides. A pull request from a fork cannot start the workflow and cannot read the secrets of the environment: a test for each claim | The operator | No |
| 16 | R13 | The first release that starts with no act of the operator in Anima. A human check | B8; the ruling of OD-W26; a runner machine of each target in operation | Its row | The operator | No |

Not on this path: W6 (the list of what runs), B6 (on-demand mode), B7 (organisations), group A (the builds of Anima's own), R12 in the form of the short form, and the tasks R7 to R10 and R8a to R8d, which stay in their own order. R7 stays the task of the first Linux release.

Until OD-W26 is decided, a release by this path still starts with an act of the operator (BLD-001, OD-12): he starts the release in Anima, the hub starts the release workflow (PRO-020), and a runner on his machine builds and signs (R-173). Who publishes is OD-W33. That is rows 1 to 15. Row 16 needs the answer to OD-W26.

One finding of the review of the contract changed this chain (SF-004 of [security-findings.md](../security-findings.md)). A daemon that reaches its hub through the link sends its token to a local port, and a job on the same machine could take it (K8 Section 1 names the weakness). A hosted hub is reached only through the link (HMD-012), and the hosted hub of the operator holds the Anima project (R-174). The operator decided (R-172: "A: fix it first"): the daemon talks to the link over a Unix socket that only it can open, before any runner work. That is task W0, row 0. Until it is built, a machine in the link mode takes no item (contract K10, Section 2).

The first task that an agent could build was W0; it is built on the branch `daemon/w0-link-socket` (2026-11-18) and waits for the check HT-037. After it, no task of this chain is free for an agent: W3 and W4 wait for the acceptance of K10 (W1; the sentence is in its Section 14) and of the isolation design (W2, OD-W39); B1 is steps of the operator; B2 waits for a ruling for the new `Change` values of K-E1. One more thing waits for him before W0 serves a hosted hub: HM-D16 of [hosted-mode.md](../proposals/hosted-mode.md), who starts the link of the daemon on a computer of a person. The recommendation of the session that built W0: the desktop application gives its own link the socket of the daemon (`-daemon-socket`, one argument) and enrols the daemon at `unix:<path>` when its hub is a link hub; a second link with an invite of its own is the other choice and costs a second device row for each computer. Task U13 of [daemon-and-hub-update-plan.md](daemon-and-hub-update-plan.md) is a task that an agent can build now; it is not on this chain.

The security review of the work route has two places, and both stay: before code, with the contract of W1 (R-160; [jobs.md](../proposals/jobs.md), "Needs"); and at the end, task S, for a native job and for a fork job on a public repository.

### How a runner job gets a signed release (OD-W31: decided, R-173)

Result (R-173, operator, 2026-11-18): the job signs, and the updater key is a secret of GitHub. This is choice C of the table below, in its second form. He first selected choice A ("A: hub signs"), then withdrew it ("also lets change it the Job signs it not the hub"), and to the question where the key then is he said: "In GitHub secrets". R-173 amends R-117 and R-171. The text and the table below are the proposal of version 2.4 and stay as the record of what he selected from; the recommendation was A and is not taken.

What follows from R-173 is not decided. Each point is an open row of [decisions.md](../plan/decisions.md): who uploads and writes the feed, and which checks of the hub the workflow must then have (OD-W33); what the release server of R-158 is still for (OD-W34); what R-174 means now (OD-W35); the protection of the key (OD-W36); and the Builds path of today, which still signs through a hub (OD-W37).

Before R-173, R-117 and R-158 said: only the hub that holds the updater key signs, and no builder holds the key. R-171 lets a runner job build. The proposal was this.

Facts that limit the design:

- In model (b) Anima gives a runner only its registration (PRO-030, PRO-040). GitHub gives the job its secrets, by the rules of GitHub (PRO-047).
- A warm runner takes the next job whose `runs-on` has its labels (PRO-042). The hub does not select the job. A job of a pull request from a fork can be that job (F20).
- The desktop builder proves what it is with a builder token that the operator enrolled (BLD-021, BLD-022), and the hub gives it upload addresses for the files of its job only (BLD-060, BLD-061).

| Choice | How it works | For | Against |
| --- | --- | --- | --- |
| A. The job hands its files to the hub and proves what it is with the identity token of GitHub | The release workflow asks GitHub for the identity token of its job (OpenID Connect). A step sends that token to the hub with the request for upload addresses. The hub checks the signature of GitHub and the fields: the repository is the repository of the Anima project, the workflow file is the release workflow on the default branch, the event is not a pull request, and the commit is the commit of the release and an ancestor of `origin/main` (BLD-041). Then the path is the path of today: upload addresses for this job only, the checks of BLD-064 and BLD-065, the signature of the hub, the feed | No stored secret in the repository, in the job or on the machine. R-117 does not change. The checks of the hub stay the last gate. A job of a fork gets a token whose fields say what it is, and the hub refuses it | The hub must read the public keys of GitHub. The fields of the token on a self-hosted runner must be proved (task B1 gets this fact). A new hub function and a new contract text |
| B. The daemon hands the files over | The job writes its files to one output folder. After the job ends, `anima-daemon` uploads them as the holder of the work item. The hub asks GitHub which job that runner did (F10, F11) and accepts the files only for the release workflow on the mark | The job gets no credential and makes no request to the hub | The proof is a second read of GitHub by the name of the runner, after the fact. The daemon must take files out of the isolation of a job whose code it does not trust. More new code on the machine |
| C. The key is a secret for one job | The hub gives the updater key to the job (JOB-010), or the key is a secret of the repository, and the job signs | The smallest change to the workflow | It reverses R-117 and the reason of R-114. Code of the repository, its build scripts and its packages run beside the key that each installed application trusts (BLD-044). Not recommended. **Selected by the operator (R-173), with the key as a secret of the repository.** OD-W36 holds the measures that limit this risk |

The recommendation was A: it keeps one place that signs, it adds no secret, and a job that is not the release workflow cannot get an upload address. It is not taken (R-173).

The Developer ID identity of the Mac is a second key with the same question. R-116 records that the Apple secrets are also in the environment `release` on GitHub. If the job that signs the updater file also signs and notarizes the Mac bundle with them is not answered; it goes with OD-W36. Until it is answered, BLD-083 says that the identity is in the keychain of the Mac, and a runner job as the build user has no keychain of the operator (PRO-022).

### The result of task R11 (2026-11-18)

The question: can one computer build for a hub that is not the hub of its sign-in? The record with each request and each answer is [r11-two-hubs.md](../research/builds/r11-two-hubs.md). Two hubs ran on one Mac; the desktop application did not run.

| Part | Answer | How it is known |
| --- | --- | --- |
| The hub | Yes. A builder is its token and the address of its enrolment. The hub that makes releases accepted its builder while the command line was signed in to the two hubs, and the other hub refused that token | Tested |
| The `anima` command line | Yes for two sign-ins at the same time (`--context`) and for `anima builder enrol` at the second hub. It has no builder loop, so it cannot build | Tested |
| The desktop application | No. It has one hub address, and its builder loop runs for that address only (`provideDesktopBuilder`). The key store already keeps a builder token for each hub origin | From the code. Not run |
| `anima-daemon` | One daemon has one hub. On a Mac the token is one fixed entry of the Keychain, so a second daemon would write over the first. On Linux the token is a file of the home folder, so two homes are possible in the code | From the code. Not tested |

What this means for the runner path (R-170). The builder of that path is `anima-daemon`, and a daemon has one hub. Thus the hub that gives the runner work to a machine is the hub at which the daemon of that machine is enrolled: for the operator, his hosted hub (R-174). No computer needs to be a builder of two hubs, and the smallest change below is not necessary for the runner path. With R-173 the job signs by itself; if a second hub has any part in a release is OD-W33 to OD-W35.

The smallest change for the desktop builder, if the operator wants it (task R14): the builder loop takes a hub origin of its own, apart from the hub of the sign-in, with one sign-in at that hub for the enrolment. It is only for the Builds path of today; the runner path does not need it.

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

### The release workflow on the runners of GitHub (R-179, R-180; version 2.5, 2026-11-18)

The operator stopped the use of the hub on the VPS and gave five answers the same day. Two are on `main` (R-170, R-171). Two are recorded here: R-179 ("Go with A": the workflow is written now, for the runners of GitHub) and R-180 (the secrets of GitHub hold the keys for now; the Secrets extension later). R-173 records the other two, "also lets change it the Job signs it not the hub" and "In GitHub secrets": the job signs, and the updater key is a secret of GitHub. R-173 is on `main`. The operator has not decided to merge the tasks B8b and B8c: a workflow file on `main` can run on GitHub.

What is built is the smallest path to a signed release with no hub: one workflow file, `.github/workflows/release.yml`, and one command of the hub program, `anima-hub release`, which holds the checks that the hub makes today.

#### The jobs

| Job | Machine | Environment of GitHub | Secrets | What it does |
| --- | --- | --- | --- | --- |
| `commit` | `ubuntu-24.04` | none | none | Reads the version from the tag `v<version>` (or from the input of a trial). Refuses a commit that is not an ancestor of `origin/main` (BLD-041). Gives the version, the commit and its time to the other jobs |
| `gates` | `macos-15` | none | none | `bun run gates` with `BASE_REF` set to the release commit (BLD-042). OD-W32 is open: see the table of requirements |
| `tool` | `ubuntu-24.04` | none | none | Builds `anima-hub` from the release commit and gives its SHA-256 as a job output. The jobs `sign` and `publish` run this file; they compile nothing |
| `macos` | `macos-15` | `release` | The Developer ID certificate and the notary credentials | The recipe of the builder, in its order (`apps/desktop/src-tauri/trial-bundle.sh` with the identity of `tauri.conf.json`): the database tree, `link/build.sh`, the Rust programs for both architectures, Tauri. A keychain that exists for the job only. The check of each Mach-O file (UPD-011, `check-mach-o.sh`), notarization, stapling, then `Anima.app.tar.gz`. No updater key |
| `linux` | `ubuntu-22.04` | none | none | The same recipe for `x86_64` and the AppImage. No key: the bundle step downloads `linuxdeploy` with no hash check |
| `sign` | `ubuntu-24.04` | `release-sign` | The updater key and its password | A new machine. It takes the two updater files, compares each with the SHA-256 that its build job gave as a job output, and signs each with `anima-hub release sign` |
| `publish` | `ubuntu-24.04` | `release-publish` | The token of the release bucket | Only for a tag, and only after each job above passed. `anima-hub release publish` for `linux-x86_64`, then for `darwin-universal`. It writes each feed last |
| `trial` | `ubuntu-24.04` | none | none | Only for a run by hand (`workflow_dispatch`). `anima-hub release check`: each check of `publish` that needs no bucket. It publishes nothing and has no token |

Three environments, not one. A job gets each secret of its environment in the memory of its runner, also a secret that no step names. So one environment `release` would give the updater key and the bucket token to the macOS build, where build scripts of the release commit run. With three environments the macOS build has only the Apple secrets, the job that signs has only the updater key, and the job that publishes has only the bucket token. This is measure (5) of OD-W36, made stronger: that row holds the choice and names what is built.

#### Which code checks, and why

The checks of the hub are not written a second time. `anima-hub release` is a command of the hub program, and it calls the functions of `crates/hub/src/fleet/builds/publishing/` that the hub calls and that have tests: the feed check (`feed::check`), the file names and `SHA256SUMS` (`files`), Signature Version 4 and the requests to the bucket (`sigv4`, `bucket`, `transfer`), the write of the `.sig`, the copy to the stable name with the `ETag`, the write of `latest.json` and its read back (`run`), and the signature crate `updater-signature`, which is the verifier of the updater itself. The command needs no database and no running hub. What it does not use is the queue of jobs and its holds (D-25), which have no meaning for one job of a workflow. The other choice, steps in shell with `aws s3` and `jq`, would be a second copy with no test and with a new program that holds the token.

Two checks are in shell, because the code that holds them is inside the desktop application and cannot run alone: the check of each Mach-O file (`apps/desktop/src-tauri/check-mach-o.sh` runs the commands of `builder/carried/mach_o.rs`) and the bundle recipe (`trial-bundle.sh`, which already ran the commands of the builder by hand; it now takes the signing identity). Task B8e names the work to make one owner of each.

#### The requirements that the workflow changes

This branch does not change the design specification. Each row gives the text now, the proposed text, and what covers it. R-173 already changed UPD-007 and the first paragraph of Section 3.10 for the signature, and kept BLD-045 and UPD-012 as the rules of the Builds path, with a note. The "text now" of each row is the text before R-173. A row that an answer of the operator covers changes in task B8e. Each other row is open, with a recommendation.

| Requirement | Text now (short) | Proposed text | Covered by |
| --- | --- | --- | --- |
| Section 3.10, first paragraph | "No machine of GitHub builds a release, and GitHub Actions does not sign or publish one." | "A workflow of GitHub Actions builds, signs and publishes each release (`.github/workflows/release.yml`). Its jobs run on machines of GitHub until a runner of the operator takes them (R-179). The build queue of the hub stays for a hub that holds an updater key." | R-179 for the machine. R-173 for "signs" (R-173 already changed this paragraph for the signature). "Publishes" is OD-W33, open |
| BLD-001 | "The operator must start a release from the Builds view with "Release <version>"." | Add: "The release workflow starts from a tag `v<version>` that the operator puts on a commit of `main`. A run by hand is a trial and publishes nothing." | Open: OD-W26. Recommendation of that row: the tag. The workflow is written for the tag |
| UPD-007 | "A release must start only from the Builds view (BLD-001). No workflow of GitHub holds the updater key or publishes a release." | "A release starts only from the Builds view or from a tag `v<version>` on a commit of `main` (BLD-001). The release workflow holds the updater key only in its job `sign`, and the bucket token only in its job `publish`. No pull request starts it." | First sentence: OD-W26, open. Second: R-173 (it already changed UPD-007 for the signature); the bucket token and the job `publish` are OD-W33, open |
| UPD-008 | "The hub must write `latest.json` only after it has verified all files of the job." | "The hub, or the job `publish`, must write `latest.json` only after it has verified all files of the target." | R-173 for the signature; that the job `publish` does it is OD-W33, open |
| UPD-012 | "Only the operator makes the updater key pair and installs the private key and its password as the hub's secret files `updater_key` and `updater_key_password`. Agents must not read the private key file." | "Only the operator makes the updater key pair. He sets the private key and its password as the secrets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` of the environment `release-sign` of GitHub, until the Secrets extension holds them (R-180). An agent must not read the private key and must not set, read or change a secret of GitHub." The backup sentence stays | R-173 and R-180. R-173 kept UPD-012 as the rule of the Builds path with a note |
| BLD-002 | "The hub must refuse a release whose version is not more than each published version of each selected target." | Add: "The job `publish` refuses a version that is not more than the version of each feed of the target, and refuses to change a file under `<target>/<version>/` that the bucket already holds." | Open: OD-W33 (who publishes and which checks). The form of the check is a recommendation: with no hub there is no list of releases, so the feed and the version folder are the record |
| BLD-041 | "The builder must refuse a commit that is not an ancestor of `origin/main`." | "The builder, or the job `commit` of the release workflow, must refuse…" (the rule itself does not change) | Open, no answer names it. Recommendation: keep the rule; the workflow has it. Note: the workflow file of the tagged commit makes this check, so a tag on a changed workflow can remove it. The protection is the approval of the environment and the tag rule (procedure, "Before the first run") |
| BLD-042 | "The builder must run `bun run gates` with `DATABASE_URL` unset and `BASE_REF` set to the job commit." | "The builder, or the job `gates` of the release workflow, must run…" | Open: OD-W32, and R-005 with CQ-010 ("Do not run gates or tests in GitHub Actions"). The recommendation of OD-W32 was "yes, on your machine only"; on a machine of GitHub this is a new question. The job is in the workflow because BLD-042 asks for the gates before a release. If the answer is no, delete the job `gates` and its name in `needs` of `sign` |
| BLD-045 | "The hub must sign the updater file. … The builder signs nothing." | "The job `sign` of the release workflow signs the updater file with `anima-hub release sign`. The key and its password come from the secrets of its environment, are written to two files that only the job user can read, and are deleted at the end of the step. The build jobs sign nothing. A hub that holds an updater key signs as before." The sentences on logs and errors stay | R-173 for the signature; that the job `publish` does it is OD-W33, open |
| BLD-046 | "The hub signs exactly the bytes it verified (BLD-064), then checks the signature with `ANIMA_UPDATER_PUBLIC_KEY`…" | Add: "The job `sign` signs only a file whose SHA-256 is the one that its build job gave as a job output. The job `publish` checks the signature over the bytes that it uploads, with `plugins.updater.pubkey` of the release commit, before it writes anything." | R-173 for the signature; that the job `publish` does it is OD-W33, open |
| BLD-064 | "The hub must start these checks only after every upload URL of the job has expired. It must read each file from the R2 bucket itself with a signed S3 `GET`… The hub then signs the updater file and writes `<file>.sig`." | For the workflow: "The job `publish` must check each file against the SHA-256 that its build job gave as a job output (not against a file of the same artifact). It uploads each file, reads it back from the bucket with a signed `GET`, and the bytes must have the same size and SHA-256. Each file must answer 200 at `https://releases.anima.ing`. It writes `<file>.sig` with the signature of the job `sign`." | R-173 for the signature; that the job `publish` does it is OD-W33, open |
| BLD-065 | "The hub must check the proposed feed…" | "The hub, or the job `publish`, …" The program makes the feed and checks it with the same function | R-173 for the signature; that the job `publish` does it is OD-W33, open |
| BLD-066 | "…copy the file to its stable name with R2 `CopyObject` and `x-amz-copy-source-if-match: <ETag>`… write `latest.json` with `Cache-Control: no-cache` last." | The same order, by the job `publish` | R-173 for the signature; that the job `publish` does it is OD-W33, open |
| BLD-067, BLD-068 | The same feed for `darwin-aarch64` and `darwin-x86_64`; the macOS feed is written | No change of the rule; "the hub" becomes "the hub or the job `publish`" | R-173 for the signature; that the job `publish` does it is OD-W33, open |
| BLD-069 | "The hub must not write a feed whose version is lower than the published feed of that target." | "The hub, or the job `publish`, must not write a feed whose version is lower than the feed that the bucket holds for that target. A feed of the same version is written again only when it is the same feed." | Open: OD-W33. The second sentence is a recommendation |
| BLD-083, BLD-084 | The certificate is in the login keychain of the builder; the notary credentials are in the keychain profile `anima-notary` | Add: "The job `macos` imports the certificate and stores the profile `anima-notary` in a keychain that it makes for the job and deletes at its end. The identity that signs is the one that `bundle.macOS.signingIdentity` of the release commit names." | R-180 (the keys are secrets of GitHub). R-116 already names these secrets |
| UPD-011 | "The macOS builder must verify the bundle…" | "The macOS builder, or the job `macos`, …" | R-179 |
| R-005, CQ-010 | "GitHub Actions runs only deployment, never gates or tests." | See BLD-042 | Open: OD-W32 |
| R-116 | "the operator keeps the `release` environment (main only)" | The environments allow the tag pattern, and `main` for a trial: the procedure has the list | The operator sets it; R-180 |
| R-117, R-158 item 1 | The hub on the VPS holds the updater key and signs until the release server has published one release | Open: OD-W34 (what the release server is for; its choice A deletes the copy of the key on the VPS after one release of the workflow) and OD-W37 (the Builds path). The key file on the VPS is a second place that can sign |

#### The names of the secrets and variables

The operator sets each one; an agent never does (R-180). R-116 says that seven of them exist in an environment `release`; nothing in this work could read GitHub to confirm that, and R-116 says that environment allows `main` only.

| Name | Kind, environment | What it must hold | R-116 says it exists |
| --- | --- | --- | --- |
| `APPLE_CERTIFICATE` | Secret, `release` | The Developer ID Application certificate with its private key, as a `.p12` file in base64 on one line | Yes |
| `APPLE_CERTIFICATE_PASSWORD` | Secret, `release` | The password of that `.p12` file | Yes |
| `APPLE_ID` | Secret, `release` | The Apple account of the notary service (an email address) | Yes |
| `APPLE_PASSWORD` | Secret, `release` | An app-specific password of that account, in the form `xxxx-xxxx-xxxx-xxxx` | Yes |
| `APPLE_TEAM_ID` | Secret, `release` | The Team ID, ten characters; the same as in the identity of `tauri.conf.json` | Yes |
| `TAURI_SIGNING_PRIVATE_KEY` | Secret, `release-sign` | The text of the file that `tauri signer generate` wrote (`~/.config/anima/updater.key`): one line of base64 | Yes, in `release`. It must move to `release-sign` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Secret, `release-sign` | The password of the key. Do not make the secret if the key has no password | Not named by R-116 |
| `R2_ACCESS_KEY_ID` | Secret, `release-publish` | The access key ID of an R2 API token with "Object Read & Write" for the bucket `anima-releases` only | No |
| `R2_SECRET_ACCESS_KEY` | Secret, `release-publish` | The secret access key of the same token | No |
| `RELEASES_ENDPOINT` | Variable, `release-publish` | `https://<account ID>.r2.cloudflarestorage.com` | No |

`APPLE_SIGNING_IDENTITY` of R-116 is not read: the identity comes from `tauri.conf.json` of the release commit, as the builder reads it (BLD-083). The bucket name `anima-releases` and the address `https://releases.anima.ing` are in the workflow file; they are not secrets.

#### Where the phones join (R-158)

The Android APK is a target of the release (R-158 item 4, task R10): it joins this workflow as one more build job and one more target of `anima-hub release publish`, after task R10 has written its BLD requirements and task M5-5 gives a signed APK. Its signing key is then one more secret, in an environment of its own. The iOS application does not join: it goes to TestFlight from a script on the Mac of the operator (R-158 items 3 and 5).

#### What moves later

To a runner of the operator (R-170): the `runs-on` line of each job; the steps that install tools on the machine of GitHub (the steps named "Install …"); the keychain of the job, which task W2 replaces with the identity for one job; and OD-W18 with task S first. The secrets and the command `anima-hub release` do not change. To the Secrets extension (R-180, task A3): the job `sign` and the job `publish` then ask Anima for the key for one job in place of `secrets.…`; the rule of R-158 applies to where that store opens the key.

#### After the rulings of the evening of 2026-11-18 (R-182 to R-184, R-190 to R-195; version 2.8)

The tasks B8b and B8c are on `main` (3ae44093). The operator then gave eight answers. What each one changes in this section:

| Ruling | What it decides | What changes here |
| --- | --- | --- |
| R-182 | No approval for a release | The environments have no required reviewer. The sentences of this section and of the table that name "the approval of the environment" as a protection no longer apply; the protections are in [releases.md](../procedures/releases.md), "What the operator accepts with this form" |
| R-183 | The version of a commit is `0.<N div 100>.<N mod 100>`, N the number of commits of its history | Task B8f: `deploy/bin/commit-version`. The job `commit` takes the version of a release from the command (the refusal of a tag of version 2.8 went with the tag trigger, task B8g). `trial-bundle.sh` with no version uses the command |
| R-184 | Each change of `main` releases the applications that it affects (OD-W26) | Built in task B8g (version 2.9); the section "A release for each change of `main`" says what |
| R-190 | The workflow runs the gates (OD-W32) | The job `gates` stays. The rows BLD-042 and "R-005, CQ-010" of the table are covered |
| R-191 | The workflow publishes (OD-W33) | Each row of the table that says "OD-W33, open" is covered |
| R-192 | The first public release carries macOS and Linux and waits for his check of the AppImage (OD-W38) | Task B8d waits for that check. The sentence that no AppImage was started on Linux is corrected |
| R-193 | No release server (OD-W34, first part); a machine of Anima as a self-hosted runner is the second way | The row "R-117, R-158 item 1" of the table: the release server is withdrawn. Task U12 is not built |
| R-194 | The updater key on the VPS stays for now (OD-W37) | Nothing. It stays a listed risk |
| R-195 | An agent set two secrets on his instruction; R-180 item 4 is amended | The sentence "an agent never does" above the table of names has that one exception |

The proposed text of the table "The requirements that the workflow changes" can now go into the design specification, except the rows BLD-001 and UPD-007 (first sentence): their text depends on the next section. That is task B8e.

#### The trial of 2026-11-18, and the time of the gates

The first run of the workflow on GitHub (a trial, run 44468487102; HT-036 has the observation) passed in each build job and failed in the job `gates`: the macOS machine of GitHub has no `shellcheck`, which three tests need. The job now installs it. A second fault was behind it: the documents gate compares a screen with the commit of its mockup (PLN-019), and the job had a shallow clone; the job now gets the full history. No other tool is missing for gates 16 to 21: they run `bun` and `git` only, and each was run in a shallow clone on a computer to find this.

The job `gates` took 75 minutes, the macOS build 53 and the Linux build 24. The jobs run at the same time, so a release waits for the slowest one.

| Part of the job `gates` | Time |
| --- | --- |
| `clippy`: compile | 7 minutes |
| Rust tests: compile, with no cache | 20 minutes |
| Rust tests: the run of 6166 tests on 3 cores | 31 minutes |
| The six Go gates | 7 minutes |
| `bun run test` (`deploy/headscale` 6 minutes of it) | 6 minutes |
| Each other gate that ran | 2 minutes |

Recommendations, not built; none changes what the gates run:

1. **A cache of compiled Rust for the job `gates` only** is a safe saving of about 25 minutes. The job holds no secret and nothing that it builds is shipped; a wrong cache could at worst make a gate pass that must fail. The workflow has no pull request trigger and one account writes, so no other person can write that cache. The action that caches must be pinned to a commit, as each other action. The jobs `macos`, `linux` and `tool` must keep no cache: what they build is shipped or holds a key.
2. **With that cache the gates take about as long as the macOS build** (about 50 minutes). A saving beyond that gives nothing until the macOS build is also faster, and that build must not use a cache.
3. **A Linux machine for the gates is not a safe saving today.** The gates were only ever run on macOS. On Linux, `clippy --all-targets` compiles the desktop shell, which needs the packages of the job `linux`, and no run has shown that each test passes there. It needs a trial of its own first. The standard machines of both kinds are free for a public repository (R-193), so the gain would be time only.
4. The run of the Rust tests (31 minutes on 3 cores) is the largest part that a cache does not remove. Two jobs that each run one half of the tests would halve it; that is a change of how the gates run, and a decision for later.

Built on 2026-11-19 (branch `release/first-public`): recommendation 1. The job `gates` installs `sccache`, which the gates runner already uses when it is on `PATH` (`quality/src/gates/cache.ts`), and keeps its folder between runs with `actions/cache`, pinned to a commit. No gate changes: the compiler outputs are kept by their inputs, as on the computer of the operator. No other job has a cache, and a test holds that. Not measured: a cache is filled by the first run that has it and read by the next, and a trial can start only from a commit of `main`, so the two trials that measure it come after the merge. The second trial of that day took 82 minutes for the gates, 62 for the macOS build and 14 for the Linux build; the expected time of the gates with the cache is about 60 minutes, which is the time of the macOS build. The last step of the job prints how much of the compile came from the cache.

Built with it: a job `bucket`, which reads one object of the bucket with the token in the first minute of a release and writes nothing. A wrong token, endpoint or environment rule thus stops a release at its start and not after each build. A trial does not run it.

The plan for a shorter release (operator, 2026-11-19, relayed: gates 82 minutes, the macOS build 62, the Linux build 14; a full gates run on a quiet Mac is 13 minutes; standard runners only). Each number is an estimate until one trial measures it, and a trial can run only from a commit of `main`.

| Target | What is planned | State | What keeps it safe |
| --- | --- | --- | --- |
| 1. The gates on Linux, in parallel jobs for Rust, Go and bun, with a compile cache: 82 to about 20 to 25 minutes | One reusable workflow, `.github/workflows/gates.yml`, that the runner-path session writes and owns (groups of gates in `quality/src/gates/gates.ts`), and that `release.yml` calls in place of its job `gates` | Built on the branch `release/call-gates` (2026-11-19), not run. The workflow `gates` is on `dev` and passed there: 31.5 minutes for a whole run (go 2, bun 6, rust-lint 12, rust-test 31). The job `gates-linux` of the release calls it with `ref`; the job `gates-macos` runs `deploy/bin/macos-tests`; the job `gates` joins them. Each job after `gates` names the result of each job that it waits for, because GitHub skips a job when a job before it in the chain was skipped. The time of `gates-macos` is not measured | The gates were never run on Linux. Some tests run only on a Mac (the Mach-O check, one test of `trial-bundle.sh`, the Keychain code, the access lists of `link/statedir`): the release keeps one macOS job for exactly those until a Linux run is shown to cover them. The split must run each of the 21 gates one time |
| 2. Do not gate one content two times | The first job looks for a run of `gates.yml` that passed for a commit with the same tree as the release commit, and then does not run the gates again | Built on the same branch, not run: `deploy/bin/gated-run` with tests of each case that must not count, and the step `gated` of the job `commit`, which may read the list of runs (`actions: read`). A fault in the list means that the gates run. The Mac tests run in each release | "The same" is the tree of the commit, which holds each file, the workflow and the gates themselves: an equal tree ran the same gates on the same content, whatever branch it was on. Only a run of that workflow file for the branch `dev`, of event `push` or `workflow_dispatch`, in this repository, with the result success, counts; the first job reads it from the API of GitHub, which a branch cannot write. A run for any other branch does not count: the runner-path session says (2026-11-19) that such a run tests the branch merged with `dev`, not the tree of its commit. The workflow has no pull request trigger |
| 3. The macOS build: 62 to about 25 to 30 minutes | Build the two architectures on two machines at the same time, each with no secret, and join and sign them in a third job | Not built. It changes `trial-bundle.sh` and the steps of the job `macos`, which the hosted onboarding session changes now for the daemon and the link service; it comes after that work | With the split, the two jobs that run the build code hold no Apple secret, which is better than today. Not recommended: a cache of the PostgreSQL tree or of compiled Rust in a job whose output is signed. A cache holds bytes that a run before made; they would be signed and shipped with no rebuild from the commit, and the builds are not reproducible bit for bit, so a check of the cache against the commit is not possible. If he wants that cache also, the key must be the hash of `database-tree/postgresql.pin`, the build script, the Rust tool version and the image of the machine, and only a run of `main` may write it; the documents must then say that a signed file can hold bytes of an earlier run |
| 4. His own runner | Tasks W1 to W4, B1 to B5 and S of this plan | Later (R-170, R-179) | OD-W18 and OD-W29 |

Recommendation of the session that writes this: release first with the workflow that two trials proved, with the cache and the job `bucket` of this day, and take targets 1 to 3 after it, each with its trial. A first public release is not the run on which to learn if a new build layout signs correctly.

### A release for each change of `main` (R-184, R-198 to R-204; built in task B8g, version 2.9)

The operator read the design of version 2.8 of this section and answered its seven questions (OD-W40 to OD-W46; R-198 to R-204). This section says what is built. The steps of the operator are in [releases.md](../procedures/releases.md); HT-036 is the check.

#### The applications, and the paths that affect each one

| Application | Released by | State | What goes into it |
| --- | --- | --- | --- |
| Desktop application (macOS universal, Linux AppImage) | The workflow, jobs `macos` and `linux` | Built | Each path of the repository that the map does not put out |
| `anima` alone (Homebrew; tasks R8a to R8d) | A job of the workflow, task B8h | Not built; next (R-203) | `crates/anima-cli` and each crate that it uses |
| iOS application (TestFlight) | A job of the workflow, task B8i | Not built; after `anima` (R-203) | `apps/client`, `apps/desktop` (the phone shell), `link/` |
| Android application (APK, F-Droid, R-181) | Not planned here; F-Droid builds from the source itself | Not built | The same as iOS |
| `anima-daemon` and the hub (the update of [daemon-and-hub-update-plan.md](daemon-and-hub-update-plan.md)) | That plan | Not built | `crates/daemon`, `crates/daemon-core`; `crates/hub` |

The map is `deploy/release/applications.txt`, and `deploy/bin/affected-applications <from> <to>` reads it. Its rules, as built:

1. **An application is affected unless each changed path is out.** The map names what never goes into an application (for the desktop application: `docs/`, `quality/`, `deploy/`, `crates/rdp-test-device/` and the three documents at the root), with the release commands and the map itself as exceptions. A folder that the map does not know thus releases. A wrong line can keep a release back; it cannot ship a change with no release.
2. **Each change inside the folders of an application releases it,** tests and comments too (R-200).
3. **Not built: the Rust part from `cargo metadata`.** The design of version 2.8 wanted the crates of each program computed. For one application it gives nothing: the desktop application carries each program but the test device. It is needed when `anima` alone joins (task B8h), because a change of the client must not release `anima`.

#### The trigger and the decision

`on: push: branches: [main]`, and a run by hand for a trial. No tag starts the workflow. The first job (`commit`) reads the version that each of the three feeds gives now (`none` for a feed that does not exist; any other answer stops the run), and runs `deploy/bin/release-decision`, which prints `release <version>` or `nothing: <reason>`:

- **The commit is compared with the published version, not with the commit before the push.** The commit of a published version is the commit at that position on the first line of `main`; no file in the bucket holds it. So a change is not lost when a run was cancelled, failed at the gates or was replaced by a newer run. A feed that does not exist releases. The feed that is behind decides, so a target that missed a release gets the next one.
- **A version that is not more than a published version releases nothing** (BLD-002). An old run that a person starts again thus ends with "nothing", and builds nothing.
- **A push that does not keep the first line of `main` fails** (R-201): the tip of `main` before the push must be on the first line of the pushed commit.
- **The switch.** When the decision is a release and the repository variable `ANIMA_PUBLIC_RELEASES` is not `on`, the job says so and nothing builds. The job `publish` also names the variable in its condition.

The reason of each decision is in the summary of the run. When the decision is "nothing", the jobs `gates`, `tool`, `macos` and `linux` do not run, and the jobs after them are skipped.

#### The version and the feeds

- The version of each application that a run publishes is the version of the release commit: N is the count of the first line of `main` (R-201). 0.8.7 is commit 807 of that line.
- Each application and each target keeps a feed of its own (BLD-067). The versions of one application have gaps, and the feeds of two applications do not move together.
- For the phones the same N can be the build number (iOS) and the `versionCode` (Android). F-Droid reads the version from the source; that is a question for task M5-7 of [mobile-plan.md](mobile-plan.md).

#### Two pushes near each other

One concurrency group, `cancel-in-progress: false`: GitHub runs one release, keeps one that waits, and cancels an older one that waits. A trial has a group of its own. A, B and C land while A runs: A publishes, B is cancelled while it waits, C runs next and carries the change of B because it compares with the published version. Releases thus publish in the order of the pushes, and the refusal of a version that is not higher is the second guard (the first job, and `anima-hub release publish`, BLD-069).

#### What is not proven

No run of this trigger was on GitHub. The tests run the two commands in repositories that they make, and read the rules of the workflow file. Not tested by any run: the answer of the three feeds through `curl`, the value of `github.event.before` for each kind of push, the behaviour of the concurrency group, and the refusal of the environment `release-publish` for a run of `main`. HT-036 has the steps; the first of them need no public release, because the switch is off.

#### The next two jobs (R-203): a plan, not built

1. **`anima` alone by Homebrew (task B8h).** It follows tasks R8a to R8d of this plan, which make the crate publishable, build a standalone `anima` for each target and write the formula and the tap. The job builds `anima` for `aarch64-apple-darwin`, `x86_64-apple-darwin` and `x86_64-unknown-linux-gnu`, the job `sign` is not needed (Homebrew checks a SHA-256, and the macOS files get the Developer ID signature in the job that holds that certificate), and a job publishes the files to the bucket and the formula to the tap. Open before it is built: the map must tell `anima` from the client (`cargo metadata`, above); the push to the tap repository needs a credential, which is a new secret in an environment of its own; tasks R8a to R8c are not done.
   What is ready, and what is left when OD-24, OD-26 and OD-W47 are answered as recommended (the standalone file; the tap of the operator; no crate for now). Ready: the crate builds alone (task R8a), the map knows `cli`, and `release-decision` takes any application of the map. Left, in this order: (a) a job `cli` beside `linux`, with no secret, that builds `anima` for `x86_64-unknown-linux-gnu` with `ANIMA_VERSION` set, and gives its SHA-256 as a job output; the macOS file is built in the job `macos`, which holds the Developer ID, as one universal file that is signed there. (b) A target of `anima-hub release check` and `publish` for the two files: `anima/<version>/anima-darwin-universal.tar.gz` and `anima/<version>/anima-linux-x86_64.tar.gz` with `SHA256SUMS`, and a small feed `anima/latest.json` with the version, so that the first job can decide for `cli` as it does for `desktop`. It needs new BLD requirements in the design specification first (task R8b says so). (c) The formula: a template in this repository with the version and the two SHA-256 values, and a job that pushes it to `TheArctesian/homebrew-anima`. That job needs the tap repository and a credential that can write only to it, in an environment of its own; the operator makes both. (d) HT-041 for the first `brew install`.
2. **The iOS application to TestFlight (task B8i).** A job on a macOS machine of GitHub with an environment of its own for the six iOS secrets (they are in `release` today, R-204): it builds the phone shell, signs with the distribution certificate and the profile, and uploads with the App Store Connect key. R-158 said a script on the Mac of the operator; R-203 moves it to the workflow. The first public TestFlight build waits for the sign-in of the phone application (R-158). Open before it is built: the build number (N), the phone tasks M3 and M5 of [mobile-plan.md](mobile-plan.md) that give a shell that builds, and the review of Apple, which no workflow can do.

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
| R8a | The crate `anima-cli` is publishable (INS-011, INS-012; R-151). One PR. Details: [Tasks R8a to R8d](#tasks-r8a-to-r8d-the-command-line-alone-by-homebrew-and-cargo). | — | `cargo package --list -p anima-cli` lists the schema and no test file. A check fails when the crate version is not the release version. Nothing is published. | Built in part on the branch `release/cli-homebrew`: parts (1), (2) and (4), and acceptance (a), (b), (c), (e) and (f). The package had a fault that only a build of the package alone shows: it used `tokio::fs` with no `fs` feature of its own. Not built: part (3), the version check, which waits for OD-W47; part (5), the build on a clean Linux computer. The entry for R8d is HT-041. | The commits of that branch |
| R8b | The release publishes a standalone `anima` file for each target, with its SHA-256 in the release record (INS-010; OD-24). One PR. | R7; R8a; the operator (OD-24) | macOS universal has the file and its SHA-256 in the record. The Linux x86_64 file waits for the Linux release of R7. Linux aarch64 is not planned. | Not started. Planned only. Blocked until OD-24 is ruled. | — |
| R8c | The Homebrew formula and its tap (INS-010; OD-24, OD-26). One PR, in the tap repository and one line in this repository. | R8b; the operator (OD-24, OD-26) | `brew install` of the formula installs `anima` on a clean Mac, and `anima --version` prints the release version. The Linux check waits for R8b. | Not started. Planned only. Blocked until OD-24 and OD-26 are ruled. | — |
| R8d | The operator publishes the first formula and the first crate version (INS-012; OD-24, OD-26). A human task, with an entry in [need-human-testing.md](../need-human-testing.md). | R8a; R8b; R8c | `cargo install anima-cli --locked` and `brew install` print the release version. The operator sets the entry to Passed. | Not started. Planned only. The agent publishes nothing. | — |
| R9 | The bucket keeps the 3 newest published versions of each target (BLD-086). The code, its migration and its tests are on the branch `builds/keep-three-versions`, which is not merged. That branch is older than the move of the documents: it changes `docs/design-spec.md`, `docs/programme/programme.md` and `docs/releases.md`, which are now `docs/spec/design-spec.md`, `docs/rulings.md` and `docs/procedures/releases.md`. Bring it to `main`: merge `origin/main` into it, put BLD-086 in Section 3.10 of the design specification, give the migration a number that is free, and give its ruling the next free number (the branch calls it R-118, which a different ruling has; PLN-021). The branch gives the operator's words of 2026-11-12 for the ruling; they are not on `main`. Source of this row: [release-four-platforms.md](../proposals/release-four-platforms.md), RLS-007. | — | `bun run gates` passes. The pruning tests of the branch pass. After the next release of a target that has 4 versions, the bucket holds 3. | Not started. Built on the branch, not merged. | — |
| R10 | The release has a target for the Android application. The hub checks the size, the SHA-256 and the signing certificate of the APK, writes it under `<target>/<version>/`, copies the stable name and writes the feed of the target (RLS-006, RLS-012 of [release-four-platforms.md](../proposals/release-four-platforms.md); as BLD-060 to BLD-066). It needs new BLD requirements in the design specification Section 3.10 first. Hub API, then `anima`, then MCP (CMD-030). | R-158 names it (OD-REL-4) and approves RLS-006 and RLS-012. The new BLD requirements go in the design specification in the first commit of the task (PLN-013). The check of a real APK needs task M5-5 of [mobile-plan.md](mobile-plan.md); the hub side can be built before it, with a test APK that the test signs | An APK that a different key signed is refused. An APK of the release key is at its stable name, and its feed gives the version. | Not started | — |
| R11 | Find out if one computer can build for a hub that is not the hub of its sign-in: the desktop application as a builder for the release server (R-158) while the application is signed in to the hosted hub, and the same for `anima-daemon` (PRO-008, BLD-021 to BLD-024, BLD-029). Read the code, then try it with two local hubs. Build nothing. Write the result in this plan, below "The automatic release of Anima", and in the row OD-W30 of [decisions.md](../plan/decisions.md). | — | The result names the files and the test that show it. If the answer is no, the result names the smallest change, as a new task row. | Done, on the branch `release/runner-path` (2026-11-18; not on `main`). The hub: yes. The command line: yes for the sign-ins and the enrolment, and it has no builder loop. The desktop application: no, it builds only for the hub of its sign-in (from the code, not run). The daemon: one hub for one daemon (from the code). The result is in "The result of task R11"; the record is [r11-two-hubs.md](../research/builds/r11-two-hubs.md). The smallest change is task R14. | The commit that adds this text |
| R12 | The start rule of a release. The hub that can publish reads the Anima repository at a slow interval and, when it finds the version mark that OD-W26 decides on a commit that is an ancestor of `origin/main` (BLD-041) with a version that is more than each published version (BLD-002), it queues the release for each target that has a builder. A setting of the hub sets the rule on; it is off by default. The hub does not start a release two times for one mark (PRO-021). It writes the start to the audit log with the actor kind of the hub (BLD-082, LOG-012), and it sends a failed release to the notification channels. New BLD requirements go in the design specification Section 3.10 in the first commit (PLN-013), with the changes of BLD-001, UPD-007 and OD-12 that the ruling gives. Hub API, then `anima release`, then MCP, then the Settings row (CMD-030). | The rulings of OD-W26 and OD-W27. R11. | A test: a mark on a commit of `main` makes one release, and a second read makes no second one. A mark on a commit that is not on `main` makes none. A version that is not more than the published one makes none. With the setting off, nothing starts. | Not started. Not on the path of R-170: OD-W27 selected the runner path, where a workflow file reads the mark and GitHub is the scheduler (PRO-047). The row stays, because no ruling cancels it and OD-W26 can still select a start rule of the hub. Do not build it. | — |
| WIN1 | Add a native Windows x86_64 desktop client, a local installer and a repeatable build procedure. Port the OS boundary, keep credentials in Windows Credential Manager, and expose only implemented host capabilities. Build and test in the operator's Windows VM. Operator instruction, 2026-11-20: "Add native Windows support". Serves work plan row 4.1. | The operator authorizes this task in this session; no public release is requested | The Windows installer builds, the installed application starts, and platform tests pass. Record remaining manual checks in need-human-testing.md | Built on branch; Windows installer, 131 shell tests and four link security/socket tests run in the VM. Awaiting [HT-NEW-windows](../need-human-testing.md#ht-new-windows-the-native-windows-desktop-client-win1) and Mac regression checks; the Linux workspace gate stopped on a hub account-sync test. [Build and test record](../procedures/setup/windows.md) | platform/windows-desktop |
| WIN2 | Native Windows feature parity with the macOS/Linux application: CLI and MCP setup, persistent device link and daemon, power controls, local hub with native dependencies, builder and Windows update packaging. Reuse shared domain code and port OS boundaries; do not require WSL. Operator, 2026-11-20: "all the same features as we have on mac and linux"; expressed concern about WSL. Serves work plan row 4.1. | WIN1; implementation authorized in this session, no public release or cloud change authorized | Inventory each desktop capability and command; Windows implementations and packaging pass platform tests, and every real-machine check has an entry in need-human-testing.md. No capability is enabled before its implementation exists | In progress: native CLI cross-build and Windows security/console boundaries implemented; runtime checks and distribution remain in progress. The VM has an unresolved Defender termination of the WIN1 link child; see the Windows build record. No additional desktop capability enabled | platform/windows-desktop |
| R14 | The desktop builder can build for a hub that is not the hub of its sign-in (result of task R11): `provideDesktopBuilder` takes a hub origin of its own; the Settings row "Build releases on this computer" names that hub and asks for one sign-in there for the enrolment (BLD-021). The key store already keeps the builder token by hub origin. | A need: the operator keeps the Builds path of today while he uses a different hub each day. R-174 selected choice C of OD-W30, so the runner path does not need it. A change of BLD-020, BLD-021 and BLD-026 in the design specification first (PLN-013) | A test: the application is signed in to hub A, and its builder claims and reports at hub B. | Not started. No ruling names it, and R-174 makes it not necessary for the runner path. Do not build it. | — |
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
| W0 | The daemon talks to the link over a Unix socket that only it can open, in place of the local port `http://127.0.0.1:<port>` (R-172; SF-004; K8 Section 1). Owners: K8 ([daemon-push.md](../../contracts/daemon-push.md)) for the transport of the daemon, and LNK-008 of [s1-link-tasks.md](one-click/s1-link-tasks.md) for the sockets of the link. First commit: the new text of K8 Section 1 (R-172 is its ruling, CON-001): the path of the socket, its owner and its mode for a user installation and for a system installation on Linux, where the daemon has a user of its own; how the daemon knows that the other end is the link; and what an installed daemon with the old origin does after its update (ROL-018). Then `anima-link device` (Go, `link/`) makes the socket, the daemon (`crates/daemon`, `config/origin.rs`) uses it and accepts no `http` origin on a port, and the installation writes the new origin. It adds no function of the hub API, so no command and no MCP tool (CMD-030). | — | A test: a program of a different user, and a program of the same user that is not the daemon where the system can tell them apart, cannot open the socket; the daemon sends no token to a TCP port; a daemon with the old origin still pushes after its update. An entry in [need-human-testing.md](../need-human-testing.md): a machine that reaches a hosted hub through the link still pushes and renews. Then SF-004 can be closed by the operator. | Built, not checked: [HT-037](../need-human-testing.md) is Open. Built (2026-11-18): the new text of K8 Section 1 and of MET-020; `anima-link device -daemon-socket <path>` makes a socket with mode 0600 and does not start when other users can reach its folder; the daemon's link origin is `unix:<path>`, it checks the folder and the socket before it makes its client, it sends nothing to a TCP port and refuses `--hub http://127.0.0.1:<port>`; a daemon with the old origin keeps its files (ROL-018) and uses `link.sock` in its own folder. Different from this row: no installation writes the new origin, because no installation writes a link origin today (the desktop application enrols the daemon only at an `https` origin, and its own link has no socket for the daemon). Who starts the link of the daemon on a computer of a person is HM-D16 of [hosted-mode.md](../proposals/hosted-mode.md), which is open; thus HT-037 uses a hub that the operator starts in the link mode, not a hosted hub. A program of the same user as the daemon can open the socket: the system cannot tell it from the daemon, and K8 says so. Not Done before HT-037 is Passed. | Branch `daemon/w0-link-socket` |
| W1 | The contract of the work route: claim, report, log, stop, a secret for one job, the work types `build` and `runner`. It states that a job reads the local copy that Anima holds and does not call the service (PRO-006, Q3). | R-160 (2026-11-18) is the ruling that changes R-115 ([jobs.md](../proposals/jobs.md), "Needs"); this task no longer waits for a ruling. The contract and the security review of the work route come before code | The programme accepts the contract. | In progress. The contract is written as a proposal: [work-route.md](../../contracts/work-route.md) (K10, version 0.3). Its security review is done as a separate pass; the twelve findings are SF-003 to SF-014 of [security-findings.md](../security-findings.md), four of severity High. Version 0.2 has the changes that the review asked for. The operator decided the four findings that needed him: SF-004 (R-172, task W0), SF-003 (R-185: a list of repositories at each machine; a machine with no list refuses), SF-006 (R-186: the device list now, the device key as task W7) and SF-013 (R-187: Accepted). Version 0.3 has those answers. SF-013 is Accepted; the others stay Open until code exists. The operator accepted the contract, version 0.3, on 2026-11-19 (R-209), with the sentence of its Section 14; each open finding keeps the state of [security-findings.md](../security-findings.md). Done (R-209). The status line of the contract and the one sentence of K8 are changed. The container work types are a later version 0.4, after their own security review (task H14 of [hosted-applications-plan.md](hosted-applications-plan.md)). | R-209; branches `release/runner-path`, `contracts/k10-accepted` |
| W2 | The isolation design, with prototypes: the container runtime on Linux, a workflow that needs Docker, the build user on a Mac, a signing identity for one job (PRO-029). | — | A record with measured results on one Linux machine and one Mac. The security reviewer and the operator accept it. | In progress. The record is [w2-isolation.md](../research/work-isolation/w2-isolation.md), with a prototype of throwaway scripts in the same folder and their logs; it is not code of the product. Measured on one Mac (2026-11-18, under a high load): a sandbox profile on the user of the operator hides the files but leaves the Docker socket of the computer usable, so it does not meet PRO-022 alone; a keychain for one job signs with no change of the search list and is gone after its deletion (each step 10 to 38 ms); a container job with no capability starts in a median of 349 ms and leaves nothing; a Docker service of its own for a job works with no privileged container: the rootless service, with three protections of its container set off, starts in about 1 second and uses 60 MiB (measured again on the quiet machine). The design is Section 4 of the record: a proposal. Not measured: Linux, a real build user, a virtual machine for each job, a Developer ID signature. The Linux measurements are made on one of the operator's own machines, not on the server of the pool and not on a new server (R-188: "B: my own Linux box"); he has not named the machine, and that is the one open question of this task (OD-W39). The build user needs an administrator on a Mac. Not Done before the measurements and the two acceptances. | Branch `release/runner-path` |
| W3 | The hub: one queue for work, the placement function, the log store (PRO-050, PRO-051). Hub API, then `anima`, then MCP. It is built in two parts. Part 1, the routes of the accepted contract K10 (R-209) and what is behind them: the domain `work/jobs` with the schema `jobs` (the jobs, their attempts, their log, the state of each machine for work); the lease rules of the build queue moved to one shared place and used by the two queues, not written a second time (PRO-050; the builds become jobs of this queue in task A2, and until then the build queue of `fleet/builds/` is not changed in what it does); the pure placement function (PRO-051; the time is a parameter); `POST /work/v1/claim` and `/work/v1/report` with the credential, the limits and the refusals of K8; `POST /work/v1/secret`, which answers `SECRET_REFUSED` to each request, because no purpose has a source before tasks B2 and A2; the end of a lease and of an attempt that is too long in the upkeep; the audit entries and the events of K10 Section 10; one function of the hub that puts a job in the queue, for tasks B4 and A2. Part 2, what a person and an API key use: the list of the jobs with their log, the stop of a job, "takes work" off for a machine in the hub, and the devices that get jobs with secrets for a project (R-186). Part 2 adds root fields to K1 (`contracts/schema.graphql`), which changes only by a ruling (CON-001): the proposed fields are OD-W50. The `anima` command and the MCP tool come with part 2 (CMD-030): part 1 adds no function that a person calls. | W1 | The gates pass. The release of Anima still works through the old path. | In progress. Part 1 is built on the branch `hub/w3-work-queue` (2026-11-19): migration 0980 (schema `jobs`; the range is recorded in `contracts/database.md`), the domain `crates/hub/src/work/jobs/`, the lease rules in `crates/hub/src/clock/lease.rs` for the two queues, the three routes in `crates/hub/src/fleet/metrics/ingest/work.rs`, the sweep in the upkeep, the bodies and the three refusal codes in `daemon-core`. Tests: 24 of the rules and 12 through HTTP with a database. Two things that part 1 did not build and that K10 asks of the hub later: the hub does not yet stop a waiting `runner` job before a rollout (K10 Section 12; no rollout exists, task U6), and no secret is given (tasks B2 and A2). Part 2 waits for OD-W50. Part 3 (R-259, 2026-11-19: "A: keep the queue, add reasons and pins"; Not started): (a) a reason for each placement decision. The placement function gives, for a machine that asks and for each job that it looked at, the requirement that failed or "fits"; the hub stores the last decision of each job for each machine and builds one line from them, as "studio: has cargo 1.83 and 22 GB free; laptop refused: 2 GB free"; the line is data (MCP-008). (b) An audit entry for each decision that gives a job to a machine, with the reason and each machine that was refused (ROU-005); no secret and no prompt. (c) Pins and exclusions (ROU-006): a job can name one machine, and then only that machine gets it and no different machine ever does; the operator can pin a machine for a project and can exclude a machine from work that names no machine; an excluded machine still takes work that names it. (d) The free memory as a tie-break only: for 20 seconds after a job came into the queue, a machine that asks does not get it when a different machine that fits, and whose last claim is at most 30 seconds old, said that it has more free memory; after 20 seconds the first machine that asks and fits gets it. The time is a parameter; a test proves that the second machine gets the job inside the 20 seconds and that the first gets it after them. For this the hub keeps what the last claim of each machine said (types, kinds, system, free). (e) When no machine fits, the job waits (JOB-014 as changed); a job that names a machine that cannot fit is refused at once with the reasons. The list of queued work with its reasons is the one list of task W6, with the fields of OD-W50; there is no second list and no `anima routing log` of its own. Part 3 changes no contract: K10 Section 2 stays (the hub never opens a connection to a machine). Not Done before parts 2 and 3. | Branch `hub/w3-work-queue` |
| W4 | The daemon: do a container job and a native job on a Mac, and delete each thing at the end (PRO-029). A test proves that a job reads the local copy and does not call the service (PRO-006, Q3). | W1, W2 | A test proves that a job cannot read the daemon token or the home folder, and that nothing stays after a stop. A test proves that a machine with no list of repositories refuses each item that names a repository, and that a machine with a list refuses a repository that is not in it (R-185, SF-003). | Not started | — |
| W5 | "Takes work" for each machine, and "native jobs" for each Mac (USE-004, USE-008, PRO-024). | W3 | A machine that is set off at the machine refuses work from the hub. | Not started | — |
| W6 | The list of what runs, in the hub and in the client (PRO-026, PRO-052). | W3 | The area shows a build and a session, and shows nothing when nothing runs. | Not started | — |
| W7 | The key of a device that signs each claim and each request for a secret, so that a daemon token alone gets no secret (R-186; SF-006; point W1-P4 of contract K10). First: the change of K10, with a ruling (CON-001): where the key is made and kept on each system, how the hub learns its public part at the enrolment, what is signed, and what a daemon without a key gets. Then the hub and the daemon. It adds no function for a person, so no command and no MCP tool (CMD-030), unless the change of the contract adds one. | W3, W4. It must be done before a machine that the operator does not own alone takes work (R-186) | A test: a request with a correct daemon token and no signature, or a signature of a different device, gets no secret and no item with a secret. | Not started | — |

Group B: runners.

| Task | Content | Needs | Done when | State | Commits |
| --- | --- | --- | --- | --- | --- |
| B1 | Prove the facts that the documentation does not state, on a test repository of the operator: the labels of a JIT runner (F12), the life of an unused JIT configuration (F6), `vars` in `runs-on` for a branch and for a fork (F13), the memory of one idle runner. | The operator's account. The steps are in "Task B1: the steps for the operator" below this table, with one more fact for OD-W31 (the identity token of a job) and one for the contract K10 (how the runner takes the JIT configuration) | Each result is in the table "Facts of GitHub". | Not started. The steps are written (2026-11-18); the operator does them. | — |
| B2 | The GitHub provider: make a JIT configuration, remove a runner, list runners, read queued jobs, read the fork settings (PRO-049). | A contract change for the new `Change` values (K-E1 Section 13) | Each function has a test with a recorded answer. The JIT configuration is not in a log or in `Debug` output. | Not started | — |
| B3 | The daemon, work type `runner`: get the program, check it, run it in isolation for one job, delete it (PRO-039, PRO-043). | W4, B1 | A workflow of the test repository runs on a Linux machine and on a Mac. Nothing stays on the machine. | Not started | — |
| B4 | The hub: runners on and off for a project, warm mode, the next runner after each job, the audit entries (PRO-042, PRO-046). Hub API, `anima runner`, MCP tools (PRO-053). | W3, B2 | With runners on, two jobs in sequence each get a new runner. With runners off, GitHub lists no runner of Anima. | Not started | — |
| B5 | The set-up in the client, and the pull request that changes `runs-on` (PRO-044, PRO-045). | B4 | From a project page, one press and one merge make a workflow run on an owned machine. Use a private test repository: `TheArctesian/anima.ing` gets no runner before W2 and S are accepted (OD-W18). | Not started | — |
| B6 | On-demand mode (PRO-042). | The webhook receiver (WHK); B4 | With no idle runner, a queued job gets a runner. A lost delivery is repaired by the poll. | Not started | — |
| B7 | Runners for an organisation (PRO-048). | A need of the operator; a new permission | Parked. | Not started | — |
| B9 | The set-up for a coding agent, and the text to copy (PRO-054 to PRO-058; R-170). The section for runners on the project page with each step and its state; the query, the command `anima runner setup` and its read tool; the refusals of PRO-057 with their sentences; the text of PRO-058 from one source, with `anima runner prompt` and the "Copy" control; the test of the text. Hub API, then `anima`, then MCP, then the client (CMD-030). The mockup of the project page changes in the same commit (PLN-019). | B4, B5. The approval of PRO-054 to PRO-058 (R-170 gives the direction only) | A coding agent that gets only the text and the MCP server brings a private test repository to the merge that the person makes, and says which steps the person must do. The test of the text passes. An entry in [need-human-testing.md](../need-human-testing.md), which the operator sets. | Not started | — |
| B8 | The release of Anima as a workflow of GitHub Actions on an owned machine (model (b)). A workflow file that runs only for the version mark of OD-W26 on `main` and never for a pull request; a hub function that takes the bundle of a runner job, with the checks of BLD-060 to BLD-066 before the hub signs; the signing identity of the Mac for one job (W2). The updater key and the R2 token do not go to GitHub (R-114, R-117). It needs new BLD requirements in the design specification first, and the changes of UPD-007, BLD-029 and Section 3.10 that the ruling gives. | R-171 decided OD-W28, R-173 decided OD-W31 (the job signs; the updater key is a secret of GitHub; it amends R-117 and R-171) and R-174 decided OD-W30 (the hosted hub of the operator holds the project and the runners). The rulings of OD-W29, OD-W32, OD-W33 (who uploads and writes the feed, and the checks), OD-W35 and OD-W36 (the protection of the key). W0. B5. S. OD-W18: no runner for `TheArctesian/anima.ing` before W2 and S are accepted. The hub that signs: task U12, or the VPS hub while it signs (R-158). OD-W26 only for the start with no act of the operator: until it is decided, the operator starts the release in Anima and the hub starts the workflow (PRO-020, BLD-001). | A release that the operator starts makes one workflow run on an owned machine, and the hub publishes its files. With the ruling of OD-W26, a mark on `main` does the same with no act in Anima (task R13). A pull request from a fork cannot start the workflow and cannot get the signing identity: a test for each claim. | Not started. R-170 and R-171 put it on the path. Blocked until OD-W29, OD-W32, OD-W33, OD-W35 and OD-W36 are decided and its tasks are done. The sentences of this row on a hub function that takes the bundle and on the key that does not go to GitHub are the text before R-173; the first commit of the task writes the row again from the rulings. | — |
| B8a | The plan and the rulings of the release workflow on the runners of GitHub: R-179, R-180, the section "The release workflow on the runners of GitHub", the rows B8b to B8e, the decision OD-W38, and what is built in the rows OD-W33, OD-W34 and OD-W36. | The answers of the operator of 2026-11-18 | The documents gate passes. | Done; on `main` since 3ae44093. | The commit that adds this row |
| B8b | `anima-hub release sign`, `check` and `publish`: the checks of the hub for one target with no queue and no database (BLD-002, BLD-045, BLD-046, BLD-064 to BLD-069), from the functions of `fleet/builds/publishing/`. | R-179. R-173 is on `main`. The merge waits for the operator | The gates pass. Tests against the fake bucket: a release is published with the feed last; a lower version, a changed file, a wrong signature and a file that the build job did not report are each refused, and the feed stays. | Built; on `main` since 3ae44093. HT-036 is open. | The commits of that branch |
| B8c | `.github/workflows/release.yml` with its two scripts (`trial-bundle.sh` takes the signing identity and `ANIMA_RELEASE_BUILD`, which alone gives the application its updater, DEF-001; `check-mach-o.sh`) and the test of its rules (`deploy/test/release-workflow.test.ts`): the triggers, the pinned actions, the permissions, which job has which environment. | B8b. R-179, R-180. R-173 is on `main`. The merge waits for the operator: a workflow file on `main` can run on GitHub | The gates pass, `actionlint` reports nothing, and the rule test passes. The workflow has not run: HT-036 of [need-human-testing.md](../need-human-testing.md) is the check, and the operator sets it. | Built; on `main` since 3ae44093. The trial run of 2026-11-18 is in HT-036, which is open. | The commits of that branch |
| B8d | The operator sets GitHub (three environments, the secrets, the tag rule), runs a trial, and makes the first release from a tag. The steps are in [releases.md](../procedures/releases.md), "The release workflow". | B8c on `main`. OD-W26 (the tag), OD-W32 (the gates), OD-W33 (the workflow publishes), OD-W36 (the protection of the key, with the three environments), OD-W38 (the Linux feed). `main` must pass the gates. The correction of DEF-001 for the check of the update | A trial run passes and publishes nothing. A tag publishes the three feeds; `stapler validate` and `spctl` accept the application; an installed 0.2.1 updates. HT-036 is Passed. | In progress. GitHub is set (R-182, R-195; [releases.md](../procedures/releases.md), "The state of GitHub"), and a trial ran on 2026-11-18 (HT-036). The first public release waits for the check of the AppImage on Linux (R-192); its version is that of its commit (R-183), not 0.2.2. | — |
| B8e | After the rulings of OD-W26, OD-W32 and OD-W33: put the proposed text of the table "The requirements that the workflow changes" in the design specification; make one owner of the bundle recipe and of the Mach-O check (today the builder in Rust and two scripts hold the same commands); add the Android target (task R10). Later, with B8: change `runs-on` to a runner of the operator. | The rulings of OD-W26, OD-W32 and OD-W33. B8d for the last part | The design specification says what the workflow does. One file holds each recipe. | Not started | — |
| B8f | `deploy/bin/commit-version`: prints the version of a commit, `0.<N div 100>.<N mod 100>` (R-183). The job `commit` of the workflow refuses a tag that does not give the version of its commit; `trial-bundle.sh` with no version uses it. | R-183 | The gates pass. Tests: the formula, an older commit keeps its version, a shallow clone and a name that is no commit are refused. | Built; on `main` since 9da268f4. Task B8g changed the count to the first line (R-201). | The commit that adds this row |
| B8g | The trigger of R-184: a push to `main` releases the applications that it changed. `deploy/release/applications.txt` and `deploy/bin/affected-applications` (the map), `deploy/bin/release-decision` (the comparison with the published version, the refusal of a version that is not higher and of a push that does not keep the first line of `main`), the switch `ANIMA_PUBLIC_RELEASES`, the concurrency group of a trial, and `commit-version` with the first line only (R-201). The branch `dev` in AGENTS.md. | R-184, R-198 to R-202. B8f | The gates pass. Tests of the map, of the decision and of the rules of the workflow file. HT-036 has the steps. | Built on the branch `release/each-push`. No run of the trigger was on GitHub; public releases are off until the operator sets the switch. | The commits of that branch |
| B8h | `anima` alone by Homebrew as a job of the release workflow (R-203). The plan is in "The next two jobs". | Tasks R8a to R8c. The map from `cargo metadata`. A credential for the tap, set by the operator | The gates pass. A push that changes only the client does not release `anima`. A human check of `brew install` | Built on the branch `release/cli-job`, not run: the map and task R8a (earlier); `deploy/bin/cli-formula` and `deploy/bin/crate-version` with tests; the jobs `cli` (the two files and the formula, no secret, also in a trial), `cli-publish`, `tap` and `crate`, each only while `ANIMA_PUBLIC_RELEASES` and `ANIMA_PUBLIC_CLI` are on. The files are below `anima/<version>/` of the bucket; they are put with the `aws` program of the machine and not with `anima-hub release`, which has no such target. Open: the steps of the operator in [releases.md](../procedures/releases.md) (the key of the tap, the token of crates.io, two environments, the switch); HT-041. Tasks R8b, R8c and R8d are done by this task in the form of R-217 to R-219, except their human check. | The commits of that branch |
| B8i | The iOS application to TestFlight as a job of the release workflow (R-203). The plan is in "The next two jobs". | B8h. The phone shell builds (tasks M3, M5 of [mobile-plan.md](mobile-plan.md)). An environment of its own for the iOS secrets, set by the operator. The first public build waits for the sign-in (R-158) | The gates pass. A human check of the build in TestFlight | Built on the branch `release/ios-job`, not run: the map entry `ios`; the jobs `ios` (build and sign, environment `release-ios`), `ios-upload` (environment `release-testflight`) and `ios-record`; the switches `ANIMA_IOS_BUILD` and `ANIMA_PUBLIC_TESTFLIGHT`, which do not depend on `ANIMA_PUBLIC_RELEASES`. It calls `apps/desktop/src-tauri/ios-archive.sh <version>` with `IOS_SIGN_IDENTITY` and `IOS_PROFILE`, the interface that the mobile session proposed for task M5-4 on 2026-11-19 (branch `mobile/account-sign-in`, not merged; its signed mode is not tested). That session names what the shell still lacks for a real upload: the iOS icon set, the export-compliance key in `Info.plist`, and the key store (M5-1). Open: the steps of the operator in [releases.md](../procedures/releases.md); HT-046. | The commits of that branch |

### Task B1: the steps for the operator

An agent does not use the GitHub account of the operator, and it did not use the `gh` program of this computer for this task. These are the steps that the operator does. They take about 1 hour of work, and then three short looks in the next 2 days. `<you>` is the name of the personal account. Each command is `gh`, signed in as the owner.

Rules for these steps:

- Never paste an `encoded_jit_config`, a token or the content of a file `cfg-*` into a chat, a document or an issue. A JIT configuration is a secret (PRO-040). The commands below keep it in a file with the mode 600 and print only the fields that are not secret.
- No runner is registered for `TheArctesian/anima.ing` (OD-W18). Use only the two test repositories.
- The runner program runs on the Mac as the operator, in a folder that he deletes at the end. That is acceptable only because the one workflow of the test repository is the file below, which he writes himself. Do not start it for a different repository.

What each step proves:

| Step | Fact | Why the plan needs it |
| --- | --- | --- |
| 1 | The labels of a JIT runner (F12), and the runner group of a personal account (F5) | PRO-041: which labels the hub must send |
| 2 | The life of a JIT configuration that no runner used (F6) | PRO-042: how long a registration can wait before a machine starts it; Section 6.2 of the contract K10 |
| 3 | How the runner program takes the JIT configuration with no command line (W1-P2 of K10) | PRO-040: a command line can be read by each user of the machine |
| 4 | The memory and the disk of one idle runner | PRO-042: the cost of warm mode, one idle runner for each repository |
| 5 | `vars` in `runs-on`, for a branch (F13) | "One press": on and off with no second merge |
| 6 | `vars` in `runs-on`, for a pull request from a fork (F13, F20) | PRO-044: what the author of a fork can select |
| 7 | The fields of the identity token of a job on a self-hosted runner | Only if OD-W33 gets choice B (a hub takes the files): what that hub can check. With R-173 the job signs, so this step can wait for OD-W33 |
| 8 | Clean-up | Nothing stays |

Step 0. Make the test repository, private, and a work folder.

```sh
gh repo create <you>/anima-runner-test --private --add-readme
mkdir -m 700 ~/b1 && cd ~/b1
```

Step 1. Make a JIT configuration and read the labels that GitHub gave. The command keeps the configuration in a file and prints only the id and the labels.

```sh
umask 077
gh api -X POST repos/<you>/anima-runner-test/actions/runners/generate-jitconfig \
  -f name=b1-a -F runner_group_id=1 -f 'labels[]=anima-b1' > raw-a.json
jq -r .encoded_jit_config raw-a.json > cfg-a && jq '{id: .runner.id, labels: [.runner.labels[].name]}' raw-a.json && rm raw-a.json
gh api repos/<you>/anima-runner-test/actions/runners --jq '.runners[] | {id, name, status, labels: [.labels[].name]}'
```

Send back: the two printed objects. If `runner_group_id=1` is refused, send the error text. The question: did GitHub add `self-hosted`, the operating system and the architecture, or only `anima-b1`?

Step 2. Make three more configurations at the same time and do not start them: `b1-1h`, `b1-6h`, `b1-24h` (the same commands, with `name=b1-1h` and the file `cfg-1h`, and so on). Write the time. Leave `b1-a` unused for the whole test.

- After 1 hour, after 6 hours and after 24 hours: run the list command of step 1, and start the configuration of that time as step 3 says. Send back for each time: is the runner in the list, and did the runner program start with it or refuse it (the text of the refusal).
- After 48 hours: run the list command one more time. Send back: is `b1-a` still in the list.

Step 3. Get the runner program and start it with a configuration. Take the newest version and its SHA-256 from the release page of `actions/runner`; the page gives both.

```sh
V=<version>; curl -fL -o runner.tar.gz https://github.com/actions/runner/releases/download/v$V/actions-runner-osx-arm64-$V.tar.gz
shasum -a 256 runner.tar.gz        # compare with the release page before the next line
mkdir r1 && tar -xzf runner.tar.gz -C r1 && du -sh r1
```

First try with no command line argument, in a second terminal window so that the first one stays free:

```sh
cd ~/b1/r1 && ACTIONS_RUNNER_INPUT_JITCONFIG="$(cat ../cfg-1h)" ./run.sh
```

If the program asks for a configuration or stops, start it with the argument: `./run.sh --jitconfig "$(cat ../cfg-1h)"`. Send back: which of the two forms started the runner, and the first 5 lines that it printed. While it runs with the argument form, run `ps -axo command | grep -c jitconfig` in the first window and send back the number: it shows if the configuration is in the list of processes.

Each runner does one job and then stops. For each later start, make a new copy of the folder (`mkdir r2 && tar -xzf runner.tar.gz -C r2`).

Step 4. While a runner waits for a job and no job is queued, wait 5 minutes, then:

```sh
ps -axo rss,command | grep -i '[R]unner\.' | awk '{sum+=$1} END {print sum/1024 " MiB"}'
```

Send back: the number, and the result of `du -sh r1`.

Step 5. Add the workflow and the variable. The workflow names the runner by a variable and falls back to a runner of GitHub.

```sh
gh variable set RUNNER_LABEL --repo <you>/anima-runner-test --body anima-b1
```

The file `.github/workflows/b1.yml` of the test repository:

```yaml
name: b1
on: [push, pull_request, workflow_dispatch]
permissions:
  contents: read
  id-token: write
jobs:
  where:
    runs-on: ${{ vars.RUNNER_LABEL || 'ubuntu-latest' }}
    steps:
      - run: echo "runner=${{ runner.name }} environment=${{ runner.environment }} label=${{ vars.RUNNER_LABEL }}"
      - name: identity token fields
        run: |
          curl -sS -H "Authorization: bearer $ACTIONS_ID_TOKEN_REQUEST_TOKEN" "$ACTIONS_ID_TOKEN_REQUEST_URL&audience=anima-b1" \
            | jq -r .value | cut -d. -f2 | tr '_-' '/+' | awk '{ while (length($0) % 4) $0 = $0 "="; print }' | base64 -d \
            | jq '{iss, aud, repository, repository_visibility, ref, sha, event_name, workflow_ref, job_workflow_ref, runner_environment, actor}'
```

The second step prints only named fields of the token, not the token. Commit the file on a branch `b1-branch` and push it. Then:

```sh
gh run list --repo <you>/anima-runner-test --limit 3
gh api repos/<you>/anima-runner-test/actions/runs/<run id>/jobs --jq '.jobs[] | {status, labels, runner_name}'
```

Send back: the `labels` of the job while it is queued. Then start a runner with a new configuration that has the label `anima-b1` (steps 1 and 3) and send back: the `runner_name` of the job after it ran, and the two printed lines of its log (`gh run view <run id> --log`). Then delete the variable (`gh variable delete RUNNER_LABEL --repo <you>/anima-runner-test`), start the workflow by hand (`gh workflow run b1.yml --repo <you>/anima-runner-test --ref b1-branch`) and send back the `labels` and the `runner_name` of that job: it must be a runner of GitHub.

Step 6. The fork. A personal account cannot fork its own repository, so this step needs a second account, or an organisation that the operator owns. If he has none, he says so and the fact stays not proved. No runner is started in this step: the question is only which labels the job asks for.

- Make a second test repository that is public, `<you>/anima-runner-test-public`, with the same workflow file on its default branch and the same variable `RUNNER_LABEL`. Register no runner for it.
- From the second account, fork it, change one line of the README on a branch, and open a pull request to `<you>/anima-runner-test-public`. Approve the run if GitHub asks (F18).
- Run the two commands of step 5 for that repository. Send back: the `labels` of the queued job. `["anima-b1"]` means that a workflow of a fork pull request reads `vars`. `["ubuntu-latest"]` means that it gets an empty text. If the job ran on a runner of GitHub, send back also what the step "identity token fields" printed, or its error: the documentation says that a fork pull request gets no token with the right to ask for one.
- Cancel the run if it waits (`gh run cancel <run id> --repo <you>/anima-runner-test-public`).

Step 7. The identity token on a self-hosted runner is the second printed object of the run of step 5 that ran on the Mac. Send back that object, and the same object of the run that used a runner of GitHub. The question: are `repository`, `ref`, `sha`, `event_name`, `job_workflow_ref` and `runner_environment` present on a self-hosted runner, and what `runner_environment` says there.

Step 8. Clean up.

```sh
gh api repos/<you>/anima-runner-test/actions/runners --jq '.runners[].id' | xargs -I{} gh api -X DELETE repos/<you>/anima-runner-test/actions/runners/{}
rm -rf ~/b1
gh repo delete <you>/anima-runner-test --yes && gh repo delete <you>/anima-runner-test-public --yes
```

The last line needs the scope `delete_repo`; to archive the two repositories in the settings page is also sufficient.

What the operator sends back, in one message: the answers of steps 1 to 7, as text. An agent then writes each result in the table "Facts of GitHub" (F5, F6, F12, F13), in W1-P2 of the contract K10 and in OD-W31, and sets task B1 to Done with that commit. A value in these steps that the documentation of GitHub does not state (the variable `ACTIONS_RUNNER_INPUT_JITCONFIG`, the group number 1, the names of the fields of the token) is a thing to prove, not a fact: if a command does not work as written, the error text is the result.

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
