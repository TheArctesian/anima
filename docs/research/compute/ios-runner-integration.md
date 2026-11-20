# Native runner diagnostics in the iOS client

| Item | Value |
| --- | --- |
| Version | 0.1 |
| Date | 2026-11-19 |
| Status | Local integration experiment |
| Owner | R-110 and OD-14 in [the design specification](../../spec/design-spec.md#11-open-decisions) |
| Work plan | Section 4, tier 4, mobile clients. Section 9 lists research records. |

## Scope

Bring the read-only portion of the iOS runner capability spike into Anima.ing Settings. Add one
native request scoped to the main iOS web view. Return a fresh versioned report on request and
present it locally. Keep the runtime provider separate from the shared client contract so the
Android provider can implement the same contract in the next increment.

Use the iOS 17 support floor of the mobile client. Probe available Apple APIs and omit values
that the current OS or simulator cannot supply. A report describes discovery. Admission,
execution, and validation of a job contract stay with the worker experiment. This integration
uses a client-local request, so it has no hub API, CLI command, or MCP tool.

Verify the signed-in Settings route with a simulator or a local frontend fixture and compile the
native iOS app. Record any remaining device checks.

## Integration and evidence

The **Runner capabilities** section in iOS Settings reads on demand. It shows power, thermal
state, app memory headroom, and free storage. **Technical report** displays the full versioned
snapshot, including OS, CPU, network, Vision, Metal, Core ML, and the continued-processing GPU
resource when the OS reports it. The iOS bridge is a native Tauri plugin and a command capability
limited to the app's main web view. See [Tauri's mobile plugin API](https://v2.tauri.app/develop/plugins/develop-mobile/).

- The app's unsigned iOS archive built with the pinned Rust toolchain and Xcode 27.2.
- The Swift report provider passed a type check with an iOS 17 deployment target.
- The client lint, type check, and all 4,867 client tests passed. The runner Settings test
  exercised the read and refresh path with a native response fixture. The first quality gate
  run flagged two size limits. Moving the runner test and shell adapter into focused modules
  cleared both. Formatting and all five repository quality gates passed.
- Opening the integrated app and reading device signals on an iPhone remains open under
  [HT-064](../../need-human-testing.md#ht-new-iosrunnerreport-read-the-native-capability-report-in-animaing-settings).

The next increment is an Android provider for the same versioned client contract. It can supply
the shared signals through Android APIs and put backend-specific discovery in the technical
report. Validate each platform's report independently before using it for placement.
