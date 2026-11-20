---
kind: Plan
state: In progress
evidence: Version 0.5 (2026-11-19). Task M3-4 (the account sign-in on a phone) is built and waits for HT-055; OD-M9 is open. The build number (M5-3) and the iOS archive script (M5-4) are built; the archive with a signature waits for HT-053. Version 0.4: R-181 adds M5-7 for F-Droid and puts Google Play aside. The operator decided OD-M1 to OD-M8 and approved the phone mockups on 2026-11-16. It serves Section 4, tier 4, "iOS client and Android client (Phase 5)", and starts now (OD-M2). MOB-001 to MOB-042. Its Section 5 holds the state of each task. The branch `mobile/m1-shell` was merged into `main` on 2026-11-17, and again with the Paste invite and Joining screens (M3-2). Its facts are from a test build on the local branch `spike/mobile`, which is not for merge.
---

# Anima Mobile Plan

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this plan (PLN-005).

| Item | Value |
| --- | --- |
| Document | Mobile plan: the iOS client and the Android client |
| Version | 0.5 |
| Date | 2026-11-19 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | The header of this document holds the state (PLN-020). History: Approved. The operator decided OD-M1 to OD-M8 on 2026-11-16 (Section 3). Version 0.2 writes Section 4.4 again for the ruling on OD-M3. MOB-032 waits for the test of task M1-4. Version 0.3: the operator approved the phone mockups on 2026-11-16 ("Mock are good enough now as well"); Section 4.2 follows them. Version 0.5 (2026-11-19): task M3-4 is built, MOB-002 names the cloud commands, and OD-M9 is added; the build number (M5-3), the iOS archive script (M5-4) and the iOS build procedure (M1-3, in part) are built. |
| Parent | [work-plan.md](../work-plan.md) Section 4, tier 4, row "iOS client and Android client (Phase 5)" (PLN-003) |
| Base | `main` at `fa4ac394`. The facts in Section 2 are from the branch `spike/mobile`, which is local and is not for merge. |
| Related | [design-spec.md](../spec/design-spec.md) Sections 1.3, 1.4, 5.3, 5.7, 9 and 10; [vpn-plan.md](vpn-plan.md); [ui-redesign-direction.md](ui-redesign-direction.md) UI-013 |

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

The operator decided the rows OD-M1 to OD-M8 on 2026-11-16. OD-M9 and OD-M10 are open.

| Number | Decision | Ruling |
| --- | --- | --- |
| OD-M1 | Is the Android client in scope? The design specification Section 1.4 put the Android package out of scope. | **Decided: yes.** One shell and one client serve both platforms (F1, F3, F6). Section 7 lists the changes to the specification. |
| OD-M2 | When does this work start? The work plan put the iOS client at tier 4. | **Decided: start now.** The operator gave the order: "lets worry about just getting the app to work first". |
| OD-M3 | How does a phone get to the hub? | **Decided: with no second application.** The operator's words: "I don't want the user to have to install tailscale ontop of the app remember one click install princible it should be as easy as possible". The device link (`anima-link`) is thus inside the client (Section 4.4). Version 0.1 recommended the Tailscale application; that recommendation is withdrawn. **Order of the work (operator, 2026-11-16): "for the tailscale thing we should just use the hub an my actual account for now".** The device link inside the client stays the decision; stage M3 waits. Until then a phone uses a hub address and the operator signs in to the real hub. |
| OD-M4 | One shell crate, or a second crate for mobile? | **Decided: the existing crate,** `apps/desktop/src-tauri`. |
| OD-M5 | The identifier of the two applications | **Decided: `ing.anima.mobile`** for iOS and for Android. An identifier cannot change after a release. |
| OD-M6 | The content of the first version | **Decided: Section 1.2.** IOS-001 to IOS-004 come later (Section 6). |
| OD-M7 | Where the access token is kept during development | **Decided: in memory.** The operator's words: "key not being read is fine for later". MOB-006 gives the rule for a release. |
| OD-M8 | How the operator gets the applications | **Decided.** iOS uses TestFlight. Android uses a release server APK first. F-Droid is the first Android catalog. Google Play waits for the D-U-N-S number (R-181). |
| OD-M9 | Does the phone application get to a hosted hub through the link in the application, through the gateway of the web hub at `hub.anima.ing` ([hosted-web-hub.md](../proposals/hosted-web-hub.md)), or through both? | Open (2026-11-19). Recommendation of the session that built task M3-4: the link now, and the gateway later only as a second path. Reasons: the link is the decision of OD-M3 and is the only path that exists (a hosted hub has no public address, HMD-037); it is built in the phone application and waits only for HT-025; the phone is then a device of the link, which the later phone functions need (IOS-001 to IOS-004). The gateway is a proposal with nothing built: it needs the pool, the site, a browser session of the hub and the second factor (HMD-006) first, and its session is not the session of the application. If HT-025 fails on real phones, or a network stops the link, the gateway is the fallback; that is its use in the application. A phone with no application gets the web hub in its browser when it exists, with no work in this plan. |
| OD-M10 | Two rules of the App Store review for the sign-in of the iPhone application (read on 2026-11-19 on the page of the guidelines, which a tool cut, so the words are from its summary). (a) Guideline 4.8: an application that uses a third-party or social sign-in for its primary account must also offer a sign-in that limits what it collects to the name and the email address, lets the person keep the email address private, and does not collect for advertising. Sign in with Apple is such a sign-in. (b) Guideline 5.1.1 (v): an application that supports the making of an account must offer the deletion of the account inside the application. | Open (2026-11-19). Recommendation for (a): the rule applies, because the page of the account offers Google and GitHub; it is met when the same page offers Apple, which the page of the account shows inside the application (task M3-5), so no sign-in of Apple is built in the application itself. He confirms that the Apple method is on in WorkOS for the production client. Recommendation for (b): the rule applies when "Sign up" stays on the first screen; add a row "Delete account" in Settings that opens the page of the account, which the site must have. It is not built and it needs the site. It matters before a build for testers outside his team (a review), not for a build for him alone. |

## 4. Requirements

The operator approved these requirements on 2026-11-16, with the limit that the Status line gives for MOB-032.

### 4.1 Shell

- **MOB-001.** One crate, `apps/desktop/src-tauri`, must build the shell for macOS, Linux, iOS and Android.
- **MOB-002.** The mobile shell must register only the commands that a phone can serve. In the first version these are the three access token commands, the link commands (`link_join`, `link_status`, `link_leave`), and for the account sign-in (task M3-4) the five cloud commands with `system_browser_open`. A phone has no `cloud_device`.
- **MOB-003.** Code for a desktop feature must not compile for a mobile target. Use `cfg(desktop)` on the module, not on each line.
- **MOB-004.** The client must have one set of answers for a phone in `$lib/platform`. Each desktop-only answer must be no on a phone (F8).
- **MOB-005.** The client must not call a shell command when the answer for its feature is no (F9).
- **MOB-006.** A build that leaves the operator's computers must keep the access token in the key store of the operating system (SEC-002, CLI-003). A build that keeps the token in memory must be a development build.
- **MOB-007.** The mobile shell must not load the updater plugin (UPD-001).
- **MOB-008.** A build setting must not change the default hub address. The test build did this, and that change must not go to `main`.

### 4.2 Phone layout

The phone layout applies below the compact breakpoint (768 px), which is an existing token. Version 0.1 said 600 px; no token has that value.

The mockups that the operator approved on 2026-11-16 are in [mockup/mobile/index.html](../mockup/mobile/index.html) (in the repository since 2026-11-17; its row in [mockup/README.md](../mockup/README.md) holds its state, PLN-019). Where a requirement below and a mockup differ, the mockup is correct. The mockups differ from the design specification in these points, which Section 7 lists: four destinations in the bar, with Logs as a page; no hub status in the bar; a connect button on each device row; controls of 44 px to 56 px.

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
- **MOB-043.** On an iPhone the sign-in with the Anima.ing account must show inside the application and must pass through WorkOS, the account that the site and the desktop application use. The person must not go to a browser and must not compare or type a code. Operator, 2026-11-19, first-hand to the orchestrating session and relayed: "Sign in on I phone should also be native tho", then "native means it passes through workos", then "you can call cloud or the website for the sign in". No ruling has a number yet. HMD-032 of [hosted-mode.md](../proposals/hosted-mode.md) names the device authorization flow; for the iPhone application this requirement names the authorization code flow with PKCE in its place, with the same public client and no secret. Android keeps the code for now.
- **MOB-039.** Only the `tunnel` package can import the Tailcat library. The library for a phone must obey this rule too.

### 4.5 Build and gates

- **MOB-040.** `docs/procedures/setup/` must have a procedure for the mobile build. It must give the tool versions of Section 2.1 and the workarounds for F20 and F21.
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
| M1-3 | M1 Shell | Mobile build procedure and the compile gate | MOB-040, MOB-041, MOB-042 | In progress | Built in part, in the commit that changes this row: [mobile-build.md](../procedures/setup/mobile-build.md) has the iOS build with the tool versions and the limit of F20. Not built: the Android part with F21, and the compile gate (MOB-041). The pinned Rust 1.100.0 of the Mac has the two iPhone targets since 2026-11-19 and no Android target. |
| M1-4 | M1 Shell | Test: build `link/devicelink` as a library for iOS and for Android; join a test hub; get an answer at the loopback address. Write the result in this plan. | MOB-031, MOB-032, MOB-039 | In progress | The build part passes (F22). Built on 2026-11-17 in the commit that changes this row: the library `link/cmd/phonelink` (`link/build-phone.sh`: a static archive for iOS, a shared library for Android with `netcgo`), linked into the shell by `build.rs`. The state folder skips the `chmod` step on iOS and the check of the folders above it on Android, where the app's sandbox keeps it private. The iPhone app builds with it, and the Android package carries `libphonelink.so`. A join from the Android emulator reached the link and wrote its folder, but the emulator had no route to the internet, so the relay never answered; this Mac joined the same test hub with `anima-link`. The iPhone simulator stopped answering before the join. Open: join a test hub from the simulator and the emulator and get an answer at the loopback address (HT-025). No session holds this task now (claim deleted, 2026-11-18): what this row names is all that is built, and HT-025 is the open check. |
| M2-1 | M2 Layout | Safe area, status bar and no zoom on a field | MOB-011, MOB-012 | Done | The commit that changes this row, on the branch `mobile/m1-shell`. Checked on 2026-11-16: on the iPhone simulator the status bar area has the page colour and a field with the focus does not zoom the page; on the Android emulator the content starts below the status bar. On 2026-11-17 the iPhone simulator showed that a scrolled page and a page moved by the keyboard went under the clock (F16); the commit that changes this row adds a band of the chrome material over the status bar, which follows the visible area when the keyboard opens. Checked on the iPhone simulator on 2026-11-17. Not checked on a real phone. |
| M2-2 | M2 Layout | Bottom navigation bar | MOB-010, MOB-019 | In progress | `cbabf3d2`, `147d3a93` on the branch `mobile/m1-shell`. The bar has the sizes of mockup 6: 56 px, the mark over the current destination, "Sessions" with a count only when a session is live, and 24 px for the Android gesture bar. Checked on the Android emulator on 2026-11-16 after the merge of `main`, and on the iPhone simulator on 2026-11-17. Open: no live session was seen, so the count was not seen. |
| M2-3 | M2 Layout | Logs as a page; names and tabs that fit | MOB-013, MOB-015 | In progress | `cbabf3d2`, and the commit that changes this row. Built: the Logs page at `/logs`, no logs panel in the phone layout, and the device list of mockup 4 (the title with the add button, the counts, the search, rows with the status, the address, the reason and a connect button; a long name ends with an ellipsis). Checked on the Android emulator on 2026-11-16; no test device had a protocol, so no connect button was seen there. Also built: the device page of mockup 5 (back and more, the name on two lines at most, the status and the reason, the address with a copy button, the uptime and response figures, the last 50 heartbeats, tabs that scroll, check rows, and the connect buttons docked above the bar), the Logs page of mockup 6 (chips for All, Errors, Warnings, Info and Audit; rows with the time, the severity, the device and the message on two lines at most), and the section and row style of mockup 7 for Settings. Checked on the Android emulator on 2026-11-17. Built later: the tint on a row that needs a look (tokens `status-down-wash` and `status-degraded-wash`); on Settings, the Account rows (user name, two-factor, Sign out), the Hub row and Leave hub (a phone has no link yet, so it signs out and forgets the hub). Open: the search of the Logs page is built (a button beside the title opens a search field; the hub, `anima event list --search` and the MCP tool `anima_events_search` with `text` search the log the same way, LOG-011; checked on the Android emulator on 2026-11-17), and a log row shows the mockup's icon before its severity; the Link and This device rows of mockup 7 (they need the link, M3-1), and the hub's name (the API has none, so the Hub row shows the address); and the device filter of the Logs page. The device list, the device page, Logs and Settings were checked on the iPhone simulator on 2026-11-17 and look as on Android. |
| M2-4 | M2 Layout | Field behaviour, wording and the notice | MOB-014, MOB-016, MOB-018, MOB-037 | In progress | The commit that changes this row. Built to mockup 3: sign-in with no doors, fields and a button of 48 px, the hub slot of a fixed height that holds the unreachable notice (MOB-018, MOB-037), and the code step with "Enter code", "Back" and "No code". Checked on the Android emulator on 2026-11-16 and on the iPhone simulator on 2026-11-17; the unreachable state of the slot was not seen. Open: MOB-014 and MOB-016. |
| M3-1 | M3 Link | The device link in the mobile shell, with its three commands | MOB-031, MOB-035, MOB-036 | In progress | Built in the commit that changes this row: `link_join`, `link_status` and `link_leave` in the mobile shell (`src/phone_link.rs`) with the desktop link's answers and error codes; the link starts when the app starts and again when it comes back from the background (MOB-036); its local port is kept, so the hub's address stays the same; the invite is wiped after the call and never logged (MOB-035). The client turns on `deviceLink` for phones, so "Join with an invite" works there; a new capability `cloudAccount` keeps Sign in off on phones ("Not on phones yet.") until the cloud commands come. Not checked end to end: see M1-4 and HT-025. No session holds this task now (claim deleted, 2026-11-18): what this row names is all that is built, and HT-025 is the open check. |
| M3-2 | M3 Link | "Join a hub" on the first run of a phone: paste and QR scan | MOB-017, MOB-033, MOB-038 | In progress | The first screen changed on 2026-11-17 (operator: "sign in with SSO and then one link at the bottom to join a hub"): Sign in, then Sign up and Join a hub (`9275c55b`). Built in the commit that changes this row, to frame 2 of mobile/index.html: "Paste invite" (Back, the invite field in 16 px machine text, Paste, which reads the clipboard, "Join hub" and "Works once. Not kept.") and "Joining the hub" (Invite read, Link started with the device key, Waiting for hub, which waits up to a minute for the link to reach the hub, then Sign in; a failed step says why and offers Retry). Built in `31378b0d`; `6ae435e3` gives the two screens the sizes of the frame in the phone layout (20 px at each side, a field of 148 px, buttons of 48 px with 16 px text, Paste as a line button, 16 px between the parts, step rows of 56 px), which they did not have before. Measured in a browser at 390 by 844 px, dark, on 2026-11-18: the paste view as built, and the joining view with a stand-in for the commands of the shell (a failed step with Retry was seen; the tests cover each mark). Not seen on a simulator, an emulator or a phone, where the keyboard and the safe areas are: HT-025 holds those checks. The paste part is built and waits for HT-025. Not built: the QR scan (it needs a camera plugin in the mobile shell; no part of it is built). No session holds this task now: the next session claims it for the scan. |
| M3-3 | M3 Link | The invite as a QR code in the desktop client | MOB-034 | Not started | — |
| M3-4 | M3 Link | The sign-in with the Anima.ing account on a phone: the cloud commands in the mobile shell, so that the capability `cloudAccount` is on there (task M3-1 left it off: "Not on phones yet."). A person who has only an account can then get to the hosted hub from a phone. | MOB-017, MOB-030; HMD-032 of [hosted-mode.md](../proposals/hosted-mode.md) | In progress | Added on 2026-11-18 from [release-four-platforms.md](../proposals/release-four-platforms.md), Section 3.3. Its two conditions are met: HT-022 is Passed (2026-11-17), and R-163 approved HMD-032, which was the reason for the ruling that the row asked for; R-158 item 6 names this task as what the first phone release waits for. This is a reading of the session that built it, not words of the operator. Built in the commit that changes this row, on the branch `mobile/account-sign-in`: the module `cloud` of the shell compiles for a phone, with `cloud_sign_in_start`, `cloud_sign_in_poll`, `cloud_sign_in_resume`, `cloud_hub` and `cloud_hub_access`; `system_browser_open` on a phone opens only an `https` address, in the browser of the phone (`tauri-plugin-opener`); the phone names itself to the cloud one time (`iphone-` or `android-` and six hexadecimal digits, kept in the data folder of the application), because iOS gives each application the same device name; the refresh token is in memory in this development build (OD-M7), so the person signs in again after the application stops; the client turns `cloudAccount` on for a phone, says "this device" in the steps (MOB-016, for these steps only), gives the steps the control sizes of frame 2 in the phone layout, and does not ask a phone for `cloud_device` (MOB-005). A phone is not added to the devices of the hub (HMD-044 names a computer). The shell compiles and passes Clippy for `aarch64-apple-ios-sim` and `aarch64-linux-android` with the stable toolchain of the Mac, not with the pinned 1.100.0, which has no phone target there (task M1-3). Not run on a simulator, an emulator or a phone: built, and waits for HT-055, and for HT-025 for the join through the link. Open: the key store for a build that leaves the operator's computers (MOB-006, task M5-1). |
| M3-5 | M3 Link | The sign-in inside the iPhone application: the page of the account in the web authentication session of iOS, the authorization code flow of WorkOS with PKCE, and the return at a loopback address of the shell. | MOB-043; HMD-032 to HMD-035 | In progress | Added on 2026-11-19 from the words of the operator in MOB-043. Chosen: the application talks to WorkOS itself, and not through the site or the pool. Reasons: it is the smaller change (the shell has the WorkOS client, the refresh and the two cloud calls already, so only the start of a sign-in changes); it needs no new route in the site or the pool; and no secret of a sign-in crosses a custom URL scheme, which a different application can register. Against it: the phone holds a WorkOS refresh token, as the desktop application does (HMD-034); a sign-in through the site would leave none on the phone. Built in the commit that changes this row, on the branch `mobile/native-sign-in`: the plugin `apps/desktop/account-session` (Swift, `ASWebAuthenticationSession`, which shares the sign-in of Safari); in the shell `cloud_sign_in_session`, the code verifier and the state (`cloud/pkce.rs`), and a listener for one sign-in at `http://127.0.0.1:47113/sign-in` (`cloud/loopback.rs`), which WorkOS documents as the return address of a native client in production; the code is traded with the verifier and no secret, and the code, the verifier and the tokens stay in the shell. The client asks the iPhone shell for this sign-in and shows no code. Read in the documents of WorkOS on 2026-11-19: the authorization URL with `code_challenge` (S256), the trade with `code_verifier` when no client secret is present, and the loopback return address. Not stated there: a custom URL scheme as a return address, and that a refresh needs no secret after this flow (the page of the device flow says so for a public client; the reference page lists the secret as required). Needs from the operator before it can work: the return address `http://127.0.0.1:47113/sign-in` in the Redirects of the application in the WorkOS dashboard. Compiled: an archive of the iPhone application with no signature holds the plugin and links it (`ios-archive.sh`, 2026-11-19); this proves the build and not the function. The image of the hub copies the new workspace member (`deploy/hub/Dockerfile`), because cargo reads the manifest of each member. Not run: HT-054. |
| M4-1 | M4 Sessions | SSH session with the on-screen keyboard | MOB-020, MOB-022 | In progress | The commit that changes this row. Built to mockup 8: in the phone layout an SSH session fills the visible area and hides the navigation bar; a bar of 48 px with back (the session stays open), the device, the time since it opened, "Hide keyboard" and the session menu (the keys and Paste); the terminal to the edges; and a row of keys (Esc, Tab, Ctrl, Alt, the arrows, `| ~ / -`) where Ctrl and Alt latch for the next key. Checked on 2026-11-17 on the iPhone simulator and the Android emulator with a throwaway SSH server that ran `cat -v`; HT-033 holds the check on real phones. On Android the keyboard plugin (`apps/desktop/keyboard`) makes the web view end where the keyboard starts and marks the page while the keyboard is open, so the key row sits on the keyboard; the terminal empties its input box after each typed character, so Android keyboards that move the caret no longer send text in the wrong order. Both checked on the Android emulator with Gboard on 2026-11-17. Open: iOS shows its form bar (up, down, Done) over the keyboard; iOS opens the keyboard only after a tap on the terminal; the hide-keyboard icon is a chevron (the icon set has no keyboard icon yet); the key row has no fade at its end; Find and Files are not in the phone bar; the session tab strip still shows over the pages, as the only way back to a session; MOB-022. |
| M4-2 | M4 Sessions | VNC and RDP sessions with touch | MOB-021, MOB-022 | Not started | — |
| M5-1 | M5 Release | Key store on iOS and on Android | MOB-006 | Not started | — |
| M5-2 | M5 Release | Signatures and the first builds for the operator | OD-M5, OD-M8 | Not started | — |
| M5-3 | M5 Release | The rule that gives the iOS build number and the Android version code from the release version: one pure function with its test, used by the two build scripts. | RLS-003 of [release-four-platforms.md](../proposals/release-four-platforms.md) | In progress | R-158 names it as a part of M5-4 and M5-5 (a reading of the session that recorded the ruling; RLS-003 itself has no ruling). Built in the commit that changes this row: `deploy/bin/phone-build-number` with its test. For the version A.B.C the number is A * 10000000 + B * 100 + C. For a version of `deploy/bin/commit-version` it is thus N, the number of commits on the first line of `main` (R-183, R-201). A higher version gives a higher number, and the number stays below the highest Android version code. Each release has a version and a number of its own, higher than each one before it, because `main` only gets longer; so App Store Connect never gets a lower or an equal number for a version, but when one version is archived two times, and that upload is refused, which is correct. The iOS script uses it (M5-4). Open: the Android script (M5-5) does not exist yet. |
| M5-4 | M5 Release | A script that makes the signed iOS archive for TestFlight from one commit and one version. It reads the certificate from the keychain of the Mac and has no secret in its arguments, its environment or its log. The upload is an act of the operator. Its steps go in the procedure of MOB-040. | OD-M5, OD-M8; RLS-010, RLS-011 | In progress | R-158 names it (OD-REL-2: a script on the Mac of the operator, kept in the repository); R-203 lets the release workflow call it (task B8i). Built in the commit that changes this row: `apps/desktop/src-tauri/ios-archive.sh`, with its steps in [mobile-build.md](../procedures/setup/mobile-build.md). It takes the version, gets the build number from M5-3, and checks both in the archive. With `IOS_SIGN_IDENTITY` (the name of an identity in a keychain) and `IOS_PROFILE` (the path of the profile) it signs by hand and exports an IPA; with neither it makes an archive with no signature. It uploads nothing. Run on 2026-11-19 on the Mac of the operator with no signature: the archive of `ing.anima.mobile` has the version and the build number that were asked for and no signature. The mode with a signature is not tested, because no agent uses a signing identity: HT-053. On 2026-11-20 an unsigned device archive of `5ed19d05` built as 0.8.39 (839) with the Anima icon and device-family entries for both iPhone and iPad. An App Store profile for `ing.anima.mobile` with the Sign in with Apple entitlement was found on this Mac, created on 2026-11-19. The operator signs and answers Apple’s export question. M5-1 (the key store) is required before distribution to other people. Check: HT-053, where the operator installs the build from TestFlight on a real iPhone. |
| M5-5 | M5 Release | A script that makes the Android APK from one commit and one version, signed with the release key from the key store of the build computer. No secret in its arguments, its environment or its log. Its steps go in the procedure of MOB-040. | OD-M5, OD-M8; RLS-010, RLS-011 | Not started | R-158 names it (OD-REL-2: a script on the Mac of the operator, kept in the repository). Needs: M5-1, M5-3, and the release key that only the operator makes. Check: an entry in [need-human-testing.md](../need-human-testing.md), where the operator installs the APK on a real Android phone and then installs a higher version over it. |
| M5-6 | M5 Release | The update notice of the Android application: the client reads the feed of its target and shows a notice with a link to the new APK. It installs nothing by itself, and the updater plugin stays out (MOB-007). | MOB-007 | Not started | R-158 names it (OD-REL-5). Needs: task R10 of [projects-and-builds-plan.md](projects-and-builds-plan.md), which writes that feed. |
| M5-7 | M5 Release | Prepare `ing.anima.mobile` for the official F-Droid catalog. Build the APK from a clean Linux checkout. Check dependency and asset licenses against the [F-Droid inclusion policy](https://f-droid.org/docs/Inclusion_Policy/). Provide an `fdroiddata` build recipe. Verify that the rebuild matches the APK that Anima signs, using the [F-Droid reproducible build procedure](https://f-droid.org/docs/Reproducible_Builds/). | OD-M5, OD-M8, MOB-006, MOB-040 | In progress | R-181. Build script and draft recipe in `42b5ccdc` and `b1bdf19a`. An x86-64 Ubuntu 24.04 build from `b1bdf19a` produced `ing.anima.mobile` 0.1.0 (1000) with both arm64 libraries. The unsigned APK matched a fresh checkout rebuilt with the recipe fixture removed. Signature comparison passed with a disposable key and `--alignment-preserved`. A fresh x86-64 Linux checkout at `929c4e49`, based on `dev`, built the same package and version. The recipe lint, source scan (with the listed fixture removed), and APK scan passed. [Android build procedure](../procedures/setup/android-apk.md) gives the recipe and checks. The signed rebuild needs the release APK from M5-5. Submission follows the public phone checks in R-158. HT-069 holds the real Android check. |

**M5-2 pending prerequisite.** On 2026-11-18, Sign in with Apple was enabled as a primary capability on Apple Developer App ID `ing.anima.mobile`. Apple warned that the change invalidated existing provisioning profiles that include this App ID. Before the next signed iOS build, regenerate the affected profiles and replace the old copies wherever the signing environment uses them.

Order: M1, then M2. Stage M3 and the open part of task M1-4 wait (OD-M3, order of the work). Each screen of M2 must match its mockup exactly (operator, 2026-11-16: "It has to match the mock ups perfectly"). M4 and M5 can run at the same time after M3. The tasks M3-4 and M5-3 to M5-6 were added on 2026-11-18 from [release-four-platforms.md](../proposals/release-four-platforms.md), which holds their order with the release tasks of the other plans; M5-2 comes after M5-4 and M5-5. R-158 (2026-11-18) names M5-3 to M5-6. The first release for other persons on a phone waits for stage M3 with HT-025 and for task M3-4; the installations of the operator from TestFlight do not wait (R-158, OD-REL-7). What stands between task M3-4 and that release, in order: HT-055 and HT-025 (the operator, on a simulator and then on real phones); task M5-1 (the key store, MOB-006); task M1-3 (the build procedure and the compile gate); task M5-3 (the build number); then task B8i of [projects-and-builds-plan.md](projects-and-builds-plan.md) for TestFlight (R-203) and tasks M5-5 and M5-7 for Android. The QR scan (M3-2) and the QR code on the desktop (M3-3) are not on this path: the account sign-in needs no invite.

If M1-4 fails, stop and ask the operator. Do not add a second application as a replacement (MOB-030).

The test for the end of M2: the operator signs in on a simulator and an emulator against a test hub, opens a device and reads its checks.

The test for the end of M3: the operator installs the application on a real iPhone and a real Android phone, scans an invite, signs in and opens a device. No other application is on the phone for Anima.

## 6. Not planned here

| Item | Reason |
| --- | --- |
| The phone as a daemon, the screen broadcast, the offline copy and the backup store (IOS-001 to IOS-004) | They stay in the design specification Section 10.1. They come after the first version (OD-M6). |
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
| [work-plan.md](../work-plan.md) Section 4 | The row for Phase 5 names Android, this plan and the start now | Made |
| `AGENTS.md` | The first paragraph names Android | Made |
| Design specification 5.3, NAV-001, NAV-003 | In the phone layout the bar has four destinations, Logs is a page, and the hub status is not in the bar (approved mockups, 2026-11-16). | Not made |
| Design specification UI-013 and the `control-*` tokens | Controls of 44 px to 56 px in the phone layout; the tokens stop at 40 px. | Not made |
| Design specification 10 | Its title and text name iOS only. A constraint of Android has no place there yet. | Not made. Do it when task M1-3 gives the Android constraints. |
| `docs/rulings.md` | The rulings OD-M1 to OD-M8 have no ruling number (R-number). | Not made. The operator gives the numbers. |

## Council follow-up: returning to the same phone account (2026-11-18)

R-152 names the preparation below. It changes no approved requirement or contract. Read current `main` and the claims before taking a task. A source review is not a Passed human check. The task owner records the tested build and the result; unknown results stay unknown.

| Task | Work and acceptance | Depends on | State |
| --- | --- | --- | --- |
| M-C1 | Prepare the restart and resume evidence for the mobile owner's existing check. Use a uniquely named link device; compare name, key fingerprint and row count before and after restart. Distinguish link identity, loopback port and per-origin login token. A configured address is not an observed running port. Keep manual self-host join and hosted account tests separate: a hosted hub refuses password login. Record a fresh authenticated read, not only a saved row. | Existing mobile claim owner; baseline join on each OS; HM-D12 and M5-1 for a hosted-account release; operator devices | Not started. Preparation only; hardware execution and hosted account integration are blocked |
