# Anima Mobile Plan

Progress: the plan register in [work-plan.md](work-plan.md) Section 9 holds the state of this plan (PLN-005).

| Item | Value |
| --- | --- |
| Document | Mobile plan: the iOS client and the Android client |
| Version | 0.3 |
| Date | 2026-11-16 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | Approved. The operator decided OD-M1 to OD-M8 on 2026-11-16 (Section 3). Version 0.2 writes Section 4.4 again for the ruling on OD-M3. MOB-032 waits for the test of task M1-4. Version 0.3: the operator approved the phone mockups on 2026-11-16 ("Mock are good enough now as well"); Section 4.2 follows them. |
| Parent | [work-plan.md](work-plan.md) Section 4, tier 4, row "iOS client and Android client (Phase 5)" (PLN-003) |
| Base | `main` at `fa4ac394`. The facts in Section 2 are from the branch `spike/mobile`, which is local and is not for merge. |
| Related | [design-spec.md](design-spec.md) Sections 1.3, 1.4, 5.3, 5.7, 9 and 10; [vpn-plan.md](vpn-plan.md); [ui-redesign-direction.md](ui-redesign-direction.md) UI-013 |

This document obeys the writing rules in Section 0 of the design specification.

## 1. Purpose and scope

### 1.1 Purpose

This plan makes the Anima client work on a phone. It uses the existing client and the existing Tauri shell. It adds no function to the hub.

The operator gave the instruction on 2026-11-16: "I want to talk about the android and ios version and eventually start building them". The operator then gave the order of the work: "lets worry about just getting the app to work first".

### 1.2 In scope

- A Tauri shell that builds for iOS and for Android from one crate.
- A phone layout of the client.
- Sign-in, the device list, the device page, checks, the event log and Settings on a phone.
- SSH, VNC and RDP sessions on a phone (RA-050).
- The device link inside the client, so that a phone gets to its hub with no other application (OD-M3).

### 1.3 Not in scope

Section 6 lists the items that this plan does not build.

## 2. Facts from the test build

A test build ran on 2026-11-16 on the branch `spike/mobile`. It used a hub on the same computer, with a test account and four sample devices. These facts are measured, not estimated.

### 2.1 Tools

| Item | Value |
| --- | --- |
| Tauri command line | 2.12.4 |
| iOS | Xcode 27.2. iPhone 17 Pro simulator, iOS 26.5. CocoaPods 1.17.0. |
| Android | SDK platform 36, NDK 28.2.13676358, JDK 21. Emulator with the Android 36 arm64 image. |
| Rust | The `rustup` toolchain 1.99.0. The Homebrew `rustc` has no mobile targets. |

### 2.2 What worked

| Number | Fact |
| --- | --- |
| F1 | All dependencies of the shell compile for iOS and for Android, `anima-cli` and the TLS code included. |
| F2 | The shell has 16 compile errors on iOS. All are in three desktop areas: `web_view/`, `stay_awake/tray.rs` and `local_hub/account.rs`. |
| F3 | A mobile entry point of about 90 lines, with the three access token commands, starts the client on both platforms. |
| F4 | The capability files `desktop.json` and `builder.json` already apply to macOS and Linux only. They needed no change. |
| F5 | Sign-in, the second-factor step and "Sign in without a code" work on both platforms. |
| F6 | On Android the device list, the device page and Settings fit the width of a phone without a layout change. |
| F7 | The hub accepts the origin of the Android web view, `http://tauri.localhost`. It is already in `CLIENT_ORIGINS`. |
| F22 | The device link builds as a library for a phone (2026-11-16, Go 1.27.4, the build tags of `link/build.sh`). A wrapper with one exported function built as a static library for the iPhone simulator and for a real iPhone (19.2 MB each) and as a shared library for Android arm64 (16.4 MB). Only the test package `tunnel/testbed` does not build for a phone. |

### 2.3 What failed

| Number | Platform | Fact |
| --- | --- | --- |
| F8 | Both | `desktopCapabilities()` answers yes to each feature inside Tauri. The client then offers builds, the local hub and stay awake on a phone. |
| F9 | Both | The client calls shell commands that the mobile shell does not have: the process log and `hub_listener_check`. |
| F10 | Both | The navigation bar is on the left side and takes about one sixth of the width. |
| F11 | Both | The logs panel takes the lower 40 % of the Devices page. Four device rows stay visible. |
| F12 | Both | A long device name is cut with no ellipsis. The tabs of the device page go off the right edge. |
| F13 | Both | The first run and Settings say "this computer". |
| F14 | iOS | The key store read fails in a build that has no signature. Sign-in then stops with "The key store could not be read". |
| F15 | iOS | The page zooms when a text field gets the focus, and it stays zoomed. The Devices page is then wider than the screen. |
| F16 | iOS | The area of the status bar is white. After a scroll, the content is below the clock. |
| F17 | iOS | The user name field starts with a capital letter. The hub address field shows word suggestions. |
| F18 | iOS | The "Can't reach" notice moves the sign-in controls down when it shows. |
| F19 | Android | The `keyring` crate has no Android store. |
| F20 | iOS | `tauri ios build` stops because the simulator runtime 27.2 is not installed. A build with `cargo` and `xcodebuild` works. |
| F21 | Android | Gradle fails with JDK 27. It works with JDK 21. |

### 2.4 Not tested

- An SSH, VNC or RDP session on a phone.
- A real phone. All tests used a simulator or an emulator.
- A hub on the VPN. The test hub was on the same computer.
- The key store on a build with a signature.
- The device link at run time inside an application. It builds (F22). No test has joined a hub from a phone or sent a request through it.

## 3. Decisions for the operator

The operator decided each row on 2026-11-16.

| Number | Decision | Ruling |
| --- | --- | --- |
| OD-M1 | Is the Android client in scope? The design specification Section 1.4 put the Android package out of scope. | **Decided: yes.** One shell and one client serve both platforms (F1, F3, F6). Section 7 lists the changes to the specification. |
| OD-M2 | When does this work start? The work plan put the iOS client at tier 4. | **Decided: start now.** The operator gave the order: "lets worry about just getting the app to work first". |
| OD-M3 | How does a phone get to the hub? | **Decided: with no second application.** The operator's words: "I don't want the user to have to install tailscale ontop of the app remember one click install princible it should be as easy as possible". The device link (`anima-link`) is thus inside the client (Section 4.4). Version 0.1 recommended the Tailscale application; that recommendation is withdrawn. **Order of the work (operator, 2026-11-16): "for the tailscale thing we should just use the hub an my actual account for now".** The device link inside the client stays the decision; stage M3 waits. Until then a phone uses a hub address and the operator signs in to the real hub. |
| OD-M4 | One shell crate, or a second crate for mobile? | **Decided: the existing crate,** `apps/desktop/src-tauri`. |
| OD-M5 | The identifier of the two applications | **Decided: `ing.anima.mobile`** for iOS and for Android. An identifier cannot change after a release. |
| OD-M6 | The content of the first version | **Decided: Section 1.2.** IOS-001 to IOS-004 come later (Section 6). |
| OD-M7 | Where the access token is kept during development | **Decided: in memory.** The operator's words: "key not being read is fine for later". MOB-006 gives the rule for a release. |
| OD-M8 | How the operator gets the applications | **Decided.** iOS: TestFlight. Android: an APK from the release server first, a store later. |

## 4. Requirements

The operator approved these requirements on 2026-11-16, with the limit that the Status line gives for MOB-032.

### 4.1 Shell

- **MOB-001.** One crate, `apps/desktop/src-tauri`, must build the shell for macOS, Linux, iOS and Android.
- **MOB-002.** The mobile shell must register only the commands that a phone can serve. In the first version these are the three access token commands and the link commands (`link_join`, `link_status`, `link_leave`).
- **MOB-003.** Code for a desktop feature must not compile for a mobile target. Use `cfg(desktop)` on the module, not on each line.
- **MOB-004.** The client must have one set of answers for a phone in `$lib/platform`. Each desktop-only answer must be no on a phone (F8).
- **MOB-005.** The client must not call a shell command when the answer for its feature is no (F9).
- **MOB-006.** A build that leaves the operator's computers must keep the access token in the key store of the operating system (SEC-002, CLI-003). A build that keeps the token in memory must be a development build.
- **MOB-007.** The mobile shell must not load the updater plugin (UPD-001).
- **MOB-008.** A build setting must not change the default hub address. The test build did this, and that change must not go to `main`.

### 4.2 Phone layout

The phone layout applies below the compact breakpoint (768 px), which is an existing token. Version 0.1 said 600 px; no token has that value.

The mockups that the operator approved on 2026-11-16 are in [mockup/mobile/index.html](mockup/mobile/index.html). Where a requirement below and a mockup differ, the mockup is correct. The mockups differ from the design specification in these points, which Section 7 lists: four destinations in the bar, with Logs as a page; no hub status in the bar; a connect button on each device row; controls of 44 px to 56 px.

- **MOB-010.** In the phone layout the navigation bar must be at the bottom of the screen (design specification Section 5.3, F10). It has four destinations: Devices, Sessions, Logs and Settings (mockup 6).
- **MOB-011.** The client must draw its background below the status bar and must keep its content inside the safe area (F16).
- **MOB-012.** A text field must not make the page zoom. Text in a field must be 16 px or larger in the phone layout (F15).
- **MOB-013.** In the phone layout the logs must be a page of the navigation bar. No logs panel shows over a page (F11, mockup 6).
- **MOB-014.** The user name field must not change the case of the text. The hub address field must show the keyboard for an address, with no suggestions (F17).
- **MOB-015.** A name that does not fit must end with an ellipsis. A row of tabs that does not fit must scroll (F12).
- **MOB-016.** On a phone the client must say "this device", not "this computer" (F13).
- **MOB-017.** On a phone the first run must show "Join a hub" first (MOB-033). It must not show a function that only the desktop application has.
- **MOB-018.** A notice that shows after the page loads must not move a control that the operator can tap (F18).
- **MOB-019.** Each control in the phone layout must obey the target size of UI-013.

### 4.3 Sessions

- **MOB-020.** An SSH session must work on a phone with the on-screen keyboard and the special keys of RA-050.
- **MOB-021.** A VNC session and an RDP session must work on a phone with the touch modes of RA-050.
- **MOB-022.** A session must connect again when the operator opens the application again (design specification Section 10).

### 4.4 Network

One click principle (OD-M3): the operator installs the Anima application and no other application.

On a desktop the shell runs the program `anima-link device`, and the client gets to the hub at `http://127.0.0.1:<port>`. iOS does not let an application start a program. Thus on a phone the device link must run inside the application.

- **MOB-030.** The operator must not have to install or set up a second application to get a phone to its hub.
- **MOB-031.** The mobile shell must have the device link (`link/devicelink`) as a library inside the application. It must not start a program.
- **MOB-032.** The device link inside the application must give the client the hub at a loopback address, as the desktop link does. Task M1-4 must show that this works on iOS and on Android before a later task relies on it.
- **MOB-033.** On a phone the first run must offer "Join a hub" with an invite. The operator must be able to paste the invite and to scan it as a QR code.
- **MOB-034.** The desktop client must be able to show the invite of a new device as a QR code.
- **MOB-035.** The invite is a secret. The mobile shell must not write it to a log and must not keep it after the join (LOG-007).
- **MOB-036.** The link stops when iOS stops the application in the background. The client must start it again and connect again when the operator opens the application (MOB-022).
- **MOB-037.** When the hub does not answer, the notice on a phone must name the link, not a VPN client (NET-004).
- **MOB-038.** A phone that is already on the same VPN as its hub can use the hub address. The first run must not show this as the first choice.
- **MOB-039.** Only the `tunnel` package can import the Tailcat library. The library for a phone must obey this rule too.

### 4.5 Build and gates

- **MOB-040.** `docs/setup/` must have a procedure for the mobile build. It must give the tool versions of Section 2.1 and the workarounds for F20 and F21.
- **MOB-041.** The gates must check that the shell compiles for one iOS target and one Android target, on a computer that has those targets.
- **MOB-042.** A change for mobile must not make a desktop gate fail and must not increase an entry of `quality/baseline.json`.

### 4.6 Command line first

This plan adds no function to the hub. Thus CMD-030 asks for no new command and no new MCP tool.

## 5. Stages and tasks

Each stage ends with a build that the operator can use. The state words are those of PLN-007.

| Task | Stage | Work | Requirements | State | Commits |
| --- | --- | --- | --- | --- | --- |
| M0-1 | M0 Rulings | The operator decides OD-M1 to OD-M8 | — | Done | Ruling of the operator, 2026-11-16 (Section 3) |
| M0-2 | M0 Rulings | Make the changes of Section 7 that the rulings need | — | Done | The commit that adds version 0.2 of this plan |
| M1-1 | M1 Shell | Mobile entry point; desktop modules behind `cfg(desktop)`; identifier `ing.anima.mobile` | MOB-001, MOB-002, MOB-003, MOB-007 | Done | `327abdc6` on the branch `mobile/m1-shell`. The application started on the iPhone simulator and on the Android emulator on 2026-11-16; sign-in and the device list worked on Android. The link commands of MOB-002 come with task M3-1. |
| M1-2 | M1 Shell | Phone answers in `$lib/platform`; no call to a missing command | MOB-004, MOB-005 | In progress | `327abdc6`: MOB-004 is built. Open for MOB-005: with a loopback hub address a phone still calls `hub_listener_check`. The call fails and the client then sends nothing, which is safe. Task M3-1 gives the phone its own check. |
| M1-3 | M1 Shell | Mobile build procedure and the compile gate | MOB-040, MOB-041, MOB-042 | Not started | — |
| M1-4 | M1 Shell | Test: build `link/devicelink` as a library for iOS and for Android; join a test hub; get an answer at the loopback address. Write the result in this plan. | MOB-031, MOB-032, MOB-039 | In progress | The build part passes (F22); the wrapper was a throwaway and is not in the repository. Open: link the library into the shell, join a hub from the simulator and the emulator, and get an answer at the loopback address. The hub link needs a relay, so this part uses the network. |
| M2-1 | M2 Layout | Safe area, status bar and no zoom on a field | MOB-011, MOB-012 | Done | The commit that changes this row, on the branch `mobile/m1-shell`. Checked on 2026-11-16: on the iPhone simulator the status bar area has the page colour and a field with the focus does not zoom the page; on the Android emulator the content starts below the status bar. Not checked on a real phone. |
| M2-2 | M2 Layout | Bottom navigation bar | MOB-010, MOB-019 | In progress | `cbabf3d2`, `147d3a93` on the branch `mobile/m1-shell`. The bar has the sizes of mockup 6: 56 px, the mark over the current destination, "Sessions" with a count only when a session is live, and 24 px for the Android gesture bar. Checked on the Android emulator on 2026-11-16 after the merge of `main`. Open: the check on iOS. |
| M2-3 | M2 Layout | Logs as a page; names and tabs that fit | MOB-013, MOB-015 | In progress | `cbabf3d2`, and the commit that changes this row. Built: the Logs page at `/logs`, no logs panel in the phone layout, and the device list of mockup 4 (the title with the add button, the counts, the search, rows with the status, the address, the reason and a connect button; a long name ends with an ellipsis). Checked on the Android emulator on 2026-11-16; no test device had a protocol, so no connect button was seen there. Open: the tint on a row that needs a look (the style rules forbid `color-mix` outside the tokens), the device filter of the Logs page, the tabs of the device page, and the check on iOS. |
| M2-4 | M2 Layout | Field behaviour, wording and the notice | MOB-014, MOB-016, MOB-018, MOB-037 | In progress | The commit that changes this row. Built to mockup 3: sign-in with no doors, fields and a button of 48 px, the hub slot of a fixed height that holds the unreachable notice (MOB-018, MOB-037), and the code step with "Enter code", "Back" and "No code". Checked on the Android emulator on 2026-11-16; not checked on iOS, and the unreachable state of the slot was not seen. Open: MOB-014 and MOB-016. |
| M3-1 | M3 Link | The device link in the mobile shell, with its three commands | MOB-031, MOB-035, MOB-036 | Not started | — |
| M3-2 | M3 Link | "Join a hub" on the first run of a phone: paste and QR scan | MOB-017, MOB-033, MOB-038 | Not started | — |
| M3-3 | M3 Link | The invite as a QR code in the desktop client | MOB-034 | Not started | — |
| M4-1 | M4 Sessions | SSH session with the on-screen keyboard | MOB-020, MOB-022 | Not started | — |
| M4-2 | M4 Sessions | VNC and RDP sessions with touch | MOB-021, MOB-022 | Not started | — |
| M5-1 | M5 Release | Key store on iOS and on Android | MOB-006 | Not started | — |
| M5-2 | M5 Release | Signatures and the first builds for the operator | OD-M5, OD-M8 | Not started | — |

Order: M1, then M2. Stage M3 and the open part of task M1-4 wait (OD-M3, order of the work). Each screen of M2 must match its mockup exactly (operator, 2026-11-16: "It has to match the mock ups perfectly"). M4 and M5 can run at the same time after M3.

If M1-4 fails, stop and ask the operator. Do not add a second application as a replacement (MOB-030).

The test for the end of M2: the operator signs in on a simulator and an emulator against a test hub, opens a device and reads its checks.

The test for the end of M3: the operator installs the application on a real iPhone and a real Android phone, scans an invite, signs in and opens a device. No other application is on the phone for Anima.

## 6. Not planned here

| Item | Reason |
| --- | --- |
| The phone as an agent, the screen broadcast, the offline copy and the backup store (IOS-001 to IOS-004) | They stay in the design specification Section 10.1. They come after the first version (OD-M6). |
| Push notifications from Apple and from Google | They need a relay that Anima operates. ntfy gives notifications now. |
| A tablet layout | The phone layout comes first. |
| An application for a watch or a widget | Not asked for. |

## 7. Changes in other documents

Version 0.2 of this plan made these changes in the same commit (operator, 2026-11-16, OD-M1 and OD-M2).

| Document | Change | State |
| --- | --- | --- |
| Design specification 1.3 | A row for Android, "Tauri mobile application", Phase 5 | Made |
| Design specification 1.4 | "Android package" removed from the list | Made |
| Design specification 5.3 | The bottom navigation bar applies to iOS and to Android | Made |
| Design specification 9 | Phase 5 names the Android client | Made |
| [work-plan.md](work-plan.md) Section 4 | The row for Phase 5 names Android, this plan and the start now | Made |
| `AGENTS.md` | The first paragraph names Android | Made |
| Design specification 5.3, NAV-001, NAV-003 | In the phone layout the bar has four destinations, Logs is a page, and the hub status is not in the bar (approved mockups, 2026-11-16). | Not made |
| Design specification UI-013 and the `control-*` tokens | Controls of 44 px to 56 px in the phone layout; the tokens stop at 40 px. | Not made |
| Design specification 10 | Its title and text name iOS only. A constraint of Android has no place there yet. | Not made. Do it when task M1-3 gives the Android constraints. |
| `docs/programme/programme.md` | The rulings OD-M1 to OD-M8 have no ruling number (R-number). | Not made. The operator gives the numbers. |
