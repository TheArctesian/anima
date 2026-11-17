# DMN-1: Rename the program `anima-agent` to `anima-daemon`

Progress: the index [README.md](../README.md) holds the state of this item (PLN-006). The plan register in [work-plan.md](../../work-plan.md) Section 9 lists the index.

| Item | Value |
| --- | --- |
| Identifier | DMN-1 |
| Kind | Work item |
| Owner | [design-spec.md](../../spec/design-spec.md) Section 6.9 (MET-011 to MET-025) and `contracts/daemon-push.md` (K8) |
| Serves | Work plan Section 4, tier 0 (finish the device manager) |
| Version | 1.0 |
| Date | 2026-11-17 |
| Origin | The operator, 2026-11-17 (R-126) |

## Reason

"Agent" now means an AI coding agent in the planning documents, in `AGENTS.md`, in the claims and in the audit log (the actor kind Agent). The program on each machine has the same word. The operator selected "daemon", because the program will also take work, restart programs and update itself.

## Decided (R-126)

- The program is `anima-daemon`. Each name of the program changes: the crates, the folders, the hub API, the command `anima daemon`, the MCP tools, the client, the installer and the documents.
- No compatibility. The hub accepts only the new paths (`/daemon/v1/...`) and the new member `daemonVersion`. A machine with the old program stops its reports until the operator installs the new one.
- Old tokens are not valid. A token starts with `anima_daemon_`. Each machine enrols again. The log redaction still hides the old prefix `anima_agent_`.
- The VPN role `agent` (`tag:agent`) becomes `daemon` in the repository. The change of the live policy on the VPS waits for the operator's confirmation (VPS-005).

## What does not change

These names are not the program and keep the word "agent":

- The AI agent as an actor: `ActorKind::Agent`, `MadeFor`, `actor_kind`, `made_for`, `declares_agent`, `as_agent`, agent runs, and the MCP code.
- The SNMP agent of the checks.
- Terms of the operating system and the network: `LaunchAgent`, `ssh-agent`, `User-Agent`, `monarx-agent`.
- An applied migration. A new migration renames the table, the column and the value.
- `docs/archive/`, which is history.

## Tasks

| Task | Content | State | Commits |
| --- | --- | --- | --- |
| T1 | The crates `anima-agent` and `agent-core` become `anima-daemon` and `daemon-core`, with their folders | Done. `crates/daemon` and `crates/daemon-core`. | The merge of the branch `rename/anima-daemon` |
| T2 | K8 (`contracts/daemon-push.md`), K1, K2 and K3: the paths, the member, the GraphQL names, the table and the column, with a new migration | Done. `contracts/daemon-push.md`; K1 `enrolDaemon`, `revokeDaemonToken`, `DaemonEnrolment`, `Device.daemonVersion`, `MetricsSource.DAEMON`, `VpnRole.DAEMON`; migration `0717_metrics_daemon_rename.sql`. | The merge of the branch `rename/anima-daemon` |
| T3 | The hub, the command `anima daemon`, the MCP tools and the client | Done. `anima daemon status`, `enrol`, `revoke`; the client panel "Install daemon". | The merge of the branch `rename/anima-daemon` |
| T4 | The installer: the new unit and label, the new paths, and removal of the old unit | Done for the unit: `anima-daemon install` stops and deletes the old unit. The old program file and its folder stay until the operator deletes them ([need-human-testing.md](../../need-human-testing.md), HT-018). | The merge of the branch `rename/anima-daemon` |
| T5 | The VPN role in `deploy/headscale/` | Done in the repository. The live policy waits for the operator (HT-019, VPS-005). | The merge of the branch `rename/anima-daemon` |
| T6 | The documents, and the entry in [need-human-testing.md](../../need-human-testing.md) for the machines that have the old program | Done. The records in `docs/plans/phase-3/`, `docs/deferred.md`, the work items and `docs/archive/` keep the old name, because they are history. | The merge of the branch `rename/anima-daemon` |
