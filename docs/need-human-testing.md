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
| HT-002 | The tool inventory from an agent to the command line | A hub and an agent of this build | Open |
| HT-003 | The tool inventory in an MCP client | HT-002 | Open |
| HT-004 | No window of Apple on a Mac without the developer tools | A Mac with no Xcode and no Command Line Tools | Open |
| HT-005 | The tool inventory of an agent that runs as a system unit | A Linux computer | Open |
| HT-006 | The probes of `claude` and `codex` leave the account alone | A computer with the two programs | Open |
| HT-007 | The pinned Rust version on the Linux computer | The Linux development computer | Open |
| HT-008 | A macOS release build with the pinned Rust version | A Mac builder | Open |
| HT-009 | The hub image builds with Rust 1.100.0 | The next hub deployment (VPS-005) | Open |
| HT-010 | "This machine" with the Anima VPN and NetBird together | A Mac with Tailscale and NetBird, and a second tailnet account | Open |

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

### HT-002: the tool inventory from an agent to the command line

Built: `crates/agent-core/src/inventory/`, `crates/agent/src/readers/inventory.rs`, the hub
migration `0715_metrics_tools.sql`, `anima capability list` (CAP-020 to CAP-031). The tests use
scripts in place of the real tools, and a scripted hub.

The hub and the agent must be of this build. A change to the hub on the VPS needs the
confirmation of the operator (VPS-005). A local hub is sufficient for this check
([setup/local-hub.md](setup/local-hub.md)).

1. Start a hub of this build. Sign in with `anima auth login`.
2. Enrol the agent of one device and start the `anima-agent` of this build on it.
3. Wait 1 minute. Run `anima capability list`. Expected: one row for the device, with its
   architecture, its operating system version and its tools.
4. On the device, run `cargo --version`, `git --version` and `bun --version`. Expected: the same
   versions as in step 3.
5. Run `anima capability list --tool cargo`. Expected: only the devices that have cargo.
6. Run `anima capability list --target <a target that the device has>` and then with a target
   that it does not have. Expected: the device is in the first list and not in the second.
7. Install or remove one tool on the device. Wait 16 minutes. Expected: the list shows the change.
8. Read the log of the agent. Expected: no version text and no output of a tool (LOG-007).

Result: Open.

### HT-003: the tool inventory in an MCP client

Needs HT-002.

1. Start an MCP client with `anima mcp` and a read API key.
2. Ask the client to call `anima_capability_list`. Expected: the same devices and tools as
   `anima capability list`.
3. Call it with `tool` set to `cargo`. Expected: the same devices as step 5 of HT-002.

Result: Open.

### HT-004: no window of Apple on a Mac without the developer tools

Built: the agent does not run the placeholder programs `/usr/bin/git` and `/usr/bin/xcodebuild`
when `xcode-select -p` fails. The review could not do this check, because its Mac has Xcode.

1. Use a Mac where `xcode-select -p` prints an error.
2. Install and start the `anima-agent` of this build.
3. Use the Mac for 20 minutes. Expected: the window "install the command line developer tools"
   does not show.
4. Run `anima capability list`. Expected: the device has no `git` row and no `xcodebuild` row,
   unless a different git is installed (for example from Homebrew).

Result: Open.

### HT-005: the tool inventory of an agent that runs as a system unit

The systemd system unit hides the home folders and denies memory that is writable and
executable. No person looked at what such an agent reports.

1. On a Linux computer, install the agent as a system unit and start it.
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
2. Run the agent of this build for 1 hour with the two programs closed.
3. Do step 1 again. Expected: the same value.
4. During step 2, look at the network connections of the processes `claude` and `codex` that the
   agent starts (for example with `nettop` or Little Snitch). Write down each connection.

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

### HT-010: "This machine" with the Anima VPN and NetBird together

Built: on the branch `claude/vpn-own-address` (bdb321f1, 71bfcbdb; ADR-005, R-060), not
merged. The desktop app takes its own VPN address from the Tailscale client that is logged in
to the hub's login server (`vpnLoginServer`). It does not read the interfaces. Security finding:
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

## Not built yet

The Tools section of the Device page and the Tools filter of the device list
([compute-plan.md](compute-plan.md) Section 11.1, CUI-010 to CUI-019) are not built. Their
builder adds the entry.

## Checks that other documents hold

These checks were open before this document. Their owner holds the steps and the state. They
are here so that one list shows all of them.

| Subject | Owner |
| --- | --- |
| The Phase 3 acceptance run (WP-53) | [programme/handoff.md](programme/handoff.md) |
| Live checks of the native VNC gateway (S2, task T9) | [research/one-click/s2-vnc-tasks.md](research/one-click/s2-vnc-tasks.md) |
| Live checks of the stay-awake control (S8) | [research/one-click/s8-stay-awake-tasks.md](research/one-click/s8-stay-awake-tasks.md), Section 6 |
| The command line and the MCP server against a real hub (#60) | [cli-mcp-plan.md](cli-mcp-plan.md) |
| A real release that confirms the signature, the notarization and the AppImage launcher (OD-16 to OD-18) | [design-spec.md](design-spec.md), Section 11 |

## Passed

None.
