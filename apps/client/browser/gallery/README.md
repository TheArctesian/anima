# Screenshot gallery

A WebKit run (MV-001) over the synthetic hub that photographs every screen and state it can produce, in two themes and three
widths, so a redesign can be compared against what ships today.

## Run

```sh
bun run --cwd apps/client build      # the hub serves apps/client/build/
bun run --cwd apps/client gallery    # about 10 minutes for all 8 passes
bun run --cwd apps/client gallery --grep "dark-1280 "   # one pass
```

If the engine is missing: `bunx playwright install webkit` inside `apps/client`.

Pictures go to `apps/client/gallery-shots/<theme>-<width>[-dyslexic]/<NN>-<screen>.png` (git-ignored). Passes: dark and light at
800, 1280 and 1600 wide (900 high), and one OpenDyslexic pass (UI-019) at 1280 dark. The theme and typeface are the stored
settings (`anima.theme`, `anima.typeface.*`, and `anima.palette` when `GALLERY_PALETTE=nord` is set) plus the matching `prefers-color-scheme`. Motion is reduced. Pages that scroll are
shot full length; overlays, popovers and the logs panel (fixed to the window) are shot as the window; Settings sections and tall
dialogs are shot as the element.

The gallery uses its own config (`gallery.config.ts`) and files named `*.gallery.ts`, so `test:browsers` never runs it. It uses
the same deny-all proxy as the browser tests; nothing reaches a network.

## Layout

- `shots.ts`: the passes and the `Screen` type. `screens.ts` joins the lists; `screens-*.ts` hold the screens by area.
- `gallery.gallery.ts`: the one runner. Add a screen by adding an entry to a list; the numbers after it move.
- `steps.ts`: shared navigation and the helpers that hold or fail a GraphQL operation.
- The synthetic hub gained `synthetic/vpn.ts` (a VPN picture and the broker answers) and a `fixtures` option for empty states.

## Screens

| No. | Screen |
|---|---|
| 01-02 | Sign-in; sign-in refused |
| 03-08 | Devices grid (window, scrolled to the end), table, collapsed navigation bar, search, search with no match |
| 09-16 | Filter menus (Status, Tags, Site, Sort) open, then one choice applied for each |
| 17-19 | Logs panel collapsed, Severity filter open, Live sessions popover |
| 20-27 | Add device: empty, invalid, then General and SSH collapsed, then RDP, VNC, Sessions and Files and clipboard open |
| 28-32 | Device overview (gpu-box with metrics, 24 hour range, device without metrics, device with a warning, a Mac) |
| 33-36 | Device Checks tab, check history open, Add check, Edit check |
| 37-41 | Device Sessions, Logs and Settings tabs, Edit, device not found |
| 42-51 | Settings: whole page, each section (Account, Appearance, Notifications, Default check settings, Data retention, API keys, VPN), scrolled to the end, scrolled to Notifications |
| 52-59 | Settings dialogs: Add channel, Edit channel (webhook, email, ntfy, Apprise), delete and revoke confirms, invalid retention value |
| 60-68 | VPN: Add a computer (choice, Linux key, Mac registration, registered), Add a router (choice, key), Approve route confirm, status error, loading |
| 69-79 | Sessions: SSH terminal, connecting, files panel, ended, host key modal; VNC; RDP; credential modals (RDP, VNC); certificate modal; two tabs |
| 80-84 | Empty: no devices, no checks, no channels, no API keys, no log events |
| 85-87 | Loading: devices, device, settings (the hub holds the answer) |
| 88-92 | Errors: hub error on devices, device and settings; hub unreachable; socket offline |
| 93 | Sign-in with the hub unreachable |

## Not covered

- **Updates section** and the log folder button: desktop only (UPD-001, CLI-001); the web client the gallery drives never shows them.
- **Files panel with a listing**, upload and download: the synthetic hub does not serve the `/files` tunnel, so the panel shows its
  "file connection could not be opened" state (picture 71). A listing needs a new tunnel fixture.
- **Session recording player and the pop-out window**: no synthetic recording and no pop-out in the web client.
- **Web interface tabs** (Web button: tab, trust page, down page): the web client opens a system tab; a window the gallery cannot photograph.
- **Create API key result and its one-time secret**, **Send test result**: the synthetic hub answers these mutations with empty values.
- **Add device after a successful save and the credential error notice**: the synthetic hub does not store new devices.
- **Wake relay and Wake-on-LAN progress**, **metrics push agent enrolment**: no fixtures.
- **Update banner**: desktop only.
- **Renew key confirmation**: the broker answer is a placeholder, so the dialog shows nothing the picture 49 does not.
