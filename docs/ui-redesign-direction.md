# Anima UI redesign: direction and spec delta (draft for approval)

Phase 1 of `docs/ui-redesign-plan.md`. Three parts: the philosophy every later phase follows, the
spec delta (UR-1) and the directions shown in `docs/mockup/redesign/index.html`. Nothing here is
in `design-spec.md` until the operator approves it.

Finding numbers refer to `docs/ui-critique-2026-11-11.md` (1–45, operator points O1–O11 in the
order of the "Operator's feedback" list: O1 inputs, O2 space, O3 logo, O4 collapse button,
O5 "Connected", O6 unformatted text, O7 typography, O8 back link, O9 overflow and spacing,
O10 tooltips, O11 box sizes).

> Numbering note: the plan's table uses "O8" for overflow and "O6" for unformatted text. This
> document uses the list order above; the finding table in the plan appendix maps both.

## 1. Philosophy

Anima is a console an operator glances at to answer "is anything wrong?" and then uses to get into
a machine. The interface is a quiet instrument panel: the only loud things are a problem and the
way in.

1. **Status first.** Every device shows one combined status (Section 3, DEV-006) as the strongest
   element after its name. Healthy is quiet (a small green dot and the word "Up"); a problem is
   loud (a tinted badge, the reason one click away). (3, 15, 20, 28)
2. **One accent, one primary action.** The accent marks the active place, focus and the single
   primary action of a view ("Add device", "Connect", "Save changes"). Everything else is
   secondary (neutral fill) or ghost (text only). Destructive actions are red and live apart from
   the form they affect. (5, 18, 19, 25, 29)
3. **Surfaces, not borders.** Hierarchy comes from four surface tiers (sunken, base, raised,
   overlay), not from outlines. A border appears only where two equal surfaces meet (a table's
   row separator, an input on a raised card) and becomes stronger under `prefers-contrast: more`.
   (2, 8, O1)
4. **Sans for words, mono for machines.** Interface text is a proportional sans. Monospace is used
   only for text a person would copy into a terminal: IP addresses, host names, ports, keys,
   fingerprints, commands, log messages, the terminal. Numbers in tables use tabular figures.
   (1, 4, O7)
5. **Dense but legible.** Desktop density with real targets: body text 14 px (0.875 rem) at
   minimum, controls 32 px tall (28 px in dense tables), a clear type scale where a section title
   is visibly larger and bolder than body text. (4, 6, 23, O11)
6. **Progressive disclosure.** Defaults, allowed ranges, explanations and fingerprints go into a
   HelpTip (an "i" control, click or focus to open). Advanced and rarely used groups go into a
   Disclosure. A constraint the operator needs to fill a field correctly stays visible, and an
   error is never hidden in a tip. (32, 39, 40, O6, O10)
7. **Structure over prose.** Facts are a key-value list, not a sentence. Prose is kept for one
   short line of explanation, at most. (31, 43, 44, O6)
8. **Use the screen.** Pages fill the width up to a readable maximum per content type: device
   grids and tables are fluid; forms and Settings sections are capped at about 44 rem with the
   section navigation beside them on wide windows. (14, 35, O2)

### Truncation and spacing rule (O9)

- A name may truncate only with an ellipsis and a full-text tooltip; badges and buttons never
  take a name's space below 12 characters. Machine text (IPs, keys) never truncates in the middle
  of a value: it wraps or is shortened with a copy button and the full value in a HelpTip.
- Spacing comes from three steps only inside a component (`space-1`, `space-2`, `space-3`) and
  two between components (`space-4`, `space-6`). Related things are closer than unrelated things
  (proximity); a section's gap above is larger than the gap inside it.
- A box is sized by its content: no card has a fixed height; an empty card shows an empty state
  sized to its message, not to its siblings.

### Icon policy (7)

- Icons appear in exactly four places: navigation items, status (beside the word, never instead
  of it), the leading slot of a primary or secondary button that opens something (Connect,
  Add device), and icon-only buttons in dense rows (each with a label and a tooltip, UI-015).
- Section headers, form labels and chips have no icon, except the device-type and OS icons in
  the device header.
- Icons are `icon-sm` (16 px) beside body text and `icon-md` (18 px) in navigation, and use the
  text colour of their context; only status icons take a status colour.

## 2. Combined status (finding 3)

A device has several signals: its checks (Up, Down, Degraded, Unknown/paused), its maintenance
window, its metric thresholds and security warnings (host key changed, certificate expiring).
Today the card shows two badges ("Up" and "Warning"). The redesign shows one.

**Precedence**, first match wins:

| Order | Combined status | When |
|---|---|---|
| 1 | Maintenance | The device is inside a maintenance window. |
| 2 | Down | Any enabled check is Down. |
| 3 | Warning | A security warning is open (host key or certificate changed), or a certificate or domain expires inside the shortest NOT-006 threshold. |
| 4 | Degraded | Any check is Degraded, or a metric is over its threshold. |
| 5 | Up | At least one enabled check is Up and nothing above applies. |
| 6 | Unknown | No enabled check has a result, or every check is paused. |

The badge shows the winner. Its HelpTip lists every reason that applied, in the same order
("Down: ICMP check failed 3 times · Warning: host key changed"). The precedence is a pure function
in `$lib/devices/` with a table test; StatusBadge only displays it.

## 3. Spec delta (UR-1)

Legend: **C** changed, **A** added, **W** withdrawn. "Was" quotes are shortened.

### 5.1 Colour (UR-2: themeable)

| ID | | New text |
|---|---|---|
| 5.1 table | C | Replaced by two tables: the **Nord palette** (nord0–nord15 plus derived values) and the **semantic roles** (Section 3.1 below). |
| UI-001 | C | All colours in the interface come from semantic roles. A role maps to a value of the active palette, for each appearance (dark, light). Component code never names a palette value. |
| UI-002 | C | Secondary text must reach 4.5:1 on every surface tier it is used on, in every palette and appearance. (The Nord-specific rule, "no nord3 for text", becomes a test of this.) |
| UI-004 | C | Text on an accent fill uses the `on-accent` role, which must reach 4.5:1 on `accent`. |
| UI-005 | C | Every palette must fill every role in dark and light. Status colours used as text reach 4.5:1 on the surface they sit on; used as a shape (dot, edge, bar), 3:1. Where a palette's raw colour misses, the palette derives a value by mixing toward its darkest (light appearance) or lightest (dark appearance) base until it passes, as WP-50 did for nord11–15. The operator chooses Dark, Light or System; the choice is saved on the device. The light accent is no longer nord3 (it read as grey, finding 3): it is a derived deep nord10. |
| UI-023 | A | **Theme registry.** Palettes are registered in one list (`design/tokens/palettes/`), each with a name, a licence line and both appearances. Nord is the default and, in this version, the only shipped palette. Adding a palette needs no component change (TOK-011). |
| UI-024 | A | A design test fails if a registered palette leaves a role empty or misses a contrast rule of UI-002, UI-004 or UI-005. |

#### 3.1 Semantic roles (both appearances)

| Role | Use |
|---|---|
| `surface-sunken` | Page background behind cards in a grid, the logs panel body, inputs on a raised card |
| `surface-base` | Window background, navigation bar |
| `surface-raised` | Cards, table, form sections, menus |
| `surface-overlay` | Dialogs, popovers, HelpTips |
| `surface-hover`, `surface-selected` | Row and item states |
| `border-subtle` | Where two equal surfaces meet; table row separators |
| `border-strong` | Inputs, focus-adjacent edges; all borders under `prefers-contrast: more` |
| `text`, `text-secondary`, `text-tertiary` | Primary, secondary (≥4.5:1), placeholder and disabled (≥3:1, never information) |
| `accent`, `accent-hover`, `accent-pressed`, `on-accent`, `accent-tint` | The one accent |
| `status-{up,down,degraded,warning,maintenance,unknown}` | Shape colour, ≥3:1 |
| `status-*-text`, `status-*-tint` | Badge text (≥4.5:1 on its tint) and badge fill |
| `danger`, `on-danger` | Destructive buttons (an alias of `status-down` by default) |
| `scrim`, `focus-ring` | Modal dim; 2 px focus outline |

Removed (TOK-004, add–migrate–remove): `bg`, `surface`, `surface-raised`, `border`,
`text-muted`, `accent-alt`, `accent-deep`, `info`.

### 5.2 Typography (UR-3)

| ID | | New text |
|---|---|---|
| 5.2 table | C | Interface text: **the chosen sans** (direction A: Atkinson Hyperlegible Next; B: IBM Plex Sans), Regular and Bold, OFL 1.1. Machine text and code: Fira Code (ligatures off). Terminal: OpenDyslexicM Nerd Font Mono (RA-011, unchanged). Icons: OpenDyslexic Nerd Font Propo (unchanged). |
| UI-010 | C | Use the interface sans for all interface text by default; OpenDyslexic Nerd Font Propo is the alternative (UI-020). |
| UI-011 | C | Use the monospace font only for machine text: IP addresses, host names, ports, keys, fingerprints, commands, log messages and code. Turn ligatures off (`font-variant-ligatures: none`). The SSH terminal is unchanged (RA-011). |
| UI-019 | C | Body text `text-md` (0.875 rem), dense tables `text-sm` (0.8125 rem), labels never below `text-sm`, `text-xs` (0.75 rem) only for badges and chart axes. Hierarchy must hold with weights 400 and 700 only, so OpenDyslexic keeps it. |
| UI-020 | C | "Interface font": the sans (default) or OpenDyslexic. "Monospace font": Fira Code (default) or OpenDyslexic. A saved choice of "Fira Code" for the interface font from an earlier version becomes the new default. |
| UI-021 | C | The bundled sans is chosen for legibility at small sizes and is a deliberate exception to "system font first"; the system font stays the fallback in every font token. |
| UI-025 | A | Numbers that are compared in columns or that update (uptime, metrics, counts, times in tables) use tabular figures (`font-variant-numeric: tabular-nums`). |

### 5.2 Size, space and controls

| ID | | New text |
|---|---|---|
| UI-013 | C | Target minimum unchanged (24 px desktop, 44 pt iOS). **Control height** tokens: `control-md` 32 px (default), `control-sm` 28 px (dense rows and table actions only), `control-lg` 40 px (sign-in, empty-state action). Text fields, selects, buttons and segmented controls in one row share one height. |
| UI-026 | A | The icon policy of Section 1 of this document. |
| UI-027 | A | The truncation and spacing rule of Section 1 of this document. |
| UI-028 | A | **HelpTip.** Defaults, explanations, allowed ranges and fingerprints go in a HelpTip; it opens on click or focus, stays while hovered, closes with Escape (WCAG 1.4.13), and is never placed over a live remote display (MAT-008). A constraint needed to fill a field stays visible; errors are never in a tip. |
| UI-029 | A | **Button tones.** Primary (accent fill), secondary (neutral fill), ghost (text) and danger. A view has at most one primary button. A destructive action never sits inside the form it affects; it goes to a separate "Danger zone" group at the end. |
| 5.5.3 table | C | Adds groups: palette, surface tiers, control height, `icon-sm/md/lg` = 16, 18, 20 px (were em), `content-max` (44 rem forms, 90 rem pages). Font family group: `font-ui`, `font-mono`, the OpenDyslexic pair, `font-icon`. |
| TOK-011 | C | A theme is a palette and an appearance. Changing either changes only role values; no component changes. |

### 5.3 Navigation

| ID | | New text |
|---|---|---|
| NAV-001 | C | Page items at the top: **Devices, Live sessions**. Settings is last, at the bottom, below a divider. Every item opens a page (finding 12). Labels are one line; the bar is wide enough for the longest label in both fonts (9). |
| NAV-002 | C | The active item has an `accent-tint` fill, accent text and icon, and a 3 px accent bar on its leading edge (10). |
| NAV-003 | C | The hub connection is a **status icon** above Settings (a dot plus the hub glyph), with an accessible name ("Hub connected: hub.anima.ing") and a HelpTip with the hub name, state and the time of the last answer. "Disconnected" also shows a banner over the page (O5, 11). |
| NAV-006 | C | The collapse control sits in the navigation bar's header row, at the trailing end of the logo row, not beside the window buttons (O4). |
| NAV-008 | A | The **Live sessions** page lists every live session (device, protocol, started, duration) with "Show" and "End session" (RA-034). With none it shows an empty state that points to the Devices page. |
| 5.3 table | C | Adds row "Top · Live sessions · Monitor-play · Live sessions page". The logo block is 24 px high (O3, UR-6). |

### 5.7.4 Logs panel

| ID | | New text |
|---|---|---|
| 5.7.4 row "Resize the logs panel" | C | Snap points: collapsed, **30 %** and **60 %** of the page. The first open goes to 30 %; later opens go to the last open stop. Severities show as an icon plus word in a fixed-width column (21). |

### 6.1 Device list

| ID | | New text |
|---|---|---|
| 6.1 card list | C | A card shows: name, host (mono), the combined status, uptime 24 h, metric bars (6 px, labelled with value) when available, the heartbeat strip **with a label and a HelpTip** ("Last 50 checks"), and Connect buttons for the configured protocols. (14–18) |
| DEV-006 | A | **Combined status.** A device shows one status chosen by the precedence of Section 2 of this document; every reason is available in the status HelpTip. |
| DEV-007 | A | The grid fills the width with cards of a minimum of 18 rem; the table fits its columns to 1280 px without a horizontal scrollbar, and the heartbeat column shrinks before any text column (20). |
| DEV-008 | A | The toolbar is one row on wide windows: search, status filter (segmented), tag filter, view switch, and "Add device" (primary) at the trailing end (19). |

### 6.7 Device detail

| ID | | New text |
|---|---|---|
| DEV-020 | C | Header: a back link row ("Devices" with a chevron, a ghost link, O8); a title row with the device-type icon, name, combined status, and at the trailing end **Connect** buttons for each configured protocol (SSH, RDP, VNC, Web; the most used one primary) and an overflow menu (Edit, Set up a protocol, Maintenance); a meta row of host and IP (mono, copyable), type and OS as plain text, and tags as chips. In a narrow window the buttons move to a row of their own. Tabs are underlined tabs, not buttons (30). |
| DEV-021 | C | Unchanged in meaning; the tooltip on the name is a native title, never overlapping a button (28). |
| DEV-022 | C | A protocol that is not set up has **no** header button. "Set up SSH/RDP/VNC" is an item in the overflow menu and opens the Settings tab at that section (29). |
| DEV-023 | A | The device page shows a loading skeleton for the header and the first tab while its data loads, and an error state with "Try again" if the load fails (27). |
| DEV-024 | A | The SSH host key fingerprint moves from the Overview into the HelpTip of a "Host key: trusted" row (32). |

### 7. Settings

| ID | | New text |
|---|---|---|
| SET-002 | W | Withdrawn (describes the Phase 1 page). Replaced by SET-004. |
| SET-004 | A | Settings has a **section list** beside the content on windows of `bp-wide` and above, and a section jump menu below it. Sections: Account, Appearance, Hub, Notifications, Checks, Data retention, Updates, API keys, VPN. Each section has an anchor (35, 36). |
| SET-005 | A | **One save model.** A section whose fields belong together has one "Save changes" button at its end, enabled only when something changed; a single toggle or choice saves at once and shows "Saved" inline. Every result (saved, failed, test passed) shows in a Notice inside the section (37, 41, 42). |
| SET-006 | A | The Updates section shows the version, the update state and "Check for updates". Packaging and signing details (Tauri packages, updater key, build host) are in a "Details for developers" Disclosure, closed by default (40). |
| SET-003 | C | VPN section: sites as rows of name, route (mono) and router state in words ("Router: none set up"); nodes as rows of name, role and address with expiry as a status ("Expires in 3 days" warning, "Never expires" plain). The "not available" message becomes: "VPN status is not available. The hub cannot reach the VPN service." with the technical cause in a HelpTip (43, 44). |

### 5.6 Materials

| ID | | New text |
|---|---|---|
| `elevation-1` row | C | Cards: surface contrast (`surface-raised` on `surface-sunken`), **no border**, no shadow; a `border-strong` outline under `prefers-contrast: more`. |
| MAT-003 | C | Adds: a modal is centred, with `elevation-3` and the scrim at 50 % (dark) / 35 % (light) so it reads as above the page (26, 45). |

## 4. The directions

All three use the philosophy above; they differ in palette, typeface and density. Open
`docs/mockup/redesign/index.html` and use the switcher (direction, appearance, composition).

| | A · Nord, layered | B · Graphite, one Nord accent | C · Nord, dense |
|---|---|---|---|
| Palette | Nord with derived surface tiers (sunken below nord0, raised nord1, overlay nord2) | Neutral greys (cool-neutral, not blue) with nord8 as the only colour besides status | As A |
| Sans | Atkinson Hyperlegible Next | IBM Plex Sans | Atkinson Hyperlegible Next |
| Mono | Fira Code | IBM Plex Mono | Fira Code |
| Density | Controls 32 px, cards 18 rem min | Controls 32 px, cards 18 rem min | Controls 28 px, table-first Devices, cards 15 rem min |
| Character | Calm, cool, unmistakably Nord; the accent reads as "frost" on ink | Quieter, more neutral; the frost accent and status colour carry all colour | An operations console for many devices |

Lead's recommendation: **A**. It keeps Anima's identity (Nord is what the operator chose), fixes the
flatness by surface tiers rather than by abandoning the palette, and Atkinson Hyperlegible Next was
drawn for low-vision legibility, which fits an app that already offers OpenDyslexic. C's density
can be a later "Compact" option on top of A rather than a separate direction.

Font licences are to be verified in Phase 2 before any subsetting (reserved font names: IBM Plex
reserves "Plex"; Atkinson Hyperlegible Next to be checked).
