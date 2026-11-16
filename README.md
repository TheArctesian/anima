<p align="center">
  <img src="design/brand/logo.png" alt="Anima.ing recursive doorway logo" width="160" height="160">
</p>

<h1 align="center">Anima.ing</h1>

<p align="center"><strong>The machines your agents run on, behind one door.</strong></p>

<p align="center">
  An open-source device manager for remote access, uptime and metrics.<br>
  Self-hosted today. A hosted network is planned.
</p>

<p align="center">
  <a href="https://www.anima.ing">Website</a>
  ·
  <a href="LICENSE">AGPL-3.0-or-later</a>
  ·
  <a href="docs/setup/anima-mcp.md">MCP</a>
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-AGPL--3.0--or--later-4C566A" alt="License: AGPL-3.0-or-later"></a>
  <a href="https://www.anima.ing"><img src="https://img.shields.io/badge/website-anima.ing-5E81AC" alt="Website"></a>
</p>

<p align="center">
  <img src="design/brand/social-preview.png" alt="Anima.ing" width="640">
</p>

<!-- TODO: Insert a product screenshot here once one exists.
     Suggested file: docs/images/devices.png (about 1280×800, Devices view on macOS). -->

Anima.ing is a self-hosted device manager for the computers and servers you already have — including the ones AI agents run on. It combines a native macOS and Linux client, a Rust hub, and a gateway for remote sessions. A command-line tool and an MCP server let you and your coding agents use the same hub.

The first public release of the client is coming soon. Until then, you can build from this repository.

## Features

**Reach a machine**

- SSH from the desktop client, a browser on the VPN, or `anima ssh <device>`
- Graphical desktops over VNC and RDP
- Device web interfaces, such as a Proxmox or Home Assistant admin page
- File transfer over SFTP on an SSH session (`anima files`)
- Wake-on-LAN from the hub (`anima device wake`)

**See what is up**

- Uptime checks: HTTP, ICMP, TCP, TLS, DNS, Docker, systemd, SNMP, push, groups, Tailscale ping, and domain expiry
- Resource metrics: CPU, memory, disk (usage and I/O), network, load, uptime, and GPU (NVIDIA, AMD, Intel, and Apple)
- Metrics from a small agent (`anima-agent`) or, without an agent, over SSH
- Alerts on webhook, ntfy, email, or [Apprise](deploy/README.md)
- A Prometheus scrape endpoint at `/metrics` ([setup](docs/setup/prometheus.md))

**Keep the network private**

- A WireGuard mesh around your own hub, run with Headscale, with no open ports required on devices ([VPN guide](docs/setup/vpn.md))
- Sign-in with a password and a TOTP code
- `anima-link`: an encrypted device-to-hub path that needs no VPN client and no administrator rights. You can run it by hand today; the desktop app does not start it yet ([setup](docs/setup/link.md))

**Use it from a terminal or an agent**

- The `anima` command line, which talks to the same GraphQL API as the client
- `anima mcp`, an MCP server on standard input and output ([setup](docs/setup/anima-mcp.md))

## Why Anima.ing

Most people assemble this stack from three or four tools: a mesh VPN, a remote-desktop gateway, and an uptime checker. Anima.ing is one hub, one native client, and one API — including an MCP server so a coding agent can list machines, read checks, and (with your permission) change them.

Self-hosting is free. A hosted plan called Solo is planned at $19 a month or $199 a year, for the case where Anima.ing runs the private network and relay for you. Solo is not for sale yet, and there is no public waitlist page at the time of writing. Paid plans are meant to sell hosting and support, not to withhold code: the same software stays in this repository.

## Status

| | Today | Planned |
| --- | --- | --- |
| Self-host the hub and client | Yes, build from source | Signed macOS and Linux downloads |
| Desktop client | macOS 14.2+ (Apple Silicon and Intel), Linux AppImage (x86-64). Linux ARM64 support. | Windows |
| Web client | Served by the hub to browsers on the VPN ([guide](docs/setup/web-client.md)) | iOS |
| Remote desktops | VNC and RDP through `guacd`; native VNC via `anima-gateway` | Native RDP |
| Private network | Headscale / WireGuard; `anima-link` by hand | An Anima.ing-operated Tailcat relay; app-started `anima-link` |
| Hosted Solo | — | $19 / month or $199 / year, waitlist only |

There is no public download yet: [releases.anima.ing](https://releases.anima.ing) is not serving builds, and this repository has no GitHub Releases. The first release is coming soon.

## Quick start (build from source)

You will need a recent [Rust](https://rustup.rs) toolchain (the hub image is built with 1.99.1), [Bun](https://bun.sh) 1.3, and a clone of this repository. PostgreSQL 17 is required to run the hub and the test gates. [Go](https://go.dev) 1.27 is required only if you build `anima-link`.

### Desktop client

On Linux, install the WebKit and related development headers first:

```sh
# Debian, Ubuntu, and similar.
sudo apt install build-essential curl wget file libssl-dev \
  libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev \
  libxdo-dev pkg-config
```

Then, from the repository root:

```sh
bun install --frozen-lockfile
bun run --cwd apps/client build
bun run --cwd apps/desktop tauri build
```

That produces a macOS `.app` / `.dmg`, or a Linux AppImage for the architecture of the machine you build on (x86-64 or ARM64). Official signed releases and the updater currently target Linux x86-64 only; the official builder has no ARM64 target yet. The bundle targets are `app`, `dmg`, and `appimage`. There is no Windows target.

A "no private key" message about the updater after the bundles are made is expected when you build locally: official updates are signed by the hub, not by a local `tauri build`.

To develop the client:

```sh
bun run --cwd apps/desktop dev
```

The dev server is the Tauri shell at `http://localhost:5173`. A hub only accepts the desktop app's own origin, so use a built app (not `tauri dev`) against a deployed hub.

### Command line

The `anima` program is meant to ship with the desktop app (Settings → Updates → Command line tool) once there is a release. Until then, build it yourself:

```sh
cargo build --release --locked --bin anima
# The binary is target/release/anima
```

```sh
anima --version
anima auth login
anima device list
anima check list
anima ssh <device>
```

`anima ssh` needs a login session, not an API key. Use `--help` on any command for its flags. `anima completion <shell>` prints shell completion.

### Hub

The supported way to run the hub is Docker Compose, which starts PostgreSQL 17, the migration job, TLS, `guacd`, and the hub. It expects a Linux host with Docker Engine 28 or later and a WireGuard address for the hub. The full steps, including secrets and the Headscale VPN, are in [deploy/README.md](deploy/README.md) and [docs/setup/vpn.md](docs/setup/vpn.md).

To build the hub image from this tree:

```sh
docker build -f deploy/hub/Dockerfile .
```

Or build the binary only:

```sh
cargo build --release --locked --bin anima-hub
```

The program understands `anima-hub migrate`, `anima-hub operator set <username>`, and related operator commands. See `anima-hub` usage output and [deploy/README.md](deploy/README.md).

### Native VNC gateway

`anima-gateway` speaks to the hub as `guacd` does, and handles VNC itself. Use it where there is no `guacd`, for example a hub running on a Mac. RDP is still passed to `guacd`. Details: [docs/setup/native-gateway.md](docs/setup/native-gateway.md).

```sh
cargo build --release --locked --bin anima-gateway
anima-gateway listen 127.0.0.1:4822
# Point the hub at it with ANIMA_GUACD=127.0.0.1:4822
```

To keep RDP on `guacd` while VNC uses the native gateway:

```sh
anima-gateway listen 127.0.0.1:4823 --other-gateway 127.0.0.1:4822
```

### Metrics agent

`anima-agent` runs on a device, samples every two seconds, and pushes points to the hub every ten seconds. The desktop app can install it on the computer it runs on. You can also enrol it by hand:

```sh
cargo build --release --locked --bin anima-agent
anima-agent enrol <code> --hub <https origin>
anima-agent install          # user unit / LaunchAgent
# anima-agent install --system
anima-agent run
```

The enrolment code comes from the Device page in the client (`anima agent enrol` on the command line). The agent does not listen on a port.

### Link (run by hand)

`anima-link` is a separate Go program that makes an encrypted connection from a device to a hub without a VPN client. Nothing in the desktop app or the hub starts the two sides together yet. To build and run it by hand, follow [docs/setup/link.md](docs/setup/link.md).

```sh
link/build.sh    # writes link/bin/anima-link-<os>-<arch>
```

## Architecture

```text
  Desktop client (Tauri, macOS and Linux)
  Web client (served by the hub, VPN only)
  anima / anima mcp
           |
           |  HTTPS / GraphQL
           v
      anima-hub (Rust, PostgreSQL)
           |
           +-- SSH, SFTP, and wake-on-LAN (the hub itself)
           +-- VNC: anima-gateway, or guacd
           +-- RDP: guacd
           +-- Device web interfaces
           +-- Checks, notifications, /metrics
           +-- Metrics ingest from anima-agent (or SSH collection)
           |
           +-- Headscale / WireGuard mesh
           +-- anima-link (optional, started by hand)
           v
      Your machines
```

| Piece | Path | Role |
| --- | --- | --- |
| Desktop client | `apps/desktop`, `apps/client` | Tauri 2 shell and Svelte 5 UI |
| Hub | `crates/hub` | API, checks, sessions, notifications |
| Native VNC gateway | `crates/anima-gateway` | VNC sessions without `guacd` |
| Command line and MCP | `crates/anima-cli` | The `anima` and `anima mcp` programs |
| Metrics agent | `crates/agent` | `anima-agent` |
| Link | `link/` | `anima-link` (Go) |
| Deployment | `deploy/` | Compose, Headscale, backups |

The design specification is [docs/design-spec.md](docs/design-spec.md). It is the source of truth for behaviour.

## Agents

`anima mcp` is an MCP server on standard input and output. A coding agent starts it and calls its tools. It uses the same hub and token as the `anima` command line. Full steps: [docs/setup/anima-mcp.md](docs/setup/anima-mcp.md).

Make an API key for the agent, not your login. A read key is a good start:

```sh
# On your computer, signed in with `anima auth login`.
anima key create --name coding-agent --for agent --scope read --expires 90d
```

The key is shown once. Do not paste it into chat or a commit. `--for agent` makes the hub record each change made with the key as the work of an AI agent.

Register the server with Claude Code:

```sh
# On your computer. Type the key; do not copy it from chat.
claude mcp add anima --env ANIMA_TOKEN=<API_KEY> -- anima mcp
```

For another agent, run `anima mcp` with `ANIMA_TOKEN` in its environment. Add `--hub <ORIGIN>` or `--context <NAME>` when the hub is not your current context.

With nothing saved, every read tool is allowed and every change tool is off. You set the state of each tool (`allow`, `ask`, or `off`) per hub:

```sh
anima mcp permissions show
anima mcp permissions set --ask-all --kind change
anima mcp permissions set --allow anima_check_update,anima_check_set_paused
anima mcp permissions set --off vpn,retention
anima mcp permissions set --read-only
anima mcp permissions reset
```

The documented tool sets are `devices`, `checks`, `maintenance`, `events`, `sessions`, `notifications`, `web`, `releases`, `vpn`, and `retention`. The server also has `extensions`, `sources`, and `projects` tools; Connect GitHub and Projects are in development and should not be treated as finished product surface.

A change that gives the agent more access works only at a terminal or in the desktop app's Settings. That keeps an agent from widening its own access through its shell.

The MCP server does not open remote sessions, accept credentials, or take secrets as tool arguments. For an interactive shell, use `anima ssh <device>` yourself.

## Roadmap

Shipped in this tree, not yet in a public download:

- Native VNC gateway, CLI, MCP server, and Stay awake (keeps a laptop on so a hub or agent on it keeps working)
- `anima-link`, run by hand

In development:

- Connect GitHub and Projects
- Starting `anima-link` from the hub and the app

Planned, not available today:

- The first signed macOS and Linux releases
- Solo hosting ($19 / month or $199 / year, waitlist only)
- A native RDP gateway
- A Windows client
- An Anima.ing-operated Tailcat relay
- Jobs across machines, and running models on hardware you already have
- A local copy of data that third-party services hold about you
- An iOS client

Do not describe the planned items as shipping features.

The work plan is [docs/work-plan.md](docs/work-plan.md).

## Documentation

| Document | Contents |
| --- | --- |
| [docs/setup/anima-mcp.md](docs/setup/anima-mcp.md) | `anima mcp` for coding agents |
| [docs/setup/native-gateway.md](docs/setup/native-gateway.md) | `anima-gateway` for VNC |
| [docs/setup/link.md](docs/setup/link.md) | `anima-link`, by hand |
| [docs/setup/vpn.md](docs/setup/vpn.md) | Headscale / WireGuard mesh |
| [docs/setup/prometheus.md](docs/setup/prometheus.md) | Scraping `/metrics` |
| [docs/setup/web-client.md](docs/setup/web-client.md) | Browser UI on the VPN |
| [deploy/README.md](deploy/README.md) | Running the hub with Compose |
| [docs/design-spec.md](docs/design-spec.md) | Behaviour and requirement IDs |
| [docs/work-plan.md](docs/work-plan.md) | Purpose, priority, plan register |
| [quality/README.md](quality/README.md) | Local gates |
| [AGENTS.md](AGENTS.md) | Contributor and agent rules |
| [design/brand/README.md](design/brand/README.md) | Logo assets |

## Development

There is no CI. Run the gates on the machine that makes the change, before you ask for review and before every merge:

```sh
bun run gates
```

The gates run `cargo fmt`, Clippy, the Rust test suite (with a throwaway PostgreSQL 17), the Go gates for `link/`, then `bun` lint, check, test, and the quality limits. The full order and machine requirements are in [quality/README.md](quality/README.md).

```sh
cargo install cargo-nextest --locked
# ~/.cargo/bin (or $CARGO_HOME/bin) must be on PATH
```

Linux machines that run the gates also need the WebKitGTK 4.1 development headers (and libsoup-3.0, librsvg2, libdbus-1), because the gates compile the Tauri shell.

Useful workspace scripts: `bun run lint`, `bun run check`, `bun run test`.

## Contributing

Read [AGENTS.md](AGENTS.md) and the design-spec sections for the area you touch before you change code. Cite requirement IDs (for example `CHK-011`) in commits and pull requests. Match the neighbouring code.

This repository is the product. Issues and pull requests are welcome once you have run `bun run gates`.

## Licence

Anima.ing is free software under the [GNU Affero General Public License, version 3 or any later version](LICENSE) (`AGPL-3.0-or-later`). If you run a changed version as a network service, you must offer its users the source of that version.
