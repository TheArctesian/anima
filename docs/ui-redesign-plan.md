# Anima UI redesign: plan and prompt (v0.2)

Bottom-up order: philosophy and spec delta → tokens → primitives → shell → pages → final.
Each layer is frozen before the next starts. The problem statement is
`docs/ui-critique-2026-11-11.md` (findings 1–45, operator points O1–O11).

v0.2 applies a Fable 5.2 review (blockers B1–B5, majors M1–M12, all minors) and the operator
rulings below.

## Operator rulings (2026-11-11)

These are recorded in `docs/programme/operator-changes.md` by the lead in Phase 1.

| ID | Ruling |
|---|---|
| UR-1 | The spec may be rewritten wherever the redesign needs it. Phase 1 lists every requirement ID it changes, adds or withdraws (the spec delta) for approval. This covers UI-001–UI-022, NAV-001–NAV-007, DEV-020–DEV-022, SET-002, SET-003, the logs snap points in 5.7.4, and any other ID. |
| UR-2 | **Colour: themeable.** Nord stays the default theme, but the system must allow other colour schemes (for example VS Code themes) to be added later. Tokens are split into **theme palettes** (raw colours, one file per scheme) and **semantic roles** (surface tiers, text, accent, status, border) that map onto any palette. A neutral palette with one accent colour is allowed. Nord is no longer required to be the only palette (it overturns the Nord-only part of UI-001/UI-005). Shipping a second scheme is out of scope, but the structure, a theme registry and a test that every palette fills every role are in scope. |
| UR-3 | **Font:** Fira Code was temporary. The interface font becomes a proportional sans (bundled, OFL); Fira Code, or the chosen monospace, is kept for machine text only (IPs, keys, commands, logs, terminal). This overturns UI-020. |
| UR-4 | **Gates:** each lane runs gates 4–12 (`install`, `lint`, `check`, `test`, `build`, `limits`, `names`, `imports`, `baseline`). The lead runs the full 12/12 on the integration branch after every merge, one merge at a time. Any lane that touches Rust or `crates/theme` runs the full 12. This is a deviation from AGENTS.md for this programme only. |
| UR-5 | The `ux-laws` audit is dropped; its checks go into the reviewer brief. The two controls lanes are merged. Every other review recommendation is accepted. |
| UR-6 | Finding 13 is overridden by O3: the logo is too big, not too small. |

## Why this order is cheapest

- Token-level problems (font, flat surfaces, weak status, small type, control size) and
  component-level problems (button hierarchy, monotonous inputs, tooltips, tabs, tables, back link,
  empty and loading states) are most of the critique. Fixing them once fixes them everywhere.
- Look **and layout** are decided once, in Phase 1, on static HTML. That is cheap to change, so the
  costly code phases do not each invent their own direction.
- Primitives are built and proved on a specimen page. Call sites move to them only inside the
  page lane that owns the folder, so no file is rewritten twice.

## Phases

| Phase | What | Who | Parallel | Stop |
|---|---|---|---|---|
| 0 | Gallery spec and baseline; dark/wide addendum to the critique; component inventory | 1 `builder` | — | — |
| 1 | Philosophy, spec delta, **2–3 switchable directions** on real compositions, page wireframes | **the lead itself** (it has the `apple-design` and `frontend-design` skills) | — | **operator picks the direction and approves the delta and wireframes** |
| 2 | Tokens: theme palettes plus semantic roles, typography, space, control sizes, icons; font bundling and the typeface setting | 1 `builder` (font and typeface logic) + 1 `builder-light` (token JSON) | 2 lanes, disjoint files | operator looks at the specimen in real tokens plus 2 canary screens |
| 3 | Primitives, evolved in place where they exist; a specimen route; keyboard and contrast check | 2 lanes: **Controls** (button, segmented control, Field, inputs, HelpTip) and **Containers** (Section/Card, Tabs, StatusBadge, Table, page header and back link, Empty/Loading/Error, Notice, Dialog) | yes | one `reviewer-interface` pass on the specimen route |
| 4 | Shell: sidebar, page frame, Live sessions page, logs drawer default | 1 `builder` | — | reviewer, a check in the real Tauri build, operator look |
| 5 | Pages: 6 lanes, one owner per folder (table below) | 6 `builder` lanes | yes | `reviewer-interface` per lane, then fixes |
| 6 | `motion-audit` (spec 5.8), screen-reader `persona-test`, fix list, full gates | the lead runs the skills; 1 `builder` fixes | audits in parallel | **operator approves the merge** |

### Phase 3 primitives: reuse first

Evolve these in place:
- `forms/FormButton.svelte`: tones become primary, secondary, ghost and danger; `feedback` keeps the width fixed.
- `forms/modal/Modal.svelte` and `ConfirmModal`.
- `forms/disclosure/Disclosure.svelte`.
- `settings/SettingsSection.svelte` and `SettingRow.svelte`.
- `devices/card/StatusPill.svelte` and `StatusEdge.svelte`.
- `routes/devices/[id]/tabs/Tabs.svelte`: move it to `$lib/` if several routes need it.
- `devices/toolbar/Menu.svelte`.
- `events/resize/`.
- `motion/loading/`.
- `nav/collapse/CollapseToggle.svelte`.
- `hub/notice`.

Create new components only for:
- **HelpTip** (`$lib/overlay/helptip/`): it opens on click or focus and closes with Escape; it is hoverable and persistent (WCAG 1.4.13). It calls `coverWhile` (MAT-008, WEB-005). It is never used over a session pane.
- **Field** (`$lib/forms/field/`): label, HelpTip slot, visible constraint line and error.
- **Table** (`$lib/tables/`): sticky header, density, severity column, day grouping, truncation with full-text tip.
- **Segmented control** (`$lib/forms/segmented/`).
- **Page header and back link** (`$lib/page/`).
- **Key-value list** (`$lib/page/`), for structured text in place of prose.

Banned words: no `info`, `components/`, `utils/` or `common/`. Each folder holds at most 12 measured files, tests included. One family goes in each folder.

HelpTip rules:
- Defaults, explanations, allowed ranges and fingerprints go in a HelpTip.
- A constraint the operator needs in order to fill a field correctly (a required format) stays visible.
- Errors are never in a tip.

Status: "one combined status" (finding 3) is a precedence function in `$lib/devices/`, with a
spec rule written in Phase 1. StatusBadge only displays it, and the reason goes in its HelpTip.

### Phase 5 lane ownership

Each folder has exactly one owner.

| Lane | Owns | Closes |
|---|---|---|
| A Devices list | `routes/devices/+page*`, `lib/devices/{card,toolbar,grid,table}`, `lib/metrics/bars` | 14–20 |
| B Device page | `routes/devices/[id]/**` except the Sessions and Logs tab bodies, plus the empty Checks tab and the RDP button | 27 (root cause first: the blank first load is probably a load bug), 28–33 |
| C Settings and VPN | `routes/settings/**`, `lib/settings`, the section libraries (`notifications`, `retention`, `updates`, `api-keys`, `hub-address`, `checks` defaults), `lib/vpn` | 35–44, VPN parts of 45 |
| D Logs and sessions tables | `lib/events`, `lib/sessions` list views, the device Sessions and Logs tab bodies | 21, 34 |
| E Forms and dialogs | `lib/devices/form` (Add device, Edit device, device Settings tab), `lib/vpn/enrol` (by agreement with C: C hands over `enrol/`) | 22–26, 45 |
| F Sessions and remaining surfaces | sign-in, the session strip and views (SSH, RDP, VNC, files panel, credential, host-key and certificate modals), `web-interfaces`, `access`, `builds`, the API key dialog | regression under the new tokens and primitives |

Lanes never edit `design/tokens/` or the primitive folders after the freeze. A lane that needs a
token or primitive change asks the lead.

### Findings no phase originally owned (all now assigned)

| Item | Where it is closed |
|---|---|
| 7 Icons | Policy in Phase 1, `icon-*` tokens in Phase 2 |
| 23 Placeholders, 24 Alignment | Phase 3 Controls: placeholder style; one control height |
| 29 "+ RDP" | Lane B, under the DEV-022 ruling |
| 33 Empty Checks tab | Lane B |
| 41, 42 Inline result | Notice primitive (`hub/notice`, `SettingRow` message) in Phase 3; used in lane C |
| 43, 44 VPN rows | Lane C |
| O6 Unformatted text | Key-value list primitive (Phase 3); lanes convert prose |
| O8 Overflow, gaps, spacing | Truncation and spacing rule in Phase 1; reviewer checklist |
| O10 Information in tooltips | HelpTip rules above; every lane |

### Gallery and screenshots

- Phase 0 commits `apps/client/browser/gallery/` (a Playwright spec on the synthetic hub). It shoots WebKit only (MV-001), at widths 800, 1280 and 1600, dark first then light, plus one OpenDyslexic pass (UI-019). Output goes to a git-ignored folder.
- **Contact sheet:** about 12 key screens per gate, sent to the operator. Use publish or attach where available; otherwise give absolute paths.
- **Phase 2 is judged on the specimen route and two canary screens.** The full gallery is expected to look rough until Phase 5.
- **Checks that need the real Tauri build:** the macOS title-bar overlap and the collapse button (NAV-007).

### Reviewer checklist (every review)

- Target sizes (UI-013).
- Contrast for text and shapes, in dark first and then light (UI-002 to UI-005).
- Focus order and focus return.
- Layout shift (MOT-007).
- Reduced motion (MOT-040 to MOT-042).
- `prefers-reduced-transparency`, `prefers-contrast: more` (MAT-006) and WebKitGTK blur cost (MAT-007).
- Truncation and spacing rule (O8).
- Placement of HelpTips and visible constraints.
- Hick, Fitts, proximity and hierarchy (folded in from `ux-laws`).
- Every finding names its CUPID property and the critique number it closes.

### Requirement IDs to cite

UI-001–UI-005, UI-010–UI-022, NAV-001–NAV-007, TOK-001–TOK-004, TOK-010, TOK-011, TOK-020,
TOK-021, MAT-001–MAT-008, MOT-001, MOT-005–MOT-008, MOT-040–MOT-042, MV-001–MV-008, DEV-005,
DEV-020–DEV-022, SET-002, SET-003, LOG-009, RA-011, RA-034, CQ-010, CQ-011, CQ-030, REPO-005.

---

## Prompt to paste into a fork of the root session

```
You are the lead for Anima's UI redesign. Work bottom-up and stop at each operator gate.

Read first: AGENTS.md; docs/design-spec.md Sections 5, 6.1, 6.2, 6.7 and 7;
docs/ui-critique-2026-11-11.md; docs/ui-redesign-plan.md (v0.2 — the plan you are executing,
including the operator rulings UR-1..UR-6, the primitive list, the lane ownership table, the
gallery rules and the reviewer checklist). Every change closes one or more numbered findings
(1–45, O1–O11). Keep a finding → phase → commit table as an appendix of docs/ui-redesign-plan.md.

Base: create the integration branch ui-redesign from claude/nostalgic-hawking-79449a at the commit
that holds this plan (git log -1 -- docs/ui-redesign-plan.md). Work in your own worktree. Every
subagent starts with `git switch -c ui-p<phase>-<lane> <current ui-redesign head>` (never
reset --hard), never pushes, ends commits with the Co-Authored-By line, and runs gates 4–12
(UR-4; full 12 if it touches Rust or crates/theme). You merge lanes one at a time and run
`BASE_REF=claude/nostalgic-hawking-79449a bun run gates` (12/12) after each merge. Agents never
merge. Lanes never edit design/tokens or primitive folders after their freeze; requests come to
you.

Stop and ask me if a change needs: a requirement outside the approved spec delta, a
quality/baseline.json entry, a lint suppression, a file over 300 code lines, or a third fix round
on one lane. Never change the VPS, push, or merge into any branch other than ui-redesign without
my word.

PHASE 0 — Gallery and baseline (1 builder).
Commit apps/client/browser/gallery/ (Playwright on the synthetic hub, WebKit, widths 800/1280/1600,
dark then light, one OpenDyslexic pass; output to a git-ignored folder) covering every route and
state: Devices grid/table/filters/Add device; device page with all tabs; Live sessions; Logs
drawer; every Settings section; VPN dialogs; sign-in; session views and their modals; empty,
loading and error states. Also write a component inventory (each $lib component, use count,
ad-hoc styles and raw sizes that bypass tokens). You then add findings 46+ to the critique from
the dark and wide shots.

PHASE 1 — Direction (you, the lead; load apple-design and frontend-design).
1. Philosophy: calm, dense but legible, status first, progressive disclosure via HelpTip and
   Disclosure, one accent, sans for interface text, monospace only for machine text, fewer
   borders (surface contrast instead), truncation and spacing rule, icon policy.
2. Spec delta: every requirement ID changed/added/withdrawn (UR-1), including the themeable
   colour model (UR-2: palettes + semantic roles + theme registry; Nord default), the font (UR-3),
   NAV-003 Connected as an icon, DEV-020/022 header and RDP, 5.7.4 logs snap points, SET-002/003,
   the combined-status precedence rule, and a Live sessions page.
3. Static HTML in docs/mockup/redesign/ with a switcher for 2–3 directions (A: Nord palette with
   sans and derived surface tiers; B: neutral palette with one Nord accent; C: denser variant if
   useful), each shown on real compositions (Devices list with toolbar and cards; device page
   header with a form section; a Settings section), dark and light; plus low-fidelity wireframes
   of the shell, Devices, Device page and Settings.
STOP: send me the page and the delta; I pick a direction and approve. Then log UR-1..UR-6 and the
approved delta in docs/programme/operator-changes.md and apply the delta to design-spec.md.

PHASE 2 — Tokens (2 lanes).
 builder-light: design/tokens/*.json and reference.md — split palettes (nord.palette, neutral
   kept only if chosen) from semantic roles; surface tiers, text, accent, status, border,
   control heights, spacing, type scale, icon sizes; TOK-004 for renames (add, migrate, remove);
   design/tests contrast rules updated, not loosened; a test that every palette fills every role.
 builder: the font — bundle the sans (verify OFL and reserved font name before subsetting),
   UI-017/UI-018 licence in the package and About, $lib/typeface (DEFAULT_TYPEFACE, saved-choice
   migration, first-paint.ts, TypefaceRows, fonts.css, fonts.test.ts), tabular figures for
   numbers, ligatures off for machine text, terminal font unchanged (RA-011), hierarchy works with
   weights 400/700 (OpenDyslexic).
Both run the full 12 (crates/theme regenerates).
STOP: I look at the specimen in real tokens plus two canary screens.

PHASE 3 — Primitives (2 lanes, then one reviewer-interface pass on the specimen route).
Build or evolve exactly the primitives listed in the plan, in the named folders, with a specimen
route (dev only) showing every primitive and state. Migrate call sites only when it is a drop-in
(same component, same props). Controls lane: FormButton tones, segmented control, Field, inputs
(placeholder style, one control height), HelpTip. Containers lane: Section/Card, Tabs,
StatusBadge (+ the precedence function in $lib/devices), Table, page header and back link,
key-value list, Empty/Loading/Error, Notice, Dialog presets. Review with the checklist; fix; merge.

PHASE 4 — Shell (1 builder).
Smaller logo; collapse control in the sidebar header; one-line nav labels; clear active item;
connection icon with a HelpTip (hub name, state); a Live sessions page with an empty state; one
page-header pattern; content width and gutters that use the screen; logs drawer default per the
delta. Review; then one check in the real Tauri build (title-bar overlap, NAV-007).
STOP: I look.

PHASE 5 — Pages (6 lanes per the ownership table; reviewer-interface per lane; fixes).
Each lane moves its folders onto the primitives, closes its finding numbers, converts prose to
key-value lists or HelpTips per the rules, and reports the findings it closed with before/after
contact-sheet shots.

PHASE 6 — Final.
Run motion-audit (report to docs/research/motion/) and a screen-reader persona-test; one builder
fixes the list; re-shoot the gallery; send me a before/after contact sheet; update the finding
table so every finding is closed or explained.
STOP: I approve merging ui-redesign.
```

---

## Appendix: finding → phase → commit

Kept by the lead. "Closed in" is the merge commit on `ui-redesign` that closes the finding; the
requirement column names the spec-delta ID from `docs/ui-redesign-direction.md`. O1–O11 follow
the order of the critique's "Operator's feedback" list (O1 inputs, O2 space, O3 logo, O4 collapse
button, O5 "Connected", O6 unformatted text, O7 typography, O8 back link, O9 overflow and
spacing, O10 tooltips, O11 box sizes). The plan's earlier tables use "O6" for unformatted text and
"O8" for overflow; they map to O6 and O9 here.

| Finding | Phase / lane | Requirement | Closed in |
|---|---|---|---|
| 1 Monospace everywhere | 2 font | UI-010, UI-011 | |
| 2 One flat colour | 2 tokens | UI-001, roles | |
| 3 Weak, double status | 3 StatusBadge + `$lib/devices` precedence | DEV-006 | |
| 4 Type scale | 2 tokens | UI-019 | |
| 5 Button hierarchy | 3 Controls | UI-029 | |
| 6 Cramped controls | 2 tokens, 3 Controls | UI-013 | |
| 7 Icons | 1 policy, 2 `icon-*` | UI-026 | |
| 8 Borders everywhere | 2 roles, 3 Section/Card | `elevation-1` | |
| 9 Nav label wraps | 4 Shell | NAV-001 | |
| 10 Active item | 4 Shell | NAV-002 | |
| 11 "Connected" text | 4 Shell | NAV-003 | |
| 12 Live sessions popover | 4 Shell | NAV-008 | |
| 13 Logo (overridden by O3, UR-6) | 4 Shell | 5.3 table | |
| 14–20 Devices list | 5 A | DEV-007, DEV-008, 6.1 | |
| 21 Logs drawer | 4 Shell (default), 5 D | 5.7.4 | |
| 22–26 Forms and dialogs | 5 E | UI-028, UI-029, MAT-003 | |
| 27 Blank device page | 5 B | DEV-023 (not reproduced in Phase 0) | |
| 28–33 Device page | 5 B | DEV-020, DEV-022, DEV-024 | |
| 34 Sessions and Logs tables | 3 Table, 5 D | UI-025 | |
| 35–42 Settings | 5 C | SET-004, SET-005, SET-006 | |
| 43–44 VPN rows | 5 C | SET-003 | |
| 45 VPN dialog | 5 E | MAT-003 | |
| 46 No page titles | 4 Shell | NAV-010 | |
| 47 Nav and page one colour | 2 roles, 4 Shell | roles | |
| 48 Empty wide columns | 4 Shell, 5 C | SET-004 | |
| 49 Nav before sign-in; 63 sign-in redesign (hub address in place) | 5 F | NAV-009 | |
| 50 One-line errors | 3 Empty/Loading/Error, all lanes | UI-030 | |
| 51 Logs panel not a layer | 4 Shell | `material-panel` | |
| 52 Card heights | 5 A | DEV-009 | |
| 53 Wake button tone | 5 A | DEV-009 | |
| 54 Uptime format | 5 A, B | UI-031 | |
| 55 Table Status column | 3 Table, 5 A | DEV-007 | |
| 56 Metric label distance | 5 B | UI-027 | |
| 57 Set-up buttons in header | 5 B | DEV-022 | |
| 58 Scrim and dialog surface | 3 Dialog presets | MAT-003 | |
| 59 Dialog cut edge | 3 Dialog presets | MAT-003 | |
| 60 VPN dialog placement | 3 Dialog, 5 E | MAT-003 | |
| 61–62 VPN rows | 5 C | SET-003 | |
| O1 Monotonous inputs | 3 Controls | roles | |
| O2 Screen space | 4 Shell, 5 all | philosophy 8 | |
| O3 Logo too big | 4 Shell | 5.3 table | |
| O4 Collapse button | 4 Shell | NAV-006 | |
| O5 "Connected" | 4 Shell | NAV-003 | |
| O6 Unformatted text | 3 key-value list, 5 all | philosophy 7 | |
| O7 Typography | 2 font and tokens | UI-010, UI-019 | |
| O8 Back link | 3 page header | DEV-020 | |
| O9 Overflow and spacing | 1 rule, every review | UI-027 | |
| O10 Tooltips | 3 HelpTip, 5 all | UI-028 | |
| O11 Box sizes | 3 Section/Card, 5 all | UI-027, DEV-009 | |
| UI-018 gap: no About view lists the font licences (found by the Phase 2 font lane; the `LICENSE-*.txt` files are in `src/styles/fonts/` but nothing shows them) | 5 C (an "About" section in Settings) | UI-018 | |
