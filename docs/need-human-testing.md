# Need Human Testing

| Item | Value |
| --- | --- |
| Document | The list of checks that only a person can do |
| Date | 2026-11-16 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Kind | Record. It adds no requirement (PLN-010). |

This document lists each function that is built and that the gates cannot prove. A person must
do each check on a real computer, a real account or a real network. `AGENTS.md`, section
"Human testing", gives the rules for agents.

## How to use this list

- Each entry has an identifier `HT-nnn`. Do not use an identifier again.
- The operator does the steps and writes the result in the entry: the state, the date and one
  line about what occurred.
- The states are Open, Passed and Failed. Only the operator, or an agent that the operator told
  the result, can set Passed or Failed.
- A Failed entry stays in the list until a correction is built. Then its state goes back to Open.
- Move a Passed entry to "Passed" at the end of this document. Do not delete it.

## Open

| Entry | Subject | Needs | State |
| --- | --- | --- | --- |
| HT-001 | `anima snapshot` against GitHub | A GitHub repository and a second computer | Open |
| HT-002 | The tool inventory from a daemon to the command line | A hub and a daemon of this build | Open |
| HT-003 | The tool inventory in an MCP client | HT-002 | Open |
| HT-004 | No window of Apple on a Mac without the developer tools | A Mac with no Xcode and no Command Line Tools | Open |
| HT-005 | The tool inventory of a daemon that runs as a system unit | A Linux computer | Open |
| HT-006 | The probes of `claude` and `codex` leave the account alone | A computer with the two programs | Open |
| HT-007 | The pinned Rust version on the Linux computer | The Linux development computer | Open |
| HT-008 | A macOS release build with the pinned Rust version | A Mac builder | Open |
| HT-009 | The hub image builds with Rust 1.100.0 | The next hub deployment (VPS-005) | Open |
| HT-010 | Merge a pull request, and a set of pull requests, on GitHub | A GitHub repository that is safe to change | Open |
| HT-011 | Run a workflow again, cancel a run and start a workflow on GitHub | A GitHub repository with a workflow | Open |
| HT-012 | Set a commit status on GitHub | A GitHub repository that is safe to change | Open |
| HT-013 | An API key cannot merge, control a run or read a file | The hub and two API keys | Open |
| HT-014 | The issue tools of the MCP server, and a workflow that a comment starts | An MCP client and the repository of HT-011 | Open |
| HT-015 | The Work, Code and History tabs of a project in the client | A hub that has the tabs (VPS-005) | Open |
| HT-016 | A release build on the Linux builder puts the bundle in its own clone | The Linux builder | Open |
| HT-017 | "This machine" with the Anima VPN and NetBird together | A Mac with Tailscale and NetBird, and a second tailnet account | Open |
| HT-018 | Each machine with the old program `anima-agent` gets `anima-daemon` and enrols again | Each machine that reports metrics, and the hub of this build | Open |
| HT-019 | The VPN role `daemon` on the VPS | A shell on the VPS (VPS-005) | Open |
| HT-022 | The first real sign-in with the Anima.ing account: a hosted hub made, joined and opened | This computer, the WorkOS account, the live site and a pool hub | Passed (2026-11-17) |
| HT-025 | The computer that signs in is a device of the hosted hub with no question | This computer, the Anima.ing account of HT-022 and its hosted hub | Open |
| HT-026 | The hosted hub is a device of its own device list, and it is Up | This computer, the Anima.ing account of HT-022, and a hub of the pool from a new hub image | Open |

### HT-001: `anima snapshot` against GitHub

Built: `crates/anima-cli/src/snapshot/` (JOB-017, JOB-018). All its tests use a local
repository. No test used GitHub.

1. Build the command: `cargo build -p anima-cli`. The program is `target/debug/anima`.
2. Go to a clone of a GitHub repository. Change one tracked file and add one new file.
3. Run `anima snapshot push`. Expected: exit code 0. It prints a ref `refs/anima/wip/<id>`, a
   commit and a `git fetch` line.
4. Run `git status`. Expected: the same output as before step 3.
5. On a second computer with a clone, run the `git fetch` line, then `git checkout FETCH_HEAD`.
   Expected: the two changes of step 2 are there.
6. Open the repository on GitHub. Expected: no new branch, no new tag and no pull request.
7. Run `anima snapshot list`. Expected: one row with the ref of step 3.
8. Add a file `.envrc` and run `anima snapshot push`. Expected: exit code 5, the text names
   `.envrc`, and `anima snapshot list` shows no new ref. Delete the file.
9. Run `anima snapshot delete <id>` and confirm. Expected: `anima snapshot list` shows no row.

Also write down: whether a ruleset or push protection of GitHub refused the push in step 3.

Result: Open.

### HT-002: the tool inventory from a daemon to the command line

Built: `crates/daemon-core/src/inventory/`, `crates/daemon/src/readers/inventory.rs`, the hub
migration `0715_metrics_tools.sql`, `anima capability list` (CAP-020 to CAP-031). The tests use
scripts in place of the real tools, and a scripted hub.

The hub and the daemon must be of this build. A change to the hub on the VPS needs the
confirmation of the operator (VPS-005). A local hub is sufficient for this check
([setup/local-hub.md](procedures/setup/local-hub.md)).

1. Start a hub of this build. Sign in with `anima auth login`.
2. Enrol the daemon of one device and start the `anima-daemon` of this build on it.
3. Wait 1 minute. Run `anima capability list`. Expected: one row for the device, with its
   architecture, its operating system version and its tools.
4. On the device, run `cargo --version`, `git --version` and `bun --version`. Expected: the same
   versions as in step 3.
5. Run `anima capability list --tool cargo`. Expected: only the devices that have cargo.
6. Run `anima capability list --target <a target that the device has>` and then with a target
   that it does not have. Expected: the device is in the first list and not in the second.
7. Install or remove one tool on the device. Wait 16 minutes. Expected: the list shows the change.
8. Read the log of the daemon. Expected: no version text and no output of a tool (LOG-007).

Result: Open.

### HT-003: the tool inventory in an MCP client

Needs HT-002.

1. Start an MCP client with `anima mcp` and a read API key.
2. Ask the client to call `anima_capability_list`. Expected: the same devices and tools as
   `anima capability list`.
3. Call it with `tool` set to `cargo`. Expected: the same devices as step 5 of HT-002.

Result: Open.

### HT-004: no window of Apple on a Mac without the developer tools

Built: the daemon does not run the placeholder programs `/usr/bin/git` and `/usr/bin/xcodebuild`
when `xcode-select -p` fails. The review could not do this check, because its Mac has Xcode.

1. Use a Mac where `xcode-select -p` prints an error.
2. Install and start the `anima-daemon` of this build.
3. Use the Mac for 20 minutes. Expected: the window "install the command line developer tools"
   does not show.
4. Run `anima capability list`. Expected: the device has no `git` row and no `xcodebuild` row,
   unless a different git is installed (for example from Homebrew).

Result: Open.

### HT-005: the tool inventory of a daemon that runs as a system unit

The systemd system unit hides the home folders and denies memory that is writable and
executable. No person looked at what such a daemon reports.

1. On a Linux computer, install the daemon as a system unit and start it.
2. Run `anima capability list`. Write down the tools that it shows.
3. Compare with the tools of the user in a shell. Expected: tools in `/usr/bin` and
   `/usr/local/bin` show. Tools in the home folder of a user do not show.
4. Write down each tool that shows as "not usable" and its reason. `node` and `bun` are
   possible.

Result: Open.

### HT-006: the probes of `claude` and `codex` leave the account alone

Built: these two probes run with an empty temporary home folder (CAP-025). The builder compared
the account folders before and after one probe. No person looked at a network request, and no
check ran for more than one probe.

1. On a computer with `claude` and `codex`, write down the output of
   `ls -laR ~/.codex | shasum` with the two programs closed.
2. Run the daemon of this build for 1 hour with the two programs closed.
3. Do step 1 again. Expected: the same value.
4. During step 2, look at the network connections of the processes `claude` and `codex` that the
   daemon starts (for example with `nettop` or Little Snitch). Write down each connection.

Result: Open.

### HT-007: the pinned Rust version on the Linux computer

Built: `rust-toolchain.toml` pins Rust 1.100.0. Before the pin, `origin/main` did not pass
Clippy on the Mac, so the two computers used different versions.

1. On the Linux development computer, get `main` and run `cargo --version` in the repository.
   Expected: `cargo 1.100.0`. rustup can download it first.
2. Run `bun run gates`. Expected: `gates: 20/20 pass`.

Result: Open.

### HT-008: a macOS release build with the pinned Rust version

rustup installs the pinned version as a separate toolchain. That toolchain has only the targets
that a person adds to it.

1. On the Mac builder, run `rustup target list --installed` in the repository. Expected:
   `aarch64-apple-darwin` and `x86_64-apple-darwin`. If one is absent, add it with
   `rustup target add`.
2. Open Settings > Builds in the desktop application. Expected: no missing tool.
3. Make one release and let the Mac build it. Expected: the job goes to published.

Result: Open.

### HT-009: the hub image builds with Rust 1.100.0

Built: `deploy/hub/Dockerfile` now starts from `rust:1.100.0-bookworm`. No person built the
image after the change.

1. At the next hub deployment, build the hub image. Expected: the build completes.
2. Start the hub. Expected: the health check passes and migration 0715 is applied.

Result: Open.

### HT-010: merge a pull request, and a set of pull requests, on GitHub

Built: the hub change `merge_pull_request` and `anima pr merge` (PRJ-033, OD-E14). The tests use
a scripted GitHub. No pull request was merged on GitHub. The only change sent to GitHub so far is
one issue comment (2026-11-16).

Use a repository that is safe to change. Make four pull requests in it that can merge, one pull
request that has a conflict, and, if you can, one pull request from a fork. Sign in with
`anima auth login`.

1. Run `anima pr list <owner/name> --checks`. Expected: each pull request, with CAN MERGE and
   CHECKS filled in.
2. Run `anima pr get <owner/name> <number>` and copy the Head commit. Run
   `anima pr merge <owner/name> <number> --method squash --sha <head>`. Expected: it shows the
   pull request and asks one question. After "yes", GitHub shows the pull request as merged with
   the squash method.
3. Run `anima pr merge <owner/name> <number> --method merge --yes` with no `--sha`. Expected: a
   refusal that names `--sha`.
4. Run `anima pr merge <owner/name> <n1> <n2> <n3> --method merge`. Expected: it shows the three
   pull requests and asks one question. After "yes", the three are merged.
5. Run `anima pr merge` on the pull request with the conflict. Expected: a refusal. GitHub
   shows it as open.
6. Run `anima pr merge` with the fork pull request and one other pull request. Expected: a
   refusal that says a fork pull request merges only alone. Then run it alone with
   `--fork <owner/name of the fork>`. Expected: it asks you to type a confirmation.
7. Run `anima audit list --limit 10`. Expected: one row for each sent merge, with the action
   `merge_pull_request`, and the target as `<owner/name>` and the pull request number. A refused
   merge that was sent shows `merge_pull_request (refused)`.

Result: Open.

### HT-011: run a workflow again, cancel a run and start a workflow on GitHub

Built: the hub changes for run controls and `anima workflow rerun`, `cancel` and `start`
(PRJ-034). The tests use a scripted GitHub. The repository `TheArctesian/anima.ing` has no
workflow, so nothing was tried.

Use a repository with one workflow that has `workflow_dispatch` with one input, and that can
fail on request (for example, an input `fail` that makes one job fail). Sign in with
`anima auth login`.

1. Run `anima workflow list <owner/name>`. Expected: the workflow, as active.
2. Run `anima workflow start <owner/name> <workflow> --branch main --input fail=false`.
   Expected: it asks one question. After "yes", GitHub shows a new run with that input.
3. Run `anima workflow runs <owner/name>`. Expected: the new run. Copy its number.
4. While a run is in progress, run `anima workflow cancel <owner/name> <run>`. Expected: GitHub
   shows the run as cancelled.
5. Start a run with `--input fail=true` and wait until it fails. Run
   `anima workflow rerun <owner/name> <run> --failed`. Expected: GitHub runs only the failed job
   again, as attempt 2.
6. Run `anima workflow rerun <owner/name> <run>`. Expected: GitHub runs all the jobs again.
7. Run `anima workflow start` with an input name that the workflow does not have. Expected: a
   refusal, and no new run on GitHub.
8. Run `anima audit list --limit 10`. Expected: one row for each change that was sent. No row
   shows the value of an input (LOG-007).

Also write down: whether a run older than 30 days can be run again, and what the command says.

Result: Open.

### HT-012: set a commit status on GitHub

Built: the hub change `set_commit_status` and `anima repo status set` (PRJ-035). The tests use a
scripted GitHub.

1. Run `anima repo commits <owner/name> --limit 1` and copy the SHA.
2. Run `anima repo status set <owner/name> <sha> --state success --context anima/test --description "A test"`.
   Expected: it asks one question. After "yes", the commit on GitHub shows the status
   `anima/test` as passed.
3. Run the same command with `--context ci/test`. Expected: a refusal, because the context does
   not start with `anima/`. GitHub shows no new status.
4. Run the command with a description of 141 characters. Expected: a refusal.

Result: Open.

### HT-013: an API key cannot merge, control a run or read a file

Built: the refusals of API keys for the changes and the file content of Projects (D-23,
PRJ-032), and the limits of GitHub requests for one key. The tests use a hub in the test
process. The hub on the VPS has no API key, so nothing was tried there.

1. Run `anima key create --name ht-read --for user --expires 1d` and
   `anima key create --name ht-write --scope write --for agent --expires 1d`. Keep the two
   secrets in the terminal only.
2. With `ANIMA_TOKEN` set to the read key, run `anima pr list <owner/name>`. Expected: the list.
3. With the read key, run `anima code show <owner/name> README.md`. Expected: a refusal.
   `anima code ls <owner/name>` gives the list.
4. With the write key, run `anima pr merge`, `anima workflow rerun`, `anima workflow cancel`,
   `anima workflow start` and `anima repo status set`, each with correct values. Expected: each
   is refused, and GitHub shows no change.
5. With the write key, run `anima issue comment <owner/name> <number> --body-file -` and type one
   line. Expected: the comment is made. `anima audit list` shows it with ACTOR `AGENT` and KEY
   `ht-write`.
6. With the read key, run `anima pr list <owner/name> --checks --limit 50 --all` on a repository
   with more than 50 pull requests, if you have one. Expected: the answer, or the refusal "This
   request asked GitHub too many times. Ask for less at once."
7. Run `anima key revoke` for the two keys.

Result: Open.

### HT-014: the issue tools of the MCP server, and a workflow that a comment starts

Built: the MCP tools `anima_issue_create`, `anima_issue_comment_add` and `anima_issue_set_open`,
and the read tools of the `repositories` set (MCP-002, MCP-012, MCP-013). The tests use a
scripted hub. Section "What a compromised agent can do" of [cli-mcp-plan.md](plans/cli-mcp-plan.md)
says that a comment can start a workflow; that was not tried on GitHub.

Use the repository of HT-011. Add to it one workflow that runs on `issue_comment` and prints the
text of the comment.

1. Start an MCP client with `anima mcp` and a write API key made for an agent. Ask the client
   to make an issue. Expected: no issue is made, because the three issue tools are off until you
   turn them on.
2. In Settings > MCP of the desktop application, set the Repositories tools to Ask. Expected:
   the question before "Allow all" names the two risks: text from a private repository in a
   public one, and a workflow that an issue or a comment sets off.
3. Ask the client to make an issue. Expected: the client shows the values and asks before the
   call. After "yes", the issue is on GitHub, made by the GitHub App.
4. Ask the client to add a comment to that issue. Expected: the comment is on GitHub. Then look
   at the Actions tab. Write down whether the `issue_comment` workflow ran, and whether its log
   shows the text of the comment.
5. Ask the client to merge a pull request and to read the content of a file. Expected: it has
   no tool for either.
6. Ask the client to close the issue. Expected: the issue is closed on GitHub.

Result: Open.

### HT-015: the Work, Code and History tabs of a project in the client

Built: the three tabs of the project page (design work, 2026-11-16) on the reads and changes of
PRJ-028 to PRJ-035. The hub reads were tried from the command line against
`TheArctesian/anima.ing` on 2026-11-16. The tabs and their buttons were not tried against
GitHub.

The hub must be of a commit that has the tabs. A change to the hub on the VPS needs the
confirmation of the operator (VPS-005).

1. Open `https://hub.anima.ing/` on a computer on the VPN, or the desktop application of the
   same build. Open the project of a repository with pull requests, issues and workflow runs.
2. Open Work. Expected: the same pull requests and issues as `anima pr list` and
   `anima issue list`, and the same runs as `anima workflow runs`.
3. Open Code. Expected: the folders and files of the default branch. Open one file. Expected:
   its content.
4. Open History. Expected: the same commits, branches and releases as `anima repo commits`,
   `anima repo branches` and `anima repo releases`.
5. Open the project of a repository with no workflow and no release. Expected: each empty list
   says that it is empty, and does not show an error.
6. In the repository of HT-010, merge one pull request with the button. Expected: a
   confirmation first, then the pull request shows as merged.
7. In the repository of HT-011, use the buttons to run a workflow again and to cancel a run.
   Expected: GitHub shows each change.
8. Make an issue and a comment from the client. Expected: they are on GitHub.

Result: Open.

### HT-016: a release build on the Linux builder puts the bundle in its own clone

Built: each build step gets `CARGO_TARGET_DIR` as the `target` folder of the builder's clone
(BLD-044; `apps/desktop/src-tauri/src/builder/steps.rs`). On 2026-11-13 the Linux builder passed
the gates and made the AppImage in `~/.cache/cargo-target`, because `~/.cargo/config.toml` of
that computer sets `target-dir`, and the job failed with "The bundle is not where the build puts
it." The test checks the variable. No release was built with the correction.

The desktop application on the Linux builder must be of commit eef4e7b1 or later. Leave
`~/.cargo/config.toml` as it is.

1. On the Linux builder, run `go version`. Write down the version. The gates need Go 1.27.4;
   Go 1.22 gets it by itself on the first Go gate.
2. Start one release from Settings > Builds, or with `anima release create`. Let the Linux
   builder build it.
3. Run `anima release list`. Expected: the Linux job does not fail with "The bundle is not
   where the build puts it."
4. Run `anima release log <job>` for the Linux job. Expected: the six Go gates pass, and the
   bundle step names a path in `~/.local/share/ing.anima.desktop/builds/anima/target/`.
5. Expected: the job goes to published, and
   `https://releases.anima.ing/linux-x86_64/latest.json` answers with the new version.

HT-008 is the same release on the Mac builder.

### HT-017: "This machine" with the Anima VPN and NetBird together

Built: merged to `main` in bdb321f1 and 71bfcbdb (ADR-005, R-060). The desktop app takes its
own VPN address from the Tailscale client that is logged in to the hub's login server
(`vpnLoginServer`). It does not read the interfaces. Security finding:
[SF-001](security-findings.md#sf-001-this-machine-can-show-and-probe-the-address-of-another-overlay-network).

The tests use a script in place of the Tailscale client. No test ran NetBird, and no test ran a
second overlay network on the same computer. One check on the operator's Mac with only the Anima
VPN gave the Anima address (`100.64.0.2`) and SSH on.

1. Install the desktop app of this build on a Mac. Connect the Anima VPN (Tailscale logged in
   to the hub's login server). Connect NetBird on the same Mac. If you can, use a NetBird network
   that gives this Mac an address in `100.64.0.0/10` that is lower than its Anima address.
2. Run `tailscale ip -4` and `netbird status`. Write down the two addresses. Expected: two
   different addresses.
3. Open the app, sign in, and open Add device > This Mac. Expected: the host is the Tailscale
   address of step 2, not the NetBird address.
4. Run `sudo tcpdump -ni any 'tcp port 22 and tcp[tcpflags] & tcp-syn != 0'` in a terminal.
   Open Add device > This Mac again. Expected: one connection to the Tailscale address, and none
   to the NetBird address. The SSH part of the form is on only if SSH answers on the Tailscale
   address.
5. Disconnect the Anima VPN (`tailscale down`). Keep NetBird connected. Open Add device again.
   Expected: This Mac has no VPN address, and the form opens empty.
6. Sign the Tailscale client in to a different tailnet, for example a personal Tailscale
   account. Open Add device again. Expected: This Mac has no VPN address.
7. Sign the Tailscale client in to the Anima login server again. Expected: step 3 gives the
   Anima address again.

Result: Open.

### HT-018: each machine with the old program gets `anima-daemon` and enrols again

Built: the program `anima-agent` is `anima-daemon` ([DMN-1](work-items/device-manager/DMN-1.md), R-126). The hub
accepts only `/daemon/v1/...` and a token that starts with `anima_daemon_`. `anima-daemon install`
stops and deletes the unit of the old program (`anima-agent.service`, or `ing.anima.agent` on macOS).
Migration 0717 renames the table and the metrics source. Migration 0718 deletes each old token
and each enrolment code, so no old token is valid under a new spelling.

The tests use a fake hub and a planned list of commands. No test ran on a machine that has the old
program, and no test ran the migration on the database of the real hub.

1. Deploy the hub of this build (VPS-005). Expected: the hub starts, and migrations 0717 and 0718 are in its
   log with no error.
2. Open the client. Expected: each device that had the source "Agent" shows the source "Daemon"
   and is stale. No device shows new metrics from the old program.
   The hub writes no event and sends no notification when the devices go stale after this
   migration, so look at the device list.
3. On one machine, install the new program: download or build `anima-daemon`, then run
   `anima-daemon install` (or `sudo anima-daemon install --system` for a system unit).
   Expected: the command ends with no error. For a system unit, the first run stops and prints a
   `useradd` command, because the account is now `anima-daemon` (it was `anima-agent`): run that
   command, then run the install again.
   The install takes out only the old unit of its own kind. If the machine had the old program
   as a system unit and you now install a user unit (the desktop application does this), stop the
   old one by hand: `sudo systemctl disable --now anima-agent.service`. Do the same the other way.
4. On Linux, run `systemctl --user status anima-agent.service` (or without `--user` for a system
   unit). On macOS, run `launchctl print gui/$(id -u)/ing.anima.agent`. Expected: the old unit
   is not found.
5. In the client, open the device, Settings, "Install daemon", and make a code. On the machine,
   run `anima-daemon enrol <code>`. Expected: "Enrolled", and the device shows new metrics in
   less than 30 seconds.
6. Run `anima daemon status <device>`. Expected: the source is DAEMON, not stale, with the
   version of the new program.
7. Do steps 3 to 6 on each other machine. Then delete the old files by hand where they stay:
   `/usr/local/bin/anima-agent`, `~/.local/bin/anima-agent`, and the folders `anima-agent` in
   `~/.config`, `/var/lib` and `~/Library/Application Support`. They hold the old token, which
   is no longer valid.

Result: Open.

### HT-019: the VPN role `daemon` on the VPS

Built: in the repository, the Headscale policy and its tests name the role `daemon` and the tag
`tag:daemon` (`deploy/headscale/`, [DMN-1](work-items/device-manager/DMN-1.md), R-126). The live
policy on the VPS still names `tag:agent`. Nothing on the VPS was changed (VPS-005).

The tests check the policy file of the repository. No test reads the live Headscale.

1. On the VPS, run `headscale nodes list`. Write down each node that has `tag:agent`.
   Expected today: none, or the nodes that you tagged by hand.
2. If no node has the tag, nothing more is necessary: the next policy that you apply from the
   repository names `tag:daemon`. Stop here.
3. If a node has the tag, give it the new tag first:
   `headscale nodes tag -i <id> -t tag:daemon`. Then apply the policy of the repository as
   `deploy/headscale/README.md` says. Expected: `headscale policy check` gives no error, and the
   node reaches the hub on TCP 443 and nothing else.

Result: Open.

### HT-020: the desktop application builds on Linux with no lint error (GH-52)

Checked, not built: [GH-52](work-items/defects/GH-52.md) reported that the Linux build stops at an
unfulfilled `dead_code` expectation in `apps/desktop/src-tauri/src/web_view/event.rs`. A simulated
build on a Mac gave no error with Rust 1.99.1 and Rust 1.100.0. No code was changed.

No gate runs on Linux, and the Mac has no Linux target.

1. On the Linux machine, get `main` and run `rustc --version` in the repository.
   Expected: `rustc 1.100.0` (`rust-toolchain.toml`).
2. Run `cargo clippy -p anima-desktop --all-targets -- -D warnings`.
   Expected: no error.
3. Run `bun run --cwd apps/desktop tauri build` with `RUSTFLAGS` not set.
   Expected: the bundles are built. If the error of GH-52 comes again, copy it into GH-52 with the
   output of step 1.

Result: Open.

### HT-021: the delay of the session picture in the Linux application (GH-54)

Checked, not built: [GH-54](work-items/defects/GH-54.md) reported that the picture of a VNC session
in the Linux application is late. Nothing was measured, and the gateway changed after the report.
No code was changed.

No test can see a real screen.

1. On the Linux machine, start the application from a terminal and open a VNC session to the Mac.
   On the Mac, show a clock with seconds and tenths. Take one photograph that shows the two screens.
   Expected: the two clocks differ by less than 0.3 s.
2. Open the same session from the Mac application to the Linux machine and do the same.
   Expected: a delay near that of step 1. Write the two values in GH-54.
3. If step 1 is slow: close the application, start it with `WEBKIT_DISABLE_DMABUF_RENDERER=1`, and
   do step 1 again. Then the same with `WEBKIT_DISABLE_COMPOSITING_MODE=1`.
   Expected: write in GH-54 which setting, if any, removes the delay.

Result: Open.

### HT-022: the first real sign-in with the Anima.ing account (HM-T1 to HM-T7)

Built: the first-run "Sign in" ([hosted-mode.md](proposals/hosted-mode.md) Section 4.1, HMD-022, HMD-023,
HMD-037 to HMD-041, HMD-043). The desktop shell does the WorkOS device sign-in and calls
`cloud.anima.ing/v1/hub` and `/v1/hub/access`; the app joins the link with the invite and trades the
claim with the hosted hub for its token (`claimDeviceToken`).

The tests use a stand-in cloud, fake answers of WorkOS and the site, and a fake link. No test signs
in with WorkOS, calls the live site, or trades a claim through a real link. Open decision HM-D13 (the
form of the WorkOS requests) is decided by step 2.

Checked without an account on 2026-11-18 (UTC), on the pool server, with a test hub that is deleted and the
program `anima-link` of `main` on this computer: the join with the invite of a hub's access (0.7 s), the trade
of the claim through the link for a token of the operator `owner`, the refusal of a second trade of that
claim, a second access for the same device name (no invite, a new claim, which trades), the refusal of
`login`, and the same link and token after a sleep and a wake of the hub (answers again in about 12 s).
`WorkOS` Production gives a device code for a form with the client ID only, and answers a refresh with no
secret by "Invalid refresh token". The site answers 401 with no token and with a wrong one. Not checked: the
sign-in of a person, a call of the site with a real token (it makes the Stripe Customer and the hub), the
join by the application, and the refresh of a real token.

`WorkOS` Production offers email only (a password, or a code by email): Google, GitHub, Microsoft, Apple and
passkeys are off, and no account exists, so step 2 makes the account. A debug build carries no `anima-link`
(`apps/desktop/src-tauri/src/carried.rs`), so step 1 gives it one; without it the join after the sign-in is
refused. A computer that is on the link of a different hub leaves that link when it joins (`link_join`
replaces the device folder); this computer is on none.

1. On this computer, in the checkout of `main`: run `link/build.sh`, copy `link/bin/anima-link-darwin-arm64`
   to an empty folder with the name `anima-link`, run `bun run --cwd apps/desktop tauri build --debug --no-bundle`,
   and start `ANIMA_DEV_PROGRAMS=<that folder, absolute> target/debug/anima-desktop`. Do not use
   `bun run --cwd apps/desktop dev` for this check: its pages come from `http://localhost:5173`, and a hosted hub
   refuses that origin (403), so the trade of the claim fails and the app says "No network" (seen on
   2026-11-17). A built app has the origin `tauri://localhost`, which the hub accepts. On the first-run screen, click
   "Sign in". Expected: the browser opens a WorkOS page, and the app shows the same short code.
2. In the browser, type your email address and make the Anima.ing account (a password, or the code that
   WorkOS sends by email). Expected: the app shows "Your hub is
   starting", then "Connecting". If it shows "Sign-in refused" at once, WorkOS refused the request
   form: write it in HM-D13.
3. Wait. Expected: the device list of the hosted hub opens. No screen asked for an invite, an
   address or a password.
4. Quit the app and start it again. Expected: the device list opens with no sign-in.

Result: Passed (operator, 2026-11-17: "Passed"), with the built app of step 1 from `main` (`cb0d9bac`), an account
made with an email address, and the hub `h6opknk7w29jw56jqd0kc` of the pool. First try on 2026-11-17 (operator: "worked till up to here", with the screen "No network"), with
`bun run --cwd apps/desktop dev`: the sign-in, the account, the hub and the join of the link worked (the log of the
pool and of the shell); the trade of the claim failed for the origin of the debug pages (step 1). The wrong
message is not corrected: a refusal of the hub by origin and no network give the same answer to the page.
Second try on 2026-11-17 with the built app of step 1 (operator, with a picture of the empty device list of the
hosted hub: "got to here the first time, reopend to the same screen"): steps 3 and 4 gave the expected result.
In the same message the operator asked for more than this check
covers (HM-D11): "the device doesn't auto add it should and the hub is not there as the default device".

### HT-025: the computer that signs in is a device of the hosted hub (HM-T8)

Built: when the app opens the device list of a hosted hub, it adds the computer it runs on as a device with
no question ([hosted-mode.md](proposals/hosted-mode.md) HMD-044, R-136). The shell command `cloud_device`
(`apps/desktop/src-tauri/src/cloud.rs`) gives the device name that the sign-in sent, the computer name and
the OS. The client (`apps/client/src/lib/hosted/this-computer.ts`) finds the link device of the hub with that
name, and adds a device with the link address as its host, in the site `Link`, when no device has that host.
The device has the kind `workstation`, no SSH and no check. A hosted hub does not ask "Add this Mac?".

The tests use a fake shell and a fake hub. No test reads the link devices of a real hosted hub, makes the site
`Link` with the range `fd41:4e49:4d41::/48` on a real hub, or adds a device through a real link.

1. Start the built app as in step 1 of HT-022, from a checkout that has this change, and sign in if the app
   asks. Expected: the device list of the hosted hub opens and shows one row with the name of this Mac. No
   window asked "Add this Mac?".
2. Open that device. Expected: its host is an address that starts with `fd41:4e49:4d41:`, its site is `Link`,
   and it has no check.
3. Quit the app and start it again. Expected: the device list shows one row for this Mac, not two.

Result: Open.

### HT-026: the hosted hub is a device of its own device list (HM-T9)

Built: a hosted hub adds itself to its devices at each start, when no device has the host `localhost`
([hosted-mode.md](proposals/hosted-mode.md) HMD-045, R-137). The device has the name `Hub` (`Hub (hosted)` when
a different device has that name), the kind `hub`, the operating system Linux and the site `Link`. It has no
SSH, RDP or VNC. It has one PUSH check, `Hub running`. The hub reports that check as Up at the start, and then
two times in each interval of the check. The parts are `crates/hub/src/fleet/devices/hub_device.rs` (the
device), `crates/hub/src/fleet/checks/push/alive.rs` (the check and its report) and
`crates/hub/src/upkeep/hub_device.rs` (the start and the timer). A hub that is not hosted adds nothing.

The tests start the hub program in the hosted mode on a test database. No test uses a hub of the pool that was
made from a new hub image, and no test looks at the row in the application. No test waits for the timer: the
tests see the report of the start only.

Do these steps after the pool has a new hub image and the hub of the account was made again from it.

1. Start the built app as in step 1 of HT-022, and sign in if the app asks. Expected: the device list shows a
   row `Hub`, and the row is Up. The row has no button to connect. Nothing asked for a step.
2. Open the device `Hub`. Expected: it has one check, `Hub running`, and the check is Up.
3. Wait 5 minutes and look at the device again. Expected: the device and the check are Up.

Result: Open.

## Not built yet

The Tools section of the Device page and the Tools filter of the device list
([compute-plan.md](plans/compute-plan.md) Section 11.1, CUI-010 to CUI-019) are not built. Their
builder adds the entry.

## Checks that other documents hold

These checks were open before this document. The document in the column "Steps", or the line above each table, holds the steps. This section holds the result of each check (PLN-014), so that one list shows all of them. The results in this version come from the text of those documents and of [handoff.md](plans/phase-3/handoff.md) (2026-11-12). No person or agent ran a check for this version.

- A task that waits for a check is not Done until the row of the check says Passed (PLN-009).
- The words for a result are the states of this document: Open, Passed, Failed. Add "Blocked" with the reason when a check cannot start.
- The operator types each real secret personally. An agent never types, reads or asks for a real secret.

| Group | Checks | Passed | Blocks |
| --- | --- | --- | --- |
| Phase 2 (O-1 to O-9) | 9 | 1 | The Phase 2 live criteria, and the Phase 3 acceptance run (work plan Section 4, tier 0) |
| Phase 3 (P-1 to P-17) | 17 | 0 | The Phase 3 acceptance run, WP-53 (tier 0) |
| One-click (S1 to S8) | 26 | 0 | The task of each row |
| Command line and MCP | 6 | 0 | The register row of [cli-mcp-plan.md](plans/cli-mcp-plan.md) |
| A real release | 1 | 0 | OD-16 to OD-18 of the design specification |

### Phase 2

Steps: [phase-2-acceptance.md](archive/phase-2/phase-2-acceptance.md), "Operator steps". Blocks: the criterion in the column "Criterion" of that document.

| Check | What it proves | Needs | Criterion | Result |
| --- | --- | --- | --- | --- |
| O-1 | RDP: the certificate, the picture, resize, copy and paste, Ctrl+Alt+Delete | A machine with RDP. R-076: no Windows; the operator tests on a second Mac. A Headscale grant on tcp:3389 | 1 | Open. It waits for the second Mac (handoff, 2026-11-12) |
| O-2 | RDP: a changed certificate stops the session; the Warning badge | The machine of O-1 | 2 | Open. It waits for the second Mac (handoff, 2026-11-12) |
| O-3 | VNC to a Mac and to a Linux desktop | A Mac with Screen Sharing on. A Linux desktop with a VNC server | 3 | Passed on the Mac and on `pop-os` (handoff, 2026-11-12; the Linux run was 2026-11-07, with input off in wayvnc) |
| O-4 | SFTP: list, upload, download, the refusals, the audit entries | A real OpenSSH server with an accepted host key | 6 | Open |
| O-5 | Wake-on-LAN through the site relay | A device that sleeps on a LAN. A relay device at that site with `wakeonlan` | 8 | Open |
| O-6 | The web view on macOS: fingerprint, a changed certificate, the Down page | A device web interface with an untrusted certificate | 9 | Open |
| O-7 | OD-7 on Linux: the trust hook spike. Also the frame time of the Devices page | A Linux desktop | 9 | Open |
| O-8 | SSH metrics: a new point each 30 s, the bars on the card | One Linux device and one macOS device with SSH | 11 | Open |
| O-9 | Docker, SNMP, Tailscale ping and systemd checks | An SSH device with Docker and systemd. An SNMP v3 agent. The LocalAPI proxy on the VPS | 15 | Open |

### Phase 3

Steps: [phase-3-acceptance.md](plans/phase-3/phase-3-acceptance.md), "Operator steps". Run order: P-1 to P-15, P-17, then P-16. Blocks: the acceptance run of Phase 3 (WP-53). The handoff of 2026-11-12 names what must come first: O-4 to O-9, the wording of the design specification Section 5.7.3, the Headscale daemon policy (R-105), the R2 set-up and the daemon binaries.

| Check | What it proves | Needs | Result |
| --- | --- | --- | --- |
| P-1 | The deployed image and the compose file are the accepted ones | A shell on the VPS | Open |
| P-2 | The T4 probe, with no key | A VPN machine with `bun` | Open |
| P-3 | Browsers and sign-in on macOS; the cookie attributes | A Mac on the VPN with Safari and Chrome | Open |
| P-4 | A browser on Linux, and the desktop application | A Linux machine with Firefox or Chromium | Open |
| P-5 | The web client: the device list and the detail view | A signed-in browser (P-3) | Open |
| P-6 | SSH and VNC sessions in the web client | A device with SSH. A device with VNC | Open |
| P-7 | The notification channels are entered | A signed-in client. The operator types each secret | Open |
| P-8 | Channels are selected on a check | A signed-in client | Open |
| P-9 | "Send test" on each channel | The receiving application of each channel | Open |
| P-10 | A Down and Up cycle sends the messages | The check of P-8, and a way to stop its service | Open |
| P-11 | Maintenance sends no message | The check of P-8 | Open |
| P-12 | A live RDAP check | A signed-in client | Open |
| P-13 | GPU metrics on the Apple silicon Mac | The Mac with the push daemon in operation | Open |
| P-14 | Optional: a real Prometheus scrape | A Prometheus server on the VPN. A Headscale grant (ask first) | Open |
| P-15 | Optional: the API key checks in Settings | A signed-in client | Open |
| P-17 | The desktop application: the update banner | A build with an older version installed, and a newer release | Open |
| P-16 | Last: the T3 log scan of the live hub log | A shell on the VPS, after all other steps | Open |

### One-click

Each row gives the document that has the steps and the task that the check blocks.

| Check | What it proves | Needs | Steps and task | Result |
| --- | --- | --- | --- | --- |
| S1-T14 | The link between two machines: the direct rate, the first-connection time, relayed Screen Sharing, the firewall prompt. It sets the keepalive numbers (LNK-009) | A second Mac | [s1-link-tasks.md](plans/one-click/s1-link-tasks.md), T14 | Open |
| S2-T9 | The native VNC gateway against macOS Screen Sharing and TigerVNC, with the figures | This Mac; the operator types the password | [s2-vnc-tasks.md](plans/one-click/s2-vnc-tasks.md), T9 | Open |
| S2-RDP | The native RDP gateway against a GNOME Remote Desktop host | The operator's GNOME Remote Desktop host | [s2-rdp-tasks.md](plans/one-click/s2-rdp-tasks.md), Section 6 | Open |
| S3-T10 | The database tree recipe on the Linux builder | The `x86_64` Linux build computer | [s3-database-tasks.md](plans/one-click/s3-database-tasks.md), Section 7.2 | Open |
| S3-T12 | Signature and notary check of the database tree | The Developer ID of the operator. A second Mac or a new user account | [s3-database-tasks.md](plans/one-click/s3-database-tasks.md), Section 7.2 | Open |
| S3-T13 | No data loss after a power cut | A Mac that can lose power | [s3-database-tasks.md](plans/one-click/s3-database-tasks.md), Section 7.2 | Open |
| S3-T14 | The database on a real Linux desktop | A Linux desktop with PostgreSQL 17 | [s3-database-tasks.md](plans/one-click/s3-database-tasks.md), Section 7.2 | Open |
| S4-T10 | The systemd user unit on Linux. The Local Network prompt on a Mac | A Linux desktop. A Mac with a LAN device | [s4-local-hub-tasks.md](plans/one-click/s4-local-hub-tasks.md), Section 7.2 | Open |
| S6-1 | The first run in the desktop window: create a hub, TOTP, the key copy, an invite | This Mac and a phone | [s6-first-run-tasks.md](plans/one-click/s6-first-run-tasks.md), Section 6.3, check 1 | Open |
| S6-2 | Join from a second Mac, then revoke | A second Mac | Section 6.3, check 2 | Open |
| S6-3 | The first run on Linux with the systemd user unit; finding F-10 | A Linux desktop | Section 6.3, check 3 | Open |
| S6-4 | No port mapping on a real router; no TCP port answers from outside | The router of the hub computer, and a machine outside the network | Section 6.3, check 4 | Open |
| S6-5 | A relayed path shows `RELAYED` | A network where UDP is blocked | Section 6.3, check 5 | Open |
| S8-1 to S8-8 | Stay awake on macOS: the tray, keep awake, the set-up, the lid, quit and failure, the battery floor, remove, eight hours (8 checks) | A Mac laptop. A second computer for the `ping` of check 4 | [s8-stay-awake-tasks.md](plans/one-click/s8-stay-awake-tasks.md), Section 6, checks 1 to 8 | Open |
| S8-9 to S8-13 | Stay awake on Linux: the tray, keep awake, the lid on GNOME, quit, the battery (5 checks) | A Linux laptop | Section 6, checks 9 to 13 | Open |

### Command line and MCP

Steps and comments: [GH-60](work-items/device-manager/GH-60.md). A suite on loopback covers the other items of that list (its comment of 2026-11-13). Blocks: the register row of [cli-mcp-plan.md](plans/cli-mcp-plan.md).

| Check | What it proves | Needs | Result |
| --- | --- | --- | --- |
| CLI-1 | `anima mcp` with a real MCP client: the tool list, one read, one change, and the approval request for a tool in state "ask" (MCP-013) | Claude Code and a hub | Open. A first change through the deployed hub was sent on 2026-11-16 (`anima issue comment`); the approval request is not checked |
| CLI-2 | The MCP section in Settings in the real desktop application | The desktop application | Open |
| CLI-3 | Linux: the crate compiles; the Secret Service token store and its file fallback | A Linux desktop | Open |
| CLI-4 | macOS: the Keychain item and its access control | This Mac | Open |
| CLI-5 | TLS against a hub with a bad certificate | A hub with a bad certificate | Open |
| CLI-6 | The list of GH-60 against the production hub: sign-in, `anima ssh`, `anima files`, a confirmation prompt | The production hub | Open. The read commands for GitHub were checked on 2026-11-16 |

### Machines that the checks need

One visit to a machine can close many rows.

| Machine or access | Checks |
| --- | --- |
| A second Mac | O-1, O-2, S1-T14, S3-T12, S6-2 |
| A Linux desktop or laptop | O-7, P-4, S3-T14, S4-T10, S6-3, S8-9 to S8-13, CLI-3 |
| The `x86_64` Linux build computer | S3-T10 |
| This Mac only | S2-T9, S6-1, S8-1 to S8-8, P-3, P-13, CLI-2, CLI-4 |
| A shell on the VPS | P-1, P-16, O-9 |
| Special set-up | O-5 (a relay and a device that sleeps), O-9 (SNMP v3 agent), S3-T13 (a Mac that can lose power), S6-4 (the router), S6-5 (no UDP), P-14 (Prometheus) |

### A real release

| Check | What it proves | Steps | Result |
| --- | --- | --- | --- |
| REL-1 | A real release confirms the signature, the notarization and the AppImage launcher (OD-16 to OD-18) | [design-spec.md](spec/design-spec.md), Section 11 | Open. For macOS 0.2.0 and 0.2.1, the signature and the notarization are checked ([releases.md](procedures/releases.md), "Release record"). Not recorded here: the update of an installed application, and the AppImage launcher on Linux |

## Passed

None.
