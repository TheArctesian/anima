---
kind: Proposal, with a task table
state: Direction decided
evidence: Version 3 (2026-11-19). R-208 (operator, first-hand, 14:17 and 14:18 PT) adds UDOC-050: the docs say that the free hosted hub is good for about 5 machines, a guide and not a limit. Version 2: R-207 (operator, first-hand, Grok Bot chat, 2026-11-19, 14:03 to 14:11 PT) decides OD-UD1 to OD-UD13, so the direction is decided (PLN-007). The requirements UDOC-001 to UDOC-050 are not approved: that is task UD20. No task is named by a ruling, so no task may start. Version 1 was made from the idea `user-documentation` (`3e5910d0`) and pushed to `dev` (`0400d45a`). Code state checked on `origin/dev` at `3353f648` (version 1) and `0400d45a` (version 2); `anima-site` checked at `3d71012`. Serves row 4.22 of the priorities (tier 1, after row 4.3; R-207, OD-UD4).
---

# Proposal: user documentation and the docs site docs.anima.ing

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this proposal (PLN-005).

| Item | Value |
| --- | --- |
| Document | Proposal: the user documentation of Anima.ing. The pages for each task and each concept, the generated references, a copy that an AI agent can read, and the site docs.anima.ing that publishes them |
| Version | 3 |
| Date | 2026-11-19 |
| Language | ASD-STE100 Simplified Technical English (STE). This rule applies to this proposal. It does not apply to the user pages in `docs/user/`, which use the plain style guide (R-207, OD-UD9) |
| Status | The header of this document holds the state (PLN-020). History: Version 1 (Draft) was made from the idea `user-documentation` (PLN-018); that commit deleted the idea folder. Version 2 records R-207: the operator decided OD-UD1 to OD-UD13 first-hand. Two answers are against the recommendation: the docs site may use Vercel Web Analytics (OD-UD7), and the docs may describe the paid hosted option (OD-UD8). The idea `agent-readable-surface` is folded in (OD-UD10); this commit deletes its folder. The state is Direction decided. The requirements are not approved (task UD20). Version 3 records R-208: UDOC-050, the text of the docs about the free hosted hub. |
| Serves | Row 4.22 of [priorities.md](../plan/priorities.md): tier 1, after row 4.3 (R-207, OD-UD4) |
| Related | [design-spec.md](../spec/design-spec.md) Section 3.12 (SITE-001 to SITE-012), Section 3.11 (INS); [cli-mcp-plan.md](../plans/cli-mcp-plan.md) (CMD, MCP); [agent-tools-as-extensions.md](agent-tools-as-extensions.md) (AXT-036, `llms.txt` of the catalogue pages); [hosted-mode.md](hosted-mode.md); the idea `agent-readable-surface`, folded in and deleted (R-207, OD-UD10) |
| Identifiers | UDOC-001 to UDOC-050, OD-UD1 to OD-UD13 (decided, R-207), tasks UD0 to UD20 |

## 1. Goal

Give Anima.ing real user documentation. A person who has never seen Anima.ing must be able to install it, add a second machine and reach it, with the docs only. A coding agent must be able to do the same from the text copy of the docs.

The model is the Rex docs (superlogical.com/rex/docs, read 2026-11-19): pages sorted by task, a generated reference of each command and flag, a Markdown copy of each page, one file `llms-full.txt` with all pages, a docs MCP server, and a privacy page that lists each outbound connection. The words of the operator (2026-11-18, relayed by Anima Product Ideas in the idea, unchanged):

> just high quality documentation in line with how Rex does it

## 2. The direction of the operator (confirmed first-hand, R-207)

Anima Product Ideas first relayed four answers (2026-11-19, 12:23 PT). The operator then decided all thirteen open decisions first-hand (2026-11-19, 14:03 to 14:11 PT). R-207 records his words. In short:

1. The docs live on a separate site at docs.anima.ing (OD-UD1).
2. The source is Markdown in this repository, `docs/user/`, and the docs site publishes it (OD-UD2).
3. The first version is the full set, in waves; the site is public after wave 2 (OD-UD3).
4. The work has row 4.22 of the priorities, tier 1, after row 4.3 (OD-UD4).
5. This proposal owns the user docs and docs.anima.ing; design specification Section 3.12 stays the marketing website only (OD-UD5).
6. SITE-007 applies: the docs site is static files only (OD-UD6).
7. The docs site can use Vercel Web Analytics under the same exception as SITE-008 (OD-UD7; against the recommendation).
8. SITE-012 does not apply: the docs can describe the paid hosted option now (OD-UD8; against the recommendation).
9. STE does not apply to `docs/user/`; the pages use the plain style guide (OD-UD9).
10. The idea Agent-Readable Surface is folded in, and its folder is deleted (OD-UD10).
11. The site code is in `apps/docs/` of this repository (R-207, OD-UD11).
12. The docs MCP is local only, inside `anima mcp` (OD-UD12).
13. The install steps of SITE-005 move to the docs; `www.anima.ing` links to the install page of the docs (OD-UD13).

## 3. Overlap check (PLN-004)

Checked on 2026-11-19 against `origin/dev` at `3353f648` and `origin/main` at `3e5910d0`. Files: [ownership.md](../plan/ownership.md), [register.md](../plan/register.md), [deferred.md](../deferred.md), [ideas/](../ideas/README.md), `docs/procedures/`, the README, and `anima-site`.

| Item | What it owns | Overlap | Result |
| --- | --- | --- | --- |
| Design specification Section 3.12, SITE-001 to SITE-012 | The marketing website `www.anima.ing` with the install links | SITE-005 asks for install steps on the website. SITE-007, SITE-008 and SITE-012 are rules that can also apply to a docs site | No plan owns a docs site. docs.anima.ing is a second site, not the marketing website. This proposal owns it (OD-UD5). SITE-005 points to the install page of the docs (OD-UD13) |
| [cli-mcp-plan.md](../plans/cli-mcp-plan.md), CMD and MCP | The `anima` command and `anima mcp` | The generated CLI reference reads the command. The docs tools of Section 6.6 are new MCP tools | The reference reads the command and changes no CMD requirement. The command `anima docs` and the tool set "docs" are defined here; Section 7 adds a pointer row to cli-mcp-plan.md Section 7 at approval |
| [agent-tools-as-extensions.md](agent-tools-as-extensions.md), AXT-036 | Catalogue pages on the public site, and their lines in `llms.txt` | Both write `llms.txt` | The `llms.txt` of `www.anima.ing` stays with the site; it gets one line that points to the docs (UDOC-030). AXT-036 does not change |
| The idea `agent-readable-surface` (Framed; from Anima Product Ideas, competitor research of 2026-11-17) | A skill file, `llms.txt`, an install prompt for agents, an `INSTALL.md` that an agent can follow | `llms.txt` and the agent copy are UDOC-026 to UDOC-032 and UDOC-049 here | Folded in (R-207, OD-UD10). This commit deletes the idea folder. Its evidence (public research, 2026-11-17, dates not verified): RustDesk publishes an `llms.txt`; NetBird offers prompts for ChatGPT and Claude; Herdr and Rex ship a skill file; Muster installs when an agent follows its `INSTALL.md`. Its two open questions are answered in UDOC-031 (the skill file is in this repository) and UDOC-032 (the prompt names only released channels). Its measure is UDOC-048 |
| `docs/procedures/` (setup guides, `releases.md`, `restore-and-move.md`), `deploy/README.md` | Procedures for the operator and for agents. They cite requirement identifiers and use STE | Their facts are the source of many user pages | They stay. A user page takes the steps and leaves out the identifiers (UDOC-019). A procedure does not link to a user page as its owner |
| The idea `remote-mcp-over-http`, now [agent-tools-as-extensions.md](agent-tools-as-extensions.md) AXT-071 (R-280: a hosted hub only) | A remote MCP server with the Anima.ing sign-in (direction decided, not approved) | A remote server could also offer the docs tools | No change now. The two docs tools of UDOC-036 can be in it with no new design |
| register.md, priorities.md | — | No plan, no proposal and no row for user documentation | A new row 4.22 (OD-UD4) |
| deferred.md | — | No item ("documentation", "docs site", "user guide", "llms"; 2026-11-19) | None |
| Hosted mode ([hosted-mode.md](hosted-mode.md)) | The hosted first run, sign-in in `anima-site` | The docs explain the hosted first run and the paid hosted option | SITE-012 does not apply to the docs (R-207, OD-UD8). The pages describe what the hosted option does, with no private fact (UDOC-020). OD-22 for `www.anima.ing` stays a separate open item |

## 4. Current state (verified 2026-11-19)

| Area | State | Evidence |
| --- | --- | --- |
| User pages | None. There is no `docs/user/` folder and no docs site | `ls docs/user` fails on `3353f648` |
| Docs for the operator and agents | Nine setup guides in `docs/procedures/setup/` (55 to 1,648 lines), `releases.md` (369), `restore-and-move.md` (134), `linux-client-setup.md` (69), `deploy/README.md` (1,005), `README.md` (342) with a Documentation table. They are procedures in STE with requirement identifiers. `first-run.md` is a Draft | `wc -l` on `3353f648` |
| CLI help | `crates/anima-cli` uses clap with derive. The text of each command is its `///` comment (3,753 comment lines in the crate; 55 files with a `clap::Subcommand`). `anima completion <shell>` prints completions with `clap_complete` from `Cli::command()` (`src/cli/program.rs`). There is no Markdown or man page output | `crates/anima-cli/src/cli/program.rs` at `3353f648` |
| Quality of the help text | Not verified for the whole command. The crate did not build on the box: the installed `rustc` is older than 1.88, which `rmcp` 3.10 needs. A sample of the source (`src/devices.rs`, `src/devices/list.rs`) has one plain sentence for each command and each flag, which is enough for a generated reference | `cargo build -p anima-cli`, 2026-11-19; read of the source |
| A docs MCP | None. `anima mcp` has no docs tool. The cleaning of text for agents is in `src/mcp/untrusted.rs` (MCP-008; a result is capped at 60,000 bytes) | `crates/anima-cli/src/mcp/` at `3353f648` |
| `anima-site` | SvelteKit on Vercel (`@sveltejs/adapter-vercel`). It has `static/llms.txt` and `static/llms-full.txt`, no docs route. It also has server routes (`api/waitlist.ts`, `src/routes/auth`, `account`, `billing`, `sign-in`, `v1/hub`), so it is no longer "static files only" as SITE-007 says | `anima-site` at `3d71012` |
| The defect in `llms.txt` | Confirmed. `static/llms.txt` line 12 and `static/llms-full.txt` line 46 link `https://github.com/TheArctesian/anima/blob/main/docs/setup/anima-mcp.md`. The live address gives HTTP 404. The real file is `docs/procedures/setup/anima-mcp.md`; its address gives HTTP 200 after the redirect of GitHub to `TheArctesian/anima.ing`. The live `https://www.anima.ing/llms.txt` has the same wrong link | `anima-site` at `3d71012`; `curl`, 2026-11-19 12:27 PT |
| docs.anima.ing | Gives no answer over HTTPS. If a DNS record exists is not verified | `curl`, 2026-11-19 |
| Design specification Section 3.12 | It says that the website "is built and is not deployed" and that the domain is open (OD-20). `www.anima.ing` answers. The text is old | `curl https://www.anima.ing/llms.txt` gives 200, 2026-11-19 |

## 5. Constraints

- **D1. One source.** Each fact has one owner (PLN-016). A user page explains; it does not define a requirement (PLN-010). A user page that disagrees with the code is a defect of the page.
- **D2. Only what a user can get.** A page describes only what is released on `main` (R-198). A function that is not released is not on the site. A function that is released but not public (for example no public download yet) says so on its page.
- **D3. The public repository.** `docs/user/` is public. No price, cost, funding or trademark fact of the business vault enters it. The pages can describe the paid hosted option (R-207, OD-UD8), but only with facts that are already public (UDOC-020).
- **D4. One outbound connection.** The docs site has one connection of its own, Vercel Web Analytics, under the exception of SITE-008 (R-207, OD-UD7). It adds no other connection to a third party. `anima docs` makes no network connection at all.

## 6. Requirements (proposed, not approved)

Each requirement is proposed. None is approved, and nothing is built.

### 6.1 The site and publishing

- **UDOC-001.** The user documentation must be published at `https://docs.anima.ing` (R-207, OD-UD1). The site is separate from the marketing website `www.anima.ing`.
- **UDOC-002.** The source of each page must be one Markdown file in `docs/user/` of this repository (R-207, OD-UD2). A change to a function and the change to its page can be in one commit.
- **UDOC-003.** The site code must be in `apps/docs/` of this repository (OD-UD11). It must build the pages from `docs/user/` and from the generated references (Section 6.4). It must not fetch a page from a different repository at build time.
- **UDOC-004.** The site must be static files only (SITE-007 applies; R-207, OD-UD6): no account, no sign-in, no form, no cookie and no server code. Search runs in the browser from a static index (UDOC-029). The intake of Vercel Web Analytics (UDOC-005) is a function of the host, not server code of the site.
- **UDOC-005.** The site must not load a script, a font, an image or a style from a third party, and must have no tracker. The one exception is Vercel Web Analytics, with the same limits as SITE-008 (R-207, OD-UD7): its script is loaded from the origin of the site, with no inline script, and it sets no cookie. The Content-Security-Policy must keep `default-src 'self'`, `script-src 'self'`, `connect-src 'self'` and `frame-ancestors 'none'`. [NEEDS CLARIFICATION: Vercel says that Web Analytics uses no third-party cookies and identifies a visitor by a hash of the request, which it discards after 24 hours (vercel.com/docs/analytics/privacy-policy, read 2026-11-19). It does not say that the script writes no first-party cookie and no local storage. Task UD4 checks the built site in a browser. If the script writes a cookie, SITE-007 and this exception conflict, and the operator decides.]
- **UDOC-006.** The site must take its colours, spaces, font sizes and motion from the design tokens (`design/tokens/`), and obey SITE-009 (contrast, focus ring, `rem` sizes, reduced motion, `prefers-contrast`). It reads the tokens of this repository; it keeps no copy.
- **UDOC-007.** The site must publish what is on `main`. A push to `main` that changes `docs/user/` or `apps/docs/` must publish the site. [NEEDS CLARIFICATION: is `apps/docs` an application of the release map of [releases.md](../procedures/releases.md) (R-198, R-200), or a Vercel project that builds each push to `main` by itself? Recommendation: a Vercel project of its own that builds `main` only; the operator connects it (VPS-005, DNS-008).]
- **UDOC-008.** Each page must show the date of its last change and a link "Edit this page" to its source file on GitHub.

### 6.2 The pages

- **UDOC-009.** The side menu must sort the pages by task, in this order: Get started, Reach your machines, Watch your machines, Use AI agents, Run your own hub, Reference, Help. A concept page is in the group of the first task that needs it.
- **UDOC-010.** The first version must have the full set (R-207, OD-UD3). The site is public after wave 2. The set is in Section 6.2.1. It is published in the waves of Section 10.
- **UDOC-011.** Each page must start with one plain sentence that says what the page is for.
- **UDOC-012.** A how-to page must give numbered steps. Each command is in a code block that a person can copy. The last step is a check that tells the reader that it worked.
- **UDOC-013.** Where there are two or more ways to do one thing, the page must compare them in a table, and must name the way that Anima.ing recommends.
- **UDOC-014.** A page must name each limit that a user can meet, in plain words (for example "There is no public download yet"). It must not hide a limit.
- **UDOC-015.** The site must have a search box that opens with Cmd+K or Ctrl+K and works with the keyboard only.
- **UDOC-016.** A page about a function that is released but not public must carry the tag "Not public yet", with one sentence that says how to get it.

#### 6.2.1 The full set (the first version)

The list is a proposal. The source of each page is a guide that exists, or the code.

| Group | Pages | Main sources |
| --- | --- | --- |
| Get started | What Anima.ing is; Install (macOS, Linux; the command line by Homebrew and Cargo, R-151; the page that `www.anima.ing` links to, UDOC-049 and OD-UD13); The first run (hosted hub or your own hub); The hosted option (UDOC-020); Add your first machine; Add a phone | `README.md`, [first-run.md](../procedures/setup/first-run.md), [link.md](../procedures/setup/link.md), INS-001 to INS-013 |
| Reach your machines | Remote access, with the table of the ways (SSH, desktop by VNC and RDP, web interfaces); SSH; Desktop; Web interfaces; The VPN; Stay awake | [vpn.md](../procedures/setup/vpn.md), [web-client.md](../procedures/setup/web-client.md), [native-gateway.md](../procedures/setup/native-gateway.md), AWK-001 to AWK-008 |
| Watch your machines | Uptime checks; Metrics; Notifications; Prometheus | [prometheus.md](../procedures/setup/prometheus.md), design specification Section 6 |
| Use AI agents | The command line; `anima mcp` for coding agents; What an agent can do ("You ask / The agent" table); MCP permissions; The agent skill; Install with an agent (UDOC-031, UDOC-032) | [anima-mcp.md](../procedures/setup/anima-mcp.md), [cli-mcp-plan.md](../plans/cli-mcp-plan.md) |
| Run your own hub | Self-host the hub with Compose; The local hub; The bundled database; Back up, restore and move; Update | [deploy/README.md](../../deploy/README.md), [local-hub.md](../procedures/setup/local-hub.md), [bundled-database.md](../procedures/setup/bundled-database.md), [restore-and-move.md](../procedures/restore-and-move.md) |
| Reference | CLI commands (generated); MCP tools (generated); Settings; Privacy and outbound connections | `crates/anima-cli`, `src/mcp/catalogue.rs`, the code (UD0) |
| Help | Troubleshooting; Uninstall; Release notes (links to the release record) | [releases.md](../procedures/releases.md) |

### 6.3 Writing

- **UDOC-017.** The pages must follow one style guide, `docs/user/STYLE.md`. ASD-STE100 does not apply to `docs/user/` (R-207, OD-UD9). The guide keeps the STE rules 1, 4 and 6 of the design specification Section 0 (one instruction in one sentence, the active voice, one term for one item, with the terms of Section 2). It permits "you" and longer descriptive sentences, as the Rex docs do.
- **UDOC-018.** The tone must be plain and honest. It names the default path ("we recommend SSH") and each limit (UDOC-014).
- **UDOC-019.** A user page must not cite a requirement identifier, a ruling, a plan or a task. These belong to the planning documents.
- **UDOC-020.** SITE-012 does not apply to the docs (R-207, OD-UD8). A page can describe the paid hosted option: what it does, what it includes and how to start it. A page must not copy a fact of the business vault (a price, a cost, a funding fact or a trademark finding). For a price, a page links to the pricing page of `www.anima.ing`. [NEEDS CLARIFICATION: `www.anima.ing/pricing` gives HTTP 404 on 2026-11-19, so no public pricing page exists yet. If a page must show a price, the operator gives the public words first.] The text of `www.anima.ing` itself stays under SITE-012 and OD-22, which is a separate open item.

### 6.4 The generated references

- **UDOC-021.** The CLI reference must be generated from `Cli::command()` of `crates/anima-cli`: each command, its arguments and its flags, with the text of its `///` comment. Nobody edits it by hand.
- **UDOC-022.** The MCP tools reference must be generated from the tool catalogue of `anima mcp` (`src/mcp/catalogue.rs`): each tool, its tool set, its kind (read, change, destructive), its default state (MCP-024) and its description.
- **UDOC-023.** The generated files must be in `docs/user/reference/` and listed in `quality/generated.txt`. A test must fail when a generated file differs from what the command gives.
- **UDOC-024.** A command or a tool that is hidden from `--help` must not be in the reference.
- **UDOC-025.** When the generator finds a command with no help text, the test must fail and name the command. [NEEDS CLARIFICATION: the number of commands with no text is not known (UD0 counts them). If it is large, UD2 can start with a list of known gaps that can only get shorter, as `quality/baseline.json` does.]

### 6.5 The copy for AI agents

- **UDOC-026.** Each page must also be served as Markdown at its address with `.md` added. The `.md` file is the source page with the same text, built at build time.
- **UDOC-027.** The site must serve `/llms.txt` (the list of the pages, one line each, in the format of llmstxt.org) and `/llms-full.txt` (all pages in one file, in the order of the side menu).
- **UDOC-028.** [NEEDS CLARIFICATION: a request with `Accept: text/markdown` to a page address, as Rex does. A static site cannot choose a file by a header without a rewrite rule of the host. Recommendation: not in the first version; the `.md` address is enough.]
- **UDOC-029.** The site must serve one static search index (`/search-index.json`), with one entry for each section: the address, the page, the heading and the text. The search box of UDOC-015 and `anima docs search` use the same format. A search address that answers a query (`search.json?q=`, as Rex has) needs server code and is not in scope (UDOC-004).
- **UDOC-030.** The `llms.txt` and `llms-full.txt` of `www.anima.ing` must point to `https://docs.anima.ing/llms.txt`. Owner of that change: `anima-site` and Section 3.12 (task UD16; R-207, OD-UD13). AXT-036 does not change.
- **UDOC-031.** (R-207, OD-UD10.) This repository must hold one agent skill file for Anima.ing, `docs/user/agents/SKILL.md`. It tells an agent how to install the `anima` command, sign in, run `anima mcp`, and check the result. The page "The agent skill" publishes it.
- **UDOC-032.** (R-207, OD-UD10.) The page "Install with an agent" must give one prompt that a person can paste into a coding agent. It names only install channels that are released (D2). Thus it does not name Homebrew or Cargo (R-151) before they are released. This answers the first question of the idea `agent-readable-surface` by D2; the operator did not answer it one by one.

### 6.6 The docs in the command line and MCP (CMD-030 order)

The docs are in the `anima` program, so an agent gets the docs of the version that it runs, with no network. CMD-030 puts the hub API first. The docs tools read no hub data, so there is no hub API and no GraphQL root field, and the coverage test CMD-031 does not change. The order is: the command, then the MCP tools, then the client.

- **UDOC-033.** The release build of `anima` must hold the pages of `docs/user/` and the search index, compressed. [NEEDS CLARIFICATION: the size that this adds to the program is not measured (UD0). The text of all pages is an estimate of 0.3 to 0.8 MB before compression, from the size of `llms-full.txt` of Rex, 608 KB on 2026-11-19.]
- **UDOC-034.** `anima docs search <words>` must print the ten best sections, best first, with the page address and the heading. `anima docs show <page>` must print the Markdown of one page. `--json` gives the same data as JSON (CMD rules of Section 5 of cli-mcp-plan.md apply).
- **UDOC-035.** `anima docs` must make no network connection and must not need a hub, a context or a sign-in.
- **UDOC-036.** `anima mcp` must have a tool set "docs" with two read tools: `anima_docs_search` and `anima_docs_read` (MCP-011). They are listed with no hub token, because they read no hub data. Their default state is `allow` (MCP-024).
- **UDOC-037.** The docs tools must obey MCP-004 to MCP-010. Their text goes through the cleaning of `untrusted.rs` (MCP-008), and `anima_docs_read` returns one page or one section, under `RESULT_CAP`.
- **UDOC-038.** There must be no remote docs MCP server in the first version (R-207, OD-UD12). The Rex server needs an account and server code; UDOC-004 forbids both.
- **UDOC-039.** The client must have a "Help" link in Settings and in the menu of the desktop application. It opens `https://docs.anima.ing` in the system browser, not in a web view.

### 6.7 The privacy page

- **UDOC-040.** The Reference group must have the page "Privacy and outbound connections". It must start with one sentence that says if Anima.ing collects telemetry.
- **UDOC-041.** The page must list each outbound network connection that each Anima.ing program makes: the desktop application, the mobile applications, `anima`, `anima-daemon`, `anima-link`, the hub and the gateway. For each one it gives: when, where to, what it sends, and if the user can turn it off.
- **UDOC-042.** Each row must name the code that makes the connection, in a source note of the page source (a Markdown comment), so that a reviewer can check it. The published page does not show the code paths.
- **UDOC-043.** A change that adds or changes an outbound connection must change this page in the same commit. `AGENTS.md` gets this rule (Section 7). [NEEDS CLARIFICATION: a gate that finds a new HTTP client in the code is not designed. Recommendation: a review rule first, a gate later.]
- **UDOC-044.** The page must also list the connections of the two websites: `www.anima.ing` (Vercel Web Analytics, OD-21) and `docs.anima.ing` (Vercel Web Analytics, R-207, OD-UD7). For each, it says what Vercel stores (the address of the page, the referrer, the country, the operating system, the browser and the device type) and that no cookie is set (UDOC-005).

### 6.8 Checks

- **UDOC-045.** `bun run quality:documents` must check the relative links of `docs/user/`, as it does for the other documents. [NEEDS CLARIFICATION: if the gate already reads `docs/user/` with no change is not verified; UD1 checks it.]
- **UDOC-046.** The build of `apps/docs` must fail on a broken internal link, a page that is not in the side menu, and a page with no first sentence.
- **UDOC-047.** The first version needs one human check (PLN-014): a person who has never used Anima.ing installs it and reaches a second machine with the docs only, on macOS and on Linux. The entry `HT-nnn` is added by the commit of task UD18.
- **UDOC-048.** A fresh coding agent session must be able to install `anima`, run `anima mcp` and list the machines, from `llms-full.txt` or the skill file only (the measure of the idea `agent-readable-surface`). This is part of the check of UDOC-047.

- **UDOC-049.** (R-207, OD-UD10, OD-UD13.) This repository must have an `INSTALL.md` at its root that a coding agent can follow ("follow INSTALL.md"). It holds only a short prompt and a link to the install page of the docs, so the steps have one owner (PLN-016). The install page of the docs is the page that `www.anima.ing` links to for the install steps (SITE-005 changes, task UD17).
- **UDOC-050.** (R-208.) The page "The hosted option" and the page "The first run" must describe the free hosted hub. They must say that it is good for about 5 machines. This number is a guide, not a limit: the hub does not stop a sixth machine, and the page must not call it a limit. The pages must also say, with the hosted-mode rules as the source of each fact:
  - the free hub sleeps when no device of the account is connected for 15 minutes, so checks, notifications and history need a hub that is always on (HMD-024, HMD-028);
  - an account has one free hub (HMD-020);
  - a free hub with no sign-in for 60 days is paused, and the person gets an email before a paused hub is deleted (HMD-021).

  The number 5 comes from the operator (R-208). It is not measured: no load test of a free hub exists, the size of the slice of a free hub is not defined (HM-D1, HM-D8 of [hosted-mode.md](hosted-mode.md)), and the estimates in [shared-pool-hosting.md](shared-pool-hosting.md) Section 5 are not measured. [NEEDS CLARIFICATION: check the number again after the load test of the shared pool, and change the page if the operator changes the number.] If the numbers of HMD-021 or HMD-024 change (HM-D8), the pages change with them; UDOC-050 does not change a hosted-mode rule.

## 7. Changes to other documents (at approval, not now)

| Document | Change | When |
| --- | --- | --- |
| [ownership.md](../plan/ownership.md) | The row "User documentation and docs.anima.ing" with UDOC (R-207, OD-UD5) | Made (version 2) |
| [priorities.md](../plan/priorities.md) | Row 4.22, tier 1, after row 4.3 (R-207, OD-UD4) | Made (version 2) |
| The idea `agent-readable-surface` | The folder is deleted; its content is in UDOC-031, UDOC-032, UDOC-048 and UDOC-049 (R-207, OD-UD10) | Made (version 2) |
| Design specification Section 3.12 | SITE-005: the install steps are on docs.anima.ing; the website links to the install page of the docs (R-207, OD-UD13). A note: Section 3.12 owns the marketing website only; docs.anima.ing is owned by this proposal (OD-UD5). The old text "built and not deployed" and the domain of OD-20 are corrected. SITE-007 and SITE-011 do not agree with `anima-site` at `3d71012` (server routes); that is a separate gap for the owner of the website, not for this proposal | Task UD17, after the approval (UD20). R-207 decides the change |
| [cli-mcp-plan.md](../plans/cli-mcp-plan.md) Section 6 and Section 7 | A pointer row for `anima docs` and the tool set "docs" (UDOC-034 to UDOC-037); no CMD or MCP identifier changes | At approval |
| `AGENTS.md` | Two lines: a change to a user-visible function changes its page in `docs/user/`; a change to an outbound connection changes the privacy page (UDOC-043) | At approval; a ruling, because it is a rule for each session |
| [work-plan.md](../work-plan.md) | A row for `docs/user/` in the table of folders: "User pages. Not a plan" | Task UD1 |
| `README.md` | The Documentation table points to docs.anima.ing | Task UD17 |
| `anima-site` (not this repository) | Correct the wrong link in `static/llms.txt` line 12 and `static/llms-full.txt` line 46; then point both files to the docs (UDOC-030), and replace the install steps on the website with a link to the install page of the docs (R-207, OD-UD13) | Task UD16; the defect can be corrected now, with no ruling (PLN-013, a correction of a defect). The link to the install page waits for the publish of wave 2 (UD18) |

## 8. Reviews

### 8.1 Scope review

- **Narrowest useful version.** Waves 0 to 2 of Section 10: the style guide, the two generators, the site, and the pages Install, The first run, Add your first machine, Remote access, and the CLI reference. That is the narrowest version of the idea. The full set (OD-UD3) adds waves 3 to 5.
- **What is in.** The pages, the site, the references, the agent copy, the privacy page, `anima docs` and its MCP tools, and the help link.
- **What is out.** A remote docs MCP server (UDOC-038); a search address that answers a query; content negotiation by `Accept` (UDOC-028); a second language; versioned docs (D2: the site shows `main`); a price on a user page before it is public (UDOC-020); a GraphQL API reference for users (the API is for the client; a later decision); comments or feedback on a page (UDOC-004).
- **Stop condition (from the idea).** Stop if the first public install is not near, or if the generated CLI reference needs more work than to write it by hand. UD0 measures the second.
- **Risk.** Effort L before the first public download. The waves let the operator stop after wave 2 with a useful site.

### 8.2 Design scores (0 to 10)

| Dimension | Score | What makes it a 10 |
| --- | --- | --- |
| Finding a page (task order, search) | 8 | A test with a new user who finds "reach my Mac from my phone" in under one minute (UDOC-047) |
| A first install with the docs only | 7 | The human check UDOC-047 passes on macOS and Linux |
| Agent readability (`.md`, `llms-full.txt`, search index, skill, MCP) | 9 | The agent check UDOC-048 passes |
| Accuracy over time (generated references, gates) | 7 | A gate that also finds a page whose command no longer exists |
| Privacy and trust (static, one analytics connection, connection list) | 8 | No analytics (the operator chose analytics, R-207, OD-UD7), and a gate for the connection list (UDOC-043) |
| Accessibility (SITE-009, keyboard search, contrast) | 8 | An audit with a screen reader on the four mockup pages |
| Same look as the product (tokens, IBM Plex) | 8 | The mockups compared with the built site (PLN-019) |
| Honesty (limits, "Not public yet") | 9 | Each limit in the troubleshooting page has a link to its page |

The mockups are in [mockup/user-documentation/index.html](../mockup/user-documentation/index.html): the docs home, a task page, the CLI reference page and the privacy page.

#### Wireframes

The docs home:

```text
+-------------------+--------------------------------------------------------+
| [A] Anima.ing Docs| Anima.ing documentation                                |
| [ Search    Cmd K]| Bring every machine you own back online, and reach it  |
|                   | from anywhere.                                         |
| GET STARTED       |                                                        |
|  What it is       | +-------------------+ +-------------------+            |
|  Install          | | Install           | | Add your first    |            |
|  The first run    | | macOS, Linux, CLI | | machine           |            |
|  Add a machine    | +-------------------+ +-------------------+            |
| REACH             | +-------------------+ +-------------------+            |
|  Remote access    | | Remote access     | | Use AI agents     |            |
|  SSH  Desktop ... | | SSH, desktop, web | | anima mcp, skill  |            |
| WATCH             | +-------------------+ +-------------------+            |
| AI AGENTS         |                                                        |
| RUN YOUR OWN HUB  | For agents: llms.txt  llms-full.txt  add .md to a page |
| REFERENCE         |                                                        |
| HELP              |                                                        |
+-------------------+--------------------------------------------------------+
```

A task page ("Remote access"):

```text
+-------------------+--------------------------------------------------------+
| (side menu)       | Remote access                                          |
|                   | Open a terminal, a desktop or a web interface of a     |
|                   | machine on your hub, from any of your devices.         |
|                   |                                                        |
|                   | | Way       | Needs           | Works from  | Default | |
|                   | | SSH       | sshd on machine | all clients | yes     | |
|                   | | Desktop   | VNC or RDP      | desktop,web | no      | |
|                   | | Web iface | a local port    | all clients | no      | |
|                   |                                                        |
|                   | 1. Open the machine.   2. Select SSH.                  |
|                   |   $ anima ssh my-mac                        [Copy]     |
|                   | Check: the prompt of my-mac shows.                     |
|                   | Limit: there is no public download yet.                |
|                   | Last changed 2026-11-19 . Edit this page . View as .md |
+-------------------+--------------------------------------------------------+
```

The CLI reference:

```text
| (side menu) | CLI commands  (generated from anima 0.x.y; do not edit)   | On this page |
|             | anima device list [--status] [--tag] [--name] [--site]    |  auth        |
|             |   List devices, with filters and pages.                   |  context     |
|             |   --tag   Only devices with this tag; repeat for more     |  device      |
|             |           than one.                                       |  docs        |
|             |   --all   Every page, not only the first.                 |  mcp ...     |
```

The privacy page:

```text
| (side menu) | Privacy and outbound connections                          |
|             | Anima.ing collects no telemetry. [NEEDS CLARIFICATION]    |
|             | | Connection   | Program | When        | Sends   | Off? |   |
|             | | Update feed  | desktop | start, 1/day| IP, UA  | no   |   |
|             | | Your hub     | all     | when used   | your data| -   |   |
|             | | Analytics    | docs    | when read   | page, OS| no   |   |
```

The lines of the CLI wireframe are the `///` text of `anima device list` at `3353f648`; the generated page can differ. The rows of the privacy wireframe and mockup are examples, except the row of Vercel Web Analytics on the docs site (R-207, OD-UD7). The real rows come from the inventory of task UD0. "Anima.ing collects no telemetry" is not verified: the two websites use Vercel Web Analytics (OD-21, R-207), and R-196 put shared install counts in version 1 of the agent tools (AXT-044, opt-in).

### 8.3 Engineering review

**Types.** In `apps/docs` (TypeScript): `DocPage { slug, title, group, summary, sourcePath, updated, markdown }`, `SearchEntry { url, page, heading, text }` and `MenuGroup { title, pages }`. In `crates/anima-cli`: a `docs` folder (an operator noun) with `DocsBundle` (the pages and the index, read from the compressed bytes in the program), `DocsQuery`, and `DocsHit { url, page, heading, text }`. The search is a pure function of the index and the words, so it is the same in the browser and in `anima` (one written rule; two implementations; one shared test fixture).

**Contracts.** No frozen contract changes: K1 to K7 do not change, K8 does not change, and `contracts/schema.graphql` gets no field. The new formats are inside the docs: the search index JSON and the front matter of a page. They are not shared between the hub, the daemon and the clients, so they are not contracts under CON-001. The MCP tool schemas of UDOC-036 are part of the tool catalogue (MCP-009).

**Failure modes.**

| Failure | Effect | Answer |
| --- | --- | --- |
| A generated reference is old | The site shows a flag that does not exist | The test of UDOC-023 fails in `bun run gates` |
| A page describes a function that is not released | A user looks for something that is not there | D2; review; the tag of UDOC-016 |
| A broken link | A 404 for the reader | UDOC-045, UDOC-046 |
| The site build fails on Vercel | The last good build stays live | Vercel keeps the last deployment; the failure shows on the commit |
| The search index is too large | A slow first search | Build the index per section; measure in UD5; load it only when the search opens |
| The docs in `anima` are older than the site | An agent reads the docs of its own version | That is the intent; `anima docs show` prints the version |
| A page from an outside contributor holds text that steers an agent | The text reaches agents through `llms-full.txt` and the MCP tools | Review of each change to `docs/user/`; MCP-008 cleaning; the tools return data, not instructions |
| A privacy row is wrong or missing | A user trusts a wrong list | UDOC-042 source notes; UDOC-043 rule; the inventory of UD0 |

**Trust boundaries.** The site holds no secret, no account and no form, and runs no server code. Its one outbound connection is Vercel Web Analytics on the same origin (UDOC-005). The build reads only this repository. `anima docs` reads only the bytes in the program. The MCP tools read no hub data, so a token does not change what they show. The only external input is a pull request from a person outside the project, which a maintainer reviews.

**Tests.** The golden test of the CLI reference and of the MCP reference (UDOC-023, UDOC-025); a unit test of the search with the shared fixture, in Rust and in TypeScript; an integration test of `anima_docs_search` and `anima_docs_read` over JSON-RPC (MCP-017); a build test of `apps/docs` (UDOC-046); a test that the CSP header has the values of UDOC-005; a check in a browser that the analytics script loads from the site origin and writes no cookie (UDOC-005); a test that `anima docs` opens no socket (run with no network in the test); the human check of UDOC-047.

### 8.4 Blast radius on shared contracts and rules

| Shared item | Effect | Needs a ruling |
| --- | --- | --- |
| K1 to K8, `contracts/schema.graphql` | None | No |
| CMD-031 coverage test | None: no root field | No |
| The tool catalogue of `anima mcp` (cli-mcp-plan.md Section 7) and the Settings panel (MCP-019) | One new tool set, "docs", with two read tools. The panel shows one more group | The direction is decided (R-207, OD-UD12); the approval of UDOC-036 is task UD20 |
| The size of the `anima` program and the Homebrew and Cargo packages (R-151) | It grows by the compressed docs (UDOC-033) | Measured in UD0 |
| The release map (R-198, R-200) | `docs/user/` is not in the folders of an application, so a change to a page releases nothing. A new release of `anima` takes the pages of its commit | Clarification of UDOC-007 |
| SITE-001 to SITE-012 | SITE-005 changes (OD-UD13). The others do not change. This proposal applies SITE-007 and SITE-008 (with its analytics exception) to the new site by its own requirements; SITE-012 does not apply to it | Decided: R-207 (OD-UD5 to OD-UD8, OD-UD13) |
| `AGENTS.md` | Two review rules (Section 7) | Yes |
| `quality:documents` | It reads one more folder | No |
| The Cloudflare zone `anima.ing` and the Vercel account | A record for `docs` and a new project | The operator does it (DNS-008, VPS-005) |

## 9. Decisions

Each row asks one question with yes, no or one choice, and gives the recommendation of the Planner (PLN-018, PLN-021). The operator answered each row first-hand on 2026-11-19, 14:03 to 14:11 PT, and R-207 records the answers. Rows OD-UD1 to OD-UD4 were first relayed by Anima Product Ideas.

| Number | Question | Recommendation, with strengths and weaknesses | State |
| --- | --- | --- | --- |
| OD-UD1 | Do you confirm that the docs live on a separate site, `docs.anima.ing`? (yes/no) | Yes. Strength: the docs site can stay static and free of trackers (SITE-007, SITE-008) while `www.anima.ing` now has sign-in and server routes; a clean address for `llms.txt`. Weakness: one more Vercel project and one more DNS record; the idea recommended `/docs` on the website. | Decided (R-207, operator, 2026-11-19, 14:03 PT, first-hand: "OD-UD1: yes, the docs live on docs.anima.ing."). |
| OD-UD2 | Do you confirm that the source is Markdown in this repository, `docs/user/`, and that the docs site publishes it? (yes/no) | Yes. Strength: a code change and its page are in one commit and one review; the gates check the links. Weakness: the public repository holds the docs, so each docs change follows the rules of this repository. | Decided (R-207, operator, 2026-11-19, 14:04 PT, first-hand: "OD-UD2: yes, Markdown in docs/user/ in the anima repo, published by the docs site."). |
| OD-UD3 | Do you confirm that the first version is the full set of Section 6.2.1 (effort L), published in the waves of Section 10? (yes/no) | Yes, in waves, with the site public after wave 2. Strength: the narrowest useful version comes first, and the full set follows with no new decision. Weakness: effort L before the first public download; waves 3 to 5 describe functions that few users can get yet. | Decided (R-207, operator, 2026-11-19, 14:04 PT, first-hand: "OD-UD3: yes, the full set in waves, public after wave 2."). |
| OD-UD4 | Do you confirm the new row 4.22 in the priorities, at tier 1, after row 4.3? (one choice: tier 1 after 4.3; tier 3 after 4.21; tier 2 beside 2a and 2b) | Tier 1 after 4.3. Strength: the first public download (rows 4.16 and 4.17) needs install docs, and the idea says to stop if the public install is not near. Weakness: writing time competes with the engine (row 4.4). Waves 3 to 5 can wait without a new row. | Decided (R-207, operator, 2026-11-19, 14:05 PT, first-hand: "OD-UD4: row 4.22 at tier 1, after 4.3."). |
| OD-UD5 | Does this proposal own the user docs and docs.anima.ing (a new row in ownership.md, UDOC), and does design specification Section 3.12 stay the owner of the marketing website only? (yes/no; it needs a ruling) | Yes. Strength: one owner for each site (PLN-016); Section 3.12 does not grow a second subject. Weakness: two documents hold site rules; UDOC-004 and UDOC-005 repeat the intent of SITE-007 and SITE-008 by reference. | Decided (R-207, operator, 2026-11-19, 14:05 PT, first-hand: "OD-UD5: yes, this proposal owns the user docs and docs.anima.ing; Section 3.12 stays the marketing site only."). |
| OD-UD6 | Does SITE-007 (static files only, no account, sign-in, form, cookie or server code) apply to docs.anima.ing? (yes/no) | Yes. Strength: nothing to attack and nothing to run; it keeps a promise that `www.anima.ing` no longer keeps. Weakness: no search address with a query, no `Accept` choice, and no remote docs MCP (UDOC-028, UDOC-029, UDOC-038). | Decided (R-207, operator, 2026-11-19, 14:05 PT, first-hand: "OD-UD6: yes, SITE-007 applies; docs.anima.ing is static files only."). |
| OD-UD7 | Does SITE-008 apply to docs.anima.ing without its exception, so the docs site has no analytics at all? (one choice: no analytics; the same exception for Vercel Web Analytics) | No analytics. Strength: the privacy page can say "docs.anima.ing makes no connection of its own"; it fits D4. Weakness: no count of which pages people read. | Decided (R-207, operator, 2026-11-19, 14:08 PT, first-hand: "OD-UD7: allow the same Vercel Web Analytics exception."). Against the recommendation (no analytics). UDOC-004, UDOC-005 and UDOC-044 are changed. |
| OD-UD8 | Does SITE-012 (no text about a paid hosted option until you approve the words) apply to docs.anima.ing? (yes/no) | Yes. Strength: one gate for the two sites (OD-22); the free hosted hub can still be documented. Weakness: the hosted pages wait for your words. [NEEDS CLARIFICATION: `llms.txt` of `www.anima.ing` already names "the hosted Solo plan"; no ruling that decides OD-22 was found.] | Decided (R-207, operator, 2026-11-19, 14:09 PT, first-hand: "OD-UD8: no, the docs may describe the paid hosted option now."). Against the recommendation (yes). UDOC-020 is changed; no fact of the business vault enters the pages. |
| OD-UD9 | Does ASD-STE100 apply to the user pages in `docs/user/`? (yes/no) | No. Use the style guide of UDOC-017: it keeps the STE rules of one instruction in one sentence, the active voice and one term for one item, and permits "you", as the Rex docs do. Strength: pages read like Rex, for people who are not engineers. Weakness: two writing rules in one repository; a reviewer must know which folder uses which. | Decided (R-207, operator, 2026-11-19, 14:10 PT, first-hand: "OD-UD9: no, STE does not apply to docs/user/; use the plain style guide."). |
| OD-UD10 | Do you fold the idea Agent-Readable Surface (a skill file, `llms.txt`, an install prompt) into this proposal (UDOC-031, UDOC-032), and delete its idea folder? (yes/no) | Yes. Strength: the agent copy is made from the human pages, so one owner and one source; the idea itself said they should be one plan. Weakness: the skill file then waits for this proposal; the idea is only Framed and you have not picked it. | Decided (R-207, operator, 2026-11-19, 14:10 PT, first-hand: "OD-UD10: yes, fold Agent-Readable Surface in and delete its idea folder."). The folder is deleted; UDOC-049 is added. |
| OD-UD11 | Where is the code of the docs site? (one choice: `apps/docs/` in this repository; a route group in `anima-site`; a new repository) | `apps/docs/` in this repository. Strength: the pages, the generators and the site build in one commit; the site reads the design tokens with no copy; the site is AGPL with the product. Weakness: a second SvelteKit app in this repository; a second design of the site look next to `anima-site`. | Decided (R-207, operator, 2026-11-19, 14:10 PT, first-hand: "OD-UD11: apps/docs/ in the anima repo."). |
| OD-UD12 | Is the docs MCP local only: two read tools in `anima mcp` that read the docs inside the program, with no remote docs MCP server in the first version? (yes/no) | Yes. Strength: no server, no account, works offline, and gives the docs of the version that the agent runs. Weakness: an agent with no `anima` program gets only `llms-full.txt`; the program grows (UDOC-033). | Decided (R-207, operator, 2026-11-19, 14:11 PT, first-hand: "OD-UD12: yes, the docs MCP is local only, inside anima mcp."). |
| OD-UD13 | Do the install steps of SITE-005 move to the docs, so that `www.anima.ing` links to the install page of docs.anima.ing? (yes/no; it changes SITE-005) | Yes. Strength: one set of install steps (PLN-016). Weakness: the marketing website shows fewer words; a change to `anima-site` and to Section 3.12. | Decided (R-207, operator, 2026-11-19, 14:11 PT, first-hand: "OD-UD13: yes, move the SITE-005 install steps to the docs; www.anima.ing links to the docs install page."). Tasks UD16 and UD17 are changed. |

## 10. Tasks

One pull request for each task. The size of each task is a few hours to three days (an estimate). All tasks are Not started. R-207 decided OD-UD1 to OD-UD13, but it names no task and does not approve the requirements, so no task starts before a ruling names it (PLN-007). Task UD20 is the approval. The waves give the order.

| Task | Wave | Content | Needs | State | Commits |
| --- | --- | --- | --- | --- | --- |
| UD20 | 0 | The operator approves UDOC-001 to UDOC-050 with a ruling, and answers the [NEEDS CLARIFICATION] markers that are his (UDOC-005 if the analytics script writes a cookie; UDOC-020 for a price; UDOC-050 after the load test). OD-UD1 to OD-UD13 are decided (R-207) | The operator | Not started | — |
| UD0 | 0 | A record only, in `docs/research/`: the inventory of each outbound connection of each program, with its code path (for UDOC-041); the count of commands with no help text; the size of all pages compressed (UDOC-033); if `quality:documents` reads `docs/user/` | A ruling that names UD0 (it can come before UD20, because it is a record) | Not started | — |
| UD1 | 0 | `docs/user/README.md` (the folder rules), `docs/user/STYLE.md` (UDOC-017 to UDOC-020; plain style, R-207), the folder skeleton of Section 6.2.1, the row in work-plan.md | UD20 | Not started | — |
| UD2 | 1 | The CLI reference generator and its golden test (UDOC-021, UDOC-023 to UDOC-025); `docs/user/reference/cli.md`; `quality/generated.txt` | UD1 | Not started | — |
| UD3 | 1 | The MCP tools reference generator and its golden test (UDOC-022, UDOC-023); `docs/user/reference/mcp-tools.md` | UD1 | Not started | — |
| UD4 | 1 | `apps/docs`: the static SvelteKit site, the side menu, the page layout, the tokens, the CSP and headers, the build checks (UDOC-003 to UDOC-006, UDOC-008, UDOC-009, UDOC-011, UDOC-046); Vercel Web Analytics from the site origin, and the check that it writes no cookie (UDOC-005); the mockups compared (PLN-019) | UD1 | Not started | — |
| UD5 | 2 | The agent copy: `.md` pages, `/llms.txt`, `/llms-full.txt`, `/search-index.json`, the search box with Cmd+K (UDOC-015, UDOC-026, UDOC-027, UDOC-029) | UD4 | Not started | — |
| UD6 | 2 | Pages of Get started: What Anima.ing is, Install (the page that `www.anima.ing` links to), The first run (UDOC-050), The hosted option (UDOC-020: public facts only, no price; UDOC-050: the free hub, "good for about 5 machines", its sleep, one per account, the pause after 60 days), Add your first machine, Add a phone; `INSTALL.md` at the root (UDOC-049) | UD1 | Not started | — |
| UD7 | 2 | Pages of Reach your machines: Remote access (with the table), SSH, Desktop, Web interfaces | UD1 | Not started | — |
| UD8 | 3 | Pages: The VPN, Stay awake; and Watch your machines: Uptime checks, Metrics, Notifications, Prometheus | UD1 | Not started | — |
| UD9 | 3 | Pages of Use AI agents: The command line, `anima mcp`, What an agent can do, MCP permissions; the skill file and "Install with an agent" (UDOC-031, UDOC-032; R-207, OD-UD10) | UD1, UD3 | Not started | — |
| UD10 | 3 | Pages of Run your own hub: Self-host with Compose, The local hub, The bundled database, Back up, restore and move, Update | UD1 | Not started | — |
| UD11 | 3 | The privacy page from the inventory of UD0, with the source notes, and the rows of Vercel Web Analytics for the two websites (UDOC-040 to UDOC-042, UDOC-044) | UD0, UD1 | Not started | — |
| UD12 | 4 | `anima docs search` and `anima docs show`, with the pages inside the program (UDOC-033 to UDOC-035) | UD5 | Not started | — |
| UD13 | 4 | The MCP tools `anima_docs_search` and `anima_docs_read`, their integration tests, the pointer row in cli-mcp-plan.md (UDOC-036, UDOC-037) | UD12 | Not started | — |
| UD14 | 4 | The client: the "Help" link in Settings and in the desktop menu (UDOC-039) | UD4 | Not started | — |
| UD15 | 4 | Pages of Help: Troubleshooting, Uninstall, Release notes; the Settings reference | UD1 | Not started | — |
| UD16 | 0 | Not a task of this repository: in `anima-site`, (a) now: correct the MCP link in `static/llms.txt` line 12 and `static/llms-full.txt` line 46 to `docs/procedures/setup/anima-mcp.md`; (b) after wave 2 is public: point both files to `https://docs.anima.ing/llms.txt` (UDOC-030), and replace the install steps of the website with a link to `https://docs.anima.ing` Install (R-207, OD-UD13) | (a) none (a defect); (b) UD18 | Not started | — |
| UD17 | 5 | `README.md` Documentation table; the two lines of `AGENTS.md` (UDOC-043); in design specification Section 3.12: SITE-005 says that the install steps are on the install page of docs.anima.ing and the website links to it (R-207, OD-UD13), a note that Section 3.12 owns the marketing website only (OD-UD5), and the stale text "built and not deployed" | UD20; a ruling for AGENTS.md | Not started | — |
| UD18 | 5 | Publish: the operator connects the Vercel project and the `docs` record (DNS-008, VPS-005); the entry `HT-nnn` for UDOC-047 and UDOC-048 | UD4 to UD7; the operator | Not started | — |
| UD19 | 5 | The human check of UDOC-047 and UDOC-048 on macOS and Linux | UD18; the operator | Not started | — |

### 10.1 Acceptance, checks and scope of each task

| Task | Pass or fail | Check commands | Out of scope |
| --- | --- | --- | --- |
| UD0 | Pass when the record lists each program with its connections and code paths, the count of commands with no text, and the measured size; fail when a program of UDOC-041 is not in it | `bun run quality:documents` | Fixes in code |
| UD1 | Pass when the folder, the README, the style guide and the work-plan row exist and the gate passes; fail otherwise | `bun run quality:register`, `bun run quality:documents` | Page content |
| UD2 | Pass when the generated file lists each visible command and flag, the golden test fails after a change to a `///` comment until the file is generated again, and the file is in `quality/generated.txt`; fail otherwise | `cargo nextest run -p anima-cli`, `bun run gates` | Better help text (its own tasks) |
| UD3 | Pass when the file lists each tool with its tool set, kind and default state, and the golden test guards it; fail otherwise | `cargo nextest run -p anima-cli` | New tools |
| UD4 | Pass when `bun run --cwd apps/docs build` gives static files only, the CSP has the values of UDOC-005, the analytics script loads from the site origin and writes no cookie and no local storage, no request goes to another origin (checked in the browser network panel and by a test of the built HTML), and the mockup row names the commit of the comparison; fail otherwise | `bun run --cwd apps/docs build`, `bun run --cwd apps/docs check`, `bun run --cwd apps/docs test`, `bun run quality:documents` | Page content; deploy |
| UD5 | Pass when each page has a `.md` copy with the same text, `llms-full.txt` holds all pages in menu order, and the search finds the fixture queries in Rust and TypeScript the same; fail otherwise | `bun run --cwd apps/docs test`, `bun run --cwd apps/docs build` | `Accept` choice, a query address |
| UD20 | Pass when a ruling approves the requirements, or names the ones that are not approved; fail otherwise | `bun run quality:register`, `bun run quality:documents` | A build |
| UD6 to UD10, UD15 | Pass when each page of the task obeys UDOC-011 to UDOC-014 and UDOC-017 to UDOC-020 (and, for UD6, UDOC-050: the free hub is "good for about 5 machines" as a guide, not a limit), each command in it runs as written on the date of the commit or the page says it is not verified, and the build passes; fail otherwise | `bun run --cwd apps/docs build`, `bun run quality:documents` | Pages of other groups |
| UD11 | Pass when each connection of the UD0 record has a row with when, where, what and off, and a source note; fail otherwise | `bun run --cwd apps/docs build` | A gate for new connections |
| UD12 | Pass when `anima docs search` and `show` work with no hub and no network (test), and `--json` follows the output rules; fail otherwise | `cargo nextest run -p anima-cli`, `bun run gates` | A remote server |
| UD13 | Pass when the two tools are listed with no token, return cleaned text under `RESULT_CAP`, and pass their JSON-RPC tests (MCP-017); fail otherwise | `cargo nextest run -p anima-cli` | Change tools |
| UD14 | Pass when the link opens the system browser in the desktop app and a new tab in the web client; fail otherwise | `bun run --cwd apps/client test`, `bun run gates` | An in-app docs viewer |
| UD16 | Pass when both files in `anima-site` link an address that gives HTTP 200 and, after (b), the install section of the website links to the install page of the docs; fail otherwise | In `anima-site`: `bun run test`, then `curl -sL -o /dev/null -w '%{http_code}'` on the link | Other site changes |
| UD17 | Pass when the README, AGENTS.md and Section 3.12 have the changes of Section 7 and the ruling; fail otherwise | `bun run quality:documents` | — |
| UD18 | Pass when `https://docs.anima.ing` serves the build of `main` and the entry `HT-nnn` exists with its steps; fail otherwise | `curl -sI https://docs.anima.ing`, `bun run quality:documents` | — |
| UD19 | Pass when the operator sets the entry to Passed; fail otherwise | — | — |

Rules for each task:

- Always: work in a worktree from `origin/dev` and merge to `dev` (AGENTS.md); claim the task (PLN-022); run `bun run quality:register` and `bun run quality:documents` before each commit; generate the references, never edit them; write "Not verified" for a step that you did not run.
- Ask first: a change to the Vercel account or the `anima.ing` zone (DNS-008, VPS-005); a new outbound connection; a price on a page; a change to `anima-site`.
- Never: a third-party script, font, image or style on the docs site (Vercel Web Analytics from the site origin is the one exception); a page about a function that is not released; a requirement identifier on a user page; a fact of the business vault (a price, a cost, a funding fact or a trademark finding); a secret in a page or an example.

Prompt for each task: "Follow AGENTS.md 'Start of a session'. Claim and build task UDn of docs/proposals/user-documentation.md. Read Sections 5, 6 and 8 first. Build only what the row of UDn says. Run the check commands of the row, then bun run quality:documents and bun run gates."

## 11. Not verified

- The number of UDOC-050 (about 5 machines on a free hosted hub). It is the operator's guide, not a measurement; no load test of a free hub exists (R-208).
- If the script of Vercel Web Analytics writes a first-party cookie or local storage. Vercel's page says only "without using any third-party cookies" and a hash of the request (read 2026-11-19). UD4 checks it.
- A public pricing page: `www.anima.ing/pricing` gives HTTP 404 on 2026-11-19 (UDOC-020).
- The quality of the help text of `anima`, and the count of commands with no text: the crate did not build on the box (`rustc` too old). UD0 measures it.
- The size that the docs add to the `anima` program (UDOC-033).
- If `quality:documents` reads `docs/user/` with no change (UDOC-045).
- If a DNS record for `docs.anima.ing` exists. The address gives no answer over HTTPS on 2026-11-19.
- If a ruling permits the server routes of `anima-site` that SITE-007 forbids. None was found in [rulings.md](../rulings.md).
- If OD-22 (text about a paid hosted option on `www.anima.ing`) is decided. `llms.txt` of the website names "the hosted Solo plan"; no ruling was found. R-207 decides only the docs; OD-22 stays a separate open item.
- The statement "Anima.ing collects no telemetry" in the privacy wireframe. It is an example, not a fact.
- The Rex docs: read on 2026-11-19 from the public pages and `llms-full.txt` (608 KB). That the site is SvelteKit on Vercel is from its HTML and the idea, not confirmed by its makers.

## 12. Review notes

- 2026-11-19: version 1. The Planner made the proposal from the idea `user-documentation` (`3e5910d0`). The idea folder is deleted in this commit. The idea `agent-readable-surface` is not deleted, because OD-UD10 is open.
- 2026-11-19: version 2. R-207 records the first-hand answers of the operator to OD-UD1 to OD-UD13 (14:03 to 14:11 PT). The state is Direction decided. Changes: UDOC-004, UDOC-005, UDOC-044 and the privacy frame for Vercel Web Analytics (OD-UD7); UDOC-020 and D3 for the paid hosted option (OD-UD8); the idea `agent-readable-surface` folded in and deleted, UDOC-031 and UDOC-032 unconditional, UDOC-049 added (OD-UD10); UD16 and UD17 for SITE-005 (OD-UD13); row 4.22 moved to tier 1 (OD-UD4); the ownership row (OD-UD5); task UD20 for the approval.
- 2026-11-19: version 3. R-208 (operator, first-hand, 14:17 and 14:18 PT): UDOC-050, the free hosted hub is "good for about 5 machines" in the docs, a guide and not a limit, not measured. Task UD6 and its acceptance are changed. No hosted-mode rule changes.
