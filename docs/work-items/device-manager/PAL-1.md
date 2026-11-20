# PAL-1: The command palette and its field "Search or jump to" in the navigation bar

Progress: the index [README.md](../README.md) holds the state of this item (PLN-006). The plan register in [work-plan.md](../../work-plan.md) Section 9 lists the index.

| Item | Value |
| --- | --- |
| Identifier | PAL-1 |
| Kind | Work item |
| Owner | [design-spec.md](../../spec/design-spec.md) Section 5.3 (NAV-011) |
| Serves | Work plan Section 4, tier 0 (finish the device manager) |
| Version | 1.0 |
| Date | 2026-11-19 |
| Origin | The operator, 2026-11-19 (R-224: "A: all, with the jump field"), and R-115, which took NAV-011 |

## Reason

The interface review of 2026-11-19 drew a field "Search or jump to" at the top of the navigation bar, and the operator approved it with the upgraded screens (R-224). The function behind the field is not new: NAV-011 already requires a command palette that opens with ⌘K or Ctrl+K (R-115). No part of it is built: the client has no palette and no shortcut. This item builds NAV-011 and adds the field that R-224 approved.

## Decided

NAV-011 (R-115), with no change:

- The palette opens with ⌘K on macOS and with Ctrl+K on Linux and the web.
- It finds devices by name, host and IP address, the pages, and the sections of Settings.
- For a found device it offers "Connect" with each protocol that is set up, "Wake" and "Open".
- It works with the keyboard only. It is an ARIA combobox with a list box.
- In a session tab the shortcut goes to the remote device, not to the palette.

R-224:

- The navigation bar has a field "Search or jump to" above the page items. The field shows the shortcut.

## Open decisions

The ruling R-224 approves the field. It does not give the words of these four points. Each has a recommendation; none is built until the operator answers.

| Decision | Question | Recommendation |
| --- | --- | --- |
| OD-PAL1 | What does the field do? | It is a button that opens the palette of NAV-011. It is not a second search: one place searches. |
| OD-PAL2 | What does the palette show when nothing is found? | One line, "Nothing found for "<text>"", and no other row. The text that was typed stays. Escape closes the palette. |
| OD-PAL3 | What does the palette show before a letter is typed? | The pages, then the devices that need a look, at most 8 rows. |
| OD-PAL4 | What is there on a phone, and in the bar that is collapsed to icons? | On a phone: no field and no shortcut; the Devices page keeps its own search. In the collapsed bar: one icon button with the same accessible name. |

If the operator accepts the four recommendations, the design specification gets this requirement, with the number of that ruling:

> **NAV-015.** The navigation bar must show a control "Search or jump to" above the page items, with the shortcut of NAV-011 beside its words. The control opens the command palette of NAV-011 and does not search by itself. In the bar that is collapsed to icons the control is one icon button with the same accessible name. The phone layout has no such control. Before a letter is typed, the palette shows the pages and then the devices that need a look, at most 8 rows. When nothing is found, the palette shows one line that says so with the typed text, and keeps the text.

## Tasks

| Task | Content | State | Commits |
| --- | --- | --- | --- |
| T1 | The operator answers OD-PAL1 to OD-PAL4; NAV-015 goes into the design specification with its ruling | Not started. Waits for the operator | |
| T2 | The mockup: the palette open, with results, with no result and before a letter, in the dark and the light appearance, in `docs/mockup` with its row (PLN-019) | Not started | |
| T3 | The pure search: devices by name, host and IP address, the pages, the sections of Settings; the order of the results; tests | Not started | |
| T4 | The palette: the combobox and its list box, the shortcut, the actions of a device (Connect, Wake, Open), no shortcut in a session tab; tests for the keyboard | Not started | |
| T5 | The field in the navigation bar and its icon form; the comparison with the mockup; the entry in `docs/need-human-testing.md` | Not started | |

The hub needs no change: the palette searches the devices that the client already has. A command of the `anima` program and an MCP tool do not apply: the palette is a control of the graphical client, and `anima devices list` already finds a device by name (CMD-030).
