# Native runner diagnostics in the Android client

| Item | Value |
| --- | --- |
| Version | 0.1 |
| Date | 2026-11-19 |
| Status | Local integration experiment |
| Owner | OD-14 of [the design specification](../../spec/design-spec.md#11-open-decisions) |
| Companion | [iOS client integration](ios-runner-integration.md) |

## Plan

Add an Android provider behind the existing version 1 report and Settings view. Read Android
power, thermal, memory, storage, network, and backend discovery APIs. Keep Java heap headroom
and system memory separate from the iOS process-headroom signal. Advertised Vulkan features
describe discovery and require a workload before scheduling jobs. Use the normal Android app
with a local debug build, verify its native bridge on the plugged-in device, and keep the
branch local.

## Device result

Built the ARM64 debug APK with the repository's pinned Rust toolchain, JDK 21, and Android NDK
28.2.13676358. Installed and launched Anima.ing on an ECOPAD C5L1 running Android 16 (API 36).
The APK used standard Android debug signing.

Two reads through the app's actual WebView bridge returned fresh version 1 reports. Then the
Settings view ran on the same phone using a temporary local hub fixture. **Read capabilities**
displayed Android system memory and the technical report. **Refresh** obtained a newer snapshot
from the real Android provider. The fixture and debug forwarding were removed after the check.
The app returned to Welcome.

| Field | Observed on the device |
| --- | --- |
| Foreground and power | Foreground, charging. Android battery diagnostics confirmed AC power. |
| Thermal and CPU | Fair, eight active logical CPUs. |
| Memory | About 1,447 to 2,051 MiB available system memory. Java heap headroom was about 184 to 187 MiB. |
| Vulkan | API 1.3.0, hardware level 1, compute level 0. Android's feature inventory agreed. |
| Clock | The initial report was dated September 2025. The Settings check reported the current date. |

The device clock change means the queue should use its receipt time for heartbeat expiry.
Keep the report timestamp for diagnostics. Vulkan advertising and Java heap headroom are
discovery signals. A schedulable Android job still needs an actual workload and separate
admission checks. See [Android memory reporting](https://developer.android.com/reference/android/app/ActivityManager.MemoryInfo)
and [Android Vulkan features](https://developer.android.com/reference/android/content/pm/PackageManager).

Rust formatting, the five repository quality checks, the client lint and type check, and
40 targeted client tests ran. The targeted tests covered the iOS and Android Settings report
format, the native client capabilities, and the Settings page.
