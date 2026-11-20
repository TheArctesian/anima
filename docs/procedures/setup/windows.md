# Build the Windows client

Tasks: WIN1 and WIN2 in [projects-and-builds-plan.md](../../plans/projects-and-builds-plan.md).
This is a local Windows 11 x86_64 client. It is not a public signed release.

## Build on Windows

Install Git, Bun, Go (the version in `link/go.mod`), Rust through rustup, and
Microsoft C++ Build Tools with Desktop development with C++ and a Windows SDK.
The builder gates also need Node.js 24 or newer for client DOM tests, `cargo-nextest`,
and MinGW GCC on `PATH` for Go's race
detector (for example, MSYS2's `mingw-w64-ucrt-x86_64-gcc`, with `C:\msys64\ucrt64\bin`
on `PATH`). These are developer tools; the installed application does not require them.
The Rust version comes from `rust-toolchain.toml`. Windows 11 supplies WebView2;
the NSIS installer can install the runtime if it is absent.

From a checkout, run in a PowerShell session permitted to execute reviewed local scripts:

```powershell
.\deploy\bin\windows-bundle.ps1
```

The script builds `anima-link.exe` with the same Tailcat tags as the Unix bundle,
the native `anima.exe` CLI and `anima-daemon.exe`, then the app and a per-user NSIS installer. It does not need an updater key.
The installer is in `target/x86_64-pc-windows-msvc/release/bundle/nsis/`.
The installer and app have no Windows signing certificate for this local trial.

## What this build supports

The shared client UI, access-token and account-refresh storage in Windows Credential
Manager, hosted account sign-in, connections to existing HTTPS hubs, remote device
sessions, device web views and local recent answers. HTTPS device interfaces use the
system browser until native certificate pinning is implemented, as on Linux.

The bundled CLI supports per-user PATH registration and shared coding-agent MCP setup.
The hosted device link runs in a per-user scheduled task, including while the app is
closed. Its state is below `%LOCALAPPDATA%\Anima\link`, protected for this user,
SYSTEM and administrators. The loopback listener check verifies the expected executable
and Windows user. The daemon is bundled, with native metrics, private local sockets
and a per-user scheduled task; automatic enrollment is under VM acceptance testing.

The native local hub, including PostgreSQL and the session gateway, is installed and reports
Ready in the test VM. Idle-sleep prevention is implemented. The branch also contains the
temporary closed-lid plan guard, Windows update packaging and native builder controls.
The native MSVC package is installed in the VM. Applicable native checks and packaging
passed across the runs recorded below; real-machine acceptance remains Open. Closed-lid behavior requires supported hardware and laptop acceptance.
This is a WIN2 development build; full parity acceptance remains Open.

## Test

Run the platform capability tests with `bun run --cwd apps/client test src/lib/platform/platform.test.ts`. Run `go test ./statedir` from `link` on Windows
with `-run TestWindows` for ACL, replacement, locking and reparse-point checks.
Run the shell unit tests with
`bun deploy/bin/windows-command.ts cargo test -p anima-app --lib`.
The wrapper supplies an actually empty `RUSTC_WRAPPER`; Windows PowerShell 5 removes
an environment variable assigned an empty string, which otherwise lets Cargo select
the repository's Unix wrapper. [HT-074](../../need-human-testing.md#ht-new-windows-the-native-windows-desktop-client-win1) covers
account sign-in, reconnecting after restart and remote sessions.

For the full gates after installing the local build, use Developer PowerShell with
the tools above on PATH:

```powershell
$env:ANIMA_POSTGRES_TREE = "$env:LOCALAPPDATA\Anima.ing\database"
$env:CARGO_BUILD_JOBS = '2'
bun run gates
```

The runner creates its own database and native offline link fixture; it does not use
the installed hub's database. On the 16 GiB test VM, run gate compilation and release
packaging sequentially. The VM uses an additional 16 GiB paging file on its build volume;
overlapping the two large Rust compilations exceeded its earlier commit limit.

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
The later WIN2 work below adds automatic coding-agent setup.
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

## Native setup and persistent-link work

WIN2 now implements user-PATH registration for the bundled CLI through `HKCU\Environment`.
It preserves unrelated PATH entries and the registry string type, refuses malformed values,
and broadcasts the environment change so new terminals can inherit it. Remove unregisters
only this app's folder. No machine-wide PATH or shell profile is changed.
The Windows VM passed four native ACL/PATH tests, including a registry round trip in an
isolated test key. The client setup/MCP tests pass (72); the existing Linux MCP installation
suite passes (23). Live Settings installation and a fresh-terminal check remain open.

Coding-agent setup now uses the Windows roaming settings locations for
[Claude Desktop](https://modelcontextprotocol.io/docs/develop/connect-local-servers) and
[Zed](https://github.com/zed-industries/zed/blob/main/docs/src/configuring-zed.md).
A redirected roaming folder must pass the same owner/ACL checks as the home folder.
The command-line setup panel and the coding-agent controls are included in this build.

The persistent link is being ported to a per-user Task Scheduler task, using an
`InteractiveToken` and `LeastPrivilege`, a logon trigger, and bounded failure restarts.
It stores no password and does not request elevation. Its executable is copied to a protected
user folder outside the app installation. The shared handoff/repair lifecycle is reused;
the Windows listener check verifies the process owner and executable path for that copy.
The Windows task XML/path tests pass in the VM. App-close, sign-in, repair and removal
acceptance remain pending until the new installer is exercised.

### WIN2 native daemon continuation

The scheduled link now writes Task Scheduler XML as UTF-16LE with a BOM. A UTF-8
encoding declaration caused registration to fail with “unable to switch the encoding”.
The corrected task was registered in the VM and its control socket reported a direct
hub connection while the application was closed. The task uses the joined state in
`%LOCALAPPDATA%\Anima\link\device`, not its parent folder.

The daemon now cross-compiles for Windows with private ACL-checked records, bounded
native AF_UNIX HTTP, native CPU/memory/disk/network sampling, tool inventory and
per-user scheduled startup. A migration allows Windows tool inventories. The shared
application installation flow is being wired to this daemon; it is not yet an accepted
end-to-end feature. Windows ran 95 daemon library tests, including a live native socket
request and real metrics sampling. Linux passed 149 daemon and 260 daemon-core library
tests and Clippy. Seven native boundary tests passed in Windows, including a job that
terminates its child and grandchild when its owner closes.

The Windows socket and task changes, native daemon deployment, non-admin execution,
login restart, repair and removal still require the HT-074 acceptance steps.
No cloud migration or public release was performed.

Automatic device registration initially rejected the shell's `WINDOWS` OS value. The
client now accepts it, with regression tests. The native socket HTTP client also needed
an explicit `Host: 127.0.0.1`: its encoded socket authority was correctly refused by the
hub's link-mode guard with HTTP 403. The guarded request has a Windows socket regression
test. With those fixes, the installed app enrolled the daemon automatically and both
per-user tasks reported Running in the VM. The updated 95-test Windows daemon suite,
54 client registration/daemon tests, and five hub inventory integration tests passed.
The device overview then displayed live CPU, memory, C: disk and Ethernet charts. Non-admin, login/restart and removal acceptance remain separate checks.

Native Windows idle-sleep requests and battery readings now feed the shared stay-awake
control and tray. The handle closes when the app exits or crashes, with no persistent
power-plan edits. Closed-lid sleep prevention is not implemented: a SystemRequired
request does not override an explicit lid action, so the app reports that option unavailable.
Nine native boundary tests pass in Windows, including acquiring/releasing the power request and battery decoding. The 58 shared Linux stay-awake tests and 22 client capability/power tests pass. Real
battery, sleep and Modern Standby acceptance is in HT-074.

### WIN2 native database continuation

Native PostgreSQL 17.12 now runs in the Windows VM without WSL. The database supervisor
uses `pg_ctl` to drop administrative groups, assigns its launcher to an owned Windows job
before it runs, and admits the server's process handle only after checking job membership.
Closing or killing the supervisor ends the database process tree. Normal shutdown requests
a checkpoint with `pg_ctl`; the managed parent can send `stop` on the private stdin pipe.

Windows SQLx connections use IPv4 loopback, SCRAM and a private TLS certificate with
`verify-full`. Other users cannot read the bootstrap password, private key or login files.
The native test rejects a different certificate, an unencrypted connection, and privileged
DDL by the runtime login. Backup, restore and restart preserve the test rows. Seven native
library tests and eight real-program tests passed, including duplicate starts, incompatible
cluster versions, forced-supervisor termination and recovery. The Linux database suite
passed all 108 tests. These results do not enable the local-hub UI: its native transport,
supervision and packaged dependencies are still being implemented.

The VM runtime came from the [EDB PostgreSQL binary archive](https://www.enterprisedb.com/download-postgresql-binaries).
Its Windows 17.12 ZIP has SHA-256
`520c8c6f19abb3b31fa83ec5e486e2f769eedd7593d94e60a7cda8a5bfb4df91` (computed after the HTTPS download;
the download page did not publish a checksum). The Microsoft Visual C++ runtime DLLs are
beside the executables; no system-wide runtime installation was needed. Reproducible
packaging and redistribution notices remain part of WIN2.

### WIN2 native gateway and supervisor continuation

The Windows gateway transfers accepted sockets to isolated workers with Winsock
`WSADuplicateSocketW`, a private pipe and an import acknowledgement. The listener does
not read session credentials. The existing 167 gateway integration tests pass in the
Windows VM, including concurrent worker sessions, failed-session isolation, saturation,
RDP and VNC protocol behavior (`win2-gateway-all3.log`). Cross-compiled tests use
`ANIMA_GATEWAY_EXE` to locate the target executable.

The first 17 native local-hub tests pass (`win2-local-hub-tests.log`): the supervisor
starts suspended children inside owned jobs, requests graceful shutdown over private
pipes, kills unresponsive members, accepts stop requests only through a private socket,
and preserves private logs and vault keys. These tests do not yet prove a complete local
hub installation or login-task lifecycle. The database/hub/gateway integration, application
bundling, builder and updater work remain in progress. The 121 affected Linux hub storage,
sealing-key, link and access-boundary tests also passed before the latest I/O extraction.

The native Windows socket boundary passed 10 tests (`win2-socket-tests.log`), including
cancellation followed by a new accept and rejection of a regular file at a socket path.
Linux and Windows Clippy checks passed for the hub, local-hub and gateway libraries and
binaries; Linux also checked their test targets. Later changes must repeat affected checks.

### WIN2 integrated local hub continuation

The eight existing local-hub integration tests now pass in Windows
(`win2-local-hub-it4.log`). They run native PostgreSQL, the real migrated hub and the native
gateway, with an offline link fixture as on Unix. They cover startup, health, ordered stop,
restart with the same database/key, a second run being refused, account commands refusing
an inactive hub, root backup checks, changed/corrupt backups being refused, recovery from a
copy and update of an uninstalled hub. No live relay is involved in this suite. Nineteen
Windows supervisor unit tests also pass (`win2-local-hub-tests2.log`), including trusted
Task Scheduler lookup and replacement of a native executable/DLL tree. The moved-root
Windows database test passes (`win2-database-tests9.log`).

Linux regression checks passed 440 database, gateway and local-hub tests
(`/tmp/anima-win2-parity-linux-tests2.log`), and 73 filtered hub configuration/sealing-key
tests passed after the I/O extraction. The Windows shell now compiles with the shared
local-hub commands registered; 25 frontend capability/local-hub tests pass. Installation
and scheduled-task lifecycle acceptance remain to be completed.

`database-tree/windows.ts` builds the native runtime from the two hash-pinned official
archives in `windows.pin.json`. It checks that the PostgreSQL version agrees with the
Unix source pin, extracts the x64 CRT beside PostgreSQL, carries both distributors'
license files and verifies `postgres --version`. It executes no redistributable installer
and makes no global runtime change. The recipe ran successfully with Bun 1.3.14 inside
Windows (`win2-runtime-recipe.exit` is 0); the target was a fresh test directory. Optional
`ANIMA_POSTGRESQL_WINDOWS_ARCHIVE` and `ANIMA_VC_RUNTIME_ARCHIVE` paths use cached archives
but still require the pinned hashes. An existing output directory is refused.

### WIN2 installed runtime and release-target continuation

The local-hub installer (69.41 MiB) installed with exit code 0 in the Windows VM
(`win2-hub-install2.exit`). The app reconnected to the hosted hub and showed live Windows
metrics. All 139 native shell unit tests passed, including local-hub paths, environment
filtering and private key export (`win2-app-hub-tests2.log`). The eight local-hub integration
tests also passed against the release GUI-subsystem executable in 94.72 seconds
(`win2-local-hub-release-it.log`). Controller invocations from PowerShell must use
`Start-Process -Wait -PassThru` with redirected streams: PowerShell does not synchronously
wait for a GUI-subsystem executable invoked with `&` in the same way as a console program.
The application already waits on its child process and reads its pipes directly.

A native test hub ran as a least-privilege scheduled task, reached ready with its link up
while the desktop was closed, and stopped, restarted and removed with exit code 0.
Standard-user login/logout acceptance remains open. The native runtime recipe's output
passed all seven database tests, including moving the root and restarting.

The release API, CLI and MCP now describe `WINDOWS_X86_64`, with a Windows x64 builder
platform, database constraint migration, NSIS filename checks and a `windows-x86_64`
update feed. The 110 affected hub build/contract tests pass. Eleven real updater-plugin
tests pass in Windows (`win2-updater-tests.log`), including altered bytes, wrong keys,
wrong versions and missing feeds. These use ephemeral test keys and do not install a public
update. The installed development build precedes the updater and builder changes.
The builder's native process/file/environment port is in progress; it is not enabled in
Settings until the Windows gates and packaging are exercised. Closed-lid support remains
unimplemented. No public release, cloud change, Mac regression run or human acceptance is
claimed by this record.


### Native builder and gate continuation

The installed local hub now reports Ready at `127.0.0.1:8443`, with its native link up
(`win2-installed-hub-status.log`). The throwaway task hub was removed after its lifecycle
checks. This is automated evidence; standard-user and UI acceptance remain Open.

The native builder process, file and environment tests pass in Windows: 143 shell tests
(`win2-builder-unit-tests.log`). The 108 corresponding Linux builder tests pass, as do
181 client build/platform tests. A Windows-only gate database runner uses the same private
TLS/SCRAM PostgreSQL boundary as the app and removes its throwaway cluster after the gate.
With it, 94 hub build/publishing unit tests pass inside Windows
(`win2-native-builds-tests4.log`). The full Rust workspace compiles with Windows tests,
and the full Windows workspace Clippy check passes before the subsequent lid-plan work.
The larger native test run is still being completed; this is not a full gate pass.

The release workflow now builds Windows on a Windows runner, requires its native gates,
verifies the installer digest before the separate signing job, and checks the signed
installer before publishing its Windows feed. The local workflow tests pass (69 tests;
one Mac-only check is skipped on Linux). No workflow was dispatched or published.
The Windows build script distinguishes local builds from releases and replaces only a
recognized generated PostgreSQL runtime after preparing its replacement.

Closed-lid support now has a native temporary-plan implementation. All 144 shell tests in
`win2-app-lid-tests.log` pass inside Windows, including actual temporary-plan restoration,
recovery after an interrupted guard, and preserving a later user-selected plan. Guard-process
termination and laptop acceptance remain Open. It copies the active plan, journals both identities
before activation, and uses a separate stdin-owned guard to restore on app termination.
Recovery preserves a later user-selected plan. Hardware without a lid or with locked lid
policy remains unavailable. See the extended HT-074 steps; no laptop pass is claimed.

The next Linux regression selection passes all 153 tests, including Windows release publishing
and the test-role login changes for SCRAM. The Windows terminal fixture uses native ConPTY;
its Clippy check passes and runtime tests are being prepared. The builder accepts Windows
tool-check responses and reports the MinGW compiler needed for Go's race tests; 36 affected
client tests pass. A clean native Windows source snapshot and build toolchain are being
prepared in the VM. These checks do not yet enable the builder capability.

The session-shutdown changes pass 213 Linux regression tests. The first full Windows hub
library run passed 2,494 tests, with 18 failures and one ignored test. Seventeen of those
failures pass on retry after the path, login and connection-timing fixes; the remaining
test-created login has been corrected and a new full run is in progress. Normal WebSocket
closure now waits for the closing handshake within its existing deadline. Oversized frames
are not drained, and their persisted protocol-error cause remains covered on Windows.

All six native console sign-in tests pass (`win2-cli-conpty5.log`): type-ahead secrecy,
Unicode and edited passwords, Ctrl-C, end of input, TOTP and sign-out. The Windows shell
suite passes 146 tests (`win2-app-final-tests.log`). Its separately invoked compiler
acceptance test also passes: with developer-shell paths removed, Anima discovers MSVC and
the SDK, compiles a Windows program and runs it (`win2-compiler-acceptance.log`).
The release workflow explicitly runs that compiler check after its ordinary gates.

The clean Windows build has run all Go race-test packages. All passed except three pure
Unix service-plan tests; their planner now uses Unix path rules independently of its host,
and the retry is in progress. Shared filesystem fixtures now check actual Windows ACLs
instead of treating Unix mode bits as Windows permissions. The affected Linux link suites
and the service planner tests pass. The complete Windows build gates remain unfinished.


Full native Windows hub unit run: `win2-hub-all-tests-final.log` reports 2,512 passed,
zero failed and one ignored. The CLI integration run passed 482 of 489; all seven subsequent platform corrections pass, including PowerShell paging
with the required Windows executable extensions in the child environment. The native Go format/vet/staticcheck/revive/race/build gates all
pass. Windows JavaScript lint and type checks pass. Native client DOM tests require
Node.js rather than Bun's Node fallback. Unix deployment shell fixtures now select
Linux/macOS explicitly; shared contracts and native Windows tests still run on Windows.
The Linux Headscale fixtures passed 750 tests; after supplying ShellCheck, the three
remaining lint tests also pass. The VM has no lid sensor, so the actual guard-child
crash check cannot activate here; journal recovery tests pass and laptop acceptance
remains Open.


The native client run with Node.js reports 4,903 passed and one failed. That failure
caught a shared daemon-install message regression: Unix keeps its chmod/ACL repair
hint, while Windows gets Properties → Security guidance. The corrected Linux daemon
UI tests pass all 61 cases. The native quality test package passes all 229 cases.
The release form now offers Windows x86_64 and renders its Windows icon; all 185
related build/desktop client tests pass locally. Native Rust gates are running with
MSVC and the installed SDK; compiler debug symbols are disabled to fit the VM disk.
Final native packaging, the updated installed application and the Mac gate remain
pending. Do not infer a full parity pass from these intermediate checks.


The Windows builder capability is now wired to the native implementation, with the same
main-webview-only command permissions as the other desktops. Platform tests pass; client
lint and type checks report no errors or warnings. A fresh local installer containing
these controls, the current sidecars and the lid-plan guard built successfully (69.48 MiB).
Installation and the native MSVC bundle run are still in progress.

The VM now has 16 GiB RAM and eight virtual CPUs. Its expanded disk preserves the recovery
partition; the extra space is an NTFS build volume. The hub, daemon and device-link tasks
returned to Running after the reboot. The full native Clippy check passed again; the Rust
workspace tests are compiling. These are intermediate results, not a full gate pass.


The refreshed installer completed with exit 0. All seven installed programs match the
current sidecar SHA-256 values (`win2-final-verify.log`); the installed CLI reports
`anima 0.1.0`. The app reconnected to the hosted hub, showed the Windows device Up with
live metrics, and opened Settings. The native local hub reports Ready with its link Up,
and the hub, daemon and device-link scheduled tasks are Running. A window that appeared
to disappear during pointer automation was minimized; restoring the Tauri window and
using native Windows pointer coordinates opened Settings normally.

The first full native nextest run stopped after 174 passes on the Unix XDG-directory
fixture. That fixture is now Unix-only, since Windows uses LOCALAPPDATA. The next full
run is collecting all 5,763 applicable tests without fail-fast. Its result, the native
MSVC bundle and the Mac regression check remain pending. The local trial UI is also
being rebuilt with VITE_ANIMA_LOCAL_BUILD so it does not attempt public update checks.


The Settings command-line Install/Confirm flow was exercised in the VM. A fresh PowerShell
session resolves `anima` to the installed application and prints `anima 0.1.0`
(`win2-cli-installed.log`). The five copied hub programs and the scheduled device-link
program match the bundle. The daemon's existing task still used the earlier development
binary, so its executable was replaced atomically while stopped, without changing its
enrollment or task; the current daemon remained running (`win2-refresh-daemon.log`).
This is a manual development-build refresh, not evidence of automatic daemon upgrades.

The full native run caught a socket fixture race: Winsock accepted a nonblocking socket
from the nonblocking listener, while the fixture expected blocking reads with a timeout.
The fixture now explicitly selects blocking mode before reading; the production connector
is unchanged. Its native rerun and the remaining gate results are pending.
The Linux full run also needs `ANIMA_POSTGRES_TREE=/tmp/anima-pg17` for the independent
database durability fixtures; setting only PATH serves the shared gate database but does
not select their runtime. The correctly configured Linux run is in progress.

The correctly configured Linux run completed all 21 gates successfully
(`/tmp/anima-win2-linux-final-gates4.log`): 6,507 Rust tests passed, one was skipped,
then doctests, pinning, all six Go checks, installation, lint, type checks, JavaScript
and shell tests, client build and all five quality gates passed. This includes the
account-sync test that failed in the earlier WIN1 run. Windows' collecting run has
also found six release-command fixture failures reaching its local fake bucket;
their clean child environment is under investigation. The full Windows result and
native installer remain pending. Mac regression and human acceptance remain Open.

The collecting native workspace run finished 5,763 tests: 5,692 passed and 71 failed,
with two skipped and one passing gateway test reported as leaky. The failures exposed
shared fixture problems: child processes lost the Windows OS environment, fake socket
permissions used a regular-file helper, a few configurations used Unix absolute paths,
wire-counting connections discarded credentials and could not use the TLS-only test
server, and local-hub tests required a manually supplied offline link executable.
The fixture corrections are being checked natively. The runner now builds its own native
offline link stand-in; no external fixture executable should be needed for the Windows gate.
The gateway test now explicitly stops and reaps its listener before returning.

The disposable Windows gate database permits SCRAM-authenticated plaintext only on
loopback, for shared fixtures and the query-counting proxy. This is confined to the
`test-server` feature and the new throwaway cluster; installed databases and the separate
durability/security fixtures retain their normal TLS-only rules.

A later Linux run found the full-schema query-budget test splitting too many batches under
load. The two full-schema batching tests now reserve nextest's worker slots, while keeping
the same query budgets and real 10 ms production batching window. With that scheduling,
all 6,507 Rust tests and the selected format, Clippy and quality gates passed (8/8).
The final related fixture run passed 113 tests, and all 230 quality-runner tests passed.
These results supplement the earlier 21/21 Linux gate run; they do not replace the pending
native Windows result, Mac regression or human acceptance.

The native focused reruns passed: 97 socket and publishing tests, followed by 201 platform
regression tests. These cover all failure categories from the collecting run, including
local-hub restart and copy recovery, query counting, device-link permissions, clean child
environments and Windows tool discovery. The full workspace run and native installer build
remain pending. The first concurrent debug-test and release builds exhausted Windows'
commit limit; the VM now has a second 16 GiB paging file, and compilation runs sequentially
with two compiler jobs. This was a build-host memory limit, not an installed app failure.

The next full native run passed formatting and Clippy, then stopped after 3,920 Rust
passes on SQLx closing a fixture setup connection (Windows reset 10054), before its test
body ran. SQLx sends PostgreSQL Terminate before TLS close_notify, so the server can close
the socket before that TLS write. The shared throwaway gate now uses loopback plaintext
with SCRAM authentication, like the Unix gate's local unencrypted socket. It does not
change installed connection settings or the independent TLS enforcement tests. The
focused and full native checks are running again with that fixture correction.

All 208 focused native tests passed with the local fixture transport, and its Clippy check
passed. A later full run found a WebSocket close race on Windows: after rejecting a frame
above 64 KiB, the hub dropped the socket with unread input, sometimes replacing the outgoing
1002 close with a TCP reset. The shared close path now retains a protocol-invalid socket
for the existing bounded two-second closing grace, without draining or allocating its
oversized payload. Native full-suite collection and Linux session regressions are checking
this change; the expected close code and input limits have not been relaxed.

The shared WebSocket change passed 176 Linux session regressions and a new complete Linux
run: all 21 gates passed (`/tmp/anima-win2-linux-final-gates5.log`), including all 6,507
Rust tests. The oversized-frame close test also passed in the collecting Windows run.
That Windows run found an intermittent Access Denied at the native database test's folder
rename after its clean shutdown assertions. The fixture now waits up to five seconds for
temporary Windows file locks before performing the same move and data-preservation checks;
persistent access failures still fail. This fixture correction is awaiting its native check.

The collecting Windows workspace run completed 5,763 tests: 5,762 passed, one failed and
two were skipped (`win2-native-rust-final.stderr`). The only failure was the immediate
folder move described above. After that fixture correction, Windows formatting and Clippy
passed, then all seven native database tests passed, including the TLS, move, restart,
backup and restore test (`win2-native-database-final.stderr`). This is a full collection
plus a corrected targeted rerun, not a claim that the collecting run itself was green.
Windows doctests, client gates and native packaging continue.

Windows doctests and pinning passed, followed by 4,906 client tests in 439 files, 229 quality
runner tests, 347 design tests, 29 DNS tests and 391 Headscale tests (the Unix-only tests
were skipped). The deploy package exposed missing Unix-shell boundaries and Windows path
separator assumptions. Certificate-container shell tests and gated-run setup now stay on
Unix, while Docker/workflow structural tests still run on Windows with POSIX container
paths. The corrected deploy package passed 348 Windows tests with 385 platform skips;
its affected Linux tests, lint and type checks passed. The remaining native build/quality
gates and installer packaging are still running.

The native MSVC build completed all sidecars and the desktop application, then produced
`Anima.ing_0.1.0_x64-setup.exe` (69.58 MiB). Its wrapper waited on the compiler's leftover
VCTIP telemetry helper after the build ended; stopping that helper released the wait and
recorded exit 0. No compiler or application process was stopped to force a successful build.
The installer is in `/home/okita/Windows/Anima-build/win2-native`, with `SHA256SUMS` and a
README. Its checksum was checked independently from Linux.

The native installer exited 0. All seven installed programs and all five hub service copies
match their native build hashes; the link and daemon task copies also match. The app opened
from the installed Start shortcut, connected to the hosted hub, and displayed the Windows
device Up with live CPU, memory and disk metrics. The local hub reports Ready with link Up;
its task and both device tasks are Running. A fresh PowerShell resolves `anima` to the
installed CLI and prints `anima 0.1.0`. Records: `win2-native-install.log`,
`win2-native-verify.log`, `win2-native-running-copies.log`, and `win2-native-cli-path.log`.
The existing daemon task copy was refreshed manually without changing its enrollment;
this is development-build maintenance, not automatic daemon-upgrade acceptance.

The temporary C: test compilation cache was removed after verification, leaving about
20.4 GiB free on C: and 42.1 GiB on the build volume. Source, release cache, installer and
logs remain. No public release or cloud deployment was performed. The Mac regression and
HT-074 remain Open, including standard-user, real-agent, laptop power-policy and
private-update acceptance. Windows GPU readings still depend on supported readers/drivers;
missing OS or hardware counters are not fabricated.

### Dev landing and release workflow, 2026-11-20

The operator explicitly authorized proceeding to dev with Mac regression checks pending.
No Mac run is claimed. Run `bun run gates:fast` on a Mac for the shared desktop, daemon and
link changes before release. The native release workflow now requires Authenticode signing;
certificate provisioning and a signed update trial remain pending. See
[the release procedure](../releases.md#windows-builds-and-releases-win2).

The dev integration run passed 6,600 Linux Rust tests, the Go gates, lint, types, client and
deployment tests, and the client build. Its size gate caught the merged link module; the
platform socket/ownership code moved into its existing platform modules. The final scoped
run then passed all 13 selected gates, including 504 desktop tests and all quality gates.
The Windows MSVC release-profile check passed for the merged desktop code. PowerShell
parsing and the refusal of a release without a publisher certificate passed in the VM.
Both the Task Scheduler XML-folder regression and Windows packaging release-map regressions
failed before their fixes and passed afterward. The dev merge train now calls the native
Windows build workflow and blocks landing on its failure; its artifacts are the trial installer
and CLI archive. GitHub execution of this new workflow and the signed trial remain pending.

After the next dev integration (50783480), all 21 Linux gates passed together, including
6,611 Rust tests. The only conflict combined new link-port tests with the existing Windows
socket fixture. The Windows release job explicitly uses the fetched remote quality baseline
so it does not depend on a local branch that exists only on a developer machine.
