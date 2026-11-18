# Anima UI critique — 2026-11-11

Critique only; solutions come later. Taken from the running desktop build (narrow window, light
theme) by tapping through every page: Devices (grid, table, Add device), a device page (Overview,
Checks, Sessions, Logs, Settings), Live sessions, the Logs drawer, Settings (every section) and
the VPN "Add a computer" dialog.

## Whole app

1. **Monospace everywhere.** Fira Code is the interface font: headings, labels, buttons, prose and
   numbers all look the same. There is no typographic hierarchy, long help text is tiring to
   read, and the app reads as a terminal rather than a product.
2. **One flat colour.** Background, sidebar, cards, inputs and dialogs are all near the same pale
   blue-grey with thin blue-grey borders. Surfaces do not separate; nothing draws the eye.
3. **Status colour is weak and confusing.** "Up" is a grey pill, "Warning" a washed-out pink. A
   device shows **Up and Warning at once** with no explanation of what is wrong. Healthy and
   unhealthy look almost the same at a glance.
4. **Type scale too small and too even.** Most text is ~11–12 px; section titles are barely
   larger than body text; helper text is tiny grey. Hard to scan, hard to read.
5. **Buttons have no hierarchy.** Nearly every button is the same small outlined grey box
   (Sign out, Test connection, Add channel, Save, Copy, SSH, VNC). The one filled button (dark
   "Save") appears only sometimes. Primary actions do not stand out from secondary or
   destructive ones.
6. **Controls are cramped.** Buttons and fields are ~19–25 px tall with tight padding; click
   targets are small for a desktop app.
7. **Icons are inconsistent and tiny.** Some sections have an icon, some buttons have one,
   most don't; icons sit at text size and add noise more than meaning.
8. **Borders on everything.** Every card, row, field and pill has a border; combined with the flat
   colour it looks busy and wireframe-like rather than finished.

## Operator's feedback (2026-11-11)

Agrees with the findings above, and adds:

- **Input boxes look monotonous**: every field is the same pale box with the same border.
- **Poor use of screen space** throughout.
- **The logo is too big** for the sidebar.
- **The collapse-sidebar button feels out of place** (floating beside the window controls).
- **"Connected" is useless as text**; at most it should be an icon.
- **Lots of unformatted text**: plain runs of prose where structure is needed.
- **Weird typography everywhere.**
- **The "‹ Devices" back link looks strange.**
- **Text overflow, odd gaps and unclear spacing.**
- **Too much information on screen** that could live in an info tooltip instead (help text,
  allowed ranges, defaults, fingerprints).
- **Boxes are sized wrongly for their content**: some too big (empty cards), some too small
  (cramped fields and buttons).

## Sidebar

9. "Live sessions" wraps onto two lines in a sidebar that is mostly empty.
10. The selected item is marked only by a faint box; the active page is hard to see.
11. "Connected" floats at the bottom as plain text, with no status dot or hub name.
12. **Live sessions does not open a page**: clicking it shows a small "No live sessions."
    popover over the content. Navigation items should not behave differently from each other.
13. The logo block and app name are small and visually disconnected from the nav.

## Devices list

14. **Grid cards waste the screen.** Two small cards at the top, ~80% of the window empty
    background.
15. **Device names are truncated** ("Stephen's Mac…") while the badges take the space.
16. **Metric bars are 2 px hairlines**: CPU / Memory / Disk / VRAM levels are hard to read; the
    warning triangle next to a label is tiny.
17. **An unlabelled striped bar** (heartbeats) is the widest element on the card and explains
    nothing.
18. SSH / VNC buttons look small, grey and disabled; the main reason to open the app is hidden.
19. **Toolbar is two disjoint rows** (search + 4 filter buttons, then Grid/Table + Add device),
    all the same weight; "Add device" does not read as the primary action.
20. **Table view**: columns cramped, Status shows Up and Warning side by side, a horizontal
    scrollbar appears with only two rows, and the heartbeat column is cut off.
21. **Logs drawer** opens over half the window, pushing the device list into a small strip; its
    own table is dense, and severities (Info / Warning / Error) are only coloured words.

## Add device / Edit device (dialog and device Settings tab)

22. A long single-column form of collapsible sections (General, SSH, RDP, VNC, Sessions, Files…)
    with no indication of which ones need attention or are complete.
23. Labels are tiny grey text above fields; help text is even smaller; the Name and Host
    placeholders look like filled values.
24. Mixed controls in one row (text field next to a dropdown with an icon) misalign.
25. Destructive "Delete device" sits at the bottom of the same scrolling form as everything else.
26. The dialog does not dim or separate strongly from the page behind; it reads as another card.

## Device page

27. **Initial load showed a blank page** (only "‹ Devices") with no loading state; a second visit
    worked. No skeleton, spinner or error.
28. **Header is crowded**: four tiny buttons (SSH, RDP, VNC, Edit) plus three tiny chips (IP, kind,
    OS) plus Up / Warning badges, all the same size. A hover tooltip ("Up") overlapped the Edit
    button.
29. "RDP" appears as "+ RDP" (set up) styled like the working SSH/VNC buttons.
30. **Tabs look like buttons, not tabs**; the active tab is not clearly marked.
31. Overview cards are unequal and half empty (Status card: two rows; Checks: a lone "1"; Metrics:
    "Waiting for metrics" in a large box). Uptime numbers are the only strong element.
32. SSH host key fingerprint is shown in full in the overview — technical detail promoted above
    useful information.
33. **Checks tab**: a single thin row with tiny Pause / edit / delete icons; large empty page.
34. **Sessions and Logs tabs**: long, dense, unstyled tables; every row the same weight; Error /
    Warning only as coloured text; long messages wrap into ragged rows; no grouping by day.

## Settings

35. **One very long page** (Account, Hub, Appearance, Notifications, Default check settings, Data
    retention, Updates, API keys, VPN) with no side navigation or anchors.
36. Section headers are small icon + bold text; sections blur into each other.
37. **Many separate Save buttons** (template, check defaults, and one per retention row) with
    different styles; it is unclear what is saved and when.
38. Notifications card has a large empty area under "No channels".
39. Data retention rows repeat "Keeps 30 days / 1 to 400 days, default 30" twice per row.
40. **Updates section exposes developer internals** to an operator: missing Tauri packages,
    updater key, "Key file password", "No computer builds releases".
41. "The last update attempt failed" appears as plain grey text, not as a problem.
42. Hub address card has unexplained empty space; "Test connection" gives no visible place for
    its result.

## VPN (Settings section and dialog)

43. The site row "VPN 100.64.0.0/10 · No router · Not known" is cryptic for a first-time user.
44. Node rows mix role, address and expiry in small grey text; the hub row "Does not expire" looks
    the same as a warning would.
45. "Add a computer to the VPN" dialog: two plain choice buttons and Close, at the top of the
    window rather than centred; the explanation text is dense; no visual distinction between the
    two paths.

## Addendum: dark theme and wide windows (Phase 0 gallery, 2026-11-11)

Taken from the Phase 0 gallery (`apps/client/browser/gallery/`, WebKit, synthetic hub) at 1280 and
1600 px, dark first, then light. The bracket at the end of each finding lists gallery picture
numbers (`browser/gallery/README.md`); "(26)" or "(O11)" inside the text names a related earlier
finding.

### Whole app

46. **No page titles.** Devices, Live sessions and the device page have no heading; the page
    starts with the toolbar, and every document title is "untitled page" (screen readers announce
    nothing useful). [pictures 3, 28]
47. **The navigation bar and the page are one colour in dark.** Only a hairline separates them;
    with finding 2 the window reads as a single flat sheet. [pictures 3]
48. **Wide windows leave whole columns empty.** At 1600 px Settings is a narrow column at the left
    with about 55 % of the window blank, and the sign-in form sits in the top-left corner of an
    empty page. [pictures 42, 1]
49. **Navigation is shown before sign-in.** The sign-in page shows Devices, Live sessions (with a
    count of 0) and "Connected"; none of them can be used yet, and the form has no product
    identity or primary button. [pictures 1]
50. **Error states are one line of text.** A hub error is a sentence and "Retry" at the top-left of
    an empty page, with no icon, no cause in plain words and no next step; meanwhile the logs panel
    below still shows rows. [pictures 88–90]

### Devices

51. **The logs panel is not a layer.** It has no shadow, edge or blur in dark, and in light it is
    translucent enough that cards show through it as ghost text; the cards under it cannot be
    scrolled into view. [pictures 3, 17]
52. **Cards stretch to the tallest card in their row.** A card with "No metrics" is as tall as a card
    with three metric bars, so most cards are half empty space (O11). [pictures 3]
53. **Two Wake buttons styles.** "Wake" is a filled grey button beside outlined SSH/VNC/Web buttons,
    so it reads as disabled while being the only action that works for a sleeping device. [pictures 3]
54. **Uptime with two decimals and a spaced percent sign** ("100.00 %") on every card and row;
    the decimals carry no information for most devices and widen every column. [pictures 3, 5, 28]
55. **The table's Status column is wider than Name** and its badges float in it; at 1600 px a
    third of the row is empty between Uptime and Heartbeats. [pictures 5]

### Device page

56. **Metric labels sit far from their values.** At 1600 px "CPU" is at the left of the card and
    "19 %" about 400 px to the right; the CPU chart fills only a third of its card. [pictures 28]
57. **"+ RDP" and "+ Set up screen sharing" are full-width text buttons** in the header,
    longer than the working SSH button, and pull the eye away from it. [picture 28; finding 29]

### Dialogs

58. **The scrim is too light to separate the dialog** in dark: the cards behind the Add device
    dialog stay readable, and the dialog surface is the same colour as a card (finding 26). Fields inside
    the dialog are barely darker than the dialog. [pictures 20]
59. **Dialog content is cut at the bottom edge** with no fade or shadow showing that it scrolls (the
    "VNC" section header is sliced in half above Cancel/Save). [pictures 20]
60. **"Add a computer to the VPN" opens at the top of the window**, not centred, with its two
    paths as plain buttons and "Close" alone at the bottom right. [picture 60; finding 45]

### VPN

61. **The "Approve route" message wraps one word per line** ("router-annex / offers /
    10.50.0.0/24 / and waits / for / approval."), and the row still says "No router" while it lists
    a waiting router. [picture 49; finding 43]
62. **Sites and nodes are one undivided list**: no heading separates "Sites" from "Computers", and
    roles ("hub", "client", "router") are bare lower-case words beside the address. [picture 49; finding 44]

### Sign-in (operator, 2026-11-12, desktop app)

63. **The sign-in page needs a redesign.** The form sits top-left beside the full navigation bar
    (Devices, Live sessions 0, "Connected", Settings); "Sign in" is an outlined button like every
    other; and the desktop hub address is a second form with its own full-width "Save address"
    button, so the page has two equal actions and no product identity. [operator's screenshot;
    findings 49, 5]
