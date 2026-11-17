# K8 — Daemon push: enrolment, push and renewal

Status: **frozen for Phase 3** (CR-15, R-092; change only through a programme ruling, CON-001). Written by
WP-54 T2. Spec: MET-011 to MET-020, LOG-007, NET-002, NET-003, P3-004. Rust types: `daemon_core::wire`.
The drive kinds and `drives` (MET-027, MET-028) were added by the operator's ruling of 2026-11-12 (PKG-001, CR-24).
The link origin of Section 1 was added by the operator's ruling of 2026-11-15 (one-click plan, S5).
The `daemonVersion` member of Section 4 was added by R-122 (operator, 2026-11-16).

This contract is the whole interface between `anima-daemon` and the hub. The daemon makes no other request.

## 1. Transport

- HTTPS to the hub's public origin (`ANIMA_PUBLIC_ORIGIN`, `https://hub.anima.ing`), tcp 443, at the hub's VPN
  address (MET-020). The daemon verifies the certificate against the web PKI roots and the host name; there is
  no pin in Phase 3.
- In the link mode (`ANIMA_LINK_SOCKET` set on the hub) the daemon's hub origin is `http://127.0.0.1:<port>`,
  the local port of `anima-link device` on the daemon's computer, from 1024. The link authenticates the hub by
  its key and encrypts the request; the hub sees the request come from the device's link address. The daemon
  accepts `http` for no other host, and not for `localhost` or `::1`. The daemon's computer must run
  `anima-link device` at that fixed port whenever the daemon runs: a program of any local user that listens on
  the port when the link does not would receive the daemon token. Nothing detects that; the planned correction
  is a Unix socket between the daemon and the link, which needs a change to this section.
- The daemon sends no cookie, follows no redirect (a 3xx is a failed request), ignores proxy variables
  (`HTTPS_PROXY`, `HTTP_PROXY`, `ALL_PROXY`), and listens on no port.
- Every route below sits behind the NET-002 layer like any other route, and is logged by its template only.
- Bodies are UTF-8 JSON, `Content-Type: application/json`. Member names are camelCase. Unknown members are
  ignored by both sides, so either side can add an optional member without a new version.
- Times are RFC 3339 UTC with `Z` and whole seconds (`2026-11-08T10:00:10Z`).
- A breaking change adds `/daemon/v2/`; the hub keeps `v1` for at least one phase.

| Route | Credential | Body limit | Success |
| --- | --- | --- | --- |
| `POST /daemon/v1/enrol` | Enrolment code, in the body | 4 KiB | 200, `Enrolled` |
| `POST /daemon/v1/push` | Daemon token, `Authorization: Bearer` | 512 KiB (MET-017) | 204, empty |
| `POST /daemon/v1/renew` | Daemon token, `Authorization: Bearer` | 4 KiB | 200, `Renewed` |

The hub reads at most the limit and answers 413 without reading further. A body that has not arrived within
5 seconds is answered 503 `UNAVAILABLE` and written to the event log like a refusal (Section 6).

## 2. Secrets

| Value | Format | Hub keeps | Lifetime |
| --- | --- | --- | --- |
| Enrolment code | `anima_enrol_` + 22 base64url characters (16 bytes, 128 bits, MET-012) | SHA-256 only | 15 minutes, one use |
| Daemon token | `anima_daemon_` + 43 base64url characters (32 bytes, MET-013, P3-004) | SHA-256 only | 365 days |

- Bytes come from the OS random source. Only canonical unpadded base64url of exactly 16 or 32 bytes is accepted,
  so each value has one spelling and no lookup runs for junk.
- The prefixes let a scanner find a leaked value. An API key (`anima_` + 43) never has the length of a daemon
  token. The access guard refuses, before any lookup, a bearer spelled exactly as a daemon token or an
  enrolment code: the prefix, then exactly 43 or 22 base64url characters. Any other text that starts with
  `anima_daemon_` or `anima_enrol_` is read as any other bearer: as an API key when it is `anima_` and the
  43-character spelling of 32 bytes, as a plain API token when it is that spelling in all, and is refused
  otherwise. Either way it is looked up only among API tokens and keys, so a daemon token never reaches
  GraphQL, a session or a read (MET-013).
- (R-126) The program was `anima-agent` and its token was `anima_agent_` + 43 base64url characters. Such a
  token is not a credential: migration 0717 deleted each token and each enrolment code, so each device enrols
  again. The access guard still refuses that spelling before any lookup, and a scanner still finds it.
- No log, event, metric, error message or response other than `Enrolled` and `Renewed` holds a code or a token
  (LOG-007, MET-013). The Rust types that hold them implement neither `Debug`, `Display` nor `Clone`, and wipe
  on drop.
- The daemon keeps the token in the macOS Keychain, or on Linux in a file with mode `0600` owned by the user that
  runs the daemon, written to a temporary file in the same folder and renamed (MET-013).

## 3. Enrolment (MET-012)

The operator makes a code with `enrolDaemon(deviceId)` (K1, signed-in operator only, never an API key, INT-003).
Making a new code for a device makes its earlier unused codes useless.

Request:

```json
{ "code": "anima_enrol_…" }
```

`Enrolled`:

```json
{ "device": "0192f5c4-…", "deviceName": "pop-os", "token": "anima_daemon_…", "expiresAt": "2027-11-08T10:00:00Z" }
```

- The hub marks the code used in the transaction that looks it up, before it makes the token. A second use
  fails, even when the first use failed later or its response was lost (MET-012).
- `deviceName` is the device's name: 1 to 64 characters (so at most 256 bytes) with no control
  character. The hub's rule for a device name has the same length (`daemon_core::wire::DEVICE_NAME_CHARS`)
  and refuses more characters, so the daemon keeps every name the hub can send. A device whose stored name
  breaks this rule (a row older than the rule) is not enrolled: the answer is `ENROLMENT_REFUSED`, its token
  and metrics source stay as they were, and the code is used, as any code is once tried (MET-012).
- A successful enrolment replaces any daemon token the device had, sets the device's metrics source to `daemon`
  (MET-011, which stops SSH collection), and writes one event (source `metrics`, severity info) naming the
  device.
- Every refusal (unknown, expired or used code, or a malformed body) is the same `401 ENROLMENT_REFUSED`, so
  the answer says nothing about which codes exist. Each is written to the event log like a push refusal
  (Section 6), never with the code, at most once a minute per client address (CR-15).

## 4. Push (MET-014, MET-015, MET-017)

`Batch`:

```json
{
  "device": "0192f5c4-…",
  "series": [
    { "kind": "cpu_percent" },
    { "kind": "disk_percent", "label": "/" },
    { "kind": "gpu_percent", "gpu": 0 }
  ],
  "points": [
    { "at": "2026-11-08T10:00:10Z", "mean": [12.5, 40.1, 3.0], "max": [31.0, null, 9.5] },
    { "at": "2026-11-08T10:00:20Z", "mean": [11.0, 40.1, null] }
  ],
  "gpus": [ { "index": 0, "name": "Apple M3 Pro", "vendor": "apple", "hasVram": false } ],
  "drives": [ { "name": "nvme1n1", "kind": "nvme", "sizeBytes": 1000204886016, "mounts": ["/", "/boot/efi"] } ],
  "inventory": {
    "architecture": "aarch64", "os": "macos", "osVersion": "27.0",
    "tools": [ { "name": "cargo", "version": "1.91.0" }, { "name": "bun", "state": "timed_out" } ],
    "buildTargets": ["aarch64-apple-darwin"]
  }
}
```

- `device`: the device ID from `Enrolled`. It must equal the token's device (MET-017).
- `series`: 1 to 256 entries, no two equal. `kind` is a `series.kind` text (`cpu_percent`, `memory_percent`,
  `memory_bytes`, `disk_percent`, `disk_bytes`, `network_in_bps`, `network_out_bps`, `uptime_seconds`,
  `load_1`, `load_5`, `load_15`, `gpu_percent`, `vram_percent`, `vram_bytes`, `gpu_temperature_c`,
  `gpu_power_w`, `gpu_memory_bytes`, `disk_read_bps`, `disk_write_bps`, `disk_busy_percent`). `gpu_memory_bytes` is the memory in use by a unified-memory GPU (Apple);
  it is never a VRAM value (MET-007, R-092 D-6). `disk_read_bps`, `disk_write_bps` and `disk_busy_percent` are
  one drive's activity (MET-028; `disk_busy_percent` is a percent kind). `label` is present exactly for the disk kinds (the mount point), the network kinds (the
  interface name) and the drive kinds (the drive name, as in `drives`): 1 to 128 bytes, no control character. `gpu` (0 to 63) is present exactly for the GPU kinds.
  The hub ignores a series whose kind it does not know, and the values in its column, so a newer daemon can
  talk to an older hub.
- `points`: 1 to 360 entries (one hour), `at` strictly increasing, each `at` a multiple of 10 seconds. A point
  covers the 10 seconds that end at `at`. `mean[i]` is the mean of the 2-second samples of `series[i]` in that
  window, or `null` when the window has none. `max`, when present, has the same length; only `cpu_percent`
  and `gpu_percent` columns may be non-null, and each is at least its mean. Every number is finite and at
  least 0; percent kinds are at most 100; a `gpu_temperature_c` is at most 1000 (°C) and a `gpu_power_w` at
  most 10 000 (W), and a batch with a value past either is `MALFORMED` (MET-002, CR-17). The daemon drops such a
  2-second sample before it makes a mean, so it never sends one.
- `gpus`: optional. The daemon sends it in its first batch after start and after each GPU discovery (at start
  and every 24 hours, MET-006), and again with the next batch until one is accepted, except after `MALFORMED`:
  then it waits for the next discovery, so a list the hub refuses cannot sink every later batch. Absent means
  "unchanged". `vendor` is `nvidia`, `amd`, `intel` or `apple`;
  `name` is 1 to 128 bytes, no control character; `hasVram` is false for `intel` and `apple` (MET-007).
- `drives`: optional (MET-027). The daemon sends the whole list in its first batch after start and again
  whenever the list it reads changes, and with each next batch until one is accepted, except after `MALFORMED`:
  then it waits for the next change. Absent means "unchanged", so a daemon older than drives keeps working.
  At most 32 drives with unique names; `name` is 1 to 128 bytes, no control character; `kind` is `nvme`, `ssd`,
  `hdd` or `disk`, and the hub reads a kind it does not know as `disk`; `sizeBytes` is at most 2^63 − 1;
  `mounts` holds at most 32 mount points, each 1 to 128 bytes starting with `/`, no control character, and is
  empty for a drive that is not mounted. macOS sends no `drives` (MET-031).
- `inventory`: optional; the tool inventory (operator, 2026-11-15, CAP-020 to CAP-031; ruling number to be
  assigned). The daemon probes its tools at start and every 15 minutes and sends the whole inventory in its
  first batch after start, again whenever a probe result changes, and again when 24 hours have passed since it
  last handed one over even if nothing changed; and with each next batch until one is accepted, except after
  `MALFORMED`: then it waits for the next change or the next 24-hour announcement, so an inventory the hub
  refused once is sent again within a day (CAP-027). Absent means "unchanged", so
  a daemon older than the inventory keeps working. `architecture` is `aarch64` or `x86_64`; `os` is `macos` or
  `linux`; `osVersion` (macOS: `sw_vers -productVersion`; Linux: `VERSION_ID` of `/etc/os-release`, or the
  kernel release when there is none) is 1 to 64 bytes of `[0-9A-Za-z.+_-]`. `tools` holds at most 64 tools with
  unique names, each a `name` of 1 to 64 bytes of `[0-9a-z_.-]` and either a `version` of 1 to 64 bytes of
  `[0-9A-Za-z.+_-]` or a `state` that says why the tool, found, is not usable, never both (CAP-024). The
  `state` is a fixed code, never text from the device: `timed_out` (the version probe ran past its 5 seconds),
  `exited` (the program could not start or exited with a failure) or `no_version` (it ran but printed no
  version the daemon reads); `failed` stands for a reason the reader does not know. A tool the daemon did not
  find is not listed. The names the hub knows are `git`,
  `rustc`, `cargo`, `cargo-nextest`, `sccache`, `bun`, `node`, `xcodebuild`, `docker`, `podman`, `nix`, `claude`
  and `codex`; the hub drops a tool of any other name, and reads a `state` it does not know as `failed`, so a
  newer daemon can talk to an older hub, and an older daemon's `failed` is still taken. `buildTargets` holds at
  most 128 unique Rust targets
  (`rustup target list --installed`), each 1 to 64 bytes of `[0-9a-z_.-]`. No text in it is free text from the
  device: the daemon sends only values that pass these rules, and a batch whose inventory breaks one is
  `MALFORMED`. The daemon looks for each program first in its fixed tool folders (`~/.cargo/bin`,
  `/opt/homebrew/opt/rustup/bin`, `/opt/homebrew/bin`, `/usr/local/bin`, `~/.bun/bin`, `~/.local/bin`,
  `~/.docker/bin`, `~/.nix-profile/bin`, `/nix/var/nix/profiles/default/bin`), then in its `PATH`. On macOS it
  does not run Apple's placeholder `/usr/bin/git` or `/usr/bin/xcodebuild` unless the developer tools are
  present (`xcode-select -p` names a folder that exists), since a placeholder offers to install them. Each
  probe runs in a process group of its own that is killed when it ends. `claude` and `codex` write into their
  home folder even to print a version, so they run with `HOME`, `CODEX_HOME` and `CLAUDE_CONFIG_DIR` set to
  an empty temporary folder that is removed afterwards; no other probe has a home folder (CAP-025).
- `daemonVersion`: optional (R-122). The version of the `anima-daemon` that sent the batch, in each batch. It
  is 1 to 64 bytes of ASCII letters, digits, `.`, `-` and `+`, and starts with a digit; any other text is
  `MALFORMED`. A release build carries the release version (`ANIMA_VERSION`, as `anima` does); any other build
  carries the crate version. Absent means a daemon older than R-122, and a hub older than R-122 ignores it.

The hub:

- stores each non-null mean as the sample of its series at `at` (10-second resolution, MET-015); the series
  key follows the SSH collector's rule for the same kind, label and GPU;
- folds `max` into the `max` column of the minute, hour and day aggregates; the samples hold the mean;
- skips a point whose series and `at` are already stored, so sending a batch twice adds nothing twice;
- keys a GPU series by its index: its label is the index as text. A GPU series with a `label` is `MALFORMED`;
- lets a device create at most 1024 series in any hour, counted by when the hub created each series, never
  by the times of its points: a batch whose new series would pass that is `MALFORMED` and stores none of it.
  One daemon has at most 615 series (at most 256 in one batch) (R-098 e, CR-20);
- with `gpus` present, replaces the device's GPU set with it; `[]` means the device has no GPUs now, and an
  absent `gpus` leaves the set as it is. A GPU that leaves the set keeps its series and their history
  (R-098 b);
- with `drives` present, replaces the device's drives with it in the same transaction; `[]` means the device
  has no drives now, and an absent `drives` leaves them as they are. A drive that leaves the list keeps its
  series and their history (MET-027);
- with `inventory` present, replaces the device's tool inventory with it in the same transaction, the tools
  and the build targets included; an empty `tools` or `buildTargets` means the device has none now, and an
  absent `inventory` leaves it as it is (CAP-029). Deleting the device deletes its inventory;
- keeps the `daemonVersion` of the newest accepted batch, or no version when that batch had none, and
  forgets it when it gives the device a new token (R-122). `Device.daemonVersion` serves it;
- records the time of the last accepted batch for MET-018 (R-098 a). Any accepted batch resets it, even one
  whose series the hub all ignores; a refused batch does not. 30 seconds after the last accepted batch the
  device's metrics are stale: the hub writes one event (source `metrics`, severity warning) and serves
  `Device.metricsStale` true. The next accepted batch clears the mark and writes one event (severity info).
  A daemon that has never pushed is not marked, and `metricsStale` is true for it. Stale changes no check:
  a stale daemon is never a Down check.

## 5. Renewal (MET-013)

The daemon calls renew with its current token when fewer than 30 days remain, at most once an hour. The body is
`{}`.

`Renewed`:

```json
{ "token": "anima_daemon_…", "expiresAt": "2027-11-08T10:00:00Z" }
```

- The hub keeps the new token's hash beside the current one. The current token keeps working until the new one
  is first used, for a push or a renewal; then the current one stops working (MET-013).
- A renewal with the current token while an unused new token exists replaces that new token, so a lost response
  costs nothing.
- The daemon stores the new token (Section 2) before it uses it. If it cannot store it, it keeps the current one.

## 6. Refusals

The body of every refusal is `{ "error": "<CODE>" }` and nothing else. It never echoes a value from the request.

| Status | Code | When | Daemon does |
| --- | --- | --- | --- |
| 400 | `MALFORMED` | The body is not a valid `Batch` or `{}` by Section 4 or 5 | Drops that batch; logs the code |
| 401 | `TOKEN_REFUSED` | The token is missing, malformed, unknown, expired or revoked, or the device's metrics source is no longer `daemon` (CR-18) | Stops pushing; keeps sampling and buffering; logs once; tries again every 10 minutes and after the token store changes |
| 401 | `ENROLMENT_REFUSED` | Section 3 | `anima-daemon enrol` exits 1 with a fixed sentence |
| 403 | `WRONG_DEVICE` | `device` is not the token's device | As `TOKEN_REFUSED` |
| 413 | `TOO_LARGE` | The body exceeds its limit | Drops that batch; sends smaller batches |
| 422 | `OUT_OF_RANGE` | A point's `at` is more than 1 hour before or more than 60 seconds after the hub's clock | Drops that batch |
| 503 | `UNAVAILABLE` | The hub cannot store now, or is busy (below) | Keeps the points; tries again |

The hub runs the push checks in this order and stops at the first refusal: the bearer (before it reads the body), the size,
the JSON, the device, the shape, the times. Every push refusal is written to the event log (source `metrics`,
severity warning) with its code, the client address and, when the token was valid, the device, and never with
the token (MET-017). A refusal with no valid token is written at most once a minute per client address, so a
flood cannot fill the event log. A refusal with a valid token, on any daemon route, is written at most once a
minute per device (CR-21). Only the event is held back: the refusal and its answer are the same.

The hub limits the daemon requests in progress (MET-017), and answers `UNAVAILABLE` to a request past a limit
before it waits for the database. That answer is not written to the event log, and the daemon keeps the batch
and tries again after its wait (Section 7).

- **Per client address:** at most 2 requests until their credential is known: an enrolment until its code
  has been looked up and exchanged, a push or renewal until its token has been looked up. All requests from one address share its 2
  turns: the node's daemon, which sends one request at a time, and an enrolment beside it. One VPN node with
  codes or bearers that are wrong cannot keep the hub busy for the others.
- **Per device:** at most one push or renewal at once, from the token lookup until the batch or renewal is
  stored or refused, so requests with one token never queue on the device's rows.
- **On the database:** at most 4 daemon requests at once, well under the hub's pool. A request waits for its
  turn in order, for at most 2 seconds, and holds it only while it looks up its credential and while it
  stores. It never holds it while its body arrives. A request whose token is valid holds its turn until its
  store ends, at most until its answer's deadline (10 seconds): so the holders of 4 valid tokens can keep the
  other daemons waiting that long.
- **Refusal events** are written after the request's turns end, at most once a minute per client address,
  or per device when the token was valid (above). When refusals of 64 client addresses have been written
  within the last minute, the refusals of further addresses are counted rather than written. The next daemon
  refusal that comes a minute or more after the first of them is followed by one event giving that count
  ("Left out N more daemon refusals from other addresses in the last minute."). No timer writes it.

## 7. Daemon buffer (MET-014)

- The daemon samples every 2 seconds and makes one point per 10-second window, ending on a multiple of 10
  seconds of its clock.
- It pushes every 10 seconds: the oldest unsent points first, at most 60 points per batch, then the next batch
  at once while points remain.
- While the hub cannot be reached (a network error, a timeout of 10 seconds, a 5xx), it keeps up to 360
  points (1 hour), dropping the oldest first, and drops any point older than 59 minutes before it sends.
- After a failure it waits 10 seconds, doubling to at most 60 seconds, then resumes the 10-second rhythm. Each
  wait after a failure is moved at random by up to 20 % either way (so at most 72 seconds), so daemons that
  failed together do not all try again together.
