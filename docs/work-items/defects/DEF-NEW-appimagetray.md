# DEF-NEW-appimagetray: the published AppImage stops at start on Ubuntu 24.04 and its derivatives

Progress: the index [README.md](../README.md) holds the state of this item (PLN-006). The plan register in [work-plan.md](../../work-plan.md) Section 9 lists the index.

| Item | Value |
| --- | --- |
| Identifier | DEF-NEW-appimagetray |
| Kind | Defect |
| Owner | [design-spec.md](../../spec/design-spec.md) AWK-004 (the tray item); [releases.md](../../procedures/releases.md), "The release workflow" (the job `linux`) |
| Serves | Row 4.1 of the work plan (the first release) |
| Version | 1.0 |
| Date | 2026-11-20 |
| Origin | The operator, on 2026-11-20, on Pop!_OS 24.04 (GLib 2.80), with `Anima_0.0.2_amd64.AppImage` from the release workflow. Not a GitHub issue (PLN-012). |

## What was seen

```text
thread 'main' panicked at .../libappindicator-sys-0.9.0/src/lib.rs:41:5:
Failed to load ayatana-appindicator3 or appindicator3 dynamic library
/lib/x86_64-linux-gnu/libayatana-ido3-0.4.so.0: undefined symbol: g_once_init_leave_pointer
```

## Cause

- The job `linux` of the release workflow builds on Ubuntu 22.04. The AppImage carries the GLib of 22.04 (2.72).
- The desktop crate took `tauri` with `workspace = true`. The Tauri CLI (2.12.4, `interface/rust/manifest.rs`, `inject_features`) does not read the features of a dependency with workspace inheritance. Thus it did not see `tray-icon` and did not put `libayatana-appindicator3.so.1` in the AppImage.
- At start, the tray item opens the library of the system. On 24.04 its `libayatana-ido3` needs GLib 2.80 (`g_once_init_leave_pointer`), but the GLib of the AppImage loads first.

Proved on 2026-11-20 with the published file: with the five tray libraries of 22.04 (appindicator, indicator, ido, the two dbusmenu) in `usr/lib` of the extracted AppImage, it starts; without them it stops with the error above.

## Correction

On the branch `fix/appimage-tray-library`:

- [x] The desktop crate names `tauri` with its version and features, not `workspace = true`. The CLI sees `tray-icon` and bundles the library, and linuxdeploy bundles what it needs. This applies to `trial-bundle.sh` and to the release builder of the application, which call the same CLI.
  Proved on 2026-11-20 with a local `tauri build --bundles appimage` on Pop!_OS 24.04: `usr/lib` of the AppImage holds `libayatana-appindicator3.so.1`, `libayatana-indicator3.so.7`, `libayatana-ido3-0.4.so.0` and the two dbusmenu libraries. Before the change, the published 0.0.2 held none of them.
- [x] The job `linux` of the release workflow refuses an AppImage that does not carry `libayatana-appindicator3.so.1` and `libayatana-ido3-0.4.so.0`.
- [ ] The start of the next published AppImage on a 24.04 computer: [HT-036](../../need-human-testing.md).
