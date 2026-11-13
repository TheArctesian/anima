# Anima UI: the website's design language (draft for review)

Status: **approved for implementation (operator, 2026-11-13).** Each phase lands on `main` once
`bun run gates` passes. The mockups are in `docs/mockup/site-language/index.html`; the spec text
for each row goes into `design-spec.md` in Phase B.

### Answers to the open questions (2026-11-13)

| Row | Answer |
|---|---|
| D1 | Drop 700. The app ships Plex Sans 200, 400 and 500; `weight-bold` is removed and its uses move to `weight-medium`. Under OpenDyslexic, light maps to regular and medium to bold |
| D2 | IBM Plex Mono everywhere, the terminal included, with a Nerd Font symbols-only face after it so shell prompt glyphs still draw |
| D5 | One display size on every top-level page, Settings included |
| D6 | The small-caps section head is enough on long forms; the rows under it carry the rest |
| D9 | The devices grid uses hairline tiles, not raised cards |
| D12 | The resolve entrance plays on first load and on a route change, never when a list re-renders in place |
| D13 | Sign-in shows the particle doors, a still mark under reduced motion |
| D15 | Glass stays on hold; the material tokens remain, unused |
| D17 | The client keeps the existing whole-device write and sends it per change (debounced for text); no new hub function, so no new command or MCP tool (CMD-030) |
| D18 | Keep the status word column on wide windows (UI-003: a word with every status) |

## Operator ruling (2026-11-13)

| ID | Ruling |
|---|---|
| UR-7 | The design of the marketing site (`anima-site`, built 2026-11-13) is the new design language of the app. Where it conflicts with an earlier design decision (UI-*, NAV-*, the redesign direction, UR-1 to UR-6, the Phase 1 philosophy), the site's language wins. This document lists each conflict and the proposed change, for approval one by one. |

## 1. The site's language

The site uses the app's own Graphite palette, so colour does not move. What is new is how the
interface is drawn.

| Trait | The site | The app today |
|---|---|---|
| Page ground | `surface-sunken` (#16181B); content sits directly on it | `surface-base` window, `surface-raised` cards |
| Grouping | Hairline rows: a `border-subtle` line between rows, no card fill | Raised cards and tables ("surfaces, not borders") |
| Display type | IBM Plex Sans **200** (Light), large, tracking −0.03 em | 700 for titles; 400 and 700 only |
| Labels | Small caps: 10.5 px, weight **500**, tracking +0.14 em, uppercase, `text-tertiary`, Plex Sans (never mono) | Sentence-case labels |
| Section head | Icon, a 28 px rule, then the label | Bold title, no icon (UI-026) |
| Machine text | IBM Plex Mono | Fira Code |
| Icons | Carbon (outline, 16 px), brand marks from Simple Icons; used widely, always beside a word | Nerd Font glyphs, four places only (UI-014, UI-026) |
| Corners | 4 px everywhere; straight-edged | 4, 6, 10 px |
| Accent | nord8, sparingly; frost ramp (nord10 → nord7) for the logo and ordered series | nord8 |
| Quiet text | A fourth text tier, `text-quiet` (#5C6169), for decoration only | Three tiers |
| Planned | nord15 (purple) marks what is not built yet | — |
| Motion | `ease-enter` (0.2, 0, 0, 1); content resolves from a short blur and a few px of rise; rows nudge right on hover | Springs and the same ease |
| Logo | The doors mark, frost blue to cyan, veering right | `logo.png` |
| Never | Glows, all-caps monospace, heavy shadows, rounded pills for containers | — |

## 2. Spec delta (each needs a yes or a change)

| # | Requirement | Today | Proposed | Open question |
|---|---|---|---|---|
| D1 | UI-010, UI-019, `weight-*` | Plex Sans 400 and 700 only, so OpenDyslexic keeps the hierarchy | Add `weight-light` 200 (display titles) and `weight-medium` 500 (labels, emphasis). Bundle Plex Sans Light and Medium. Under OpenDyslexic, light maps to 400 and medium to 700 | Keep 700 at all, or drop it for 500? |
| D2 | UI-011, `font-mono` | Fira Code | IBM Plex Mono for machine text; one family with the interface face | The terminal: Plex Mono too, or keep a Nerd Font mono for glyphs in shell prompts? |
| D3 | UI-014, UI-016, UI-022 | Nerd Font glyphs, one class-name table | Carbon icons as inline SVG components, brand and OS marks from Simple Icons; one name table stays (`$lib/icons`). The Nerd Font Propo files remain only for OpenDyslexic text | — |
| D4 | UI-026 | Icons in four places | Icons also lead section heads, key-value labels and detail lists. Still never in place of a word; status still pairs icon and word (UI-003) | — |
| D5 | Page title | `text-2xl` 1.5 rem, bold | New `text-display` 2.25 rem, weight light, tracking −0.03 em, for the `h1` of top-level pages only (Devices, Live sessions, Settings). Detail pages use the detail bar's title instead (D16) | Size on dense pages (Settings)? |
| D6 | Section titles | Bold, larger than body | Section head = icon + rule + small-caps label; the section's content carries the hierarchy | Is a small-caps head enough on long forms? |
| D7 | `tracking-*` | −0.01, 0, +0.01 em | Add `tracking-tight` −0.03 em (display) and `tracking-label` +0.14 em (small caps) | — |
| D8 | `radius-*` | 4, 6, 10, 999 px | `radius-sm` 2, `radius-md` 4, `radius-lg` 4; `pill` only for status dots | — |
| D9 | Philosophy 3, `elevation-1` | Surfaces, not borders: raised cards | Ground is `surface-sunken`; lists and key-value groups are hairline rows; `surface-raised` only for inputs, menus, dialogs and the session view | Devices grid: hairline tiles or keep cards? (mockup shows both) |
| D10 | Colour roles | Three text tiers | Add `text-quiet` (decoration only, never the only carrier of information, exempt from 4.5:1 like `text-tertiary` today) and `planned` (nord15) | — |
| D11 | `chart-*` | Series colours | Withdrawn: the device pages' charts and meters are already right and stay as they are (operator, 2026-11-13). The mockup's frost-ordered lines are illustration only | — |
| D12 | Motion, 5.7 | Springs, fades | Add the "resolve" entrance: opacity, 4 px rise, 6 px blur to none, `duration-base`, `ease-enter`; no blur under reduced motion. Rows nudge 6 px on hover | Page transitions too, or first load only? |
| D13 | Logo, NAV and sign-in | `logo.png`, 24 px block | The vector doors mark. Sign-in shows the particle doors mark behind the form (a still mark under reduced motion) | Particles on sign-in: yes or too much? |
| D14 | Content width, density | `content-form` 44 rem | Unchanged. Row height 44 px for list rows, 36 px for dense tables | — |
| D16 | Detail page header, §6 and every detail page | Back link, large title, badge, a row of buttons, a warning banner, then tabs | The detail bar (operator chose it for the device page, 2026-11-13, over a statement title and an identity column; then for every detail page: devices, sessions, and later projects, builds and runs). One line: the breadcrumb "Devices / name" is the title (OS mark, `text-xl` medium, not `text-display`), then the combined status as a word (DEV-006), the address in mono, and on the right one split button to connect (the default protocol, with the others and "Set up RDP" behind its arrow), Edit and More. The status's reason is one sentence under it with a Review link, not a banner. The bar and the tabs pin together while the page scrolls; once pinned, the reason folds away (no animation under reduced motion) and stays on the status as a tooltip. The Health list drops its Status row, since the bar carries it. Each detail page fills the same slots: its parent list in the breadcrumb, its own status, one mono identifier, its main action as the split button, Edit and More | — |
| D17 | Device settings tab, §7 and the device form | Five bordered accordion cards (General, SSH, RDP, VNC, Files and clipboard, plus Sessions), labels above full-width inputs, green "Credential stored" text with an empty password field, a red VNC block, Cancel and Save always at the bottom, the check defaults as a footnote | Draft, for the operator's notes. One column (46 rem) of related groups, each with a section head, plus a sticky "On this page" index on wide windows. **Device**: name, address, kind, operating system, tags (chips), MAC address, as settings rows (label and hint left, control right). **Monitoring**: metrics source as a segmented control; the check defaults become a row with "Open Checks". **Connections**: one hairline row per protocol (name, summary such as "Port 22 · okita · password", an on/off switch); a row opens to its fields. A protocol that is not set up shows "Set up" instead of a switch. A stored secret is a row ("Stored, in the hub, encrypted", Replace, Remove), not an empty field. The SSH host key and the last test result ("Passed 2 min ago · 41 ms") sit in the SSH row. The VNC warning is one inline sentence in the warning colour. **Sessions and files**: session limit as a stepper; upload, download, copy and paste as pairs of switches. Every change saves as it is made (operator, 2026-11-13): there is no Save or Cancel. Switches, segmented controls, steppers and tags save at once; text and selects save on leaving the field or Return, so a half-typed value never saves. Beside the label, "Saving…" turns into "Saved" and fades; a value the hub refuses stays unsaved, with its input outlined and one line under the fields saying why. A secret is the exception: Replace opens its own field with "Save password" and Cancel. One quiet line at the top of the tab says how saving works. The macOS Remote Login button shows only for a Mac | Does the hub need a per-field write (PATCH) for this, or does the client send the whole device each time? |
| D18 | Devices list, §6 | A label, title and subtitle, then a separate "Needs attention" list, a toolbar with a status segmented control, then the table or cards | Draft, for the operator's notes. The light display title with Add device on its right. Under it, one count line that is also the status filter: "12 devices · 1 down · 2 with a warning · 1 unknown · 8 up"; a count filters on click and again to clear. One quiet toolbar between hairlines: search, Tags, Site on the left; Sort (Status, problems first; Name; Uptime, lowest first) and List or Grid on the right. No separate attention list: sorted by status, the list has two groups, "Needs a look" and "Up". A problem row puts its reason, how long ago, and the fix ("Open checks", "Review key") where an up row shows CPU, memory and disk, and carries a 2 px status edge. Columns: status word, OS mark with name and host, 24-hour uptime, last 50 checks, meters or reason, connect as icon buttons (SSH, VNC or RDP, Wake) in a fixed-width column so every row lines up. Each meter is an icon and a word over its value and a thin bar: CPU (Carbon `chip`), Memory (`sim-card`), Disk (`vmdk-disk`, the root volume; the tooltip names how many drives) and, only for a device with a GPU, VRAM (`spyre-accelerator`). Carbon has no RAM or GPU icon; these are the closest. A meter over 85% turns its icon and value the warning colour. Below 1100 px each device takes two lines (name; status or host and uptime) with the meters or reason on the right; on phones the right column goes and the reason moves to the second line. Grid view uses hairline tiles with the same groups; a problem tile shows its reason and, if the device reports them, its meters too | Keep the status word column on wide windows, or let the edge and the group carry it? |
| D15 | Materials, 5.6 (MAT-001 to MAT-010) | Translucent chrome, panel and overlay with background blur | On hold: the mockups use opaque surfaces only. The sidebar and the pinned tab bar sit on `surface-sunken`; sign-in is `surface-raised` with a hairline; menus are `surface-overlay` with a `border-strong` edge and a shadow. Frosted glass was tried and taken out (operator, 2026-11-13); the spec's material tokens are not removed, only unused for now | Bring glass back later, and where? |

Unchanged: the Graphite and Nord palettes, the light appearance, every contrast rule (UI-002,
UI-004, UI-005, UI-024), control heights (UI-013), the 4 px grid (UI-012), tabular figures (UI-025),
HelpTip (UI-028), button tones (UI-029) and the empty, loading and error states (UI-030).

## 2a. Planned surfaces the tokens must serve

Source: operator rulings of 2026-11-13 in the Coolify integration sessions, recorded on branch
`claude/fixes-integration` (local) in `docs/extensions-plan.md` (Sections 2.8, 3.3, 4.2, 4.2.2, 4.15) and
`docs/programme/proposals/projects-and-builds.md`. Planned, not built; listed so Phase B does not add
tokens that are about to be orphaned or miss ones that are about to be needed.

| # | Change | Token consequence |
|---|---|---|
| P1 | The top-level Builds item leaves the navigation; build status moves to projects (BLD-080 is being changed) | Add no Builds-only layout tokens. The mockup still shows the Builds item; it goes when this lands |
| P2 | A "running now" area at the bottom of the navigation, above Settings: builds in progress and live sessions. Reminders also sits there; order undecided | A compact row (smaller height, tighter spacing, `text-xs` or `text-sm`, truncation), a status mark legible at that size, a "live" motion with a reduced-motion form, an empty state that takes no space. Inside the bar's material: no second blur (MAT-009) |
| P3 | One shared status set across builds, sessions, deployments, workflow runs and devices | Status roles named for the state, not the page: queued, running, succeeded, failed, degraded, unknown (today's `up`, `down`, `degraded`, `unknown` map onto it; `queued` and `running` are new). Always with a word (UI-003) |
| P4 | Weekly charts on project cards; usage charts and budget meters for the Model gateway | None new: reuse the device pages' charts and metric bars as they are (`chart-series-1` to `-6`, `chart-axis`, `chart-grid`, `chart-threshold`, and the bars' over-threshold step for "near limit") (operator, 2026-11-13) |
| P5 | Project page with tabs and dense lists of pull requests and workflow runs, each with "Synced 2 min ago · Stale" | Dense row height (D14), a stale marker that reuses `degraded` |
| P6 | Navigation groups shown only when their extension is on | The bar's length varies; the bottom group must not depend on a fixed count |

## 3. Phases once approved

| Phase | What | Gate |
|---|---|---|
| A | This document and the mockups; operator decides D1 to D14 | Operator approval |
| B | Tokens: weights, `text-display`, tracking, radius, `text-quiet`, `planned`, chart order; spec text for every approved row | `bun run gates`; design tests |
| C | Fonts and icons: bundle Plex Sans Light/Medium and Plex Mono (pinned in `fonts.lock.json`); replace the Nerd Font icon table with Carbon and Simple Icons; update every icon call site | Gates; a screenshot pass of every page |
| D | Primitives: page header (display title), section head, row list, key-value row, small-caps label, the resolve entrance | Specimen route review |
| E | Shell and pages: sidebar, sign-in, Devices, device page, Live sessions, Settings | `reviewer-interface` per page; operator look |
| F | `motion-audit`, screen-reader check, full gates | Operator approves the merge |

## 4. Risks

- **OpenDyslexic.** It has two weights. The light display title and the medium labels must still
  read as a hierarchy in it; Phase D tests every layout in both faces (UI-019 stays).
- **Hairline rows on light.** Thin lines carry the grouping; `border-subtle` must still separate
  rows in the light appearance and under `prefers-contrast: more` (`border-strong`).
- **Icon swap size.** Every icon call site changes in one lane; the name table keeps it mechanical.
- **Light display weight at small sizes.** 200 is only for the page title (2.25 rem and up).
