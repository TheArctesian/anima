# A terminal interface for Anima.ing

- From: the operator, 2026-11-18 (14:51, Pacific time)
- Source: the vault note "Terminal UI"
- Status: Framed
- Serves: No row names it (gap). The CLI and the MCP server are not a category (priorities.md Section 3). Most likely row: 4.1 Finish the device manager (See, Reach). Not confirmed.
- Owner if it becomes work: `cli-mcp-plan.md` (CMD), as a new section, or a new proposal. Not decided.
- Effort: M for a read-only dashboard; L if a remote terminal is embedded (estimates). Recommendation: should (read-only first)

## The idea

A full-screen terminal interface for people who work only in a terminal: over SSH, on a headless server, or on a small machine with no desktop. It would be a subcommand of the `anima` command line (for example `anima tui`), built with the Ratatui library on the `crossterm` terminal layer that the command line already uses. It would use the same hub API, the same sign-in and the same output rules as the command line, and show devices, checks, events and sessions live. Opening a terminal on a device is the notable use: from the screen, a key could hand the terminal to `anima ssh`, and later a pane inside the screen could show the remote terminal.

## Evidence

- The operator, 2026-11-18: "https://github.com/ratatui/ratatui could we use this to make a terminal UI for the app as well".
- Ratatui, checked 2026-11-18 on crates.io and GitHub: version 0.30.2 (published 2026-06-19), MIT licence, minimum Rust 1.88 (the repository pins 1.100.0), about 72.8 million downloads, 23,354 stars, the repository pushed on 2026-11-18. It has a `crossterm` backend (supports `crossterm` 0.28 and 0.29; this repository uses 0.29). Its own tutorial shows an async application on tokio with the `crossterm` event stream.
- Apps built with it, from the Ratatui showcase (checked 2026-11-18): gitui, bottom (system monitor), oxker (Docker), trippy (network), yazi (files), atuin, and the Codex coding agent.
- This repository, checked 2026-11-18 at commit `59053ec0`: `crates/anima-cli` already depends on `crossterm`, has a raw-mode guard (`raw_mode.rs`) that restores the terminal on every exit, a subscription client for live data (`hub/subscription.rs`, used by the `watch` commands) and `anima ssh` (a terminal on a device through the hub). No document or code mentions Ratatui or a terminal interface. The workspace has no Ratatui dependency.
- Other tools with a terminal interface, from the private competitor notes (checked 2026-11-15 to 2026-11-17): Herdr, Agent Manager, Homebutler.

## Overlap check

Files: [plan/ownership.md](../../plan/ownership.md), [plan/register.md](../../plan/register.md) and [deferred.md](../../deferred.md), with `cli-mcp-plan.md`.

- ownership.md: the `anima` command line and the MCP server belong to `cli-mcp-plan.md` (CMD, MCP). No concern covers a full-screen terminal interface.
- register.md and deferred.md: no item found. A keyword search of `docs/` for Ratatui, TUI and terminal interface found nothing.
- CMD-002: one folder for each noun. CMD-003: `hub/` is the only code that talks to the network. A terminal interface fits if it calls `hub/` and adds no network code.
- CMD-023 and CMD-027: ask only at a terminal; no colour off a terminal. The interface must start only when both input and output are terminals. A pipe or `--json` never starts it.
- CMD-030: a new function is built as hub API, `anima` command, MCP tool, graphical client. A terminal interface shows existing functions, so it adds none; where it sits in this order is open.
- Design specification Section 2 (Terms) says the command line "is not the client". A terminal interface would be a second way to use the command line or a new client; that is a ruling for the Planner.
- R-151 and INS-010 to INS-013: the command line alone ships by Homebrew and cargo; OD-25 (how a command line alone signs in to a hosted hub) is open and would also decide whether a terminal interface works against a hosted hub.
- `anima ssh` is built on Unix signals only. A terminal interface on Windows would need that checked.
- Not checked in detail: `ssh-terminal.md` (R-062) rules for the app's terminal (OSC 52 off, links confirmed) would apply to any embedded terminal pane.
- Checked on 2026-11-18 against the repository at commit `59053ec0`. This is a keyword search, not a full read.

## Narrowest version, and when to stop

- Narrowest version: `anima tui` opens a read-only screen on the signed-in hub: a device list with live status, and the events of the selected device, using the existing subscription client. Quit restores the terminal. A key on a device suspends the screen and runs `anima ssh` for it, then returns. No change tool, no new network code.
- Stop if: the live screen cannot be built on the existing `hub/` client without new network code, or if the extra dependencies make the standalone `anima` file too large to ship by Homebrew (size not measured).

## Open questions

- Should the terminal interface be a subcommand of the `anima` binary (`anima tui`), not a separate program? (yes/no)
- Is the first version a read-only dashboard that hands the terminal to `anima ssh`, with an embedded remote terminal pane left for later? (yes/no)
- When should it start? (one choice: after the publishing tasks R8a to R8d of R-151, now in parallel, or park it)

## Reviews

- 2026-11-18: round 1. The bot framed the operator's raw idea. The Serves row is not confirmed. The operator has not picked this idea.

## Source text

The operator wrote, on 2026-11-18 at 14:51 Pacific time (copied with no change):

> https://github.com/ratatui/ratatui could we use this to make a terminal UI for the app as well
