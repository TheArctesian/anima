# User documentation: high-quality docs for the application, in the style of the Rex docs

- From: the operator, 2026-11-18 (17:04 PT and 22:27 PT)
- Source: the vault note "User Documentation"
- Status: Ready for Planner
- Serves: a new row of its own, which the Planner adds (operator, 2026-11-19).
- Owner if it becomes work: Open. The marketing website is owned by design-spec.md Section 3.12 (SITE-001 to SITE-012, `ownership.md`). Docs on that site need a change to that section. The Planner decides.
- Effort: L for the full set, which the operator chose for the first version (an estimate). Recommendation: should.

## The idea

Write real user documentation for Anima.ing: a docs site with one page for each task and each concept, a generated reference for the command line, and a text copy of every page that a coding agent can read. The operator named the Rex docs (superlogical.com/rex/docs) as the standard: "just high quality documentation in line with how Rex does it".

## Evidence

- Operator, 2026-11-18, 17:04 PT: "also we should starting writing documentation for the application like rex’s docs they are forgo" and "gorgeous". 22:27 PT: "just high quality documentation in line with how Rex does it" (relayed through chat; the first message has a typing error that is kept).
- Rex docs, read 2026-11-18 (22:30 PT). What makes them good:
  - Pages are sorted by task: Installation, Customize, Automate, Build, Server (one set of pages for each operating system), References, Beta. About 35 pages; a fixed side menu and a search box (Cmd+K).
  - Every page is also plain Markdown (add `.md` to the address, or ask for `text/markdown`). One file, `llms-full.txt`, holds all pages. `search.json?q=` gives the best ten sections as JSON. The docs also come as an MCP server.
  - The reference pages (every CLI command and flag, the server API, actions, settings, events) look generated from the program. Entries that are not released yet carry a tag (Snapshot, Bleeding Edge), so there is no second docs version.
  - A page starts with one plain sentence. A quickstart is numbered steps with commands to copy and a check at the end. Tables compare options (for example the six ways to reach a server, with "Needs", "Works with", "Identifies callers").
  - The tone is plain and honest: it names limits ("This is a short-term limitation"), and it names the default path ("we recommend SSH").
  - A privacy page lists every outbound network connection and what each one sends.
  - A page documents the agent skill and what an agent can do with the product, in a "You ask / The agent" table.
  - The site is SvelteKit on Vercel (response headers), the same stack as the Anima.ing website. This is detected, not confirmed by the makers.
- Ghostty docs (ghostty.org/docs, same maker), read 2026-11-18: a Next.js site with About, Install, Configuration, Linux, Features, Terminal API, Help and Release Notes, and a search box. The option reference is one long page that says "use your browser's search". No `llms.txt` was found there. Ghostty is the better model for a human reference; Rex is the better model for agents.
- State of Anima.ing, checked 2026-11-18 at commit `9f056a5a`: see "Overlap check".

## Overlap check

Files: [plan/ownership.md](../../plan/ownership.md), [plan/register.md](../../plan/register.md) and [deferred.md](../../deferred.md). Keyword search, not a full read.

- User docs today: `README.md` (features, quick start by building from source, a Documentation table), `docs/procedures/setup/` (nine guides, from 55 to 1,648 lines), `docs/procedures/releases.md`, `restore-and-move.md`, `deploy/README.md` (1,005 lines). These are procedures written for the operator and for agents. They cite requirement IDs and plan items (for example `vpn.md` has 68 IDs), and they use ASD-STE100 English. `first-run.md` is marked Draft. There is no docs site and no page for the desktop client, remote access, uptime checks, metrics or the CLI.
- The website (`anima-site`) has no docs route. It has `llms.txt` and `llms-full.txt` (4.6 KB). The `llms.txt` link to the MCP guide points to `docs/setup/anima-mcp.md`, which does not exist; the file is `docs/procedures/setup/anima-mcp.md`. This is a defect to tell the owner of the website.
- `ownership.md`: the website is owned by design-spec.md Section 3.12 (SITE-001 to SITE-012). SITE-005 asks for install steps on the site. SITE-007 asks for static files only and SITE-008 for no third-party script (a docs search must be built into the site). SITE-012 forbids text about a paid hosted option.
- register.md and priorities.md: no plan, proposal or row for user documentation or a docs site.
- deferred.md: no item on user docs (searched "documentation", "docs site", "user guide", "llms").
- Related idea: Agent-Readable Surface (a skill file, `llms.txt`, an install prompt). It covers the agent copy of the docs. This idea would be the human docs that the agent copy is made from. The two should become one plan, with one owner.
- Related, already ruled: R-151 (CLI on Homebrew and cargo) gives the install commands the docs would show.
- Not checked: the idea folder `browser-first-run`; whether the CLI help text is good enough to generate a reference; how much of the product a first user can use today (there is no public download yet).

## Narrowest version, and when to stop

- Narrowest version: a docs section on the website with four pages: Install, Add your first device, Remote access (SSH, desktop, web interface), and CLI reference generated from the `anima` command's own help. Source Markdown is in this repository under `docs/user/`. Every page is also served as `.md`, with an `llms-full.txt` built from the pages. Measure: a person who has never seen Anima.ing installs it and reaches a second machine, with only the docs.
- Stop if: the first public install is not near, so the docs would describe features that a user cannot get; or the generated CLI reference needs more work than writing it by hand.

## Open questions

1. Where do the docs live? (one choice: the website at /docs, a separate site at docs.anima.ing, or in this repository only. Recommended: the website at /docs: one domain and the same stack as Rex.)
2. Is the source Markdown written in this repository (`docs/user/`) and published by the website? (yes/no. Recommended: yes, so a docs change and a code change go in one commit.)
3. How big is the first version? (one choice: install, first device, remote access and CLI reference; or the full set. Recommended: the first four.)

## Answers (operator, 2026-11-19)

1. Where the docs live: a separate site at docs.anima.ing.
2. Source: yes. Markdown in this repository (`docs/user/`), published by the docs site.
3. First version: the full set.
4. Serves: a new row of its own, which the Planner adds.

Points for the Planner: docs.anima.ing is a new site outside design-spec Section 3.12 (SITE-001 to SITE-012), so it needs an owner; decide whether SITE-007, SITE-008 and SITE-012 also apply to it; rule whether STE applies to user docs; and decide whether the Agent-Readable Surface idea folds in.

## Reviews

- 2026-11-18: round 1. The bot framed the idea from the operator's two messages and a read of the Rex and Ghostty docs. The Serves row is not confirmed.
- 2026-11-19: round 2. The operator answered all questions and asked for a new row. Ready for Planner.

## Source text

> also we should starting writing documentation for the application like rex’s docs they are forgo

> gorgeous

> just high quality documentation in line with how Rex does it
