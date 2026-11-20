# Build the Windows client

Tasks: WIN1 and WIN2 in [projects-and-builds-plan.md](../../plans/projects-and-builds-plan.md).
This is a local Windows 11 x86_64 client. It is not a public signed release.

## Build on Windows

Install Git, Bun, Go (the version in `link/go.mod`), Rust through rustup, and
Microsoft C++ Build Tools with Desktop development with C++ and a Windows SDK.
The Rust version comes from `rust-toolchain.toml`. Windows 11 supplies WebView2;
the NSIS installer can install the runtime if it is absent.

From a checkout, run in a PowerShell session permitted to execute reviewed local scripts:

```powershell
.\deploy\bin\windows-bundle.ps1
```

The script builds `anima-link.exe` with the same Tailcat tags as the Unix bundle,
the native `anima.exe` CLI, then the app and a per-user NSIS installer. It does not need an updater key.
The installer is in `target/x86_64-pc-windows-msvc/release/bundle/nsis/`.
The installer and app have no Windows signing certificate for this local trial.

## What this build supports

The shared client UI, access-token and account-refresh storage in Windows Credential
Manager, hosted account sign-in, the device link while the app is open, connections
to existing HTTPS hubs, remote device sessions, device web views and local recent
answers. HTTPS device interfaces use the system browser until native certificate
pinning is implemented, as on Linux. The application log is in Tauri's user log folder.

The link stores its state below `%LOCALAPPDATA%\Anima\link`, with a protected ACL.
Only this user, SYSTEM and administrators have access. The Windows loopback check
accepts only a port served by the app's live device-link child; an unrelated local
hub is refused. Closing the app stops its device link.

Local hub and daemon installation, persistent background link services, the release
builder, CLI PATH installation, automatic coding-agent configuration, stay-awake controls and automatic updates
are not available on Windows in this trial build. The bundled CLI and MCP tool permissions are available. Their Unix implementations are not
compiled into this shell. Windows capabilities hide their controls.

## Test

Run the platform capability tests with `bun run --cwd apps/client test src/lib/platform/platform.test.ts`. Run `go test ./statedir` from `link` on Windows
with `-run TestWindows` for ACL, replacement, locking and reparse-point checks.
Run the shell unit tests with `cargo test -p anima-app --lib`, with `RUSTC_WRAPPER`
empty on Windows. [HT-NEW-windows](../../need-human-testing.md#ht-new-windows-the-native-windows-desktop-client-win1) covers
account sign-in, reconnecting after restart and remote sessions.

Cross-compilation for local testing is also possible with Tauri's `cargo-xwin` runner,
LLVM and NSIS. Use the same Windows config and target. Always run the resulting installer
and tests on Windows; a successful cross-build is not a runtime check.

## Local trial evidence

The Windows 11 VM ran 131 shell tests, including a real Credential Manager
write, reopen and delete, and four link tests for locking/replacement, exposed ACLs,
reparse-point refusal and the local control socket. The NSIS installer exited with
code 0; its installed link file matched the built file's SHA-256. The installed
application opened its welcome screen and responded to Windows.
The PowerShell build script passed Windows' parser; the executable and installer
were cross-compiled on Linux. A complete build on a Windows development machine
and the real-account acceptance steps remain to be run.

The Linux workspace regression run is not green: 5,563 tests passed before
`anima-hub::it sources::controls::src_004_a_paused_account_refuses_sync_now_until_it_is_resumed`
failed at `crates/hub/tests/it/sources/controls.rs:94` (no observed `/pages/1` request).
That hub test and implementation were not changed by WIN1. The run stopped with
920 tests not run. No retry was used to turn that failure into a pass. The Mac
regression check required by `AGENTS.md` also remains pending; this branch is not
ready to publish or merge.

Other local checks passed: Linux Clippy, Rust doctests, all Go gates including race
tests, Windows Go vet/staticcheck/revive, Windows Clippy, client/deploy lint and type
checks, and 727 deployment tests. The client suite completed with 434 files passing
and one old assertion that rejected Windows. That assertion now rejects Android;
the changed device tests and platform tests pass (43 tests). A default-worker client
run was killed; the completed client run used `--maxWorkers=2`.

## WIN2 work in progress

The native CLI now cross-compiles. Windows paths, Credential Manager selection,
console control events, VT input and handle-based ACL checks replace the corresponding
Unix boundaries. MCP configuration replacement preserves an existing ACL. The WIN2 installer bundles the CLI and exposes the shared MCP permissions panel;
automatic coding-agent setup and the other host capabilities remain disabled.
The CLI build uses a static CRT (Tauri configures the desktop runtime separately): the first CLI smoke test could not start because
`VCRUNTIME140.dll` was absent (exit `0xc0000135`). After the static-CRT change, the
VM ran `anima --version` and `anima mcp agents`, and both native ACL test cases passed.
These are smoke/security checks, not full MCP agent acceptance. The Linux CLI's 207
library tests pass, including its 23 MCP installation tests. Limits, names, imports
and document checks pass. The cross-linker warns about absent Microsoft CRT debug
symbols (`LNK4099`); the binaries run without the external CRT DLL.

The VM's Defender history on 2026-11-20 records repeated successful termination of
`anima-link.exe`, with `Behavior:Win32/SuspClickFix.F` (threat ID 2147939205). The app
subsequently showed its local connection as disconnected. The app was stopped to end
its child restart loop. Defender was not disabled and no exclusion was added. This is
an unresolved Windows acceptance blocker; the prior successful connection does not
prove sustained operation. The detection's cause is not established. Microsoft documents
[this behavior detection](https://www.microsoft.com/en-us/wdsi/threats/malware-encyclopedia-description?Name=Behavior%3AWin32%2FSuspClickFix.F)
and provides a [developer analysis submission](https://www.microsoft.com/en-us/wdsi/filesubmission).
No file was submitted.

Remaining WIN2 work includes native CLI distribution and MCP agent paths, desktop setup
controls, persistent link/daemon, power controls, native PostgreSQL/local hub, builder,
update packaging and runtime acceptance. The new internal Windows boundary crate also
needs a distribution design that preserves the CLI's standalone Cargo package; do not
publish either crate from this trial.

## Normal-launch investigation and restart handling

The unchanged installed app was opened from the Windows Start menu on 2026-11-20.
It connected to the hosted hub. Its helper remained PID 5932 for six minutes
(02:04:55 to 02:10:55 local time), with the same SHA-256 as the packaged link recorded above. Defender reported
antivirus, real-time protection and behavior monitoring enabled; its latest detection
still preceded this launch. This supports a launch-context hypothesis for the prior
behavior detection, but does not prove its cause or complete Windows acceptance.
Use the installed shortcut to open the app; the test procedure must not launch it
through a Run-dialog PowerShell command with execution-policy bypass.

The child supervisor now stops after three consecutive runs shorter than one minute,
reports the exhausted restart budget, and logs that it stopped. A stable run resets
that budget. An explicit restart clears the previous failure status immediately.
The Linux link tests pass (36), including the early-exit stop and explicit-restart case;
the client link tests pass (8). The first new restart test found a stale-state race;
`DeviceLink::start` now records Starting before spawning the worker, and the corrected
implementation passes the same test. This is not a Defender exception or a detection
bypass.

For cross-built CLI binaries and native boundary tests, set `RUSTFLAGS="-C target-feature=+crt-static"`. Do not set it globally for the desktop build: it conflicts with Tauri's static-vcruntime configuration.

The Windows CLI also installed and removed its MCP entry in an isolated Cursor-format
configuration below the VM's temporary folder. The pre-existing `other` entry remained
unchanged. This verifies file editing on Windows; it does not replace testing a real
coding agent's MCP connection.

The updated installer completed with exit 0, and its CLI printed `anima 0.1.0` from
the installed folder. After a Windows restart, launching the app from Start reconnected
to the hosted hub; Defender's latest detection still preceded the normal-launch tests.
The installed desktop executable matches the build after the expected three-byte Tauri
bundle marker change from `UNK` to `NSS`; the CLI hash matches its build directly.

The native MCP permissions module passes its Windows tests as part of 136 shell tests.
The client platform, link and MCP tests pass (79). The Windows settings view exposes
tool permissions but hides automatic coding-agent setup until that native installer
integration is complete. Frame 3 of the MCP mockup records this platform variant.
