# Proposal: projects first (import from GitHub, the builder as a job, sign-in)

From: a side chat with the operator on 2026-11-13, sent to the planning session.
Status: the six points in "Decision" are decided by the operator. The points in "Proposals to
confirm" are recommendations that the operator heard and did not refuse; they are not approved.
Nothing is built.
Map: [work-plan.md](../../work-plan.md), categories See and Use, decision OD-W11.
Identifiers: PRO-001 to PRO-019. The prefix `PRJ` belongs to the extensions plan and is not
changed here.

## Problem

Projects (XP-02, #13) is one extension in a list. But the rented accounts of the operator
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
  the hub that enrolled it (BLD-021, BLD-022 keep this).

## Proposals to confirm

The operator heard these and did not refuse them. Each needs an approval.

- **PRO-009. A build is a job.** The builder becomes the first job type of
  [jobs.md](jobs.md). Thus jobs come directly after Projects: Projects (See), then builds (Use).
  This agrees with OD-W8 (jobs before the extensions) and moves Projects before the other
  extensions. Fine-tuning (C7, #40) is then the second job type, not the first.
- **PRO-010. Anima schedules builds.** A GitHub App tells the hub about a push. The hub places
  the job on a machine. The result goes back to the commit as a status. Anima does not register
  machines as self-hosted runners of GitHub Actions. Reason: one coordinator (ETH-001), and the
  same path works for a different forge (PRO-007).
- **PRO-011. Native jobs on a Mac come early.** A macOS or iOS build cannot run in a container.
  See "Conflicts".
- **PRO-012. Keys belong to the project.** A signing key and a secret belong to a project, not
  to a builder. A builder gets them for one job (JOB-010, SCR-001).
- **PRO-013. Build only what is yours.** The default is: build only the repositories and the
  branches of the operator. Never build a pull request from a fork, because a build runs the
  code of the repository on an owned machine.
- **PRO-014. Read first, write later.** The import uses read access only. Write access (a commit
  status) comes with the builder, as a second grant. A sign-in to a source of data that is held
  about the operator stays read-only always (ETH-007).
- **PRO-015. Two different things with GitHub.** "Connect GitHub" makes a source. "Sign in with
  GitHub" is identity. They are separate.
- **PRO-016. Forge sign-in is an extra method, later.** GitHub, GitLab and Bitbucket can become
  sign-in methods, mainly for the hosted option ([other-operators.md](other-operators.md)). The
  local sign-in stays the root, so that a forge that is down or an account that is locked never
  stops access to the owned machines.

## Conflicts with text that exists

| Text | Says | This proposal |
| --- | --- | --- |
| PRJ-001 | "The operator must accept each project before Anima adds it." | The import takes each repository and the operator leaves some out (decision 2). PRJ-001 needs new wording. |
| JOB-001, USE-001 | A job is a container image. Jobs run in containers. | A macOS or iOS build needs the Mac directly. JOB-004 ("a later version can run a Nix recipe on a Mac without a container") must come early, and it cannot be only Nix: Nix does not supply Xcode or the macOS SDK. A native job needs its own limits, because it has no container around it (see "Open points"). |
| [jobs.md](jobs.md), "Fine-tuning becomes the first job type" | C7 is first | The build is first (PRO-009). |
| BLD-001 to BLD-082 | The builder builds the Anima desktop application, started from the Builds view | The same machinery, with a project as a parameter. The identifiers stay; the release of Anima becomes one project. |
| Design specification, sign-in (SEC-002, NAV-009, the `access` domain) | One sign-in: user name, password, authenticator code | Not changed now. PRO-016 adds methods later and keeps this one as the root. |
| Work plan tier 3b | "Projects comes early in that order" | Projects moves before the other extensions and gets its own row. |

## Rules that already apply

- **PRO-017.** Each function is built in this order: the hub API, the `anima` command, the MCP
  tool, the graphical client (CMD-030, [cli-mcp-plan.md](../../cli-mcp-plan.md) on the branch
  `claude/anima-cli`). Projects and builds must obey it. An MCP tool must not take or return a
  signing key or a provider token (MCP-004).
- **PRO-018.** The hub only reads a rented service (PRJ-003). An action uses the CLI tool of the
  service (OD-W1).
- **PRO-019.** A machine with `anima-agent` takes work by default, and the operator can set a
  machine off (OD-W10, USE-004). A build is such work.

## A fact about the builder today

The build agent fails the universal macOS build of the Tauri application with "can't find crate
for core ... x86_64-apple-darwin target may not be installed". The branch `main` has no
`rust-toolchain.toml` and no `flake.nix`. A `rust-toolchain.toml` that lists the two macOS targets
is the small correction. A Nix flake is the large one, and it cannot supply Xcode or the macOS
SDK. Nothing is changed. Put this on issue #1 or make a defect issue.

## Open points

- The limits of a native job on a Mac: which user runs it, what it can read, how the machine is
  made clean after it. A container gave these for free (USE-001); a native job does not.
- The GitHub App: who registers it, and its permissions. Each developer application is registered
  to the operator ([other-operators.md](other-operators.md)).
- Where a signing key is kept, and how a builder gets it for one job. The Developer ID and the
  updater key have rulings (R-116, R-117); read them before the design.
- The cost data of decision 1 ("cost"): which providers give cost through a read-only API. Not
  verified.
- Nothing here is verified against the documentation of a provider (as in
  [new-sources.md](new-sources.md)).

## Needs

- A ruling number in `programme.md` for OD-W11, and an approval for each of PRO-009 to PRO-016.
- A change to PRJ-001 and to Section 4.2 of the extensions plan (with #13 and #26).
- A change to [jobs.md](jobs.md): native jobs early, the build as the first job type.
- A security review of builds: they run repository code on owned machines and use signing keys.
- After approval: a checklist on #13 for the import, and one issue for builds as jobs with the
  labels `compute` and a priority label.
