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
- The states are Open, Passed, Failed and Deferred. Only the operator, or an agent that the operator told
  the result, can set Passed or Failed.
- Deferred means that the check is put aside because it cannot run now (PLN-014, R-169). Only a
  ruling of the operator sets it. The entry gives the reason and the ruling. It is not deleted, and
  a ruling or the operator sets it to Open again.
- A Failed entry stays in the list until a correction is built. Then its state goes back to Open.
- Move a Passed entry to "Passed" at the end of this document. Do not delete it.
- The operator approved a sorting of the open checks for the hosted hub on 2026-11-18 (R-169). [sort-2026-11-18.md](research/human-checks/sort-2026-11-18.md) gives the reason for each check, and one order of eight groups for the operator. Fifteen checks are Deferred (section "Deferred"). Twenty checks have steps for the hosted hub: each says "On the hosted hub (R-169)".
- The table "Open" has one row for each entry `HT-nnn` that is Open or Failed. A Deferred entry is in the section "Deferred". A Passed entry is in the section "Passed".

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
| HT-010 | Merge a pull request, and a set of pull requests, on GitHub | A GitHub repository that is safe to change, GitHub on the hosted hub, and a sign-in of the command line (OD-25) | Open |
| HT-011 | Run a workflow again, cancel a run and start a workflow on GitHub | A GitHub repository with a workflow, GitHub on the hosted hub, and a sign-in of the command line (OD-25) | Open |
| HT-012 | Set a commit status on GitHub | The repository of HT-010, GitHub on the hosted hub, and a sign-in of the command line (OD-25) | Open |
| HT-013 | An API key cannot merge, control a run or read a file | The hosted hub with GitHub, and two API keys | Open |
| HT-014 | The issue tools of the MCP server, and a workflow that a comment starts | An MCP client, the repository of HT-011, and the application on the hosted hub | Open |
| HT-015 | The Work, Code and History tabs of a project in the client | The application on the hosted hub, with GitHub connected | Open |
| HT-016 | A release build on the Linux builder puts the bundle in its own clone | The Linux builder | Open |
| HT-018 | Each machine with the old program `anima-agent` gets `anima-daemon` and enrols again | Each machine that has the old program, and a daemon on the hosted hub (HM-D16) | Open |
| HT-020 | The desktop application builds on Linux with no lint error (GH-52) | The Linux computer | Open |
| HT-021 | The delay of the session picture in the Linux application (GH-54) | The Linux computer, the Mac, and a hub that has VNC sessions (a local hub) | Open |
| HT-023 | The hash chain of the audit log on the real hub | The hosted hub, and a key for the command line or an MCP client | Open |
| HT-025 | A phone joins a hub with an invite, through the link in the app; the Paste invite and Joining screens | An iPhone and an Android phone with a build of `mobile/m1-shell`, the simulator and the emulator, a hub with an invite | Open |
| HT-026 | The use of AI agent programs from real session files, on a Mac and on a Linux system unit | A hub and a daemon of this build, a Mac with Claude Code or Codex sessions, a Linux computer | Open |
| HT-027 | The computer that signs in is a device of the hosted hub with no question | This computer, the Anima.ing account of HT-022 and its hosted hub | Open |
| HT-028 | The hosted hub is a device of its own device list, and it is Up | This computer, the Anima.ing account of HT-022, and a hub of the pool from a new hub image | Open |
| HT-029 | The computer is Up while it is connected to the link of its hosted hub, and Down after the application quits | This computer, the Anima.ing account of HT-022, a hub of the pool from a new hub image, and an application with the new link program | Open |
| HT-030 | The application on a computer shows its copy of recent data when the hub does not answer | A computer with the installed application, signed in to a hub | Open |
| HT-031 | A hosted hub reaches SSH of a computer on its link, only after the person turned port 22 on | This computer with Remote Login, the Anima.ing account of HT-022, a hub of the pool from a new hub image, and an application with the new link program | Open |
| HT-032 | The containers of a real device: the list and the log (hosted applications, stage 1) | A hub of this build and a device with Docker and an SSH credential in Anima | Open |
| HT-033 | An SSH session on a phone, with the key row | An iPhone and an Android phone with a build of `mobile/m1-shell`, and a hub that has a device with SSH | Open |
| HT-034 | A photo of a phone through the file store: location kept or removed, and a large file | A local hub of this build, a phone photo with a GPS position, a file of about 200 MB | Open |
| HT-035 | A server gets, follows and drops an operational key, with the Keychain of a Mac | A hub of this build, a Mac and a Linux server with the daemon of this build | Open |
| HT-036 | The release workflow of GitHub Actions: a trial, then a release from a push to `main` | The GitHub account of the operator (the three environments and their secrets are set), a Mac with 0.2.1 installed from its DMG, a Linux computer | Open |
| HT-037 | The daemon pushes through the Unix socket of the link, and an old daemon still pushes after its update | A hub in the link mode, and one computer with `anima-link` and `anima-daemon` of this build | Open |
| HT-038 | The details of a container, the state of a Compose project, and the images, volumes and networks of a real device (hosted applications, task H8) | A hub of this build and a device with Docker, a Compose project and an SSH credential in Anima | Open |
| HT-039 | A newer image for a container, with real public registries (hosted applications, task H13) | A hub of this build that can reach the internet, and the device of HT-038 with one image that is older than its tag at Docker Hub | Open |
| HT-040 | A phone with the OwnTracks app sends fixes to the hub, and a file transfer gets its context | A hub of this build that the phone can reach, a phone with the OwnTracks app, a device with SSH in the hub | Open |
| HT-063 | The hub pulls the points of your Dawarich server with an API key | A hub of this build that can reach your Dawarich server over `https`, an API key of your Dawarich account | Open |
| HT-041 | The first formula and the first crate version of `anima` alone, from the release workflow (tasks B8h, R8d) | A crates.io account of the operator, the key of the tap, a public release, a Mac and a Linux computer with no Anima | Open |
| HT-042 | The tab Containers with the details, the newer-image mark, the log tools and the images, volumes and networks (hosted applications, task H9) | A hub of this build, the application of this build, and the device of HT-038 | Open |
| HT-043 | A reminder rule fires and reaches a phone through ntfy, with fixed words for a private record | A hub of this build, a phone with the ntfy app, an ntfy channel with a random topic | Open |
| HT-044 | The application does not start Tailscale on a hosted hub (DEF-005) | This Mac with Tailscale installed and quit, the application of this build, signed in to the hosted hub | Open |
| HT-045 | The Linux app opens the account sign-in screen when no hub is chosen (HMD-022) | The local Linux AppImage from `fix/linux-start-sign-in` | Passed |
| HT-051 | The workflows `gates` and `merge-to-dev` on GitHub: the first run and the first merge with no session | The repository on GitHub, and the settings of [quality/README.md](../quality/README.md) | Open |
| HT-046 | The iOS application from the release workflow to TestFlight (task B8i) | The Apple Developer account and App Store Connect of the operator, the two environments set, the archive script of M5-4 on `main`, an iPhone with TestFlight | Open |
| HT-047 | The Devices page after the interface review: name first, named connect buttons, the logs as one line, the hub named in the bar (R-224) | The desktop application of a build that holds the branch `ui/v2-devices`, signed in to a hub with some devices | Open |
| HT-048 | The tab Overview of a device after the interface review: the facts as one strip, cards with no border (R-224) | The desktop application of a build that holds the branch `ui/v2-device-overview`, a device with checks and metrics | Open |
| HT-049 | The application carries `anima-daemon` (HMD-052, task HM-T16) | This Mac with the tools of a trial bundle; for step 4 a release | Open |
| HT-050 | The device link runs as a unit of the user (HMD-058, task HM-T24 part a) | A Mac or a Linux computer with a device link that joined a hub, and `anima-link` of this build | Open |
| HT-060 | The Projects page after the interface review: the choice of the group, tiles with no border (R-224) | The desktop application of a build that holds the branch `ui/v2-projects`, the extension Projects on, GitHub connected | Open |
| HT-061 | Sign in to a hosted hub, and the metrics of this computer appear (HMD-053, HMD-054; tasks HM-T17 to HM-T19) | This Mac and the Linux computer, each with an application of this build that carries `anima-daemon` and `anima-link`, signed in to the hosted hub | Open |
| HT-057 | The Settings page after the interface review: the index and the sections in three groups (R-224) | The desktop application of a build that holds the branch `ui/v2-settings`, and a phone | Open |
| HT-052 | The tab Containers of a device after the interface review: one State column, one pane for the log and the details (R-224) | The desktop application of a build that holds the branch `ui/v2-device-containers`, a device with Docker and an SSH credential | Open |
| HT-055 | The sign-in with the Anima.ing account on a phone, to the hosted hub | The iPhone simulator or the Android emulator with a build of `mobile/account-sign-in`, then a real iPhone and a real Android phone, and an Anima.ing account | Open |
| HT-053 | The iOS archive with a signature, and the build from TestFlight on a real iPhone | This Mac with the Apple Distribution certificate in the login keychain and the App Store profile of `ing.anima.mobile`, App Store Connect, and a real iPhone | Open |
| HT-054 | The sign-in inside the iPhone application, through WorkOS | The WorkOS dashboard, a build of `mobile/native-sign-in` on the iPhone simulator or a real iPhone, and an Anima.ing account | Open |
| HT-067 | The tab Overview of a device with the cards Connect and About, the chart figures and the storage table (R-224) | The desktop application of a build that holds the branch `ui/v2-overview-full`; a Linux device with a daemon and a drive that is not mounted; a device with a web interface | Open |
| HT-065 | The measurements of the isolation design on Linux (task W2): a job in a rootless container on the computer `pop-os` | The computer `pop-os`, a terminal there, and one `sudo` for the installation of Podman | Open |
| HT-062 | `anima mcp install` with the real coding agents (MCP-027 to MCP-038) | A Mac and a Linux computer with the `anima` of a build that holds the branch `mcp/install`, signed in, and the agents of Section 12.2 of the command line plan that you use | Open |
| HT-056 | The bar: Live sessions at the foot, Extensions as its own page above Settings (R-255) | The desktop application of a build that holds the branch `ui/bar-foot`, and a phone | Open |
| HT-059 | The application hands its link to a unit of the user on a hosted hub (HMD-058, task HM-T24 parts b and c) | This Mac, an application of this build that carries `anima-link`, the sign-in to the hosted hub | Open |
| HT-068 | A Linux computer whose `~/.local/bin` its group can write gets the link as a unit and the daemon, and shows metrics (DEF-007) | The Linux computer `pop-os`, an AppImage that holds the branch `fix/link-private-folder`, the sign-in to the hosted hub; this Mac with its unit of before | Open |
| HT-066 | A browser gets the session of a hosted hub through the gateway (HMD-061, HMD-062; tasks HM-T25, HM-T26) | A hub of the pool with an image of this build, and the gateway of the pool at `hub.anima.ing` | Open |

### What a check on the hosted hub needs

Many entries were written for the hub on the VPS. The operator uses only the hosted hub (R-130) and starts fresh on it (R-140). An entry that R-169 changed has a paragraph "On the hosted hub (R-169)". Those paragraphs use these four terms.

- The application on the hosted hub: the desktop application, signed in with the Anima.ing account as in HT-022. It replaces `https://hub.anima.ing/` and a browser on the VPN.
- A key for the command line: `anima auth login` asks for a user name, a password and an authenticator code, and a hosted hub has no such account (OD-25 of the [design specification](spec/design-spec.md), open). Make an API key in Settings of the application, and give it to the command line with `anima auth login --with-token` or in `ANIMA_TOKEN`. How the command line finds the address of a hosted hub is not verified: write down what you used. An API key cannot make a change that D-23 refuses (a merge, a run control, a commit status, the content of a file). A step with such a change waits for OD-25.
- GitHub on the hosted hub: connect GitHub on the hosted hub first. The connection of the hub on the VPS does not move (R-140). No check connected GitHub on a hosted hub: not verified.
- A daemon on the hosted hub: the `anima-daemon` of a computer, enrolled at the hosted hub. No document shows such an enrolment and no check ran one (HM-D16 of [hosted-mode.md](proposals/hosted-mode.md), open). If the hub refuses the enrolment, write that in the entry and in HM-D16. Where the entry permits a local hub, do the check with a local hub.

The steps for a hub with a user name and a password (the self-host path) stay in each entry.

### HT-066: a browser gets the session of a hosted hub through the gateway (HMD-061, HMD-062; tasks HM-T25, HM-T26)

Built: on the branch `hosted/hm-t26-browser-session`. A hosted hub gives the Anima.ing cloud a
one-time ticket (`hostedBrowserTicket`), and trades it for its own session cookie when the request
comes from its public origin (`hostedBrowserSignIn`). The session ends 12 hours after the sign-in,
or after 30 minutes with no request.

The tests cover, through the routes of the hub as it serves them: the ticket only for the key of the
cloud; the trade only from the public origin, once; the cookie and its attributes; a request with
that cookie that is signed in; a foreign host name or origin, refused; a hub that is not hosted;
the rule of the 30 minutes as a rule of time. Not covered: a real gateway and a real browser, a
session that ends after 30 minutes on a real hub, a terminal that stays open for longer than that,
and the web client on a hosted hub (task HM-T27).

It needs: the gateway of the pool, and a hub of the pool whose image has this build. The steps are
for the time when both exist.

1. In a browser with no session, open `https://hub.anima.ing`. Sign in with the account. Expected:
   the hub of the account opens. No password of a hub is asked.
2. In the tools of the browser, look at the cookies of `hub.anima.ing`. Expected: `__Host-anima`,
   with `Secure`, `HttpOnly` and `SameSite=Strict`, that ends in 12 hours.
3. Leave the tab with no page that updates (for example a settings page) for 31 minutes. Press a
   link. Expected: the gateway asks the hub for a new session with no question, or the sign-in of
   the account shows; no page of the hub shows with the old session.
4. Open a terminal session to a device and leave it open for 40 minutes with no key pressed. Press
   a key. Expected: the session still works.
5. In a second browser profile, open `https://hub.anima.ing/graphql` with no session. Expected: no
   data of a hub.

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

### HT-010: merge a pull request, and a set of pull requests, on GitHub

Built: the hub change `merge_pull_request` and `anima pr merge` (PRJ-033, OD-E14). The tests use
a scripted GitHub. No pull request was merged on GitHub. The only change sent to GitHub so far is
one issue comment (2026-11-16).

Use a repository that is safe to change. Make four pull requests in it that can merge, one pull
request that has a conflict, and, if you can, one pull request from a fork. On a hub with a user
name and a password, sign in with `anima auth login`.

On the hosted hub (R-169): connect GitHub on the hosted hub first. Step 1 is a read: do it with a key
for the command line. Each `anima pr merge` of steps 2 to 6 needs a sign-in of the command line,
because the hub refuses a merge from an API key (D-23, HT-013). Thus steps 2 to 7 wait for OD-25.
Until then, step 6 of HT-015 does one merge through the hosted hub with the button of the
application.

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
fail on request (for example, an input `fail` that makes one job fail). On a hub with a user name
and a password, sign in with `anima auth login`.

On the hosted hub (R-169): connect GitHub on the hosted hub first. Steps 1 and 3 are reads: do them
with a key for the command line. Steps 2 and 4 to 7 are run controls, which the hub refuses from an
API key (D-23, HT-013). Thus they wait for OD-25, and step 8 with them. Until then, step 7 of HT-015
runs a workflow again and cancels a run through the hosted hub with the buttons of the application.

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

On the hosted hub (R-169): connect GitHub on the hosted hub first, and use the repository of HT-010.
Step 1 is a read: do it with a key for the command line. Steps 2 to 4 set a commit status, which the
hub refuses from an API key (D-23, HT-013). Thus they wait for OD-25.

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

On the hosted hub (R-169): connect GitHub on the hosted hub first. `anima key create` (step 1) and
`anima key revoke` (step 7) need a sign-in of the command line (OD-25). Until then, make and revoke
the two keys in Settings of the application, with the same name, scope, kind (user or agent) and
expiry. If Settings does not offer one of these values, write that down. Steps 2 to 6 use the keys
themselves and run as they are written. Step 5 expects that a write key of an agent can make a
comment.

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

On the hosted hub (R-169): connect GitHub on the hosted hub first. Make the write API key of step 1
in Settings of the application on the hosted hub, for an agent. Do step 2 in that application.

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

On the hosted hub (R-169): the hub of the account must come from a hub image that has the tabs.
Which image has them is not verified: if the project page shows no tabs, write that down and stop.
Connect GitHub on the hosted hub first. The `anima` commands of steps 2 and 4 are reads: do them with
a key for the command line. Steps 6 and 7 are also the only way to do a merge and a run control on a
hosted hub until OD-25 is decided (HT-010, HT-011).

On the self-host path, the hub must be of a commit that has the tabs, and a change to the hub on the
VPS needs the confirmation of the operator (VPS-005).

1. Open the application on the hosted hub. (Self-host path: open `https://hub.anima.ing/` on a
   computer on the VPN, or the desktop application of the same build.) Open the project of a
   repository with pull requests, issues and workflow runs.
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

Result: Open.

### HT-018: each machine with the old program gets `anima-daemon` and enrols again

Built: the program `anima-agent` is `anima-daemon` ([DMN-1](work-items/device-manager/DMN-1.md), R-126). The hub
accepts only `/daemon/v1/...` and a token that starts with `anima_daemon_`. `anima-daemon install`
stops and deletes the unit of the old program (`anima-agent.service`, or `ing.anima.agent` on macOS).
Migration 0717 renames the table and the metrics source. Migration 0718 deletes each old token
and each enrolment code, so no old token is valid under a new spelling.

The tests use a fake hub and a planned list of commands. No test ran on a machine that has the old
program, and no test ran the migration on the database of the real hub.

On the hosted hub (R-169): the hub on the VPS is not deployed again, and the hosted hub starts with no
device (R-140). Steps 1 and 2 are for the self-host path only: do not do them, and write "not done
(R-169)" for them in the result. Do steps 3 to 7. Step 5 needs a daemon on the hosted hub (HM-D16, not
verified): make the code in the application on the hosted hub. Step 6 is a read: do it with a key for
the command line. The migrations 0717 and 0718 on a database with old tokens are thus not checked by a
person.

1. Self-host path only. Deploy the hub of this build (VPS-005). Expected: the hub starts, and migrations 0717 and 0718 are in its
   log with no error.
2. Self-host path only. Open the client. Expected: each device that had the source "Agent" shows the source "Daemon"
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

### HT-020: the desktop application builds on Linux with no lint error (GH-52)

Checked, not built: [GH-52](work-items/defects/GH-52.md) reported that the Linux build stops at an
unfulfilled `dead_code` expectation in `apps/desktop/src-tauri/src/web_view/event.rs`. A simulated
build on a Mac gave no error with Rust 1.99.1 and Rust 1.100.0. No code was changed.

No gate runs on Linux, and the Mac has no Linux target.

1. On the Linux machine, get `main` and run `rustc --version` in the repository.
   Expected: `rustc 1.100.0` (`rust-toolchain.toml`).
2. Run `cargo clippy -p anima-app --all-targets -- -D warnings`.
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

### HT-027: the computer that signs in is a device of the hosted hub (HM-T8)

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

Result: Open. First try on 2026-11-18, after the hub of the account was made again from the new image
(operator, with a picture of the device list: "the hub is an ubuntu server but otherwise it looks good"): the
list showed one row for this Mac, Unknown, "No checks yet", with a host that starts with `fd41:4e49:4d41:`, and
the log showed "Device ... added" with no question. The app made the new hub and joined its link with no step,
although it held the link and the token of the deleted hub. Not seen in the picture: steps 2 and 3.

Linux check passed on 2026-11-19. The Devices list showed one `pop-os` row after sign-in,
and the operator confirmed that it still showed one row after reopening the AppImage. The
operator's screenshot showed `pop-os` with a link address while the `Link` site filter was
selected, which confirms the site. The Mac device details and restart checks above are open.

### HT-028: the hosted hub is a device of its own device list (HM-T9)

Built: a hosted hub adds itself to its devices at each start, when no device has the host `localhost`
([hosted-mode.md](proposals/hosted-mode.md) HMD-045, R-137). The device has the name `Hub` (`Hub (hosted)` when
a different device has that name), the kind `hub`, the operating system Linux and the site `Link`. It has no
SSH, RDP or VNC. It has one PUSH check, `Hub running`. The hub reports that check as Up at the start, and then
two times in each interval of the check. The parts are `crates/hub/src/fleet/devices/hub_device.rs` (the
device), `crates/hub/src/fleet/checks/push/own.rs` (the check and its report) and
`crates/hub/src/upkeep/hub_device.rs` (the start and the timer). A hub that is not hosted adds nothing.

The tests start the hub program in the hosted mode on a test database. No test uses a hub of the pool that was
made from a new hub image, and no test looks at the row in the application. No test waits for the timer: the
tests see the report of the start only.

Checked on the pool on 2026-11-18 (operator: "Yes, test it"), with a hub image built from `eb2c7461` and a test
hub that is deleted: a device joined the link and traded its claim, and the device list held one device `Hub`
(host `localhost`, kind `hub`, Linux, site `Link`) that was Up, with the check `Hub running` reported at the
start and again 60 s later. Not checked: the row in the application, and the hub of the account, which still
runs the programs of the old image.

Do these steps after the hub of the account was made again from the new hub image.

1. Start the built app as in step 1 of HT-022, and sign in if the app asks. Expected: the device list shows a
   row `Hub`, and the row is Up. The row has no button to connect. Nothing asked for a step.
2. Open the device `Hub`. Expected: it has one check, `Hub running`, and the check is Up.
3. Wait 5 minutes and look at the device again. Expected: the device and the check are Up.

Result: Open. First try on 2026-11-18 (operator, with a picture of the device list: "the hub is an ubuntu
server but otherwise it looks good"): the row `Hub` was Up, 100.00 %, with the host `localhost` and "No
metrics". Two differences. The row shows the mark of Linux with no distribution: the hub reads a distribution
over SSH only, and it has no SSH to itself (the image is Debian 13). The row shows the button "+ VNC", which
adds a protocol; step 1 expected no button. Not seen in the picture: steps 2 and 3.

On 2026-11-19 the operator's screenshot showed the `Hub` device Up and its single Push check,
`Hub running`, also Up. The operator confirmed that `Hub running` was still Up on the recheck
more than five minutes later. The health check passed. The connection buttons in step 1 remain
open under HMD-046.

### HT-029: the computer is Up while it is connected to the link (HM-T11)

Built: a hosted hub keeps the check `Connected` for each device whose host is the link address of a computer
on its link ([hosted-mode.md](proposals/hosted-mode.md) HMD-047, R-143). The link program of the computer
(`anima-link device`, `link/devicelink/presence.go`) keeps one stream open to port 446 of the hub link and
writes the line `here` on it each 15 seconds. The hub link (`link/presence/presence.go`, `link/hublink/hublink.go`) records the time of
the last line, and `devices.list` gives it as `seen`. Each 15 seconds the hub
(`crates/hub/src/upkeep/link_presence.rs`) reads that list. For a computer that was seen in the last
45 seconds it adds the PUSH check `Connected` to the device, if the device does not have it, and reports the
check as Up. For a computer that was seen before and is not seen any more it reports the check as Down one
time, with the message "Not connected"; after that the check gets no report, and the retries of the check
apply. The check opens no port of the computer.

The application starts `anima-link device` and stops it when the application stops
(`apps/desktop/src-tauri/src/link/device.rs`). Thus the computer is Down while the application is closed.

After a start, and after a sleep, the hub does not count the time in which it did not run: each check
`Connected` waits one whole interval again, and no computer is reported Down in the first 45 seconds.

The tests use a hub link and a device link in one process, and the hub program beside a fake control socket.
No test uses a hub of the pool that was made from a new hub image, the real application with the new link
program, or a hub that sleeps and wakes. No test looks at the row in the application. A hub or an application
with the old link program is not tested on a real link: the tests show only that the device link stays in
operation when the hub link refuses port 446.

Do these steps after the hub of the account was made again from the new hub image, with an application that
was built from a checkout that has this change (`link/build.sh` makes the link program).

1. Start the built app as in step 1 of HT-022, and sign in if the app asks. Wait 1 minute. Expected: the row
   of this Mac is Up. It does not show "No checks yet".
2. Open the device of this Mac. Expected: it has one check, `Connected`, and the check is Up.
3. Wait 5 minutes with the app open. Expected: the device and the check are Up, and the history of the check
   shows no Down.
4. Quit the app. Wait 3 minutes. Look at the device list of the hub from a different computer or a phone, or
   run `anima devices list` against the hub. Expected: the row of this Mac is Down. The history of the check
   shows "Not connected". If no second client is available, do step 5 and look at the history of the check
   there.
5. Start the app again. Expected: the row of this Mac is Up again in 1 minute.
6. Only for a free hub that sleeps: quit each client of the account, wait until the cloud says that the hub
   sleeps, wait 10 minutes more, and start the app. Expected: the hub wakes and the row of this Mac is Up. The
   history of the check `Connected` shows no Down for the time in which the hub slept. The history of the
   check `Hub running` shows none for that time.

Result: Open.

On 2026-11-19 the operator saw `pop-os` go Down in Anima on the Mac after quitting the
Linux AppImage. After reopening the AppImage, `pop-os` came back Up. The Linux disconnect
and reconnect behaved as expected. The check history and the free-hub sleep and wake are open.

### HT-031: a hosted hub reaches SSH of a computer on its link (HM-T12)

Built: the path from a hub to a local TCP port of a computer on its link
([hosted-mode.md](proposals/hosted-mode.md) HMD-048, HMD-049, R-143).

- The hub link asks for a port on the stream that the computer keeps open to port 446, and the computer gives
  the connection on a new stream to port 447 of the hub link (`link/dial/`, `link/hublink/hublink.go`,
  `link/devicelink/presence.go`). The computer connects to the port on its own loopback address only, and only
  when the port is in its list of open ports (`link/ports/ports.go`). The list is the file `open-ports` in the
  state directory of the link program of the computer. It is empty at first, and the hub cannot change it.
- `anima-link ports -state <DIR>` shows the list. `anima-link ports -state <DIR> on 22` and `off 22` change
  it, also while the link program runs (`link/cmd/anima-link/device.go`). The ports can be 22 and 5900, and
  3389 on Windows only. The switches in the application are task HM-T13: until then, this command is the only
  way to turn a port on.
- The hub link makes the socket `dial.sock` beside its control socket. The hub connects through it with its
  one connect function (`crates/hub/src/fleet/targets/reached.rs`) when the address is a link address. The TCP
  checks, the TLS checks and each SSH connection of the hub (terminal, files, metrics, the checks over SSH, the
  connection test) use that function. A check that needs ICMP or UDP (Ping, SNMP) on a computer of the link is
  Unknown, with a message that says that the link carries TCP only.
- Not built: VNC and RDP sessions through the path (HM-T14), the switches (HM-T13), Windows (HM-T15).
  Wake-on-LAN is not possible for a computer of the link.

The tests use a hub link and a device link in one process with a relay on loopback, the two programs with the
command `ports`, and the hub beside a fake dial socket that connects to a port on loopback. No test uses a hub
of the pool from a new hub image, the real relay, the SSH server of macOS (Remote Login), or the terminal of
the application. No test uses the hub program and the link program together.

Do these steps after the hub of the account was made again from the new hub image, with an application that
was built from a checkout that has this change (`link/build.sh` makes the link program). HT-029 must show the
row of this Mac as Up first. `<LINK>` is the link program that the application carries, and `<DIR>` is the
state directory that the application gives it: read the two from the output of
`ps ax | grep "anima-link device"` while the application runs.

1. On this Mac, turn on System Settings, General, Sharing, Remote Login. Run `<LINK> ports -state <DIR>`.
   Expected: the answer is `{"open":[]}`.
2. In the application, open the device of this Mac and add a TCP check of port 22. Wait 1 minute. Expected:
   the check is Down, and its message is "the connection failed: permission denied".
3. Run `<LINK> ports -state <DIR> on 22`. Expected: the answer is `{"open":[22]}`. Do not start the
   application again. Wait 1 minute. Expected: the TCP check is Up.
4. On the device of this Mac, add SSH with the user of this Mac and its password or key, and run the
   connection test. Expected: the test shows the host key of this Mac (compare with
   `ssh-keyscan -t ed25519 localhost | ssh-keygen -lf -`). Accept the key.
5. Open a terminal session on the device. Run `hostname`. Expected: the name of this Mac. Open the files of the
   device. Expected: the home folder of the user.
6. Wait 5 minutes with the terminal open, and type in it now and then. Expected: the session does not end.
7. Add a Ping check to the device. Wait 1 minute. Expected: the check is Unknown, not Down, and its message
   says that a computer of the link is reached with TCP only.
8. Run `<LINK> ports -state <DIR> off 22`. Expected: the answer is `{"open":[]}`. Open a new terminal
   session. Expected: it does not open. In 1 minute the TCP check is Down again. The check `Connected` stays Up.
9. Run `<LINK> ports -state <DIR> on 80`. Expected: the command refuses, and the list stays empty.
10. Turn Remote Login off again if it was off before step 1, and remove the checks of steps 2 and 7.

Result: Open.

### HT-023: the hash chain of the audit log on the real hub (GH-2 L0, LOG-021 to LOG-028)

Built: migration `0506_events_audit_chain.sql` and the hash chain of the audit log
([design-spec.md](spec/design-spec.md) Section 3.8.2). At its first start the hub links each audit entry
that exists. `anima audit verify` and the MCP tool `anima_audit_verify` check the chain.

The tests use a throwaway database with a few entries. No test ran the migration and the first link
on the real hub, with its year of entries, and no test ran the daily check there.

On the hosted hub (R-169): step 1 links the entries that a hub had before the migration. The hosted
hub is new (R-140), so step 1 is for the self-host path only: do not do it, and write "not done
(R-169)" for it in the result. The first link of the year of entries of the hub on the VPS is thus not
checked by a person. Do steps 2 to 4 on the hosted hub. For `anima audit verify`, use a key for the
command line; if the hub refuses the key, use the MCP tool `anima_audit_verify` and write the refusal
down. If an API key can read the audit log is not verified.

1. Self-host path only. Update the hub, start it, and look at its process log.
   Expected: the hub starts. When the audit log had entries, one line says "audit entries linked in
   the hash chain", with the count.
2. Run `anima audit verify`.
   Expected: "Chain whole". "Links" is the number of audit entries. "Head" shows a number and 64
   hexadecimal characters. Write the head down outside the hub.
3. Change something in the client, for example a tag of a device, and run `anima audit verify` again.
   Expected: "Chain whole", with one more link than in step 2.
4. The next day, open the Logs panel and filter for Error.
   Expected: no event that says "The audit log chain is broken".

Result: Open.

### HT-033: an SSH session on a phone, with the key row

Built: on the branch `mobile/m1-shell` (mobile plan M4-1, MOB-020, RA-050; mockup 8). In the phone
layout an SSH session fills the screen. A bar holds back, the device and the time, "Hide keyboard"
and the session menu. A row of keys (Esc, Tab, Ctrl, Alt, the arrows, `| ~ / -`) sits over the
on-screen keyboard. Ctrl and Alt latch for the next key.

The tests cover the bytes of the latched keys and the time text. On 2026-11-17 a throwaway SSH
server that ran only `cat -v` was used on the iPhone 17 Pro simulator and on the Android emulator:
Ctrl then L gave `^L`, typing worked on iOS, and back kept the session open. No real phone and no
real shell were used.

Later on 2026-11-17 the Android keyboard plugin (`apps/desktop/keyboard`) and the emptied input box
(`sessions/screen/operator/typing.ts`) were added. On the Android emulator with Gboard the key row then
sat directly above the keyboard, and keys tapped on Gboard after typing in the device search arrived in
order ("hello wo" and Backspace gave "hello w"). Before, the keyboard covered the key row and that text
came out as "elloh". The same typing was correct on the iPhone simulator. Only Gboard was tried.

1. Install a build of this branch on an iPhone. Sign in to a hub that has a device with SSH set
   up, and open the device. Tap SSH. Expected: the session fills the screen, the bar shows the
   device name and "SSH" with a time that counts up, and the navigation bar is hidden.
2. Tap the terminal. Expected: the keyboard opens, and the key row is directly above it.
3. Type `ls` and Return. Expected: the list of files. Tap Ctrl, then type `c` while a command
   runs (for example `sleep 30`). Expected: the command stops, and Ctrl is no longer highlighted.
4. Tap the up arrow of the row. Expected: the shell shows the last command again.
5. Tap "Hide keyboard". Expected: the keyboard closes and the key row moves to the bottom of the
   screen.
6. Tap back. Expected: the device list with the navigation bar. The session is still open.
7. Do steps 1 to 6 on an Android phone. Expected: the same results. Before step 1, type a word in
   the device search; then in step 3 type a full sentence. Expected: the text arrives in the order
   typed. If the phone has a second keyboard app (for example SwiftKey or Samsung Keyboard), do step 3
   with it too.
8. On the Android phone, tap the device search. Expected: the navigation bar sits directly above the
   keyboard, with no gap between them.

Result: Open.

### HT-025: a phone joins a hub with an invite, through the link in the app

Built: on the branch `mobile/m1-shell` (mobile plan M1-4, M3-1 and M3-2, MOB-031 to MOB-033, MOB-035, MOB-036). The Go link
runs inside the phone app as a library (`link/cmd/phonelink`, built by `link/build-phone.sh`). "Join a hub", then
"Join with an invite", joins the phone to a hub over the link, and the app reaches the hub at a loopback address.

The tests cover the error codes of the library, a join with text that is not an invite, and a start with no hub. On
2026-11-17 the Android emulator reached the link and wrote its folder, but the emulator had no route to the internet,
so the relay never answered. The iPhone simulator stopped answering before the join. No phone joined a hub.

The "Paste invite" and "Joining the hub" screens (M3-2, `31378b0d`, `6ae435e3`) follow frame 2 of
`docs/mockup/mobile/index.html`. On 2026-11-18 they were measured in a desktop browser at 390 by 844 px, dark; the
joining view had a stand-in for the commands of the shell. They were not seen on the iPhone simulator, on the Android
emulator or on a phone: the keyboard, the safe areas, the paste permission of iOS and the wait for a real hub are not
covered. Steps 6 to 8 are for the simulator and the emulator and need no hub; steps 1 to 5 need real phones.

1. Build this branch for an iPhone and an Android phone (`link/build-phone.sh`, then the Tauri build). On the hub, in
   Settings, make an invite for a new device.
2. On the phone, copy the invite, open the app, tap "Join a hub", then "Join with an invite". Tap Paste, then
   "Join hub". Expected: Paste puts the invite in the field (iOS may ask to allow the paste first); "Joining the hub"
   shows four steps, and "Invite read", "Link started" (with the device key under it) and "Waiting for hub" get a
   check one after the other, within a minute. Tap "Sign in". Expected: the sign-in page of the hub opens.
3. Sign in. Expected: the device list.
4. Put the app in the background for one minute, then open it again. Expected: the device list loads again, with no
   new invite.
5. Settings, Leave hub. Expected: the welcome screen. On the hub, the phone is no longer on the link.
6. On the iPhone simulator and on the Android emulator, open a build of this branch, tap "Join a hub", then "Join
   with an invite". Open `docs/mockup/mobile/index.html` beside it. Expected: the screen looks as the "Paste invite"
   frame of row 2: Back, the title, the field, Paste with a line around it, "Join hub" and "Works once. Not kept.",
   with the same space at each side.
7. Tap the field. Expected: the keyboard opens, the page does not zoom, and "Join hub" stays above the keyboard. Type
   `abc` and tap "Join hub". Expected: a line under the field says that the text is not an invite, and `abc` stays in
   the field.
8. Copy some text in a different app, come back and tap Paste. Expected: the text is in the field (iOS asks to allow
   the paste first). Tap Back. Expected: the screen before it.
9. On each real phone, during step 2, look at "Joining the hub" beside the "Joining" frame of row 2. Expected: four
   rows with a mark each, the device key in machine text under "Link started", and Cancel low on the screen, above the
   home bar. Turn the hub off before a join with a new invite. Expected: after a minute "Waiting for hub" has a red
   mark, a line says that the hub did not answer, and Retry replaces Cancel.

Result: Open.

### HT-032: the containers of a real device, and the log of one (HOST-001, HOST-013; tasks H1 to H4)

Built: the hub domain `crates/hub/src/work/containers/` (`Query.containers`, `Query.containerLog`),
`anima containers list` and `anima containers logs`, the MCP tool `anima_container_list`, and the tab
"Containers" of the device page (`apps/client/src/lib/containers/`). The tests use a test SSH server
that answers with fixed text, a scripted hub and a fake of the hub calls. No test ran the `docker`
commands on a device: the form of the list (`docker ps --format`) ran only against a Docker with no
container, and `docker logs` did not run. No person saw the tab or compared it with its mockup
(`docs/mockup/containers/index.html`).

The check only reads. It starts, stops and changes no container and no server (VPS-005, R-050). The
hub must be of this build; a local hub is sufficient
([setup/local-hub.md](procedures/setup/local-hub.md)). Use a device that runs Docker with at least one
Compose project, with SSH on, its host key accepted and its SSH credential stored in Anima. The SSH
user must be able to run `docker ps`.

1. Run `anima containers list <device>`. Expected: one row for each container that `docker ps -a`
   shows on the device, with the same name, state, status text, image and ports, and the name of
   the Compose project in the last column.
2. Run `anima containers list <device> --json`. Expected: `createdAt` of each container is the time
   that `docker ps` shows, in UTC, and is not null.
3. Run `anima containers logs <device> <name of a container> --lines 20`. Expected: the last 20
   lines of `docker logs --tail 20 <name>`, lines of standard error included, oldest first.
4. Run the same command with `--timestamps`. Expected: each line starts with its time.
5. Run `anima containers logs <device> nosuchname`. Expected: "The device has no container named
   nosuchname." and no log.
6. Run `anima containers logs` with an API key in `ANIMA_TOKEN`. Expected: the hub refuses with
   FORBIDDEN. Run `anima containers list` with the same key. Expected: the list.
7. In an MCP client, call `anima_container_list` for the device. Expected: the same containers. No
   tool gives a log.
8. Open the device in the client and select the tab "Containers". Expected: the containers in
   groups by Compose project, with the running count of each group, and the containers with no
   project last.
9. Select "Log" in one row. Expected: the last 200 lines below the table. Select 1000 lines.
   Expected: more lines, if the container has them. A line with colour codes of a terminal shows
   as text and changes nothing of the page.
10. Compare the tab with its mockup at 1040 px, in the light and the dark appearance. Write down each
    difference.
11. Open the tab for a device with no SSH credential in Anima, and for a device with no Docker.
    Expected: a sentence that says so, and "Retry".
12. Read the log of the hub for the time of the steps. Expected: no name of a container, no image
    and no line of a container log (LOG-007).
13. On the device, run `docker ps -a` again. Expected: no container changed its state because of the
    steps.

Result: Open.

### HT-026: the use of AI agent programs from real session files

Built: `crates/daemon-core/src/agent_usage/`, `crates/daemon/src/readers/agent_sessions/`, the hub
domain `crates/hub/src/fleet/agent_usage/` with the migration `1020_agent_usage_schema.sql`,
`anima agents usage`, the MCP tool `anima_agents_usage` and the page "Agent usage" (AIS-018 to
AIS-024, tasks AU-1 to AU-4). The tests use synthetic session files, a scripted hub and the mock
hub. The push test of the daemon runs only on Linux and did not run on the Mac of the build. The
browser specs of the page did not run. No person compared the mockup with the page.

The hub and the daemon must be of this build. A local hub is sufficient
([setup/local-hub.md](procedures/setup/local-hub.md)).

1. Start a hub of this build. On a Mac that has sessions of Claude Code or Codex, enrol and start
   the `anima-daemon` of this build as the launchd agent of the user.
2. Wait 2 minutes. Run `anima agents usage`. Expected: one row for the Mac and each program, with
   sessions, tokens and an estimate. macOS shows no window that asks for access to a folder.
3. Run `anima agents usage --by model,day --days 7`. Compare the rows of one day with what you know
   you ran on that day. Expected: the models are the models that you used.
4. Run `anima agents usage --by repository`. Expected: the names of your repositories, in the form
   `host/owner/name`, with no user name, no token and no path of a folder.
5. Open the page "Agent usage" in the client. Expected: the same numbers as step 2, and under the
   table the text that the cost is an estimate and not the bill of a subscription.
6. Compare the page with its mockup (`docs/mockup/agent-usage/index.html`). Write down each
   difference.
7. Read the log of the daemon. Expected: no text of a session, no title and no path of a folder
   (LOG-007).
8. Add one folder to `excludedFolders` in `agent-usage.json` in the folder of the daemon. Start a
   new session of an AI agent program in that folder, and start the daemon again. Wait 2 minutes.
   Expected: the new session is not in `anima agents usage`.
9. Set `off` in the same file and start the daemon again. Expected: the command and the page say
   that the counts are off in the settings of the daemon of this device.
10. On a Linux computer, install the daemon as a system unit and start it. Expected: the command
    and the page say "This daemon runs as a system unit and cannot see the sessions of a user."
11. Stop and start the daemon of the Mac. Wait 2 minutes. Expected: the numbers of step 2 do not
    change, apart from new use.

Result: Open.

### HT-034: a photo of a phone through the file store (FIL-001 to FIL-005, E0-5 T2 and T4)

Built: on the branch of E0-5 T2 and T4 (files part). The hub keeps files as sealed pieces in
`ANIMA_FILES_DIR`, sealed with a data key of `ANIMA_DATA_KEY_DIR`; the local hub gives both folders in
its root. `anima file upload|download|list|remove` and `anima file archive add|remove` use them.

The tests use a JPEG that the test makes, a generated body of 256 MB + 1 byte, and a store in a
temporary folder. Not covered: a photo of a real camera (with its maker notes, XMP and thumbnail), a
viewer of the operating system, a real network, and a hub that runs as the installed program.

1. Install the local hub of this build and start it. Expected: the root of the hub has the folders
   `data-keys/` and `files/` after the first upload, and `data-keys/engine.key` has the mode 0600.
2. Take a photo with a phone that records the location. Copy it as a JPEG to this computer. Run
   `anima file upload <photo> --owner book:1 --keep-location`. Expected: the answer shows the size and
   the hash; `anima file list book:1` shows the place of the photo in the column LOCATION.
3. Run `anima file download <id> --output copy.jpg`. Expected: "Location removed: yes". Open `copy.jpg`
   in the photo viewer of the system and show its information. Expected: the photo looks the same and
   has no location.
4. Upload the same photo again without `--keep-location`. Expected: LOCATION is `-` for the new file,
   and the folder `files/engine/` holds one piece for the two files.
5. Upload a file of about 200 MB, then download it to a new file, and compare the two with
   `shasum -a 256`. Expected: the same hash. Then try a file larger than 256 MB. Expected: the upload is
   refused, and no new piece is in `files/`.
6. Run `anima file remove <id>` for each file. Expected: the piece goes with the last file.

Result: Open.

### HT-061: sign in to a hosted hub, and the metrics of this computer appear (HMD-053, HMD-054; tasks HM-T17 to HM-T19)

Built: on the branches `hosted/hm-t17-daemon-socket` (the unit of the link makes the socket of the
daemon; the shell enrols the daemon at that socket) and `hosted/hm-t19-daemon-at-sign-in` (the
application enrols and installs the daemon when it opens a hosted hub). They need
the branch `hosted/hm-t24b-shell-link-unit` (the link as a unit that the application watches).

The tests cover: the folder and the socket of the daemon, the arguments of the unit, the origin
`unix:<socket>` for the local address of the link, the search of this computer among the devices,
the one code for each try, the wait for the socket, and each case in which no code is made. Not
covered: a real link, a real daemon, the Keychain and the `LaunchAgent` on a Mac, the user unit and
its sandbox on Linux, the hub of the pool that takes an enrolment and a batch through the link, and
the time until the first numbers show.

It needs: an application of this build on each computer (a trial bundle of this build carries both
programs), and the sign-in to the hosted hub.

On this Mac:

1. Open the application and wait until the Devices page shows. Leave it open for 3 minutes. Nothing
   asks for a password or a question. Expected: after at most 3 minutes the row of this Mac shows
   numbers (processor, memory, disk) in the place of "No metrics".
2. Run `ls -l ~/Library/LaunchAgents/ | grep anima`. Expected: `ing.anima.link.plist` and
   `ing.anima.daemon.plist`.
3. Run `ls -l "$HOME/Library/Application Support/anima-daemon/"`. Expected: `link.sock` (its line
   starts with `srw-------`) and `daemon.json`.
4. Open the device of this Mac, Settings, the panel "Daemon". Expected: "Metrics come from the daemon
   on ...", with a time of the last report that is less than 2 minutes old.
5. Quit the application. Wait 5 minutes. Open it again. Expected: the chart of this Mac has no hole
   for those 5 minutes: the link and the daemon ran with the application closed.
6. Open Agent usage after you used Claude Code or Codex on this Mac. Expected: the page shows counts
   for this Mac (this is also HT-026 on a hosted hub).

On the Linux computer: steps 1, 4 and 5 as written. For step 2 run
`systemctl --user status anima-link.service anima-daemon.service`: both are "active (running)". For
step 3 run `ls -l ~/.config/anima-daemon/`.

Observation (2026-11-19, 19:50 local time; the orchestrating session on the Mac of the operator, a local build 0.8.17 of the first form of these branches, the application already signed in; not a result of the operator): four minutes after the start the link was still the child of the application, with no socket for the daemon; no unit, no copy in `~/.local/bin`, no daemon, and no line in the log. Cause: the hand-over of the link ran only after a sign-in, and the application was signed in already. Corrected on the branch `hosted/hm-t19-daemon-at-sign-in`: the hand-over also runs at each start, and each step writes a line to the log.

Observation (2026-11-20, 04:03 to 04:08 UTC; the orchestrating session on the Mac of the operator, a local build 0.8.17 with the corrected branch, the application already signed in; not a result of the operator): the log said at 04:03:37 "this hub is a hosted one: a unit of the user will keep the device link"; at 04:04:22 "the device link goes to a unit of the user" and "a unit of the user keeps the device link"; `ing.anima.link.plist` was loaded, and the link ran from `~/.local/bin/anima-link` with `-daemon-socket`. At 04:07:31 the log said "the daemon was installed on this computer" and "the daemon of this computer: try 1 of 5: installed"; `ing.anima.daemon.plist` was loaded, and `~/.local/bin/anima-daemon run` ran and was still up a minute later, with the exit status 0 in `launchctl list`. This agrees with steps 2 and 3 on the Mac. Not known at that time: whether the hub received an enrolment and a batch, so step 1 (the numbers on the row) and steps 4 to 6 are open. The first try came about three minutes after the start, later than the plan of this entry: not explained yet.

Observation (2026-11-20; the operator, first-hand to the orchestrating session, with a picture of the Devices page of his hosted hub; relayed to the session that writes this; not a result, only he sets Passed or Failed): his words: "mac os metrics are back". The row of his Mac showed Up, an uptime of 100.00 %, and CPU 85 %, Memory 59 %, Disk 95 %. So the hub of the pool took an enrolment and batches of the daemon through the link. This agrees with step 1 on the Mac. Steps 4 to 6 were not reported. The row of the Linux computer `pop-os` still said "No metrics": it ran an older build, and no Linux build with these branches exists yet, so the Linux part of this entry is not started.

Where to look, with no change of the system: the daemon and the link write their lines to standard error, and a `LaunchAgent` of this build names no file for it, so on a Mac those lines are lost. A push that succeeded shows only at the hub: the row of the device has numbers, and Settings > Daemon of the device says "Metrics come from the daemon" with the time of the last report. `launchctl print gui/$(id -u)/ing.anima.daemon` shows the state, the count of starts and the last exit of the unit; a count of starts that grows means that the daemon ends and is started again. On Linux `journalctl --user -u anima-daemon.service` has the lines.

What the log says when it works (`~/Library/Logs/ing.anima.desktop/Anima.log`), in this order: "this hub is a hosted one: a unit of the user will keep the device link"; about 45 seconds later "the device link goes to a unit of the user" and "a unit of the user keeps the device link"; lines "the daemon of this computer: try N of 5: ...", the last of them "installed"; and "the daemon was installed on this computer".

A cleaning tool can delete a launch agent that it does not know (seen on the Mac of the operator, 2026-11-20: after a clean-up of the disk, `ing.anima.link.plist` was gone and no `anima-link` ran; the log of the application has no line of a removal, so the application did not do it; which tool did it is not known). If numbers stop and `ls ~/Library/LaunchAgents/ | grep anima` does not show both files: an application that holds task HM-T37 puts the unit of the link back after about 30 seconds and writes "the device link of the unit did not answer: the unit was installed and started again"; an older one does it at its next start. The unit of the daemon is not put back while the application runs: start the application again.

If step 1 shows no numbers after 3 minutes: quit the application, open it again and wait 3 minutes
more (a start of the application tries once). If there are still none, write here what
`tail -20 ~/Library/Logs/ing.anima.desktop/*.log` says about the device link and the daemon, and the
output of step 2.

### HT-059: the application hands its link to a unit of the user on a hosted hub (HMD-058, task HM-T24 parts b and c)

Built: on the branch `hosted/hm-t24b-shell-link-unit` (task HM-T24 of
[hosted-mode.md](proposals/hosted-mode.md), parts b and c; R-240). After a hosted hub gives this
computer its access, the shell (`apps/desktop/src-tauri/src/link/kept.rs`) copies `anima-link` to
`~/.local/bin`, has it install the unit of the user, and watches the running link in place of a
child of its own. A computer that is on the link already is handed over at the next start of the
application. When the unit cannot be made, the application runs the link itself as before.

The tests cover: the calls of `anima-link unit` and `ctl status` with a stand-in link, the watch of
a kept link (up, down, up again, and a stop of the application that leaves the unit), the mark of
the folder. Not covered: the copy to the real home folder, `launchctl`, the hand-over inside a real
sign-in, the start of the application with a unit that runs, and an update of the application.

Changed by DEF-007: the copy is `~/Library/Application Support/ing.anima.desktop/link/program/anima-link`,
not `~/.local/bin/anima-link`. In the steps below, read `~/.local/bin/anima-link` as that path, in double quotes.

It needs: this Mac with the application of this build (a trial bundle is enough), signed in to the
hosted hub. `<DIR>` is `~/Library/Application Support/ing.anima.desktop/link/device`; put it in
double quotes in a command, because the path has a space.

1. Open the application and sign in (or let it open the hosted hub as before). Use it for a minute.
   Expected: the hub opens as before. Nothing asks for a password.
2. Quit the application and open it again. Wait 30 seconds. Run `~/.local/bin/anima-link unit show`.
   Expected: `{"installed":true,...}`. The hub opens in the application.
3. Quit the application. Run `~/.local/bin/anima-link ctl -state <DIR> status`. Expected: a JSON
   answer with `"role":"device"`: the link runs with the application closed. On the hub (in a
   browser later, or after you open the application again) the check `Connected` of this Mac did not
   go Down while the application was closed.
4. Log out of the Mac and log in again. Do not open the application. Run step 3's command.
   Expected: the same answer.
5. Open the application. Expected: the hub opens at once; the application log has no line "the
   device link was not handed to a unit of the user".
6. Install a newer build of the application and open it. Run
   `cmp ~/.local/bin/anima-link /Applications/Anima.ing.app/Contents/MacOS/anima-link`. Expected: no
   output (the copy is the program of the new application), and the hub still opens.
7. Only if you want to leave the hub on this Mac: leave it in the application (Settings). Run
   `~/.local/bin/anima-link unit show`. Expected: the file is gone ("No such file"), and
   `ls ~/Library/LaunchAgents/ing.anima.link.plist` finds nothing.
The repair while the application runs (task HM-T37), on this Mac, after step 1 of this check passed: run `launchctl bootout gui/$(id -u)/ing.anima.link` and `rm ~/Library/LaunchAgents/ing.anima.link.plist`, and leave the application open. Expected: after about 40 seconds the file is there again, `pgrep anima-link` prints a number, and the log of the application has the line "the device link of the unit did not answer: the unit was installed and started again". A cleaning tool can do the same as these two commands: it may delete a launch agent that it does not know (seen 2026-11-20, see HT-061).


### HT-068: a Linux computer whose `~/.local/bin` its group can write gets the link as a unit and the daemon (DEF-007)

Built: on the branch `fix/link-private-folder`. The shell (`apps/desktop/src-tauri/src/link/kept.rs`) copies
`anima-link` to `<application>/link/program/` (`~/.local/share/anima/link/program/` on Linux), which the
application makes, and no longer to `~/.local/bin`. A unit of an earlier application that runs
`~/.local/bin/anima-link` is moved to the new copy at the next start. The log has the cause when the link
refuses the unit.

The tests cover: the paths of the copy, and that a unit of an earlier application is found by its copy in
`~/.local/bin`. Not covered: the copy into a real home folder (the check of the folders refuses the temporary
folder of a test), systemd and `launchctl`, the install of the daemon, and the metrics at the hub.

Seen by the session that wrote this (2026-11-20, on `pop-os`, with the desktop program of this branch in the
0.0.3 AppImage; not a result): "a unit of the user keeps the device link"; `anima-link.service` active; the link
ran from the new copy with `-daemon-socket`; the socket was made; `~/.local/bin/anima-link` was deleted.

It needs: `pop-os` (Pop!_OS 24.04, `~/.local/bin` mode 775), an AppImage that holds this branch, the sign-in to
the hosted hub; for step 5, this Mac with the unit that an earlier application made.

1. On `pop-os`, run `stat -c %a ~/.local/bin`. Expected: `775` (the case of the defect). Do not change it.
2. Open the AppImage and sign in. Wait two minutes. Expected: `systemctl --user is-active anima-link.service`
   prints `active`, and `ls ~/.local/share/anima/link/program/anima-link` finds the file.
3. Run `systemctl --user is-active anima-daemon.service`. Expected: `active`. The log of the application
   (`~/.local/share/ing.anima.desktop/logs/Anima.log`) has "the daemon of this computer: try N of 5: installed".
4. On the hub, open Devices. Expected: the row `pop-os` shows CPU, Memory and Disk, not "No metrics".
5. On this Mac, install the application of this branch over the one before and open it. Wait one minute.
   Expected: `launchctl list | grep ing.anima.link` shows the unit; `ls ~/.local/bin/anima-link` finds nothing;
   the copy is in `~/Library/Application Support/ing.anima.desktop/link/program/`; the Mac row still shows metrics.

Result: Open.

### HT-050: the device link runs as a unit of the user (HMD-058, task HM-T24 part a)

Built: on the branch `hosted/hm-t24-link-unit` (task HM-T24 of
[hosted-mode.md](proposals/hosted-mode.md), part a; R-240). `anima-link unit install` writes a
`LaunchAgent` (macOS) or a systemd user unit (Linux) that runs `anima-link device`, and starts it;
`unit remove` stops it and deletes the file; `unit show` says whether it is there (`link/unit`). The
application does not use the command yet.

The tests cover: the text of the two unit files, the commands and their order, the refusal of a value
that could break out of a unit file, of a program inside an application, and of a program that other
users can replace; the writing and the removal of the file with commands that are not run. Not
covered: `launchctl` and `systemctl` themselves, a login, and a link that runs from the unit.

It needs: a computer with a device link that joined a hub (Section 3 of
[link.md](procedures/setup/link.md)); `<DIR>` is its state directory and `<PORT>` a free port, for
example 41001. Stop each `anima-link device` and the Anima.ing application first.

1. Copy the program to a folder that stays: `mkdir -p ~/.local/bin && install -m 0755 <anima-link of
   this build> ~/.local/bin/anima-link`.
2. Run `~/.local/bin/anima-link unit install -state <DIR> -listen 127.0.0.1:<PORT>`. Expected: no
   output and exit code 0. No password is asked.
3. Run `~/.local/bin/anima-link unit show`. Expected: `{"installed":true,"file":"..."}`.
4. Run `~/.local/bin/anima-link ctl -state <DIR> status`. Expected: a JSON answer with
   `"role":"device"` and `"local":"127.0.0.1:<PORT>"`.
5. Log out and log in again (or start the computer again). Do not open the application. Run step 4
   again. Expected: the same answer. On the hub, the device shows as connected.
6. Run the command of step 2 from the application: `/Applications/Anima.ing.app/Contents/MacOS/anima-link
   unit install -state <DIR> -listen 127.0.0.1:<PORT>`. Expected: a refusal that says that the program
   is inside an application.
7. Run `~/.local/bin/anima-link unit remove`, then steps 3 and 4. Expected: `"installed":false`, and
   step 4 fails because no link runs.

### HT-049: the application carries `anima-daemon` (HMD-052, task HM-T16)

Built: on the branch `hosted/hm-t16-daemon-in-bundle` (task HM-T16 of
[hosted-mode.md](proposals/hosted-mode.md); R-206). `apps/desktop/src-tauri/trial-bundle.sh` and the
bundle step of the builder (`apps/desktop/src-tauri/src/builder/carried/rust_programs.rs`) build
`anima-daemon` and name it in `bundle.externalBin`; `check-mach-o.sh` and the check of the AppImage in
the release workflow expect it.

The tests cover: the lists of programs of the builder and of the Mach-O check. Not covered: a real
bundle. No bundle was made with this change, so nothing shows that Tauri takes the seventh program,
that the signature and the notary service accept it, or that the shell finds it.

1. Run `apps/desktop/src-tauri/trial-bundle.sh`. Expected: it ends with the place of the
   application, with no error.
2. Run `ls <place>/Anima.ing.app/Contents/MacOS`. Expected: eight files, and `anima-daemon` is one.
3. Run `<place>/Anima.ing.app/Contents/MacOS/anima-daemon --help` (or with no argument). Expected:
   the usage text of the daemon, not an error of the system.
4. After the next release from `main`: the jobs `macos` and `linux` of the release workflow pass
   their checks of the carried programs, and the notary step accepts the application. Expected: the
   release is published. If a check fails, write its message here.

### HT-044: the application does not start Tailscale on a hosted hub (DEF-005)

Built: on the branch `fix/install-daemon-link` ([DEF-005](work-items/defects/DEF-005.md)). For a hub at a
port of this computer the client no longer asks which computer this is over the VPN
(`apps/client/src/lib/desktop/this-machine.ts`, `apps/client/src/lib/hub/address.ts`), so the shell
does not run the Tailscale client.

The tests cover: for such a hub the client sends no question to the hub and none to the shell; for
an `https` hub it asks as before. Not covered: the real application, the real Tailscale application
of macOS, and that no other path starts it.

It needs: this Mac with the Tailscale application installed, the application of this build, and the
sign-in to the hosted hub.

1. Quit Tailscale (its menu, "Quit"). In Activity Monitor, look for "Tailscale". Expected: no such
   process.
2. Open Anima.ing on the hosted hub. Open Devices. Expected: Tailscale does not start.
3. Open the device of this Mac and press "+ VNC". Expected: Tailscale does not start. The button
   shows a result of the test; it does not open System Settings (that part waits for HM-T13 and
   HM-T14).
4. Open the settings of the device (the panel "Daemon"), and press "Add device" on the Devices page
   and close the form. Expected: Tailscale does not start.
5. Look again in Activity Monitor. Expected: no process "Tailscale".

### HT-055: the sign-in with the Anima.ing account on a phone, to the hosted hub (mobile plan M3-4)

Built: on the branch `mobile/account-sign-in` (mobile plan M3-4; MOB-017, MOB-030; HMD-032 to HMD-035, HMD-043). The
phone app has the cloud commands of the desktop app (`apps/desktop/src-tauri/src/cloud.rs`), so "Sign in" and "Sign up"
on the first screen are on. The steps are those of the desktop app: a code, the page of the account in the browser of
the phone (`phone_browser.rs`), "Your hub is starting", then the link in the app joins the hub (M3-1) and the device
list opens. The phone gives the cloud a name that it makes one time and keeps, `iphone-` or `android-` with six
hexadecimal digits.

The tests cover the answers of WorkOS and of the cloud, the name of the phone, the refresh token in memory, the check
of the address that the browser gets, and the choices and the words of the screens. The shell compiles for the iPhone
simulator and for Android with the stable Rust toolchain of this Mac. The steps of the sign-in were seen in a desktop
browser at 390 by 844 px with a stand-in cloud. Not covered: no build ran on a simulator, an emulator or a phone; no
phone called WorkOS or the cloud; the browser of a phone was not opened; no phone joined a hosted hub. The join
through the link in the app is also not proven (HT-025).

This build keeps the refresh token and the hub token in memory (OD-M7): after the app is closed and opened again, the
person signs in again. That is correct for this check and not for a release (MOB-006, task M5-1).

Steps 1 to 4 need no real phone. Use the Android emulator with a route to the internet, or the iPhone simulator.

1. Build this branch for the simulator or the emulator (`link/build-phone.sh`, then the Tauri build). Open the app.
   Expected: the first screen, with "Sign in" and "Sign up" on, and no line "Not on phones yet.".
2. Tap "Sign in". Expected: "Starting", then "Continue in your browser" with a code of eight letters, and the browser
   of the phone opens the page of the account with the same code. If the browser does not open, the line says so, and
   "Open the page" opens it.
3. In the browser, sign in with the Anima.ing account and confirm the code. Go back to the app. Expected: within some
   seconds "Your hub is starting", then "Connecting" with the words "this device", then the device list of the
   hosted hub. No step asks for an invite, an address or a password.
4. On the hub, in Settings, look at the devices of the link. Expected: one device with the name `iphone-…` or
   `android-…`. Close the app, open it, and sign in again. Expected: the device list again, and still one such device
   of this phone on the link, with the same name.
5. Do steps 1 to 4 on a real iPhone and on a real Android phone, on mobile data and on Wi-Fi. Expected: as above.
6. On a real phone, at step 2, do not sign in: tap "Back". Expected: the first screen. Tap "Sign in" again, and in the
   browser refuse the sign-in. Expected: "Sign-in cancelled" with "Try again".
7. On a real phone, look at the steps: "Back" and the button are easy to tap, and no text is smaller than the text of
   the invite steps (HT-025, step 6).

Result: Open.

### HT-053: the iOS archive with a signature, and the build from TestFlight on a real iPhone (mobile plan M5-3, M5-4)

Built: on the branch `mobile/account-sign-in`: `apps/desktop/src-tauri/ios-archive.sh` and
`deploy/bin/phone-build-number` ([mobile-build.md](procedures/setup/mobile-build.md)). The tests cover the rule of the
build number and what the script refuses before a build. On 2026-11-19 the script made an archive with no signature on
this Mac, with the version and the build number that were asked for. Not covered: the mode with a signature (no agent
uses a signing identity), the export of the IPA, the upload, and the application on a phone.

Before step 3 the application must keep its tokens in the key store (task M5-1) if the build is for a person other than
the operator (MOB-006).

1. In Keychain Access, find the name of the Apple Distribution identity (it starts with "Apple Distribution:"). Have the
   App Store profile of `ing.anima.mobile` as a file; it must be made after 2026-11-18 (the mobile plan, "M5-2 pending
   prerequisite").
2. In the repository run `IOS_SIGN_IDENTITY="<that name>" IOS_PROFILE=<that file> sh apps/desktop/src-tauri/ios-archive.sh`.
   Expected: no question about a new certificate or a new profile; the last lines are `version=`, `build=`,
   `signed=yes`, `archive=` and `ipa=`, and the IPA file exists. If the keychain asks to let `codesign` use the key,
   allow it.
3. Upload the IPA with Transporter or with Xcode (Window, Organizer). Expected: App Store Connect takes the build and
   shows it under TestFlight with the version and the build number of step 2. Note each warning of Apple (for example
   about the icons or the export question).
4. Install the build from TestFlight on a real iPhone and open it. Expected: the first screen. Then do HT-055, step 5.
5. Run step 2 again at a later commit of `main` and upload. Expected: App Store Connect takes it; its build number is
   higher.

Result: Open.

### HT-054: the sign-in inside the iPhone application, through WorkOS (mobile plan M3-5, MOB-043)

Built: on the branch `mobile/native-sign-in`. On an iPhone "Sign in" shows the page of the Anima.ing account over the
application (the web authentication session of iOS) and comes back by itself; there is no code and no browser to go
to. The shell uses the authorization code flow of WorkOS with PKCE and listens for the return at
`http://127.0.0.1:47113/sign-in`.

The tests cover the code challenge (the example of RFC 7636), the address of the page, the listener with a real
connection, the check of the state, a stopped sign-in, and the fields of the trade of the code. Not covered: the Swift
part, the page on a screen, a real answer of WorkOS, the refresh after this flow, and the rest of the sign-in
(HT-055). No build of this branch ran on a simulator or a phone.

1. In the WorkOS dashboard, in the production environment: Applications, the application of Anima.ing, the tab
   Redirects. Add `http://127.0.0.1:47113/sign-in`. Do not make it the default. Expected: the dashboard takes it. If it
   refuses the address, write the words of the refusal here and stop.
2. Open a build of this branch on the iPhone simulator or an iPhone. Tap "Sign in". Expected: iOS asks once to let
   "Anima.ing" use a website to sign in; after "Continue" the page of the account shows over the application, with
   the methods of the account (Apple, Google, GitHub, password).
3. Sign in. Expected: the page goes away by itself, with no code shown; "Your hub is starting", then "Connecting",
   then the device list of the hosted hub.
4. Sign out, tap "Sign in" again, and tap "Cancel" on the page of the account. Expected: "Sign-in cancelled" with the
   line "The sign-in was closed." and "Try again".
5. Leave the application open for more than one hour on the device list, then open a device. Expected: no new
   sign-in. (This is the refresh of the session with no secret.)
6. If Safari on the phone is signed in to the account, do step 2 again after a sign-out. Expected: the page signs in
   with no password.

Result: Open.
Observation (2026-11-19; the operator, from screenshots of a local build of `main` at `132c24dc`, version 0.8.16, signed in to his hosted hub; first-hand to the orchestrating session, relayed to the session that writes this; not a result, only he sets Passed or Failed): he pressed "+ VNC" on a row of the Devices list with Tailscale quit. A small panel said "No answer on port 5900. Screen Sharing (or the VNC se", cut off at the edge of the window, with a Close button. He did not report that Tailscale started. This agrees with step 3. Steps 1, 2, 4 and 5 were not reported. The message that is cut off is a fault of the layout, not of this check; the interface session has it.

### HT-043: a reminder reaches a phone through ntfy (E0-7 T2 to T4, REM-001 to REM-010)

Built: on the branch `engine/e0-7-reminders`. The hub evaluates reminder rules each minute and sends
each firing through the notification channels. `anima reminder` makes and changes rules, and shows,
snoozes and acknowledges firings.

The tests use a stopped clock, a database of the test and an HTTPS server of the test in the place of
ntfy. Not covered: the task of each minute in a hub that runs for hours, a real ntfy server and the
ntfy app on a phone, the text on a real lock screen, and a change of the clocks in a real time zone.

1. Run `anima notification` to make an ntfy channel with a topic of 16 characters or more that is
   on by default, and subscribe to the topic in the ntfy app on your phone.
2. Run `anima reminder add --name Stretch --message "Stand up" --trigger
   '{"type":"time","at":"<the time in 3 minutes, as HH:MM>","zone":"<your time zone>"}'`. Expected: the
   rule is shown with the revision 1.
3. Wait for the time. Expected: in not more than 2 minutes the phone shows a notification with the
   title `Stretch` and the body `Stand up`. `anima reminder firings` shows one firing in the state
   `SENT`.
4. Run `anima reminder test <rule>` two times. Expected: two more notifications in the next minutes:
   the cooldown does not stop a test.
5. Run `anima reminder snooze <firing> 2` for a sent firing. Expected: the same notification comes
   again after 2 to 3 minutes. Then run `anima reminder ack <firing>`. Expected: the state
   `ACKNOWLEDGED`, and no more notification of it.
6. Make a rule with the trigger `{"type":"place","place":"<a place of yours>","on":"arrive"}` and
   send a fix of the OwnTracks app from inside the place, after one from outside (HT-040). Expected:
   one notification with the title `Anima` and the body `You have a reminder.`: no name of the rule
   and no name of the place is on the lock screen.
7. Make an ntfy channel with the topic `home`, and a rule that names only this channel. Run
   `anima reminder test <rule>`. Expected: no notification; `anima reminder firings` shows the firing
   as `SUPPRESSED` with the reason `topic_not_random`.
8. Run `anima reminder set <rule> --message "Changed"` for the rule of step 2 in the minute before it
   is due again on the next day, or run `anima reminder disable <rule>`. Expected: a firing that
   waited is `SUPPRESSED` with the reason `rule_changed`, and the phone shows nothing.

Result: Open.

### HT-063: the hub pulls the points of a Dawarich server (E0-5 T3 and T4, R-261)

Built: on the branch `engine/e0-5-dawarich`. `anima source connect dawarich` connects a Dawarich
server with an API key of its account. The hub then reads `GET /api/v1/points` of that server each
15 minutes and stores each point as one location sample of the provider `DAWARICH`.

The tests use a server of the test that answers with points that the test wrote. Not covered: a real
Dawarich server and its real answers (the names of the fields, the pages, the time form of
`start_at`), a real certificate, a server on a private address of the VPN, and a hub that is hosted
outside your network.

1. In Dawarich, open the account settings and copy the API key.
2. Run `anima source connect dawarich --label Home --url https://<your Dawarich address>` and paste
   the key at the question. Expected: the account is shown with the source `dawarich` and
   "credential stored"; the key is not on the screen.
3. Wait 1 minute, then run `anima source list`. Expected: the account is healthy and its last run
   says how many samples it added.
4. Run `anima location list --from <a day that Dawarich has points for>T00:00:00Z --to <the next
   day>T00:00:00Z`. Expected: rows with the provider `DAWARICH`, with the times and the positions
   that Dawarich shows for that day. A point with no accuracy in Dawarich shows `?`.
5. Run `anima source sync <account>` two times. Expected: the runs add 0 samples when Dawarich has
   no new point: no point is stored two times.
6. Run the command of step 2 with a wrong key. Expected: "The server refused this key.", and the
   account of step 2 still works.
7. If your Dawarich answers only plain `http`, run the command of step 2 with `http://`. Expected:
   the hub refuses the address (OD-E26 asks if this must change).
8. On a hosted hub, with a Dawarich server that only your home network reaches: run the command
   of step 2. Expected: the hub says that it cannot reach the server; nothing is stored.
9. Run `anima source pause <account>`. Expected: no new run until you resume it.

Result: Open.

### HT-040: a phone sends fixes and a file transfer gets its context (E0-5 T3 and T4, R-145)

Built: on the branch `engine/e0-5-locations`. The hub takes location samples on
`POST /locations/v1/owntracks` with an ingest token, and from `anima location import`; it seals each
position with the sealing key `locations` of `ANIMA_DATA_KEY_DIR`. Each transfer of the files tunnel
gets a context, and `anima location context <event>` shows it with the evidence for where the
operator was.

The tests use posts that the test makes, a stopped clock and an SSH server of the test. Not covered:
the OwnTracks app on a real phone, a real network between the phone and the hub, a real fix of a
phone, and a transfer through the client between real devices.

1. Run `anima location token add Phone`. Expected: one line with a token that starts with `anl_`, and
   the note that it is shown one time.
2. In the OwnTracks app select the HTTP mode. Give the address `<hub>/locations/v1/owntracks`, a user
   name of your choice, and the token as the password. Send one fix by hand. Expected: the app shows
   no error.
3. Run `anima location list`. Expected: one row with the provider `OWNTRACKS`, the time of the fix
   in OBSERVED, your latitude and longitude, and the accuracy that the phone gave.
4. Run `anima place add` for the place where you are, with a radius of 100 m, and link the site of a
   device to a different place with `anima place site`.
5. Send one more fix from the app. In less than 2 minutes, download a file from that device in the
   files window of the client. Run `anima event list` and find the event "An operator downloaded a
   file from the device." Run `anima location context <event>`. Expected: `source place` is the place
   of the site of the device; the three lines of `destination` say `unknown`; `operator evidence`
   says "a sample at the time"; `operator place` is the place where you are, not the place of the
   device.
6. Wait 15 minutes and send no fix. Download a file again and run `anima location context` for the
   new event. Expected: `operator evidence` says "unknown: no sample within 10 minutes", and
   `operator place` says `unknown`. The context of the event of step 5 did not change.
7. Run `anima location token revoke <id>`, then send a fix from the app. Expected: the app shows an
   error (401), and `anima location list` shows no new row.
8. Run `anima event list` and `anima audit list`. Expected: no line holds a latitude, a longitude or
   the token.

Result: Open.

### HT-035: a server gets, follows and drops an operational key (E0-6 T2, R-147)

Built: on the branch of E0-6 T2 (K-E5 Section 2, K8 Section 9). The hub keeps operational keys in the
`credentials` domain (`crates/hub/src/fleet/credentials/operational/`) and gives them to the daemon of a granted
device (`POST /daemon/v1/keys/fetch`). The daemon keeps a cache beside its token (`crates/daemon/src/keys/`)
and reads it with `anima-daemon key read <name>`.

The tests use a throwaway database and in-process requests. The daemon cache was tested as pure logic and as a
file on this machine. No test used the macOS Keychain item `key-cache`, a real daemon pushing to a real hub, two
machines, or a wait of 7 days. There is no `anima key` command yet (the noun waits for a ruling), so the steps use
GraphQL through the client's developer tools or `curl` with a login token.

1. On a Linux server with an enrolled `anima-daemon` of this branch, and on the hub, make a key with
   `createOperationalKey` (value `test-value-1`) and grant it to the server with `grantOperationalKey`.
   Expected: within one push interval, `operationalKey` shows the consumer with `heldVersion` 1.
2. On the server, run `anima-daemon key read <name> | cat`. Expected: `test-value-1`. Run it with no pipe.
   Expected: a refusal that says standard output is a terminal, and no value.
3. On Linux, look at the file `keys` in the daemon's folder. Expected: mode `0600`, owner the daemon's user.
4. On the hub, `rotateOperationalKey` with `test-value-2`. Expected: within one push interval, `key read` gives
   `test-value-2` and `heldVersion` is 2.
5. `revokeOperationalKey`. Expected: within one push interval, `key read` says the machine holds no such key.
6. Do steps 1 and 2 on a Mac. Expected: the same; Keychain Access shows the item `ing.anima.daemon` /
   `key-cache`. Then delete the device on the hub. Expected: the daemon logs that its token is refused, the
   Keychain item is gone, and the audit log of the hub has one entry for the key that names the removed device
   and the version that it had, with no value.
7. Grant a key to a server, stop the hub for more than 7 days (or set the server's clock 8 days ahead), and run
   `key read`. Expected: no value, and the cache record is gone.

Result: Open.

### HT-036: the release workflow of GitHub Actions (tasks B8b to B8d and B8g, R-179, R-180, R-182 to R-184, R-190 to R-192, R-198 to R-201)

Built: on `main` since 3ae44093; the trigger on a push to `main`, the decision and the switch (task B8g) on the
branch `release/each-push`. `.github/workflows/release.yml` builds the macOS and the Linux
application on machines of GitHub, signs the macOS one with the Developer ID and has it notarized, signs the two
updater files with the updater key, and publishes with `anima-hub release publish`
(`crates/hub/src/fleet/builds/publishing/built/`). The state of GitHub and the steps of the operator are in
[releases.md](procedures/releases.md), "The release workflow".

The tests ran the three `anima-hub release` commands against a bucket in the process of the test, with a
throwaway key. `actionlint` read the workflow file, and a test reads its rules. `check-mach-o.sh` was run on the
installed application 0.2.1. No test runs the workflow, reads a secret of GitHub, imports the certificate,
sends anything to the notary service or reaches the R2 bucket. The operator says that an older AppImage of
Anima ran on a Linux computer (R-192); the repository has no record of a start of the AppImage of today.

1. Read "The state of GitHub". Expected: three environments with no required reviewer (R-182), each with its
   rules for tags and branches, and the secrets under the names of the table.
2. Do "A trial that publishes nothing" with the version `0.0.1`. Expected: no approval is asked, each job
   passes, `publish` is skipped, and no feed of `https://releases.anima.ing` changed. `stapler validate` and
   `spctl` accept the DMG and the application of the artifact `macos`. The AppImage of the artifact `linux`
   starts on a Linux computer; the first public release waits for this (R-192). In each of the two, Settings >
   Updates does not say "This is a local build", and it offers the published version (the build of the workflow
   has the updater; a build by hand with `trial-bundle.sh` has not, DEF-001).
3. With the switch off (the variable `ANIMA_PUBLIC_RELEASES` does not exist), push a change of a document to
   `main`. Expected: a run of `release` starts, only its job `commit` runs, and the summary says "nothing: this
   push is a release of <version>, and public releases are off" (the Linux feed does not exist, so each push is
   a release until the first one is published). Nothing is built, and no feed changed.
4. Do "To switch public releases on" and "The first public release". Expected: the push builds and publishes
   the version that `sh deploy/bin/commit-version origin/main` prints (R-183, R-201); the three feeds give that
   version; `stapler validate` and `spctl` accept `https://releases.anima.ing/Anima.dmg`; an application 0.2.1
   that was installed from its DMG offers the version and starts it after "Restart to update".
5. After that release, push a change of a document only to `main`. Expected: only the job `commit` runs, and the
   summary says "nothing: no change of desktop after the published <version>".
6. Push two changes of the application to `main` within some minutes. Expected: the runs do not overlap; the
   feeds give the version of the second push in the end, and never a lower version after a higher one.
7. Start the first job of an old run again ("Re-run all jobs" on a run before the newest release). Expected:
   the summary says "nothing: the version ... is not more than the published ...", and nothing is built.

This entry names "Re-run" and pushes to `main`; each push to `main` is a release when the switch is on (R-198).

Observation (2026-11-18, evening; 2026-11-19 UTC; read from GitHub by an agent, not a result of the operator):
the operator started a trial, run 44468487102, from `main` at 9f056a5a with the version `0.0.1`. No approval
was asked. The run failed in the job `gates`.

- Passed: `commit`; `tool` (6 minutes); `linux` (24 minutes); `macos` (53 minutes: the build with the Developer
  ID signature 50 minutes, the check of each Mach-O file, the notary service and the staple 2 minutes, the
  updater file). So the certificate, its password and the notary credentials of the environment `release` are
  usable.
- Failed: `gates`, after 75 minutes, at gate 15 of 21 (`bun run test`). Three tests of `deploy/headscale` need
  `shellcheck`, and the macOS machine of GitHub does not have it: 750 of 753 tests passed there. Gates 1 to 14
  passed, with 6166 Rust tests. Gates 16 to 21 did not run.
- Skipped, because they wait for the gates: `sign`, `trial` and `publish`. So this run proves nothing about the
  updater key and its password in `release-sign`, about the checks of `anima-hub release check`, or about the
  bucket token.
- Not looked at by a person: the artifacts `macos` and `linux` of the run (steps 2 and the Linux start).

Corrected on the branch `release/operator-decisions`: the job `gates` installs `shellcheck`, and it gets the
full history, because the documents gate (gate 20) fails in a shallow clone; that second fault was found by a
run of gates 17 to 21 in a shallow clone on a computer, not by the trial. The operator starts the next trial
after the merge.

Observation (2026-11-19, 08:51 UTC; read from GitHub by an agent, not a result of the operator): the operator
started a second trial, run 44479803653, from `main` at 9da268f4 with the version `0.0.1`. Each job passed:
`commit`, `tool`, `linux` (14 minutes), `macos` (62 minutes), `gates` (82 minutes, 21 gates), `sign` and
`trial`. `publish` was skipped, as a trial must. So the updater key and its password in `release-sign` are
usable, the two updater files were signed, and `anima-hub release check` accepted both targets. Not proven by
this run: the bucket token (a trial does not use it), and each check of step 2 that a person makes: the
commands on the DMG, the start of the AppImage on Linux, and Settings > Updates in the two applications. The
run was of the tag trigger of that commit; the trigger on a push to `main` (task B8g) has not run.

Observation (2026-11-19, 09:43 UTC; read from GitHub by an agent, not a result of the operator): the first run
of the trigger on a push to `main`, run 44498918610, for the merge commit eb226c8d. Only the job `commit` ran,
and it passed; each other job was skipped. Its log has the decision "nothing: this push is a release of 0.8.12,
and public releases are off (the variable ANIMA_PUBLIC_RELEASES is not `on`)". The feeds still gave 0.2.1 for
macOS and none for Linux. This is the expected result of step 3, seen for a merge commit and not for a change
of a document; the check that the tip of `main` before the push is on the first line passed for a merge with
`--no-ff`.

Changed on 2026-11-19, on the branch `release/first-public`, and not run on GitHub: the job `gates` keeps
compiler outputs between runs, and a release starts with the job `bucket`, which reads one object of the bucket
with the token. Expected in step 4: `bucket` passes in the first minute. Expected in the second trial after the
merge: the job `gates` is shorter than 82 minutes, and its last step shows cache hits.

Changed on 2026-11-19, on the branch `release/call-gates`, and not run on GitHub: the release calls the workflow
`gates` on Linux (job `gates-linux`), runs the tests that only a Mac runs in the job `gates-macos`, and does not
call the Linux gates for a content that passed them on `dev` ([releases.md](procedures/releases.md), "The gates
of a release"). The line above about the job `gates` and its cache no longer holds. Expected in the next trial
from `main`, started with Gates `reuse`: the summary says "The Linux gates are not run again" and names a run of
`dev`; `gates-linux` is skipped; `gates-macos`, `gates`, `sign` and `trial` pass. Expected in a trial with Gates
`run`: the five jobs of `gates-linux` pass. A fault to look for in both: a job after `gates` that is skipped
when it must run. Not proven before that trial: the time of `gates-macos`, and the test filter of
`deploy/bin/macos-tests`, which was not run in full on a computer (its bun and Go parts were).

Result: Open.

Observation (2026-11-19; the operator, from screenshots of a local build of `main` at `132c24dc`, version 0.8.16, signed in to his hosted hub; first-hand to the orchestrating session, relayed to the session that writes this; not a result, only he sets Passed or Failed): for the Linux part, his hosted hub listed the Linux computer `pop-os` as a device on the link, Up. Whether the application on it was a file of the release workflow or a local build is not known here, so this shows nothing about the published AppImage yet.

Observation (2026-11-20; the operator, first-hand, in the session that writes this; not a result): `Anima_0.0.2_amd64.AppImage` of the release workflow stopped at start on Pop!_OS 24.04 with "Failed to load ayatana-appindicator3 or appindicator3 dynamic library" ([DEF-006](work-items/defects/DEF-006.md)). Corrected on the branch `fix/appimage-tray-library`. Expected in the next release: the job `linux` passes its new check of the tray libraries, and the AppImage opens its window on a 24.04 computer.

### HT-038: the details of a container, the state of a Compose project, and the images, volumes and networks of a real device (HOST-017, HOST-019, HOST-020; task H8)

Built: in the hub domain `crates/hub/src/work/containers/`, `Query.containerDetail`,
`Query.containerStorage`, `Query.containerNetworks` and the field `Container.composeProjectState`;
`anima containers inspect`, `images`, `volumes` and `networks`; the MCP tools
`anima_container_storage` and `anima_container_networks`. The client does not show them yet (task H9).
The tests use a test SSH server that answers with fixed text, and a scripted hub. The fixed text is
what Docker 29.9.3 wrote on the build machine (a Mac) for the same command lines, run in a shell and
not through the hub. No test ran the commands through the hub on a device, on Linux, or on an older
Docker. A Docker older than about 20.10 can refuse `docker system df -v --format`; then step 7 and
step 8 give "docker did not succeed on the device": write down the Docker version.

The check only reads. It starts, stops, removes and changes nothing on the device (VPS-005, R-050).
The hub must be of this build; a local hub is sufficient
([setup/local-hub.md](procedures/setup/local-hub.md)). Use the device of HT-032: Docker with at least
one Compose project, SSH on, the host key accepted, the SSH credential in Anima. Use a login for
step 1 to step 4; on a hosted hub, where the command line has only an API key, write that step 1 to
step 4 were not possible (OD-25).

1. Run `anima containers inspect <device> <name of a running container>`. Expected: the command, the
   restart policy, the mounts, the networks with the addresses, the labels and the processes agree
   with `docker inspect <name>` and `docker top <name>` on the device.
2. Look at the row "Environment". Expected: the names of the environment variables of the container,
   and no value. Run the command with `--json`. Expected: no value of an environment variable is in
   the output.
3. Run the command for a container that has a health check. Expected: the health check, the health
   and the exit status of its last run. Run it for a stopped container. Expected: its details, the
   exit status of its last run, and no process.
4. Run `anima containers inspect <device> nosuchname`. Expected: "The device has no container named
   nosuchname."
5. Run `anima containers inspect` with an API key in `ANIMA_TOKEN`. Expected: the hub refuses with
   FORBIDDEN.
6. Run `anima containers list <device>`. Expected: the last column gives, for each container of a
   Compose project, the name of the project and its state, for example `dawarich (running)`. If the
   device has a project of which one container is stopped and one runs, its state is "partly
   running"; this is also the state of a project whose set-up container ended with exit status 0.
   Do not stop a container for this step.
7. Run `anima containers images <device>`. Expected: one row for each image of `docker image ls` on
   the device, with the same size, and in the last column the names of the containers that use it
   (or a number when the hub could not find the names). The line below the table gives the space
   that `docker system df` shows as reclaimable for images.
8. Run `anima containers volumes <device>`. Expected: one row for each volume of `docker volume ls`,
   with the size that `docker system df -v` shows and the containers that mount it. Write down how
   long the command took.
9. Run `anima containers networks <device>`. Expected: one row for each network of
   `docker network ls`, with the containers on it.
10. In an MCP client, call `anima_container_storage` and `anima_container_networks` for the device.
    Expected: the same images, volumes and networks. No tool gives the details of a container.
11. Read the log of the hub for the time of the steps. Expected: no name of a container, of an
    image, of a volume or of a network, no label and no name of an environment variable (LOG-007).
12. On the device, run `docker ps -a`, `docker image ls` and `docker volume ls` again. Expected:
    nothing changed because of the steps.
13. Added with task H15 (R-211): run `anima containers projects <device>`. Expected: one row for
    each Compose project, with its state, its running count, and the folder and the files from
    which it was started; the paths agree with `docker compose ls` on the device. Anima changed no
    file.

Result: Open.

### HT-039: a newer image for a container, with real public registries (HOST-021; task H13)

Built: `Query.containerImageUpdates` in the hub domain `crates/hub/src/work/containers/updates/`,
`anima containers updates`, and the MCP tool `anima_container_updates`. The client does not show the
answer yet (task H9). The tests use a test SSH server and a local HTTPS server that stands in for each
registry. The answers of the four real registries were read by hand on the build machine on
2026-11-19: the digest that Docker 29.9.3 keeps for `alpine:3` was the digest that Docker Hub gave
for the tag. No test reaches a real registry, and no test ran on a hub in the pool: it is not known
if a hosted hub can reach the registries.

The check only reads. It pulls nothing and updates nothing (ETH-004, VPS-005, R-050).

1. On the device, find a container whose image tag has a newer image at Docker Hub (for example a
   container made some weeks ago from a tag such as `latest`). If there is none, write that step 2
   had no such container.
2. Run `anima containers updates <device>`. Expected: one row for each container. The container of
   step 1 has the state "newer image". A container whose image was pulled today has "up to date".
3. For one row with "up to date", run on the device
   `docker image inspect --format '{{json .RepoDigests}}' <image>`, and compare with
   `anima containers updates <device> --json`. Expected: `localDigest` is the digest of that
   output, and `registryDigest` is the same.
4. Look at a container of an image that was built on the device, if there is one. Expected: "not
   known", reason "no local digest". Look at a container of an image of a registry other than
   Docker Hub, ghcr.io, quay.io and mcr.microsoft.com, if there is one. Expected: "not known", reason
   "other registry".
5. Write down how long the command took. Expected: less than 40 seconds.
6. In an MCP client, call `anima_container_updates` for the device. Expected: the same answers, and
   `newer` is the number of rows with "newer image".
7. On a hosted hub, do step 2. Expected: the same answers. If each row says "registry not read",
   the hub of the pool cannot reach the registries: write that down.
8. On the device, run `docker image ls` and `docker ps -a`. Expected: no image and no container
   changed because of the steps.
9. Read the log of the hub for the time of the steps. Expected: no name of an image, no digest and
   no token (LOG-007).

Result: Open.

### HT-042: the tab "Containers" with the details, the mark of a newer image, the log tools, and the images, volumes and networks (HOST-015, HOST-017, HOST-019, HOST-020, HOST-021; task H9)

Built: `apps/client/src/lib/containers/` (the table, `detail/`, `log/`, `storage/`), in the tab
"Containers" of the device page. The tests use a fake of the hub calls and, in a browser, a hub of
example values. No test used a real hub or a real device, and no person saw the screen or compared it
with its mockup (`docs/mockup/containers/index.html`, frames 1, 4 and 5). The download of a log ran
in a browser of the tests, not in the desktop application.

The check only reads (VPS-005, R-050). Do HT-038 and HT-039 first, or in the same sitting: this entry
checks the screen, those check the values. Sign in to the application with a login, not an API key.

1. Open the device and select the tab "Containers". Expected: a row "Containers, Images, Volumes,
   Networks" above the table, and in each group header the state of the Compose project (for
   example "Running" or "Partly running") before the running count.
2. Select "Details" in the row of a running container. Expected: below the table, the sections
   Container, Health check, Mounts, Networks, Environment, Labels and Processes. The section
   Environment has names only. The values agree with `anima containers inspect` of HT-038.
3. Select "Log" in the same row. Expected: the log is in the place of the details.
4. In the log, type a word of one line in the search field. Expected: only the lines with that word,
   and the count "n of m lines". Clear the field.
5. Select "Error", then "Warning", "Info" and "Debug". Expected: each shows only the lines with such
   a level word. Write down if the container writes levels in a form that the filter does not find.
6. Turn "Wrap" off. Expected: a long line stays on one row and the region scrolls sideways. Turn
   "Timestamps" off. Expected: no time before the lines.
7. With a search text that shows some lines, select "Download". Expected: the application saves a
   file named for the container, `.log`, that holds only the shown lines. In the desktop
   application, write down where the file went, or that nothing was saved.
8. Select "Look for newer images". Expected: after some seconds, each row has a mark beside its
   image ("Newer image", "Up to date", or a reason), and the count says how many have a newer image.
   The marks agree with `anima containers updates` of HT-039.
9. Select "Images", then "Volumes", then "Networks". Expected: each list agrees with
   `anima containers images`, `volumes` and `networks` of HT-038, and the line above the images and
   the volumes says how much a clean-up can free. Write down how long "Images" took to show.
10. Compare frames 1, 4 and 5 of the mockup with the screen at 1040 px, in the light and the dark
    appearance. Write down each difference.
11. Make the window 400 px wide. Expected: write down what cannot be read or reached; the phone
    layout has no special form yet.
12. Sign in with a browser session of an API key, if the hub permits one, and select "Details".
    Expected: the sentence of the hub that it refuses, and the table stays.

Result: Open.

### HT-030: the application on a computer shows its copy of recent data when the hub does not answer (FND-007, task F2)

Built: in 53af82a1 and 3b46dfef (R-157). The application keeps the last answer of the hub for the device list,
one device, its metrics and the event log (`apps/client/src/lib/hub/recent/`). It does not keep the checks of a
device, because their settings can hold a credential. When the hub does not answer, it shows that copy, and the
banner "Not on the network" gives its age. Sign-out deletes the copy.

The tests cover the rules with a store in memory. The real store (IndexedDB) was run in Chromium on 2026-11-18. It
was not run in the web view of the installed application (WebKit on macOS, WebKitGTK on Linux), and no screen was
looked at with the network off.

1. Open the application while the hub answers. Open Devices, then one device, then Logs. Expected: the usual screens.
2. Turn the network off (Wi-Fi off, or stop the VPN client). Quit the application and open it again. Expected: the
   banner "Not on the network" with "Data from" and an age of some minutes, and the device list with the devices of
   step 1.
3. Open the device and the Logs of step 1. Expected: the same content as in step 1, and the charts of the device
   show their lines. The checks of the device, and a screen that was not opened in step 1, show that the hub did
   not answer.
4. Try a change, for example rename the device. Expected: the application says that the hub did not answer, and the
   name does not change.
5. Turn the network on. Expected: in less than one minute the banner goes away, the page loads again by itself and
   shows live data.
6. Turn the network off again. In Settings, sign out. Quit the application, open it, and sign in is asked. Expected:
   no device of step 1 is shown anywhere before the sign-in.

Result: Open.

### HT-037: the daemon pushes through the Unix socket of the link (task W0, R-172, SF-004)

Built: on the branch `daemon/w0-link-socket` (task W0 of
[projects-and-builds-plan.md](plans/projects-and-builds-plan.md); K8 Section 1). `anima-link device
-daemon-socket <path>` makes a Unix socket that only its user can open (`link/statedir/listenowned.go`,
`link/devicelink`). The daemon reaches a hub in the link mode only through that socket
(`crates/daemon/src/hub.rs`, `hub/link_socket.rs`, `config/origin.rs`): its origin is `unix:<path>`,
and it refuses `--hub http://127.0.0.1:<port>`. A daemon that was enrolled at a port keeps its files
and uses `link.sock` in its own folder.

The tests cover: a request on the socket reaches a test hub through a real device link; the mode of
the socket; the refusal of a folder that other users can reach, by the link and by the daemon; a push
and its token on the socket with a scripted hub; no request to the port for an old origin; the start
of a daemon with old files. Not covered: a real hub, the renewal of a token over days, a daemon that
runs as its installed unit (the sandbox of the unit must let it open the socket), a second user of
the computer, and a hosted hub. On a hosted hub only the desktop application starts the link today,
with no socket for the daemon (HM-D16), so this check uses a hub that you start in the link mode.

It needs: a hub in the link mode ([local-hub.md](procedures/setup/local-hub.md), or Section 7 of
[link.md](procedures/setup/link.md)), and one computer with a device link that joined it (Section 3
of link.md) and the two programs of this build. `<FOLDER>` is `~/.config/anima-daemon` on Linux and
`~/Library/Application Support/anima-daemon` on a Mac.

1. On the computer, stop the device link and start it with the socket:
   `mkdir -p -m 700 "<FOLDER>"`, then `anima-link device -state <DIR> -daemon-socket "<FOLDER>/link.sock"`.
   Expected: the log line "the device link is ready" names the socket, and `ls -l "<FOLDER>/link.sock"`
   starts with `srw-------`.
2. Make an enrolment code for the computer in Anima. Run `anima-daemon enrol - --hub "unix:<FOLDER>/link.sock"`
   and paste the code. Expected: "Enrolled", with the name of the device.
3. Run `anima-daemon install`. Wait 2 minutes. Expected: in Anima the device shows metrics, and its
   check of the daemon is Up. This proves that the installed unit can open the socket.
4. Run `anima-daemon enrol - --hub http://127.0.0.1:8443` with any text as the code. Expected: a refusal
   that names `-daemon-socket`, and no request to the hub.
5. Stop the device link for 3 minutes, then start it again as in step 1. Expected: the metrics have no
   hole after some minutes (the daemon kept its samples), and no new enrolment was necessary.
6. As a second user of the computer, if it has one: `nc -U "<FOLDER>/link.sock"`. Expected:
   "Permission denied".
7. Only if a computer still has a daemon that was enrolled at a port of the link: install this build
   of the daemon there and start its link with `-daemon-socket "<FOLDER>/link.sock"`. Expected: the
   metrics continue with no new enrolment.
8. Leave the daemon in operation for 8 days. Expected: the device is still Up (the token was renewed
   through the socket).

State: Open.

### HT-041: the first crate version and the first formula of `anima` alone (INS-010 to INS-012; tasks R8a and R8d)

Built: on the branch `release/cli-job` (task B8h; R-217 to R-219). The release workflow makes a standalone `anima`
for macOS and for Linux from the applications of the same run, puts them below `anima/<version>/` of the bucket,
pushes the formula to the tap `TheArctesian/homebrew-anima`, and publishes the crate `anima-cli` with the version
of the release. The steps of the operator are in [releases.md](procedures/releases.md), "`anima` alone".

The tests read the rules of the jobs and run the two commands that write the formula and the version of the crate.
`cargo publish --dry-run` passed on 2026-11-19. No test runs a job: no file was put in the bucket, nothing was
pushed to the tap, nothing was sent to crates.io, and no `brew install` ran. It is not known if the Linux program
starts on a computer with no `libdbus-1`, and if the build of the crate needs the packages `libdbus-1-dev` and
`pkg-config` on each Linux (the job installs them).

1. Start a trial after the merge. Expected: the job `cli` passes; its log shows `ldd` of the Linux program and no
   "not found"; the artifact `cli` holds two `.tar.gz` files, `SHA256SUMS`, `anima.rb` and `latest.json`.
2. Do the steps of "`anima` alone" (the key of the tap, the token, the two environments), then set
   `ANIMA_PUBLIC_CLI`, and release. Expected: the jobs `cli-publish`, `tap` and `crate` pass after `publish`.
3. On a Mac with no Anima: `brew install thearctesian/anima/anima`, then `anima --version`. Expected: the version
   of the release; no desktop application and no daemon is installed. `brew audit --strict --online anima` and
   `brew test anima` pass.
4. On a Linux computer with `x86_64`: the same with Homebrew, and `cargo install anima-cli --locked`. Expected:
   the same version from both. Note each package of the system that the build or the program asked for.
5. Compare with `anima --version` of the program inside the disk image of the same release. Expected: the same
   version (INS-012). crates.io shows `anima-cli` with that version.

Result: Open.

### HT-046: the iOS application from the release workflow to TestFlight (R-203; task B8i)

Built: on the branch `release/ios-job`. The release workflow has the jobs `ios` (build and sign), `ios-upload`
(send to App Store Connect) and `ios-record`, behind the variables `ANIMA_IOS_BUILD` and `ANIMA_PUBLIC_TESTFLIGHT`.
The steps of the operator are in [releases.md](procedures/releases.md), "The iOS application: TestFlight".

The tests read the rules of the jobs: the switches, which job holds which key, the build number. No job ran. The
archive script `apps/desktop/src-tauri/ios-archive.sh` (task M5-4 of the mobile plan) does not exist, so no test
built or signed the application, and nothing was sent to App Store Connect.

1. Do steps 1 to 3 of "The iOS application: TestFlight". Expected: two environments for `main`, each with its
   three secrets.
2. When the archive script is on `main`: set `ANIMA_IOS_BUILD` and start a trial. Expected: the job `ios` passes
   and the artifact `ios` holds `Anima.ipa`; no upload job runs.
3. Set `ANIMA_PUBLIC_TESTFLIGHT` and push a change of the application to `main`. Expected: `ios-upload` passes;
   App Store Connect shows a build with the version of the commit and the build number of its count (0.8.15
   gives 815); `ios-record` passes.
4. Install the build from TestFlight on an iPhone. Expected: the application starts and shows the same version.
5. Push a change of a document only. Expected: the summary of the run says "nothing: no change of ios after the
   published <version>", and no iOS job runs.

Result: Open.

### HT-051: the workflows `gates` and `merge-to-dev` on GitHub (R-253, R-254, R-252)

Built: on the branch `quality/faster-gates`. `.github/workflows/gates.yml` runs the 21 gates on Linux
machines of GitHub for each pushed branch, as merged with `dev`. `.github/workflows/merge-to-dev.yml`
merges a branch whose run passed into `dev`.

The tests cover the rules of the two files (no secret, pinned actions, no pull request, the order of the
steps), the choice of the gates and the merge script, which was run in a clone with an origin of its own.
Not covered: no run on GitHub. Nobody saw the packages install on `ubuntu-24.04`, the Rust tests pass on
Linux with 4 processors, the time of a run, the cache, or a push by the workflow to `dev`. Expect that the
first runs fail for a reason of the machine; each such fault is a correction of the workflow, not of the code.

Seen by the session that built it, on 2026-11-19, on the branch `quality/faster-gates` (not a result of this entry: only the operator sets it): the sixth run, 44652644005 on commit `152773ff`, passed each job in 31.5 minutes (`go` 2, `bun` 6, `rust-lint` 12, `rust-test` 31). The five runs before it failed for the reasons that [quality/README.md](../quality/README.md), "The gates on GitHub", lists; two of them were the step "Merge the dev of this run into the checkout" refusing a branch that did not merge with `dev`, as step 5 expects. Steps 3 and 4 were not run: `merge-to-dev.yml` is not on `main`.

1. Push a branch that changes one document. Open the run `gates` of that branch on GitHub. Expected: the
   job `base` prints "tested with dev at:" and a commit; the five other jobs pass; write the time of each job here.
2. Push a branch that changes one line of Rust. Expected: the same, and the second run of the same branch is
   faster than the first (the cache).
3. When `merge-to-dev.yml` is on `main` and the settings are made: push a branch with one changed document and
   one placeholder of a number. Expected: after its gates, a run `merge-to-dev` makes a branch `train/...`
   and a run `gates` starts for it; when that run passes, the next run `merge-to-dev` moves `dev` to the
   commit of the train: `dev` has a merge commit that names the number, the placeholder is gone, the branch
   `train/...` is gone, and no run `gates` starts for `dev`.
4. Push three such branches at the same time. Expected: one train holds each branch whose gates passed
   when it was made; one run of the gates lands them in one step; no branch runs a second time. `main`
   did not move. (Seen on 2026-11-20 with the workflow before the train: `ui/bar-foot` and
   `fix/first-use-observations` merged with no session, and each merge started the gates of each waiting
   branch again; the train replaces that.)
5. Push a branch that has a conflict with `dev` in a document. Expected: its gates fail in the step "Merge
   the dev of this run into the checkout", and nothing is merged.

State: Open.

### HT-064. Read the native capability report in Anima.ing Settings

Open the integrated iOS build on a physical device with a test hub. In Settings, open Runner
capabilities and tap Read capabilities. Expect the current power state, thermal state, app
memory headroom, and available storage. Open Technical report and expect the platform, OS,
observation time, and discovery data. Disconnect or reconnect power, wait for iOS to update
the battery state, and tap Refresh. Expect the Last checked time to advance and the report to
show the updated power state. With VoiceOver on iOS or TalkBack on Android, refresh the report
and listen for “Capability report refreshed.” Verify that the browser client has no report entry.

### HT-065: the measurements of the isolation design on Linux (task W2, R-188, R-264)

Built: the script [linux-job.sh](research/work-isolation/linux-job.sh) of the record
[w2-isolation.md](research/work-isolation/w2-isolation.md) (task W2 of
[projects-and-builds-plan.md](plans/projects-and-builds-plan.md)). It is throwaway code of a prototype. It
runs small containers with Podman and no root right, with the flags that the design gives a job, and
writes what it measured to `~/w2-linux.log`. It installs nothing, uses no `sudo`, changes no setting and
removes each container that it made. Two images stay on the disk (`alpine:3` and `docker:dind-rootless`):
about 500 MB.

Not covered by a test: all of it. No session can reach `pop-os`, and the script was not run on a Linux
computer when it was written: only its syntax was checked. If a step gives an error, send the error; do
not try to correct it.

It needs: the computer `pop-os`, a terminal as your own user (not root), the internet, and about
10 minutes. Step 1 is the only step with `sudo`.

1. Install Podman: `sudo apt update && sudo apt install -y podman`. Then run
   `podman run --rm docker.io/library/alpine:3 id`. Expected: after the download of the image, one line
   that starts with `uid=0(root)`. Then run `podman info --format '{{.Host.Security.Rootless}}'`.
   Expected: `true`. If the first command says that `newuidmap` is missing, run
   `sudo apt install -y uidmap slirp4netns` and try again.
2. Get the script:
   `curl -fsSLO https://raw.githubusercontent.com/TheArctesian/anima.ing/dev/docs/research/work-isolation/linux-job.sh`.
   Expected: no output, and `ls linux-job.sh` shows the file. Read it if you want to: it has about 120 lines.
3. Close programs that use much processor time, then run `bash linux-job.sh`. Expected: lines appear for
   3 to 8 minutes, in six parts with the titles `== E1` to `== E6`; the last lines are `== done` and
   `The log is /home/<you>/w2-linux.log`. A line that starts with `WARNING` or says `nothing measured`
   is a result too: send the log as it is.
4. Look at the log: `cat ~/w2-linux.log`. Expected: it holds no address and no name of the computer; it
   holds the name of the system, the kernel, the number of processors and the memory. If it holds a
   thing that you do not want in a public repository, remove that line.
5. Send the log: paste the whole text of `~/w2-linux.log` into the chat of the orchestrating session.
   Expected: a session writes the results into Section 3 of the record and tells you what they mean
   for the design.

After the check you can remove everything again: `podman system reset` removes the images and the
containers of your user, and `sudo apt remove podman` removes the program.

## Not built yet

The Tools section of the Device page and the Tools filter of the device list
([compute-plan.md](plans/compute-plan.md) Section 11.1, CUI-010 to CUI-019) are not built. Their
builder adds the entry.

## Checks that other documents hold

These checks were open before this document. The document in the column "Steps", or the line above each table, holds the steps. This section holds the result of each check (PLN-014), so that one list shows all of them. The results in this version come from the text of those documents and of [handoff.md](plans/phase-3/handoff.md) (2026-11-12). No person or agent ran a check for this version.

- A task that waits for a check is not Done until the row of the check says Passed (PLN-009).
- The words for a result are the states of this document: Open, Passed, Failed, Deferred. Add "Blocked" with the reason when a check cannot start.
- R-169 (operator, 2026-11-18) defers eleven rows of this section and changes twelve rows for the hosted hub. A deferred row says "Deferred (R-169)" with the reason. A changed row stays Open; its steps for the hosted hub are in "The changed checks on the hosted hub" below.
- The operator types each real secret personally. An agent never types, reads or asks for a real secret.

| Group | Checks | Passed | Deferred | Blocks |
| --- | --- | --- | --- | --- |
| Phase 2 (O-1 to O-9) | 9 | 1 | 3 | The Phase 2 live criteria, and the Phase 3 acceptance run (work plan Section 4, tier 0) |
| Phase 3 (P-1 to P-17) | 17 | 0 | 8 | The Phase 3 acceptance run, WP-53 (tier 0). What that run is on a hosted hub is open (OD-W25) |
| One-click (S1 to S8) | 26 | 0 | 0 | The task of each row |
| Command line and MCP | 6 | 0 | 0 | The register row of [cli-mcp-plan.md](plans/cli-mcp-plan.md) |
| A real release | 1 | 0 | 0 | OD-16 to OD-18 of the design specification |

### Phase 2

Steps: [phase-2-acceptance.md](archive/phase-2/phase-2-acceptance.md), "Operator steps". Blocks: the criterion in the column "Criterion" of that document.

| Check | What it proves | Needs | Criterion | Result |
| --- | --- | --- | --- | --- |
| O-1 | RDP: the certificate, the picture, resize, copy and paste, Ctrl+Alt+Delete | A machine with RDP. R-076: no Windows; the operator tests on a second Mac. A Headscale grant on tcp:3389 | 1 | Deferred (R-169, 2026-11-18). A hosted hub has no RDP session yet (HM-T14); S2-RDP is the RDP check of the path that stays. Before: Open, it waited for the second Mac (handoff, 2026-11-12) |
| O-2 | RDP: a changed certificate stops the session; the Warning badge | The machine of O-1 | 2 | Deferred (R-169, 2026-11-18). As O-1. Before: Open, it waited for the second Mac (handoff, 2026-11-12) |
| O-3 | VNC to a Mac and to a Linux desktop | A Mac with Screen Sharing on. A Linux desktop with a VNC server | 3 | Passed on the Mac and on `pop-os` (handoff, 2026-11-12; the Linux run was 2026-11-07, with input off in wayvnc) |
| O-4 | SFTP: list, upload, download, the refusals, the audit entries | A real OpenSSH server with an accepted host key | 6 | Open. Steps for the hosted hub: below (R-169) |
| O-5 | Wake-on-LAN through the site relay | A device that sleeps on a LAN. A relay device at that site with `wakeonlan` | 8 | Deferred (R-169, 2026-11-18). Wake-on-LAN is not possible for a computer of the link (HT-031). A relay that the hub reaches through the link is not verified. Before: Open |
| O-6 | The web view on macOS: fingerprint, a changed certificate, the Down page | A device web interface with an untrusted certificate | 9 | Open |
| O-7 | OD-7 on Linux: the trust hook spike. Also the frame time of the Devices page | A Linux desktop | 9 | Open |
| O-8 | SSH metrics: a new point each 30 s, the bars on the card | One Linux device and one macOS device with SSH | 11 | Open. Steps for the hosted hub: below (R-169) |
| O-9 | Docker, SNMP, Tailscale ping and systemd checks | An SSH device with Docker and systemd. An SNMP v3 agent. The LocalAPI proxy on the VPS | 15 | Open. Steps for the hosted hub: below (R-169). SNMP and the Tailscale ping stay for the self-host path |

### Phase 3

Steps: [phase-3-acceptance.md](plans/phase-3/phase-3-acceptance.md), "Operator steps". Run order: P-1 to P-15, P-17, then P-16. Blocks: the acceptance run of Phase 3 (WP-53). The handoff of 2026-11-12 names what must come first: O-4 to O-9, the wording of the design specification Section 5.7.3, the Headscale daemon policy (R-105), the R2 set-up and the daemon binaries. R-169 defers eight of these checks (P-1 to P-6, P-14, P-16). What the acceptance run is on a hosted hub is an open decision of the operator: OD-W25 in [decisions.md](plan/decisions.md).

| Check | What it proves | Needs | Result |
| --- | --- | --- | --- |
| P-1 | The deployed image and the compose file are the accepted ones | A shell on the VPS | Deferred (R-169, 2026-11-18). The hub on the VPS runs `408bbbab` and is not deployed again. Before: Open |
| P-2 | The T4 probe, with no key | A VPN machine with `bun` | Deferred (R-169, 2026-11-18). The probe runs against the hub on the VPS, from the VPN. Before: Open |
| P-3 | Browsers and sign-in on macOS; the cookie attributes | A Mac on the VPN with Safari and Chrome | Deferred (R-169, 2026-11-18). A browser on the VPN. If a hosted hub serves the web client to a browser is not verified. Before: Open |
| P-4 | A browser on Linux, and the desktop application | A Linux machine with Firefox or Chromium | Deferred (R-169, 2026-11-18). As P-3. The Linux application has its own checks (HT-020, HT-021, S6-3). Before: Open |
| P-5 | The web client: the device list and the detail view | A signed-in browser (P-3) | Deferred (R-169, 2026-11-18). As P-3. Before: Open |
| P-6 | SSH and VNC sessions in the web client | A device with SSH. A device with VNC | Deferred (R-169, 2026-11-18). As P-3. Before: Open |
| P-7 | The notification channels are entered | A signed-in client. The operator types each secret | Open. Steps for the hosted hub: below (R-169) |
| P-8 | Channels are selected on a check | A signed-in client | Open. Steps for the hosted hub: below (R-169) |
| P-9 | "Send test" on each channel | The receiving application of each channel | Open. Steps for the hosted hub: below (R-169) |
| P-10 | A Down and Up cycle sends the messages | The check of P-8, and a way to stop its service | Open. Steps for the hosted hub: below (R-169) |
| P-11 | Maintenance sends no message | The check of P-8 | Open. Steps for the hosted hub: below (R-169) |
| P-12 | A live RDAP check | A signed-in client | Open. Steps for the hosted hub: below (R-169) |
| P-13 | GPU metrics on the Apple silicon Mac | The Mac with the push daemon in operation | Open. Steps for the hosted hub: below (R-169) |
| P-14 | Optional: a real Prometheus scrape | A Prometheus server on the VPN. A Headscale grant (ask first) | Deferred (R-169, 2026-11-18). A Prometheus server on the VPN and a Headscale grant. Before: Open |
| P-15 | Optional: the API key checks in Settings | A signed-in client | Open. Steps for the hosted hub: below (R-169) |
| P-17 | The desktop application: the update banner | A build with an older version installed, and a newer release | Open. The same act as REL-1 (R-169): 0.2.1 is installed and 0.2.2 is published. One run gives the result of the two rows |
| P-16 | Last: the T3 log scan of the live hub log | A shell on the VPS, after all other steps | Deferred (R-169, 2026-11-18). The log of the hub on the VPS. Before: Open |

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
| CLI-6 | The list of GH-60 against the production hub: sign-in, `anima ssh`, `anima files`, a confirmation prompt | The production hub | Open. Steps for the hosted hub: below (R-169). The read commands for GitHub were checked on 2026-11-16, against the hub on the VPS |

### The changed checks on the hosted hub

R-169 changed these twelve checks, because the document that holds their steps names the hub on the VPS, the VPN or a browser. Do each check as its document says, with the differences in this table. The terms are those of "What a check on the hosted hub needs" in the section "Open". Each row stays Open.

| Check | Do it on | What is different from the steps of its document |
| --- | --- | --- |
| O-4 | A computer on the link of the hosted hub, after HT-031 (port 22 on, SSH added, the host key accepted) | Do steps 1 to 5 in the application on the hosted hub. Step 5 of HT-031 opens the files; this check adds the upload, the download, the two refusals and the audit entries. Turn port 22 off again after the check. |
| O-8 | This Mac on the link, after HT-031. Then a Linux computer on the link | Set the metrics source of the device to SSH. Watch the Overview for 2 minutes: a new point each 30 s, and the bars on the card. The Linux part needs the Linux application, which joins the computer to the link: it waits for the Linux release. Write the two results apart. |
| O-9 | A computer on the link with Docker, and a Linux computer on the link with systemd, after HT-031 | Add one Docker check and one systemd check. Each must reach Up, or Down with the fixed message. Do not add an SNMP check or a Tailscale ping check: the link carries TCP only (HT-031), and the LocalAPI proxy is on the VPS. Those two types stay for the self-host path and are not checked on the hosted hub. |
| P-7 | The application on the hosted hub | "Any signed-in client" is the application; there is no browser. Steps 1 to 3 as written. The operator types each secret. |
| P-8 | The application on the hosted hub | As written. The service that the check watches must be one that the hosted hub reaches: a public address, or a port of a computer on the link that is on (HT-031). |
| P-9 | The application on the hosted hub | As written. |
| P-10 | The application on the hosted hub, with the check of P-8 | As written. A free hub that sleeps sends nothing while it sleeps (HMD-028): write down if the hub slept in the time of the check. |
| P-11 | The application on the hosted hub, with the check of P-8 | As written. The same note as P-10. |
| P-12 | The application on the hosted hub | As written. |
| P-13 | This Mac, with a daemon on the hosted hub (HM-D16, not verified) and SSH through the link (HT-031) | "The push agent" is `anima-daemon` (R-126). Steps 1 to 3 as written, in the application on the hosted hub. |
| P-15 | The application on the hosted hub | As written. For "use it once more" give the revoked key to the command line in `ANIMA_TOKEN`; the address of the hub for the command line is not verified. |
| CLI-6 | The hosted hub, which is now the production hub | The first line of GH-60 (`anima auth login` with a password and an authenticator code) cannot run on a hosted hub: it waits for OD-25. For `anima ssh` and `anima files`, use a computer on the link after HT-031. For these two and for the confirmation prompt (`device delete`), try a key for the command line; if the hub refuses the key, they wait for OD-25. If a key can do them is not verified. |

### Machines that the checks need

One visit to a machine can close many rows.

| Machine or access | Checks |
| --- | --- |
| A second Mac | S1-T14, S3-T12, S6-2 |
| A Linux desktop or laptop | O-7, O-8 (the Linux part), O-9 (systemd), S3-T14, S4-T10, S6-3, S8-9 to S8-13, CLI-3 |
| The `x86_64` Linux build computer | S3-T10 |
| This Mac only | S2-T9, S6-1, S8-1 to S8-8, CLI-2, CLI-4 |
| This Mac with the application on the hosted hub | O-4, O-8, O-9 (Docker), P-7 to P-13, P-15, CLI-6 |
| Special set-up | S3-T13 (a Mac that can lose power), S6-4 (the router), S6-5 (no UDP) |
| Not now: Deferred (R-169) | O-1, O-2, O-5, P-1 to P-6, P-14, P-16; the SNMP and Tailscale ping parts of O-9 stay for the self-host path |

### A real release

| Check | What it proves | Steps | Result |
| --- | --- | --- | --- |
| REL-1 | A real release confirms the signature, the notarization and the AppImage launcher (OD-16 to OD-18) | [design-spec.md](spec/design-spec.md), Section 11 | Open. For macOS 0.2.0 and 0.2.1, the signature and the notarization are checked ([releases.md](procedures/releases.md), "Release record"). Not recorded here: the update of an installed application, and the AppImage launcher on Linux |

## Deferred

The operator deferred these fifteen checks on 2026-11-18 (R-169: "Approve the sorting"). Each can run only against the hub on the VPS or its VPN, which the operator leaves (R-130, R-140), or on a server path that he does not use now. No check is deleted, and none is Passed. A ruling or the operator sets a check to Open again (PLN-014). [sort-2026-11-18.md](research/human-checks/sort-2026-11-18.md) has the reason for each.

| Check | Subject | Reason | Where |
| --- | --- | --- | --- |
| HT-009 | The hub image builds with Rust 1.100.0 | The hub on the VPS is not deployed again. If a hub of the pool uses `deploy/hub/Dockerfile` is not verified | Entry below |
| HT-017 | "This machine" with the Anima VPN and NetBird together | A computer of a hosted hub has a link address and no VPN address | Entry below |
| HT-019 | The VPN role `daemon` on the VPS | The policy of the VPN that the operator leaves | Entry below |
| HT-024 | A hub restored on a second server from its backup | The operator starts fresh on the hosted hub (R-140). Row 4.2 of the priorities waits for nothing because of this ruling | Entry below |
| HT-058 | The page Extensions as a tight list and as a grid (EXT-001) | The desktop application of a build that holds the branch `ui/extensions-grid` | Open |
| O-1, O-2 | RDP | A hosted hub has no RDP session yet (HM-T14) | "Phase 2" above |
| O-5 | Wake-on-LAN through the site relay | Not possible for a computer of the link (HT-031) | "Phase 2" above |
| P-1 to P-6 | The deployed image, the T4 probe, and the web client in a browser | The hub on the VPS and its VPN | "Phase 3" above |
| P-14 | A real Prometheus scrape | A Prometheus server on the VPN | "Phase 3" above |
| P-16 | The log scan of the live hub log | The log of the hub on the VPS | "Phase 3" above |

### HT-009: the hub image builds with Rust 1.100.0

Built: `deploy/hub/Dockerfile` now starts from `rust:1.100.0-bookworm`. No person built the
image after the change.

1. At the next hub deployment, build the hub image. Expected: the build completes.
2. Start the hub. Expected: the health check passes and migration 0715 is applied.

Result: Deferred (R-169, operator, 2026-11-18: "Approve the sorting"). Reason: the hub on the VPS is not deployed again, so there is no "next hub deployment" on that path now. A hub of the pool runs `anima-local-hub run` (HM-T7); if its image uses `deploy/hub/Dockerfile` is not verified. Before this ruling the entry was Open, and no step was done.

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

Result: Deferred (R-169, operator, 2026-11-18: "Approve the sorting"). Reason: the check needs the Anima VPN of the hub on the VPS. A computer of a hosted hub has a link address and no VPN address. Before this ruling the entry was Open, and no step was done.

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

Result: Deferred (R-169, operator, 2026-11-18: "Approve the sorting"). Reason: the check is about the live Headscale policy of the VPN that the operator leaves. Before this ruling the entry was Open, and no step was done.

### HT-024: a hub restored on a second server from its backup (FND-006, task F1)

Built: the procedure [restore-and-move.md](procedures/restore-and-move.md). An agent did its database
steps (3 and 6 to 10) on a new, empty setup in Docker on 2026-11-17; the section "Proof" of the
procedure has the results.

Not covered: a real server. No test joined the VPN, ordered the certificate, started the hub on the
restored database or opened a session with a restored credential.

1. On a second Linux server, do steps 1 to 5 of the procedure. Expected: each `bin/preflight` command
   prints no refusal.
2. Do steps 6 to 10 with the newest dump of the hub. Expected: "Restored ... into anima.", `migrate`
   exits with code 0, and "Restore check passed".
3. Do steps 11 and 12. Expected: `docker compose ps` shows `hub` as running, and the client connects
   at the address of the hub.
4. In the client, open the device list and the audit log, then open a session to a device that has a
   stored credential. Expected: the devices and the log are those of the old hub, and the session
   opens with no question for the credential.

Result: Deferred (R-169, operator, 2026-11-18: "Approve the sorting"). Reason: the operator starts fresh on the hosted hub (R-140), so no hub is restored on a server now. The function is built (task F1); the check stays for the self-host path. Before this ruling the entry was Open, and no step was done on a real server.

## Passed

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

### HT-045: Linux opens the account sign-in screen before a hub is chosen (HMD-022)

The preview was built from `665fb16a` on `fix/linux-start-sign-in`. Its startup guard in
`apps/client/src/lib/access/guard.ts` opens `/welcome` when no hub address is saved. Tests cover a missing token, an old token
for the default hub and a refused key store. They also check that a chosen hub still resumes
with its token. An automatic Linux WebKit check reached the screen with an old default-hub
token and with no token. The packaged preview also showed the screen in a virtual X11 display
without `LD_PRELOAD` (2026-11-19). These checks do not verify the normal desktop session or
complete a real account sign-in.

1. Start the rebuilt Linux application on the installation that showed the wrong first screen.
   Expected: "Welcome to Anima.ing", with "Sign in", "Sign up" and "Join a hub". No Devices
   sidebar or hub password form appears.
2. Choose "Sign in" and complete the account sign-in in the browser.
   Expected: the app joins the hosted hub and opens its device list, as in HT-022.
3. Quit the app and start it again.
   Expected: the chosen hub's device list opens without another sign-in.

Result: Passed. On 2026-11-19 the operator reported that the Linux preview worked and showed the
signed-in Devices list. The screenshot shows one `pop-os` row on the link, Up, and `Hub` Up.
The operator then quit and reopened the preview. It returned to the same hub without another
sign-in and still showed one `pop-os` row. The row showed the generic Linux logo and no metrics.

Observation (2026-11-19; the operator, from screenshots of a local build of `main` at `132c24dc`, version 0.8.16, signed in to his hosted hub; first-hand to the orchestrating session, relayed to the session that writes this; not a result, only he sets Passed or Failed): the Devices list of his hosted hub showed three rows, all Up: `Hub` with the host `localhost`, his MacBook with a link address, and `pop-os` with a link address. Each row showed "No metrics", which is expected until the daemon is installed (task HM-T19). So a Linux computer joined the hosted hub through the application. Which build ran on `pop-os` is not known here.

### HT-047: the Devices page after the interface review (R-224)

Built on the branch `ui/v2-devices` (`132e03b3`): in `apps/client/src/lib/devices/card/DeviceTable.svelte` the name is the
first column and the status the second; the connect buttons of the list show the name of their protocol
(`lib/devices/actions/DeviceActions.svelte`); the logs panel of the Devices page starts as one line
(`lib/devices/screen/DeviceScreen.svelte`); the foot of the bar names the hub (`lib/nav/status/HubStatus.svelte`).
The tests cover the order of the columns, the name of the hub and the start of the panel. The page was looked at only
in pictures of the gallery tool, at 1280 px, in the dark and the light appearance, with sample devices. No person saw it
in the desktop application, with real devices, at a narrow window or with a device that is down.

1. Open the application and look at Devices in the list view.
   Expected: each row starts with the name and the address, then the status. A device that needs a look has its reason
   beside the status.
2. Look at the column "Connect".
   Expected: each way in is a button with an icon and a word (SSH, RDP, VNC, Web). Two buttons fit on one line.
3. Look at the foot of the page.
   Expected: the logs are one line with the newest event. A click on the line opens the panel; a second click closes it.
4. Look at the foot of the bar.
   Expected: on a hosted hub the line above "Connected" reads "Hosted by Anima.ing"; on a different hub it is the host name.
5. Make the window narrow (below 1100 px) and look again, in the light and the dark appearance.
   Expected: no text is cut in a way that hides its meaning, and no button lies over another part.
### HT-048: the tab Overview of a device after the interface review (R-224)

Built on the branch `ui/v2-device-overview` (`f3d25339`): `apps/client/src/routes/devices/[id]/overview/geometry.css` draws
the four facts cards (Status, Uptime, Checks, SSH host key) in one row from a panel width of 56rem, and a card has no
border. No test reads a layout. The page was looked at only in pictures of the gallery tool, at 1280 px, in the dark and
the light appearance, with one sample device. No person saw it in the desktop application, at other widths, with a long
host key, with many check states or while it loads.

1. Open a device in the application, in a window about 1280 px wide.
   Expected: Status, Uptime, Checks and SSH host key are one row. The three uptime figures are on one line. No card has a border.
2. Make the window narrow, then wide, in the light and the dark appearance.
   Expected: the cards go to two columns and then to one, and back; no text lies over another part; the whole host key shows.
3. Reload the page and watch the first second.
   Expected: the loading shapes are where the cards then are; nothing moves when the values arrive.

Result: Open.

### HT-060: the Projects page after the interface review (R-224)

Built on the branch `ui/v2-projects` (`0d1c1303`), in `apps/client/src/lib/projects`: the page has a choice "All", "Needs a
look", "Deployed" and "Not deployed" above the tiles, each with the number of projects of the page that it holds, and a
tile has a raised surface and no border. The tests prove the choice. The page was not seen at all: the test hub of the
gallery tool has no project, so there is no picture of it, in the dark or the light appearance.

1. Turn the extension "Projects" on, connect GitHub, and open Projects in a window about 1280 px wide.
   Expected: above the tiles is one line with the choice of the group, then Sort, Language and Size. Each choice ends with
   a number. Each tile is a little lighter than the page and has no border; a tile that needs a look has a coloured edge
   at its left.
2. Select "Needs a look", then "Deployed", then "Not deployed", then "All".
   Expected: only the tiles of that group show, and the number of tiles is the number of the choice. A group with no
   project says "No project in this group".
3. Select "Left out" at the top of the page, then "Projects".
   Expected: the choice of the group is not there for "Left out", and comes back for "Projects".
4. Set Sort to "Last commit".
   Expected: the choice of the group is not there, and the tiles are one list.
5. Make the window narrow (about 800 px), in the light and the dark appearance.
   Expected: the controls go to a second line; no part lies over another.
### HT-057: the Settings page after the interface review (R-224)

Built on the branch `ui/v2-settings` (`1ffb6bf8`): the index of the Settings page (`apps/client/src/routes/settings/index.ts`,
`apps/client/src/lib/page/contents/PageIndex.svelte`) names three groups, "Hub and account", "This computer" and
"Connections", and the sections of the page are in the same order. The tests prove the order and the group names. The
page was looked at only in pictures of the web client at 1280 px. No person saw it in the desktop application, which
has the sections Hub, Stay awake, Updates and MCP, or on a phone.

1. Open Settings in the desktop application, in a window about 1280 px wide.
   Expected: the index at the left has three small titles. Under "Hub and account": Account, Hub, Notifications, Default
   check settings, Data retention. Under "This computer": Stay awake, Appearance, Updates, MCP. Under "Connections":
   Integrations, API keys, Extensions, and VPN on a hub that is not hosted.
2. Scroll the page from the top to the foot.
   Expected: the sections come in the order of the index, and the index marks the section at the top of the window.
3. Select "MCP" in the index, then "Account".
   Expected: the page goes to that section each time.
4. Make the window narrow (below 1100 px).
   Expected: the index is one row of links above the sections, with no group title.
5. Open Settings on a phone.
   Expected: Account and Hub are first and "Leave" is last; no part lies over another.
### HT-052: the tab Containers of a device after the interface review (R-224)

Built on the branch `ui/v2-device-containers`, in `apps/client/src/lib/containers`: the table has one column "State" with
the state word and the status text of Docker; the log and the details of the selected container are one raised pane
with a switch "Log" and "Details"; the count of the lines is in the bar of the log tools. The tests prove that the switch
reads the details and the log. No test reads a layout. No person saw the tab in the desktop application, with long
names, with many containers or at a narrow width.

1. Open a device that has Docker, then the tab "Containers", in a window about 1280 px wide.
   Expected: each row has the name, then the state word with a dot and the status text of Docker beside it (for example
   "Running" and "Up 3 hours (healthy)"), then the image and the ports. No text lies over another part.
2. Select "Log" on a row.
   Expected: a pane opens under the table with the title "Log of" and the name, a switch "Log" and "Details", the choice
   of the lines, "Refresh" and a close button. The search, the level, "Wrap", "Timestamps", the count ("200 of 200 lines")
   and "Download" are one bar.
3. Select "Details" in the switch of the pane, then "Log".
   Expected: the same pane shows the details, then the log again. The table does not move.
4. Make the window narrow (about 900 px) and look again, in the light and the dark appearance.
   Expected: the status text is cut with an ellipsis and its full text is the tooltip; the state word is whole; the tools
   of the log go to a second line and none lies over another.

Result: Open.

### HT-067: the tab Overview with the cards Connect and About, the chart figures and the storage table (R-224)

Built on the branch `ui/v2-overview-full` (`32e6881a`), in `apps/client/src/routes/devices/[id]/overview` and
`apps/client/src/lib/metrics/storage`. The tests prove the rows of the card "Connect", the texts of the card "About",
the figures of a chart and the rows of the storage. No test reads a layout. The tab was looked at only in pictures of the
web client at 1280 px, with one chart and with no drive. No person saw it with real metrics, with drives, with a
daemon, on a hosted hub, at other widths or while it loads.

1. Open a device in the application, in a window about 1280 px wide.
   Expected: at the left, Status, Uptime and Checks are one row, then Metrics, then Storage. At the right, from the top:
   "Connect", "SSH host key", "About". Nothing lies over another part.
2. Look at "Connect".
   Expected: one row for SSH, one for RDP, one for VNC when it is on, and one for each web interface of the device. A
   way that works has "Open"; the first one is the filled button. SSH with no accepted host key has "Test". A way that
   is not set up says "Not set up" and has "Set up", which opens its section of the Settings tab.
3. Select "Open" for SSH, then for a web interface.
   Expected: the session or the web tab opens as it does from the button at the top of the page. On the hosted hub,
   write down what a web interface does: the client offers the button on every hub.
4. Look at "About" for a device with a daemon and for one with none.
   Expected: "Daemon" gives the version and "sending metrics", or "not sending now" when the hub marks the daemon as silent, or
   "Not installed". "Sessions" gives the limit or "No limit".
5. Look at "Metrics" for a device that sends metrics, and select "24 hours" and "7 days".
   Expected: each chart of one line has "peak" with the highest value of the range and then the newest value on its
   title line; the charts change with the range and nothing else moves.
6. Look at "Storage" on a Linux device with a drive that is not mounted.
   Expected: a line of column names, then one row for each drive: name, kind, size, each mount with its value and bar,
   and read, write and busy. The drive with no mount says "Not mounted" and has no bar and no read and write. A drive
   with two mounts has one line for each mount under its name.
7. Open a device with no metrics.
   Expected: "Metrics" is one line with "Open Settings" at its end. On a hub that Anima.ing hosts the line is "Metrics
   are off. They appear when the daemon is installed on this device."; on a different hub it is "Metrics are off.
   Install the daemon or turn on SSH metrics in Settings.". There is no Storage card.
8. Make the window narrow (about 900 px), then reload the page, in the light and the dark appearance.
   Expected: "Connect", "SSH host key" and "About" are above the facts as one column; while the page loads, grey shapes
   are where the cards then are.
### HT-062: `anima mcp install` with the real coding agents (MCP-027 to MCP-038)

Built on the branch `mcp/install`, in `crates/anima-cli/src/mcp/install`. The tests prove each way against a temporary
home folder and a stand-in program: the bytes that stay, the copy, the refusal of a symbolic link, the dry run of the
library. No test starts a real agent and none reads a real configuration. The place and the form of each entry are from
the documentation of each product on 2026-11-19 and were not tried. The part "Coding agents" of Settings (task MI4)
is built and was not seen: no desktop build was made.

1. Run `anima mcp agents`.
   Expected: one row for each of the eight agents. An agent that you have says "not set up"; one that you do not have
   says "not on this computer".
2. Run `anima mcp install --dry-run`.
   Expected: for each agent that you have, the file that would be written with its copy, or the command line that would
   run, with the full path of `anima`. Look at a file that it names: it is not changed.
3. Run `anima mcp install` and answer yes. Then run `anima mcp agents`.
   Expected: each agent says "set up". Beside each written file there is a file with the same name and `.before-anima`,
   which is the file as it was. Compare the two: only the entry `anima` is different; your comments and your other
   servers are as they were.
4. Start each agent anew and look at its list of MCP servers.
   Expected: a server `anima` is there and its tools work. Write down for each agent: yes or no. The points that the
   documentation does not settle are: Claude Code at user scope; VS Code through `~/.copilot/mcp-config.json`; opencode
   under `mcp`, `servers`; Zed at `~/.config/zed/settings.json`; whether an agent needs a restart.
5. Run `anima auth logout`, start one agent anew, then sign in again.
   Expected: while signed out the agent shows the server as failed or with no tool; nothing asks for a token.
6. Run `anima mcp uninstall` and answer yes.
   Expected: each agent says "not set up". Each file is the same as its copy, byte for byte (an object that Anima had to
   make stays, empty).
7. Put a server named `anima` with a different command into one agent by hand, then run `anima mcp agents`,
   `anima mcp uninstall`, and `anima mcp install`.
   Expected: the state is "set up, starts a different anima"; uninstall leaves it; install replaces it after it shows
   it in its list.
### HT-056: the bar with Live sessions at the foot and Extensions beside Settings (R-255)

Built on the branch `ui/bar-foot` (`66c730ad`): in `apps/client/src/lib/nav/Nav.svelte` Live sessions is directly above the
name of the hub, and Extensions is an entry directly above Settings that opens the new page
`apps/client/src/routes/extensions`. The Settings page has no section Extensions on a computer. The tests prove the
order of the entries. The bar was looked at only in pictures of the web client at 1280 px; the page Extensions was
pictured with no extension. No person saw the narrow bar, the panel of Live sessions at its new place, the page with
its switches, or the phone.

1. Open the application in a window about 1280 px wide.
   Expected: the top of the bar has Devices, Agent usage, and Projects while that extension is on. At the foot, from the
   top: Live sessions with its count, the name of the hub with "Connected", Extensions, Settings.
2. Select "Live sessions" while a session is open.
   Expected: its panel opens beside the bar and is whole inside the window; "Stop" ends the session.
3. Select "Extensions".
   Expected: a page "Extensions" opens with each extension and its switch. Set Projects on: "Projects" comes into the
   top of the bar at once. Set it off again.
4. Open Settings.
   Expected: there is no section Extensions, and the index does not list it.
5. Make the bar narrow with the control at its top.
   Expected: the same order as icons; each icon has its name as a tooltip; Live sessions keeps its count.
6. Open the application on a phone, then Settings.
   Expected: the bottom bar has Devices, Sessions, Logs and Settings. Settings has a section "Extensions" with the link
   "Open Extensions", which opens the page.
8. In the desktop application, open Settings, then MCP, with the command line tool not installed (Settings, Updates).
   Expected: the part "Coding agents" says to install the command line tool first and shows no agent.
9. Install the command line tool, then open Settings, MCP again.
   Expected: one row for each agent that you have, with its state in words, and "Set up all". Select "Set up" on one
   row: a box lists the file that is written with its copy, or the whole command line, and "Set up" and "Cancel".
   "Cancel" changes nothing. "Set up" changes the row to "Set up" with a button "Remove".
10. Select "Set up all", confirm, then select "Remove" on one row and confirm.
   Expected: the same results as steps 3 and 6 for those agents. A row that was refused says why under it.

Result: Open.

### HT-058: the page Extensions as a tight list and as a grid (EXT-001)

Built on the branch `ui/extensions-grid` (`189a48a5`): `apps/client/src/lib/extensions/ExtensionsSection.svelte` draws each
extension as one line or as a tile, and `apps/client/src/routes/extensions/+page.svelte` has the control "List" and
"Grid" and keeps the choice on the device. The tests prove the switches, the marks and that the choice is kept. The
page was looked at only in pictures of the web client at 1280 px with a sample list. No person saw it with the
extensions of a real hub, at other widths or on a phone.

1. Open "Extensions" from the bar, in a window about 1280 px wide.
   Expected: one line for each extension: its mark, its name, one line of words, and at the end its switch, "On" for
   one that is always on, or nothing for one that is not built. A long line of words is cut and its full text is the
   tooltip. The rows are closer than they were in Settings.
2. Select "Grid".
   Expected: the same extensions as tiles, each with its mark on a plain field, its name, its switch and its words in
   full. No tile shows a screen that does not exist.
3. Set "Projects" on in each view.
   Expected: "Projects" comes into the bar at once. A switch that needs a second extension asks first, as before.
4. Close the application and open it again, then open "Extensions".
   Expected: the view that you chose last is shown.
5. Make the window narrow (about 800 px), in the light and the dark appearance, and look at both views.
   Expected: no part lies over another; the tiles go to fewer columns.

Result: Open.
