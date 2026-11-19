# Record: one computer, two hubs (task R11)

| Item | Value |
| --- | --- |
| Document | The test of task R11 of [projects-and-builds-plan.md](../../plans/projects-and-builds-plan.md) |
| Date | 2026-11-18 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Kind | Record. It adds no requirement (PLN-010). |

## Question

Can one computer build for a hub that is not the hub of its sign-in?

## Set-up

Two hubs ran on one Mac (macOS 27.3), on the loopback address only: hub A at `http://127.0.0.1:18441` (the hub of the sign-in) and hub B at `http://127.0.0.1:18442` (the hub that makes releases). Each had its own PostgreSQL 17 cluster, its own key file and its own operator, made with `anima-hub migrate` and `anima-hub operator create`. The programs were debug builds of a branch that has the builds code of `main` (`aae8831e`). Nothing was compiled for the test. Each password and each token was made for the test and is deleted with its folder. No hub had a release bucket or an updater key. The desktop application did not run.

The command line used a configuration folder of its own (`XDG_CONFIG_HOME`) and `ANIMA_TOKEN_STORE=file`, so the key store of the computer was not used.

## What was sent, and the answers

| Step | Request | Answer |
| --- | --- | --- |
| 1 | `anima context add a http://127.0.0.1:18441`, the same for `b`; `anima auth login --context a` and `--context b` | "Signed in to http://127.0.0.1:18441 as opa with a login token.", and the same for port 18442 and `opb`. `auth status` shows the two sign-ins at the same time |
| 2 | `anima builder enrol --context b --name r11-mac --os macos --arch aarch64 --to-file <file>` | "Builder r11-mac (MACOS AARCH64) enrolled from 127.0.0.1". `anima builder list --context a` is empty |
| 3a | `POST /graphql` to hub B with the builder token of B and no operator session: `mutation { claimBuildJob(platform:{os:MACOS,arch:AARCH64}) { id state } }` | `{"data":{"claimBuildJob":null}}` (no job waits). `builder list --context b` then shows the builder online |
| 3b | The same request with the same token to hub A | "Sign in to continue.", code `UNAUTHENTICATED` |
| 3c | The builder token of B with the query `{ builders { id name } }` to hub B | "A builder token may only claim and report build jobs.", code `FORBIDDEN` |
| 4 | The same claim through a second listener of hub B at `[::1]:18443`, thus from a different address | "This builder must call from the address it enrolled from.", code `FORBIDDEN` (BLD-024) |
| 5 | `anima release create --context b --version 9.9.9 …` | "Publishing is not set up on this hub.", exit code 6 (R-118), as expected with no key |

## What the code says

| Part | Where | Fact |
| --- | --- | --- |
| The hub | `crates/hub/src/fleet/builds/builders.rs` | A builder is a token of the kind `builder` and the address of its enrolment. It stands for no operator. The hub does not know or ask which hub the application of that computer is signed in to |
| The desktop application | `apps/client/src/lib/builds/desktop.svelte.ts` (`provideDesktopBuilder`), `apps/client/src/routes/+layout.svelte` | The application has one hub address. The builder loop runs for that address only. The key store keeps the builder token by hub origin, so the tokens of two hubs can be in it at the same time, but only the loop of the hub of the sign-in runs |
| The command line | `crates/anima-cli/src/builders/`, `crates/anima-cli/src/contexts/` | `anima builder enrol`, `list` and `revoke` work with `--context`. The command line has no builder loop |
| The daemon on macOS | `crates/daemon/src/token_store/keychain.rs` | The token is one fixed entry of the Keychain (`ing.anima.daemon`, `daemon-token`). The home folder is not part of it. A second daemon on the same Mac would write over the token of the first. Not tested: a test would write to the Keychain of the operator |
| The daemon on Linux | `crates/daemon/src/token_store/file.rs`, `config/home.rs` | The token is a file in the home folder of the daemon (`ANIMA_DAEMON_HOME`). Two homes are thus possible in the code. Not tested |

## Result

- The hub side: yes. Hub B accepts its builder with no relation to hub A (steps 3a to 3c).
- The command line: yes for the sign-ins and for the enrolment (steps 1 and 2). It cannot build: it has no loop.
- The desktop application as it is: no. It builds only for the hub of its sign-in. From the code; the application was not run.
- `anima-daemon`: the daemon does not build today. One daemon has one hub. On a Mac a second daemon is not possible as the code is. From the code; not tested.

Not verified: the desktop application itself, a job from its claim to its upload, an address that is not the loopback address, TLS, and the daemon.

Three small faults that the test showed, not corrected: with `ANIMA_TOKEN_STORE=file`, `anima auth status` still says "from the key store"; `anima builder enrol --to-file` says that the token "works once", and it claimed two times; `pg_ctl start` of the test needed `LC_ALL=en_US.UTF-8`.
