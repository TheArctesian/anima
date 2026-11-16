# Proposal: update the agent and the hub on each machine (one press, and automatic)

Progress: the plan register in [work-plan.md](../../work-plan.md) Section 9 holds the state of this plan (PLN-005).

Version: 1. Date: 2026-11-16.
From: the operator's request of 2026-11-16 for two proposals: a watchdog
([watchdog.md](watchdog.md)), and the update of the agent and the hub.
Status: Draft. A proposal for a ruling. No requirement is approved. Nothing is built. The
operator decided its tier (OD-W23) and OD-ROL-8 on 2026-11-16. The other decisions are open.
Map: [work-plan.md](../../work-plan.md), category Foundation, decision OD-W23.
Identifiers: ROL-001 to ROL-052. The prefix `UPD` belongs to the desktop application (design
specification Section 3.9) and is not changed here.

## Problem

The desktop application updates itself (UPD-001 to UPD-012). No other program of Anima does.

- To update `anima-agent`, a person builds it from source on each machine and installs it again.
- To update the hub, a person builds its image on the VPS with Docker Compose.
- The hub does not know the version of an agent. The agent does not report it.

Other plans depend on an update that does not exist. USE-008 says that an agent "takes work
after its update". The watchdog reaches a machine only in a new agent. R-115 lists "signed agent
self-update with rollback after 3 failed starts" as later work with no specification. This
proposal is that specification.

## Terms

| Term | Meaning |
| --- | --- |
| Release | One version of each program of Anima, built from one commit. |
| Release record | The signed list of the files of a release for one target. |
| Rollout | The update of the hub and the machines to one release, in steps. |
| Step | The machines that update at the same time in a rollout. |
| Previous version | The program files that a machine had before its update. The machine keeps them. |

## What exists

- A release starts in the Builds view. Builders build the desktop application for `linux-x86_64`
  and `darwin-universal`. The hub checks the files, signs the updater file with the updater key
  and writes the feed at `https://releases.anima.ing` (BLD-045, BLD-064 to BLD-066, R-117).
- `crates/updater-signature` signs and verifies. It is pure and has no file access.
- No release is published. Tasks R5 to R7 of [projects-and-builds.md](projects-and-builds.md)
  are not started. The update of a real installed application is not verified (OD-16 to OD-18).
- A release build of the desktop application carries `anima-agent` for its own target, and
  installs it on its own computer (MET-019). No agent file is published alone.
- The Linux system unit of the agent runs with no capability and `ProtectSystem=strict`. Thus the
  agent cannot replace its own program file. The rule MET-002 says the same.
- The agent connects only to the hub (MET-020). It makes three requests: enrol, push and renew
  (K8). The push answer is empty. The hub cannot tell an agent which version to get.
- The hub on the VPS is the image `anima-hub:local`, built there from source. A one-shot
  `migrate` job runs before the hub. The way back from a migration is a restore of the backup.
- The hub as a background program on the operator's computer (S4) gets its programs from the
  desktop application. A second `install` is its update.

## Proposal

### One release, one version

- **ROL-001.** A release must give one version and one commit to each program of Anima: the
  desktop application, `anima`, `anima-hub`, `anima-agent`, `anima-gateway` and `anima-link`
  (INS-002 says this for two of them).
- **ROL-002.** A release must publish `anima-agent` for each target that a machine of the
  operator has, and the hub for each target that runs a hub. Decision OD-ROL-8 gives the list.
- **ROL-003.** Each target of a release must have one release record. The record gives the
  version, the commit, the target, and the name, the program, the size and the SHA-256 of each
  file. It also gives the lowest version that this release can replace (ROL-041).
- **ROL-004.** The hub must sign the release record after its checks of the files (BLD-064), with
  the version in the signature (as BLD-046). One signature then covers each file of the target.
- **ROL-005.** The release record and its files must be under `<target>/<version>/` of the
  release host (as BLD-060). There is no package manager channel (INS-008).

### How a release is fetched and verified

- **ROL-006.** The hub must read the release host at start and then each 4 hours (as UPD-001).
  A hub that cannot reach the release host must say so in the Updates section, with no dialog.
- **ROL-007.** The hub must verify the signature of the record and the size and the SHA-256 of
  each file before it offers the release. The public key must be in the program of the hub, not
  in its configuration.
- **ROL-008.** An agent must get the record and its file from its hub, with its agent token. It
  must not connect to the release host. MET-020 stays as it is. The hub must keep the files of
  the current release and of the release before it.
- **ROL-009.** The hub must tell an agent only the version to get. It must never send an
  address, a path, a command or a file name.
- **ROL-010.** The machine must verify by itself. The part that replaces the program (ROL-015)
  must verify the signature of the record with the public key in the program, then the version,
  the target and the program name in the record, then the size and the SHA-256 of the file. It
  must not trust the part that did the download.
- **ROL-011.** If one check of ROL-010 fails, the machine must install nothing. It must report
  "Refused" with the cause, and continue with its version (as UPD-002).
- **ROL-012.** A machine must refuse a version that is lower than its own. The one exception is
  its own previous version in a rollback (ROL-031).
- **ROL-013.** A file must name its program in the signed record. Thus a signed file of the
  desktop application cannot be installed as the agent or as the hub.
- **ROL-014.** A hub that holds no signing key gets the releases of the Anima project in the same
  way. It cannot publish one (PRO-037, PRO-038, OD-W20).

### Who replaces a program

- **ROL-015.** The part of the agent that speaks to the hub must not hold the right to replace a
  program file. A second unit of the same program must hold that right. It must have no network
  access. This is the same division as WDG-040 of the watchdog.
- **ROL-016.** The machine must replace a program file with one rename, so that a loss of power
  leaves the old file or the new file, never a part of one.
- **ROL-017.** The machine must keep the previous version of each program that it replaces. It
  keeps one previous version only.
- **ROL-018.** A release must not change a file that the agent keeps (configuration, token,
  buffer) in a way that the previous version cannot read.
- **ROL-019.** On a server, the hub is a container. The second unit of the agent on that machine
  must replace it: load the image, compare its digest with the record, start the hub, and keep
  the previous image. The container of the hub must never get the Docker socket.
- **ROL-020.** A hub that runs as a background program on the operator's computer (S4) updates
  with the desktop application on that computer (UPD-003). A rollout must show it and must not
  replace it.
- **ROL-021.** A user installation of the agent (MET-019) updates with the same checks. Its
  files belong to the user, so it needs no administrator password (as INS-009).

### The rollout

- **ROL-022.** The operator must be able to start a rollout with one press. The hub must write
  the start, each pause, each stop and each rollback to the audit log (LOG-005).
- **ROL-023.** A rollout must update in this order: the hub, then one machine, then the other
  machines in steps. Each step has at most a quarter of the machines, and at least one.
- **ROL-024.** While a step replaces a program of Anima on a machine, the watchdog of that
  machine must not restart that program (WDG-030).
- **ROL-025.** A policy must select the first machine: a machine that is connected, that is not
  the machine of the hub and that runs no job. The policy must be a pure function. The operator
  can select a different machine before the press.
- **ROL-026.** A step is good when each of its machines passes its check (ROL-029). The next step
  must start 10 minutes after a good step, not earlier.
- **ROL-027.** A rollout must stop when one machine rolls back or refuses the release. The
  machines that have the new version keep it. The operator can then continue, or roll back each
  updated machine with one press.
- **ROL-028.** A machine must not start its update while it runs a job, a build or a job of a
  runner. The operator can hold a machine: a rollout leaves a held machine out and says so. A
  machine that is set off for updates at the machine must stay off; the hub cannot set it on.

### The check after an update, and the rollback

- **ROL-029.** A new agent passes its check when, in 3 minutes: it starts, it passes the checks
  of `anima-agent doctor` (MET-023), the hub accepts one push from it, and the hub sees the new
  version.
- **ROL-030.** The decision to roll back must not depend on the new version. A new version that
  cannot start, or that stops, must be rolled back by the machine, also after a restart of the
  machine. The limit is 3 failed starts, or 3 minutes with no good check (R-115).
- **ROL-031.** A rollback must put the previous version back, start it, and report "Rolled back"
  with the cause. The hub must write an event with the severity Error and send a notification.
- **ROL-032.** A machine must try one version at most two times without the operator. After the
  second rollback, it waits for "Try again".
- **ROL-033.** Before the hub is replaced, the machine of the hub must make a database backup
  (DB-007) and must wait for its success.
- **ROL-034.** The hub of a release must be able to use the database of the release after it. A
  migration in one release can add; it must not remove or change what the previous hub reads. A
  gate must run the previous hub on the new database.
- **ROL-035.** A new hub passes its check when, in 5 minutes: `anima-hub health` is good, the API
  gives the new version, and the hub accepts one push of an agent. If not, the machine must start
  the previous image. The database stays as it is (ROL-034).
- **ROL-036.** A release that cannot obey ROL-034 must say so in its record. A rollout of it
  needs a second confirmation that names the restore procedure. An automatic rollout must never
  take it.
- **ROL-037.** The hub and the agent of a release must work with the agent and the hub of the
  release before it. A gate must test each of the two pairs.
- **ROL-038.** While the hub restarts, the client must show "The hub is updating", with the time
  that it started. It must not show the error state of a lost hub (UI-030) for 5 minutes.

### A machine that is not connected

- **ROL-039.** A rollout must not wait for a machine that is not connected. That machine leaves
  its step with the state "Waiting: not connected". The rollout can finish without it.
- **ROL-040.** When the machine connects again, the hub must tell it the version of the newest
  rollout that includes it. That can be newer than the version of the rollout that it missed. The
  machine then updates alone, with the same check and the same rollback. Its failure sends a
  notification and does not change a finished rollout.
- **ROL-041.** A machine must be able to go to the newest release directly. If a release cannot
  replace a version, its record says so (ROL-003), and the hub gives the releases between them in
  order.
- **ROL-042.** The hub must keep the push route and the update route of a contract version while
  one enrolled machine last reported that version. Before a hub update that removes a contract
  version, the client must name the machines that the new hub cannot update.
- **ROL-043.** A machine whose agent token ended while it was not connected (MET-013) cannot get
  an update. Its state is "Enrol again".
- **ROL-044.** An agent that was installed before this function has no update function. Its
  state is "Needs a manual installation". One installation by hand, or by the desktop application
  on its own computer, gives it the function.
- **ROL-045.** If the hub stops while a machine is in its check, the machine rolls back
  (ROL-030). That is safe. The hub must then give that machine the update one more time.

### Automatic update

- **ROL-046.** The setting "Update automatically" must have three values: "Off", "Machines" and
  "Machines and hub". The default is "Off".
- **ROL-047.** With the setting on, the hub must start a rollout by itself when it has verified a
  new release (ROL-007). The rules of the rollout stay the same, but the wait after the first
  machine is 1 hour.
- **ROL-048.** The operator can set a window for automatic rollouts, as a rule with fields
  (MW-004). An automatic rollout must start a step only in the window.
- **ROL-049.** An automatic update of the hub must start only when no session is live and no
  build runs. If that does not occur in the window, the hub waits for the next window.
- **ROL-050.** An automatic rollout that stops (ROL-027) must send a notification. It must not
  start again for the same release without the operator.

### What the hub records and shows

- **ROL-051.** The agent must report its version with each push. The hub must keep the version of
  each machine and its own version, and give them through the API.
- **ROL-052.** The hub must write one event for each machine that updates, rolls back or refuses.
  The actor kind of a rollout from the press is User. The actor kind of an automatic rollout is
  Hub (LOG-013). An API key must not start a rollout (OD-ROL-9).

## In the client

The build order is CMD-030: the hub API, the `anima` command, the MCP tool, then the client.

| Place | What the operator sees |
| --- | --- |
| Navigation | One banner with `role="status"`, as UPD-004: "Anima 0.3.0 is available for the hub and 7 machines", with the button "Review update". A rollout in progress shows in the "running now" area (PRO-026). |
| Settings, Updates (SET-006) | The row of the desktop application stays. A new group "Hub and machines" shows the available version, the versions in use ("The hub and 5 machines have 0.2.4. 2 machines have 0.2.3.") and the primary button "Update all". |
| The confirmation | One sheet before the rollout starts. It gives the order (the hub, then the first machine by name, then the others), and three facts: "2 machines are not connected. They update when they connect." "1 live session will stop when the hub restarts." "The hub is not reachable for about 1 minute." The operator can change the first machine here. |
| A rollout in progress | The steps in order, each with its machines and their states. The time to the next step. The buttons "Pause" and "Stop". After the end, one summary line, for example "Done. 2 machines wait." |
| The list of machines | One row for the hub and one for each machine: the name, the version and the state in words, never as a colour only: "Up to date", "Update available", "Waiting for its step", "Updating", "Checking", "Waiting: not connected", "Rolled back" with the cause, "Refused" with the cause, "Held", "Set off at the machine", "Needs a manual installation", "Enrol again". Row actions: "Update now", "Roll back", "Hold". |
| The hub row, local hub | "The hub updates with the application on this computer" (ROL-020), with no button. |
| Automatic update | A segmented control "Off", "Machines", "Machines and hub", and the window. Each saves at once (SET-005). |
| Devices page and device detail | The Overview shows "Agent 0.2.4", with "Update available" as a link to the Updates section. "Rolled back" puts the device in "Needs attention" (DEV-026), and the reason sentence of DEV-020 gives the cause. The Logs tab has the events of ROL-052. |
| While the hub restarts | "The hub is updating" (ROL-038), then the result of the hub's check. |
| `anima` and MCP | `anima update status`, `start`, `pause`, `stop`, `rollback`, `hold`. The read tool is `allow`. The change tools are `off` by default (MCP-002). |

## What depends on unfinished work

| Dependency | State today | Effect |
| --- | --- | --- |
| Signed releases: the first release (tasks R5 to R7 of [projects-and-builds.md](projects-and-builds.md)) | Not started. No release is published. OD-16 to OD-18 are open. | Nothing can be fetched or verified until one release exists. The signature path is proved only in tests. |
| The builder | It builds the desktop application only, from a fixed recipe (OD-W16), for two targets. The general builder (group A) is not started. | The release needs new files: the agent alone for each target, the image of the hub, and the release record. This is a change to the recipe of the Anima release, or a task of group A. |
| A builder for each target (BLD-003) | `linux-x86_64` and `darwin-universal` only | A machine of a different target, for example Linux on ARM, gets no agent file (OD-ROL-8). |
| The local hub in the release (task R6; S4 T9) | Not started | ROL-020 needs it. |
| A change to K8, which is frozen (CON-001) | Not asked | ROL-051 needs an optional member of the push. ROL-008 and ROL-009 need a new route, or the work route (task W1, not started). |
| A ruling number for OD-W10, which changes R-115, and a change to MET-002 | Decided, with no ruling number | The agent replaces a program on the machine. MET-002 permits it to write only its configuration, its token and its buffer. |
| The hub on the VPS is built there from source | No image is published | ROL-019 needs a published image and a program on the VPS with the Docker right. The first installation of that program is a change to the VPS (VPS-005). |
| The restore procedure (FND-006, work plan tier 1) | Proposal, not written | It is the last way back when the hub cannot roll back (ROL-036). Decision OD-ROL-4 puts the hub after it. |
| The isolation design and the security review (tasks W2 and S) | Not started | ROL-015, ROL-019 and ROL-030 are security subjects. |
| The gates for ROL-034 and ROL-037 | They do not exist | Without them, a rollback of the hub and a rollout that stops in the middle are not safe. |
| The watchdog ([watchdog.md](watchdog.md)) | Proposal | ROL-024 applies only when the watchdog exists. This proposal does not need the watchdog. |

The key is a larger risk after this proposal. R-117 accepted that a hub that holds the updater
key can sign an application that an installed application trusts. With this proposal, the same
key signs a program that runs on each machine, and on some machines with the right to replace
programs. The operator must accept this again, or select a second key (OD-ROL-2).

## Conflicts with text that exists

| Text | Says | This proposal |
| --- | --- | --- |
| MET-002 | The agent writes only its configuration, token and buffer files | Add its own program files, through the second unit (ROL-015). |
| R-115, "Later, not yet spec" | Signed agent self-update with rollback after 3 failed starts | This proposal is that specification (ROL-030). |
| K8 | Three requests; the push answer is empty | See "What depends on unfinished work". |
| MET-020 | The agent connects only to the hub | Kept (ROL-008). |
| Design specification Sections 3.9 and 3.10 | A release is the desktop application | Add the agent, the hub and the release record (ROL-001 to ROL-005). The `UPD` requirements stay. |
| BLD-069, Section 3.9 "Rollback" | The hub writes no lower feed; a rollback is a manual write | Kept. A rollback of a machine uses its own previous version (ROL-017), not the feed. |
| INT-003 | The calls that an API key must not make | Add the start of a rollout (OD-ROL-9). |
| SET-006 | The Updates section shows the version of the application | Add the group "Hub and machines". |
| `deploy/README.md` | The operator builds and starts the hub by hand | That path must stay and must work beside ROL-019. |
| VPS-005 | No change to the VPS without the operator's confirmation in chat | The rule is for agents that build Anima. A press in the client is the operator's own act. An automatic update of the hub needs a ruling that the setting is that confirmation. |

## Open decisions

| Number | Decision | Recommendation |
| --- | --- | --- |
| OD-ROL-1 | Where an agent gets its files: from its hub, or from the release host | From its hub (ROL-008). MET-020 and the VPN rules stay, and a machine with no internet access can update. |
| OD-ROL-2 | The signing key: the updater key of UPD-012 for each program, or a second key for the agent and the hub | One key in the first version, with the program name in the signed record (ROL-013). It is one key to keep safe and to back up. A second key is correct if the operator wants a limit on the risk of R-117. |
| OD-ROL-3 | The form of the published hub: an image file on the release host, an image in a registry, or a build from source on the machine of the hub | An image file on the release host, with its digest in the signed record. GitHub is not in the release path (R-107), and a build from source is slow and needs the tools on the VPS. |
| OD-ROL-4 | The order of the build | Three stages. A: the versions show (ROL-051). B: the machines update with one press. C: the hub joins, after the restore procedure (FND-006) is written. Until C, the hub row shows the command for an update by hand. |
| OD-ROL-5 | The values and the default of the automatic update | "Off" by default. "Machines" in stage B. "Machines and hub" only in stage C. |
| OD-ROL-6 | The numbers: a quarter for each step, 10 minutes between steps, 1 hour after the first machine of an automatic rollout, 3 minutes and 5 minutes for the checks | As written. Fixed in the first version. |
| OD-ROL-7 | Who selects the first machine | The policy, and the operator can change it in the confirmation (ROL-025). |
| OD-ROL-8 | The targets of the agent and the hub | **Decided (operator, 2026-11-16): no Linux machine on ARM is in use ("don't have any").** The agent: `linux-x86_64` and `darwin-universal`. The hub: `linux-x86_64`. Add a target only when the operator has such a machine and a builder for it (ETH-009). |
| OD-ROL-9 | Can an API key or an AI agent start, stop or roll back a rollout? | No. Add those calls to the list of INT-003. A key can read the state. |
| OD-ROL-10 | Does a machine that has "Takes work" off (USE-004) get updates? | Yes. The update has its own switch at the machine (ROL-028). If not, a machine that takes no work keeps an old agent. |
| OD-ROL-11 | What a rollout does when it stops: only stop, or also roll back each updated machine | Only stop (ROL-027). The updated machines passed their check. The operator has "Roll back" for one and for all. |
| OD-ROL-12 | A release whose database change the previous hub cannot read | Forbid it in the gates (ROL-034). Permit an exception that the record names, with a second confirmation and no automatic rollout (ROL-036). |
| OD-ROL-13 | On a computer that has the desktop application, who updates the agent: the rollout, or the application when it updates itself | The rollout only. The application installs the agent only when the computer has none (MET-019). Two updaters for one program can roll each other back. |
| OD-ROL-14 | Is the desktop application part of a rollout? | No. UPD-001 to UPD-006 stay. The application installs with no prompt and waits for "Restart to update". |

The tier of this proposal is decision OD-W23 of the work plan.

## Tasks

U1 is done. The other tasks are not started. The states obey PLN-007.

| Task | Content | Needs | State | Commits |
| --- | --- | --- | --- | --- |
| U1 | Stage A. The agent reports its version; the hub keeps it and gives it. Hub API, then `anima`, then MCP, then the Device page. | R-118 (operator, 2026-11-16): the optional member of K8 | Done | `e29e59db`, `553a6522`, `68b293fe`, `328f6ff4`, `d162234f`. Reviewed by an independent reviewer agent: accepted, no blocker. Open: the agent test `r_118_each_batch_carries_the_version_of_the_agent` runs only on Linux and has not run yet; the release builder does not build `anima-agent` with `ANIMA_VERSION` yet (task U2). |
| U2 | The release record: its fields, its signature, its place on the release host. The recipe of the Anima release makes the agent files and the record. | Tasks R5 to R7; OD-ROL-2 | Not started | — |
| U3 | A prototype on one Linux machine and one Mac: the second unit, the rename, the rollback of a version that stops at once and of one that does not answer, also after a restart of the machine (ROL-030). A record with measured results. | — | Not started | — |
| U4 | The contract: how the hub tells a version, how an agent gets the record and the file, the states of a machine. | A ruling for OD-W10 and for K8; OD-ROL-1 | Not started | — |
| U5 | The pure rules: the order of a rollout, the steps, the first machine, the states (ROL-023 to ROL-028, ROL-039 to ROL-041). | U4 | Not started | — |
| U6 | Stage B in the hub and the agent: fetch, verify, serve, replace, check, roll back. Hub API, then `anima`, then MCP. | U2, U3, U5 | Not started | — |
| U7 | Stage B in the client: the Updates group, the confirmation, the rollout, the list of machines. | U6 | Not started | — |
| U8 | The automatic update of machines (ROL-046 to ROL-050). | U6 | Not started | — |
| U9 | The gates of ROL-034 and ROL-037. | The first release (task R7): before it, no earlier release exists to test against | Not started | — |
| U10 | Stage C: the image of the hub in the release, the replacement of the hub with the backup and the check (ROL-019, ROL-033, ROL-035). | U6, U9; FND-006; the operator for the first installation on the VPS (VPS-005) | Not started | — |
| U11 | The security review of the update. | U6; then U10 | Not started | — |

No release is published (operator, 2026-11-16: "were still pre release"). Thus no machine has a
released agent, and ROL-044 applies only to the agents that the operator built by hand. If U1 is
done before the first release (task R7), each released agent reports its version from the start.
U1 and U3 need no release. U2 and each later task wait for task R7.

## Needs

- A decision on OD-W23 (the tier) and on OD-ROL-1 to OD-ROL-14.
- The operator accepts the larger risk of the key again, or selects a second key (OD-ROL-2).
- A ruling number in `programme.md` for OD-W10, with the change to MET-002.
- A ruling for the change to K8, or for a new contract (CON-001).
- The first release (task R7), which only the operator can do.
- The security reviewer for ROL-010, ROL-015, ROL-019 and ROL-030 (a `security` task).
- After approval: one issue for each stage, with the labels of Section 4 of the work plan.
