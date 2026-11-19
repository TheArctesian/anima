# Agent tools as extensions: open Herdr, T3 Code and Hermes in a pane

- From: the operator, 2026-11-18 (16:57 PT)
- Source: the vault note "Agent Tools as Extensions"
- Status: Framed
- Serves: Work plan Section 4, row 4.8 (the other extensions: AI coding sessions, Section 4.12, and Bots, Section 4.16). Closest other rows: 4.4 (community extensions, EXT-009) and 4.9 (hosted applications). Not confirmed.
- Owner if it becomes work: `extensions-plan.md` (AI coding sessions, Bots) and `bots.md`. Not decided: that is a question for the Planner.
- Effort: S for tabs only (an estimate); L for a real extension host (an estimate). Recommendation: should for tabs only; could for the host.

## The idea

Do not build a second Herdr, a second T3 Code or a second assistant bot inside Anima.ing. Run the existing open-source tools on the operator's machines and open each one in a pane of the application, as an extension. The operator named Herdr, T3 Code, and Hermes (Hermes Agent) as the assistant in place of a bot of our own. The application has to give the pane. It does not have to give the tool.

## Evidence

- Hermes Agent: MIT, Python, v0.23.2 (2026-11-16), 264,501 stars (GitHub API, 2026-11-18). `hermes dashboard` serves a web page on `127.0.0.1:9119`, with a chat tab that runs its terminal screen through a browser terminal. `hermes --tui` is a terminal screen. A desktop application exists. A remote or public bind needs the dashboard's own sign-in. Source: hermes-agent.nousresearch.com docs (Web Dashboard, TUI), read 2026-11-18.
- T3 Code: MIT, TypeScript, v0.1.10 (2026-11-17), 65,252 stars (GitHub API, 2026-11-18). `t3` starts a server and a web page (default port 3773 in `apps/server/src/config.ts`); desktop and phone applications exist. Source: github.com/pingdotgg/t3code, read 2026-11-18.
- Herdr: Apache-2.0, Rust, terminal app v0.9.5 (2026-10-14), 51,757 stars (GitHub API, 2026-11-18). A terminal app and a separate native desktop application, both on `herdr-core`, a background server with a socket interface; the vendor says the two programs "don't share agents". Source: herdr.dev, read 2026-11-18.
- Anima.ing already specifies a device web interface that opens in a session tab with an embedded web view (WEB-001 to WEB-010, design specification Section 6.3.4), and a terminal tab for SSH. This is approved text in the design specification; its build state was not checked in this review.

## Overlap check

Files: [plan/ownership.md](../../plan/ownership.md), [plan/register.md](../../plan/register.md) and [deferred.md](../../deferred.md), and the documents named below. Checked on 2026-11-18 against the repository at commit `2ba73005`. This is a keyword search and a read of the sections named, not a full read.

- Web view as a tab: WEB-001 to WEB-010 already give a device web interface in a session tab. T3 Code (port 3773) and the Hermes dashboard (port 9119) are such interfaces. Nothing new may be needed for the tab itself.
- Agent panes: `extensions-plan.md` Section 4.12.1 (AIS-009 to AIS-017) is operator direction of 2026-11-16 to build Herdr's function into Anima.ing "so they are free but we just do everything they do". Open decision 1 there is "embed Herdr's core or write our own". This idea points to a third choice, which is to host Herdr as it is. It may reverse the direction of 4.12.1.
- Bots: `bots.md` is a native bot runtime (BOT-001 to BOT-013). BOT-043 lists Hermes Agent as a Tier 2 agent, and BOT-053 names it as a fallback harness. BOT-058 to BOT-077 install an agent on a machine. A bundled Hermes as the assistant would overlap the native bot runtime.
- Community extensions: `extensions-plan.md` EXT-009 says community extensions wait until the 15 extensions are built, and the hub must not run other people's code. Section 3.4 describes a sandboxed frame for a community view. A pane that opens a tool the operator runs on their own machine runs no code in the hub, but whether it counts as an extension is a ruling.
- Terms: `bots.md` BOT-021, BOT-032 and its terms table: T3 Code drives Claude through the Agent SDK, which the proposal bans for Anima.ing itself. `bots.md` "Not in scope" and the scope review keep T3 Code and Orca out of the first version. Hermes documents that it can read the Claude Code login file (`~/.claude/.credentials.json`) in its credential pools (hermes-agent docs, `user-guide/features/credential-pools.md`, read 2026-11-18). BOT-003 and BOT-023 forbid a bot to read or keep a subscription login. A Hermes that Anima.ing sets up must use an API key, a local model or another provider, and never the Claude login.
- Ownership: the web view belongs to the design specification (WEB), the agent panes to `extensions-plan.md` 4.12.1, the install to `bots.md`. No concern in ownership.md covers "a third-party application as an extension".
- `deferred.md`: no item found (searched Herdr, T3 Code, Hermes, extension, embed).
- Not checked: whether WEB-004 is built in the client; how a web view reaches a tool on a remote device for a hosted user (no inbound port); the Hermes dashboard sign-in over the link; whether T3 Code and Hermes allow a web view inside another application.

## Narrowest version, and when to stop

- Narrowest version: no extension host. The operator installs the tool on a device (by hand, or by an install recipe of BOT-058 to BOT-077). The device gets a web interface entry (WEB-001) for T3 Code or the Hermes dashboard, and a terminal tab opens Herdr or the Hermes terminal screen on the device. One page lists the entries as "Agent tools". Check with one real device.
- Stop if: the tool's page cannot reach the operator's client without opening a port on the device, or the Hermes sign-in cannot be used safely over the link.

## Open questions

- In version 1, is a pane only a tab that opens the tool's own screen (a web view for T3 Code and the Hermes dashboard, a terminal tab for the Herdr and Hermes terminal screens), with no extension host? (one choice: tabs only / a real extension host / park it; recommended: tabs only)
- Does this replace the native Herdr-style agent panes (4.12.1) and the native bot runtime of the Bots proposal, or sit beside them? (one choice: replace the build, and keep the usage counts, the status and the install / sit beside / decide after the tab test; recommended: replace the build, and keep the usage counts, the status and the install)
- Is Hermes the first assistant that the "Add an agent" list suggests, never installed without the operator's click? (yes/no; recommended: yes)

## Reviews

- 2026-11-18: round 1. The operator's words were logged. Facts about the three tools were read on the vendors' pages. The Serves row is not confirmed. The operator has not answered the questions.

## Source text

The operator, 2026-11-18 16:57 PT (relayed by an agent, unchanged):

> also i’m thinking instead of rebuilding herdr or t3 code or grok bot (we should just use hermes instead) we integrate them all as extensions such that they can be opened in a pane in the app or something like that
