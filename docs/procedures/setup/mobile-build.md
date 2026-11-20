# Mobile build guide: the iOS application

This guide builds the phone application for iOS by hand. Plan: [mobile-plan.md](../../plans/mobile-plan.md),
MOB-040 (task M1-3) and tasks M5-3 and M5-4. Source: `apps/desktop/src-tauri/` (the shell),
`apps/desktop/src-tauri/ios-archive.sh` and `deploy/bin/phone-build-number`.

**Status.** Written on 2026-11-19 from one run on the Mac of the operator. The archive with no
signature was built there. The archive with a signature was not built: no agent uses a signing
identity. For MOB-040 on Android, use the [Android APK and F-Droid procedure](android-apk.md).

## 1. Tools

The versions are those of the run. A later version is not tested.

| Tool | Version | Note |
| --- | --- | --- |
| Xcode | 27.2, with the iOS platform | `xcodebuild -version` |
| Rust | the version of `rust-toolchain.toml`, with the target `aarch64-apple-ios` | `rustup target add aarch64-apple-ios`. `tauri ios init` also adds `aarch64-apple-ios-sim` and `x86_64-apple-ios` |
| Go | 1.27.4 | For the link, `link/build-phone.sh` |
| bun | 1.3.14, with `bun install --frozen-lockfile` done | It carries the Tauri command line 2.12.4 |
| xcodegen | 2.47.0 | `brew install xcodegen`. Tauri makes the Xcode project with it |

Run each `bunx tauri` command in `apps/desktop`. In a different folder `bunx` gets a different package
with the name `tauri`.

## 2. The archive with no signature

It proves that the application builds for an iPhone. No phone installs it.

```sh
sh apps/desktop/src-tauri/ios-archive.sh
```

Expected last lines: `version=`, `build=`, `signed=no` and `archive=` with the path of
`apps/desktop/src-tauri/gen/apple/build/anima-app_iOS.xcarchive`. The script builds the link, makes the
Xcode project again (`src-tauri/gen/` is not in the repository), builds the client and the shell, and
checks the identifier, the version, the build number, and the Anima icon for iPhone and iPad in the
archive. The iOS icon uses the artwork in [`design/brand/app-icon.png`](../../../design/brand/app-icon.png)
on a solid `#17191c` background.

`sh apps/desktop/src-tauri/ios-archive.sh --plan 0.7.95` builds nothing and prints the version, the build
number (795) and if the run signs.

## 3. The build number

`deploy/bin/phone-build-number <version>` gives the build number of iOS and the version code of Android
(M5-3). For a version of `deploy/bin/commit-version` it is the number of commits on the first line of
`main` (R-201). The script of Section 2 calls it; no person types the number.

## 4. The archive with a signature

Not tested (see Status). Only the operator, or the job of the release workflow (task B8i of
[projects-and-builds-plan.md](../../plans/projects-and-builds-plan.md)), runs it:

```sh
IOS_SIGN_IDENTITY="<the name of the Apple Distribution identity>" \
IOS_PROFILE=<the path of the App Store profile of ing.anima.mobile> \
sh apps/desktop/src-tauri/ios-archive.sh
```

Expected last lines: `signed=yes`, `archive=` and `ipa=`. The script uploads nothing. The identity is in a
keychain of the search list; the script reads no certificate file and no password.

For the first operator install, open the signed archive in Xcode Organizer and choose **Distribute App**
with **TestFlight Internal Only**. The App Store Connect app record must use `ing.anima.mobile`.
Complete Apple's export compliance question if requested, assign the processed build to your internal
group, and install it from TestFlight on an iPhone and an iPad. See [Apple's internal testing guide](https://developer.apple.com/help/app-store-connect/test-a-beta-version/add-internal-testers/)
and [the release workflow setup](../releases.md#the-ios-application-testflight-r-203-task-b8i).

## 5. Known limits

- **Do not run `bunx tauri ios build` for a device by hand on a Mac that is signed in to Xcode.** Tauri
  runs `xcodebuild` with `-allowProvisioningUpdates`, and Xcode then signs with the development identity
  of the Mac and can make a new development profile in the developer account. This occurred on
  2026-11-19. `ios-archive.sh` prevents it: it writes the signature settings into the generated project
  before the build.
- **The simulator build of Tauri stops** with "Xcode Simulator SDK 27.2 is not installed" when the
  simulator runtime is older than the SDK of Xcode (F20 of the plan; on 2026-11-19 the Mac had the
  runtime 27.0). `xcodebuild -downloadPlatform iOS` gets the runtime of the SDK; it is a large download.
  `xcodebuild` by itself does not replace Tauri here: the build step of the project needs the Tauri
  command to run.
- **Tauri's own IPA step fails** with `--no-sign`: it looks for `Anima.app`, and the application is
  `Anima.ing.app` (tauri-cli 2.12.4). The script does not use that step.
