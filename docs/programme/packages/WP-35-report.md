# WP-35 report — one-button screen sharing

**Status: done.** Both tasks are accepted and merged into `phase-2` (45ff62ba) under R-067: T1 at 2e711b19, then
T2 at 7608ab7b. The plan, decisions and full review log are in [WP-35.md](WP-35.md).

## What was built (R-079 as amended, R-080)

- **One button.** The device header and the card and table actions show a set-up button on any macOS or Linux
  device whose VNC is off (RA-030, DEV-022).
  - In a header wider than the 56rem container query it reads "Set up screen sharing".
  - Everywhere else it is a dashed "+ VNC". Its accessible name is "Set up screen sharing for <name>", with the
    same text as a tooltip (D-6).
- **The flow** (`apps/client/src/lib/devices/screen-sharing/`):
  1. It tests the device with `testConnection`: host, `protocol: VNC`, port 5900, and no credential (DEV-013, D-1).
  2. If the device answers, the flow turns VNC on at port 5900 and stores no password. The username is this
     computer's OS user when the device is this computer (R-060 rule); otherwise any stored username is kept. Then
     it opens the VNC session. The gateway asks for the password through the existing credential prompt, which
     stores it only on "Save" (GW-015, DEV-011).
  3. If this Mac does not answer and the flow runs in the desktop app, it opens System Settings → Sharing. It shows
     "Turn on Screen Sharing" as static text and tests again every 3 seconds. Cancel or Escape stops the repeats.
     When the device answers, the flow carries on from step 2.
  4. Otherwise it shows the R-079 text: no answer on port 5900, either because Screen Sharing or the VNC server is
     off, or because the VPN rule does not allow port 5900. The app never changes VPN rules.
- **Shell command rename (T2, D-3).** `remote_login_settings_open` is now `sharing_settings_open`.
  - Permission: `allow-sharing-settings-open`.
  - Client functions: `openSharingSettings` and `sharingSettingsAvailable`.
  - The "unavailable" text: "Sharing settings cannot be opened on this computer."
  - Behaviour is unchanged: one fixed pane, `/usr/bin/open`, no URL from the web view (R-059).
- **Motion and tokens.** The status panel uses the popover role and token values only. Under reduced motion it
  appears in place. Nothing repeats (MOT-011, D-7). The panel is clamped 8 px inside the window in WebKit and
  Chromium.

## Tasks

| Task | SHA | Base | Gates | Review rounds |
| --- | --- | --- | --- | --- |
| T1 `setup-flow` | 2e711b19 | phase-2 69eaa66b | 4–12 pass (234 files, 3003 tests); 1–3 not run, no Rust (R-068) | 3 (R-066 + R-080): reviewer accept; reviewer-interface accept in round 3 |
| T2 `sharing-command` | 7608ab7b | T1 2e711b19 | 12/12 pass | 2: reviewer accept; round 2 a checked mechanical fix |

Final package review: accept, no musts.

## Contract change requests

None. There was no hub, schema or contract change. The Devices query gained `os` and `vnc.username`, which the
schema already serves.

## Handoff notes

- `RemoteLoginButton.svelte` keeps its Remote Login name and its button text. It serves SSH (R-059), so the name
  is still true.
- `lib/devices/actions/placement.ts` `placeNear` now measures `offsetWidth` and `offsetHeight` and resets the start
  edge before measuring. WebMenu shares it and was re-measured.
- The spec gains R-079 with R-058 after Phase 2, as the ruling says.

## Open risks and coulds for issue #1

- **CLS when the strip appears.** A CLS of 0.0197 (Chromium) when the session tab strip appears about 1.4 s after
  the click. The strip insert belongs to the session shell; a late plain VNC open does the same (MOT-007).
- **Double placement.** The status panel is placed twice on mount, by the effect and by the attach (Idiomatic).
- **Exit during "testing".** There is no visible Cancel while testing; Escape works from the button.
- **Button width.** The "+ VNC" set-up button is 57 px against 56 px for the VNC button.
- **Shared `normalizeHost`.** `normalizeHost` is imported from `forms/device/host` into the devices domain; a
  shared hosts module would suit it better (Domain-based).
- **Not tested live.** No live check was run on a real Mac with Screen Sharing or on a Linux VNC server. The
  operator should try the flow on this Mac (Screen Sharing off, then on) and on one Linux device.
- **Escape while "starting".** Escape is ignored while "starting", because the update cannot be undone. A pointer
  user waits for the one hub call to finish.
