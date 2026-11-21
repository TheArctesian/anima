# K8 — Daemon push: enrolment, push, renewal and update

Status: **frozen for Phase 3** (CR-15, R-092; change only through a programme ruling, CON-001). Written by
WP-54 T2. Spec: MET-011 to MET-020, LOG-007, NET-002, NET-003, P3-004. Rust types: `daemon_core::wire`.
The drive kinds and `drives` (MET-027, MET-028) were added by the operator's ruling of 2026-11-12 (PKG-001, CR-24).
The link origin of Section 1 was added by the operator's ruling of 2026-11-15 (one-click plan, S5). R-172
(operator, 2026-11-18) changed it from a local port to a Unix socket (task W0).
The check of the peer of the socket and the refusal of the old link origin on macOS (Section 1) were added by
R-NEW-socketpeer (made overnight by the orchestrator, not yet confirmed by the operator).
The `daemonVersion` member of Section 4 was added by R-122 (operator, 2026-11-16).
The `agentUsage` member of Section 4 was added by R-135 (operator, 2026-11-17; AIS-018, AIS-019, AIS-022 to
AIS-024).
Section 8 (the update), the `update` member of Section 4 and the two update routes were added by R-139
(operator, 2026-11-17; ROL-008 to ROL-013, ROL-028 to ROL-032, ROL-040 to ROL-045). They are ruled and
not built: no hub and no daemon has them yet.
Section 9 (operational keys), the `keys` member of Section 4 and of `Accepted`, the fetch route and the refusal
`NO_KEY` were added by R-147 and R-148 (operator, 2026-11-18; K-E5 Section 2.1). They are optional members, so
they need no new version of this contract (Section 1).

This contract and the work route contract K10 ([work-route.md](work-route.md); accepted by R-209, 2026-11-19) are the whole interface between `anima-daemon` and the hub. The daemon makes the requests of K8 and of K10 and no other.

## 1. Transport

- HTTPS to the hub's public origin (`ANIMA_PUBLIC_ORIGIN`, `https://hub.anima.ing`), tcp 443, at the hub's VPN
  address (MET-020). The daemon verifies the certificate against the web PKI roots and the host name; there is
  no pin in Phase 3.
- In the link mode (`ANIMA_LINK_SOCKET` set on the hub) the daemon reaches the hub through `anima-link device`
  on its own computer, over a Unix socket (R-172, operator, 2026-11-18; task W0 of
  [projects-and-builds-plan.md](../docs/plans/projects-and-builds-plan.md)). The daemon's hub origin is
  `unix:<path>`: the absolute path of the socket that the link makes when it starts with
  `-daemon-socket <path>`. The daemon sends plain HTTP on the socket. The link authenticates the hub by its
  key and encrypts the request; the hub sees the request come from the device's link address.
  - The socket. The link makes it with mode 0600, in a folder that only its own user can reach: the folder
    has no access for the group or for others, that user owns it, and no folder above it lets a different
    user replace it. The link does not start with a different folder, and it does not replace a file that is
    not a socket. The link and the daemon run as the same user. For a user installation the folder is the
    daemon's own folder (Section 2) and the path is `<folder>/link.sock`. For a system installation on Linux
    the link for the daemon runs as the account of the daemon (`anima-daemon`) and the path is
    `/var/lib/anima-daemon/link.sock`, in the state folder of the unit, which the sandbox of the unit does
    not hide. No installation starts that link today: who starts the link of a computer whose desktop
    application does not run is HM-D16 of [hosted-mode.md](../docs/proposals/hosted-mode.md).
  - How the daemon knows that the other end is the link. Before it makes its client, the daemon checks the
    folder of the socket by the rule of its own folder (Section 2): a real folder, owned by the user that
    runs the daemon, with no access for the group or for others, below no folder that others can write to
    without the sticky bit. If a thing is at the path, it must be a socket that the same user owns. If one
    check fails the daemon stops with an error and sends nothing. Then, for each request, after it connects
    and before it writes a byte, the daemon reads the peer of the connection from the kernel (R-NEW-socketpeer):
    the user of the peer must be the user that runs the daemon; on macOS, a daemon with a Developer ID
    signature also requires that the process of the peer is `anima-link` (its signing identifier) with a valid
    Developer ID signature of the same team, and that its file is the program that runs (`codesign --verify`
    with the requirement of the release). The daemon takes its own team from its own signature only when
    `codesign --verify` of the running daemon agrees, so a file put in the place of the daemon stops it with an
    error; it does not make it a daemon with no signature. If the peer fails, the daemon closes the
    connection, sends nothing, and the request fails (Section 7). A daemon with no Developer ID signature (a
    build for development) and each daemon on Linux check the user only: on Linux a program of the same user
    can read the token file in any case (Section 2). Thus a program of a different user can neither open the
    socket nor listen at the path, and on macOS a program of the same user that only listens at the path
    gets nothing. A program of the same user that tries harder can still get the token: it can let a signed
    program take the process that made its socket, or start the signed link with settings of its own, and on
    Linux it can read the token store. A job must thus run as a different user (task W2).
  - The daemon sends no request to a TCP port in the link mode. It accepts `http` for no host:
    `enrol --hub http://127.0.0.1:<port>` is refused.
  - A daemon that was enrolled with the old link origin, `http://127.0.0.1:<port>`. Its configuration file
    and its token record keep that text; an update does not change them (ROL-018). The daemon reads the
    text as "the link of this computer": it uses the socket `link.sock` in its own folder and sends nothing
    to the port. On macOS it refuses the old link origin and stops with an error that asks for a new
    enrolment (R-NEW-socketpeer): its folder can come from the environment (`ANIMA_DAEMON_HOME`), and no value of
    the environment may decide where a token from the Keychain goes. The link of that computer must then start with `-daemon-socket <folder>/link.sock`. Until
    it does, a push fails and the daemon keeps its samples (Section 7).
  - `anima-link device` keeps its local port for the desktop client. The daemon does not use it.
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
| `POST /daemon/v1/push` | Daemon token, `Authorization: Bearer` | 512 KiB (MET-017) | 204, empty; or 200, `Accepted` (Section 8) |
| `POST /daemon/v1/renew` | Daemon token, `Authorization: Bearer` | 4 KiB | 200, `Renewed` |
| `POST /daemon/v1/update/record` | Daemon token, `Authorization: Bearer` | 4 KiB | 200, `ReleaseRecord` (Section 8) |
| `POST /daemon/v1/update/file` | Daemon token, `Authorization: Bearer` | 4 KiB | 200, the bytes of the file (Section 8) |
| `POST /daemon/v1/keys/fetch` | Daemon token, `Authorization: Bearer` | 4 KiB | 200, `FetchedKey` (Section 9) |

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
  token is not a credential: migration 0718 deleted each token and each enrolment code, so each device enrols
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
  },
  "agentUsage": {
    "state": "reading",
    "sessionDays": [
      {
        "source": "claude_code", "id": "6f0c2d9e-3b1a-4c55-9e7d-2a8b4c6d8e10", "day": "2026-11-08",
        "startedAt": "2026-11-07T22:14:03Z", "endedAt": "2026-11-08T09:58:41Z",
        "repository": "github.com/example/anima", "prompts": 6, "toolCalls": 41,
        "models": [
          { "model": "claude-opus-5-5", "inputTokens": 1520, "outputTokens": 18311,
            "cacheReadTokens": 2210450, "cacheWriteTokens": 90112 }
        ]
      }
    ]
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
- `agentUsage`: optional (R-135; AIS-018, AIS-019). The use of the AI agent programs on the device: counts
  only, never the text of a session, a title, the path of a folder, a login, the name of an account or a
  usage limit (AIS-018, AIS-022). Absent means "nothing new": a daemon older than R-135 sends none and a hub
  older than R-135 ignores it, so the change needs no `/daemon/v2/`.
  - `state` says what the daemon can read: `reading` (it reads the session files of its user),
    `system_unit` (it runs as the systemd system unit, which cannot see the files of a user, AIS-024) or
    `turned_off` (the operator turned the function off on this computer, or its settings file cannot be
    read). It is 1 to 32 bytes of `[a-z_]`; the hub ignores a `state` it does not know and keeps the one it
    has. Only `reading` comes with session days: with any other `state`, `sessionDays` is `[]`.
  - `sessionDays` holds 0 to 16 entries, no two with the same `source`, `id` and `day`. Each entry gives the
    counts of one session on one UTC day, so the hub can sum exactly by day and by model:
    - `source`: 1 to 32 bytes of `[a-z_]`. The sources the hub knows are `claude_code` and `codex`; the hub
      drops an entry of any other source, so a newer daemon can talk to an older hub.
    - `id`: the session ID of the program, 1 to 64 bytes of `[0-9A-Za-z_-]`. A sub-agent's use counts under
      the session that started it: for Claude Code the session of the folder `<id>/subagents/`, for Codex
      the root of the chain of `parent_thread_id`.
    - `day`: the UTC day, `YYYY-MM-DD`, a real date. A count belongs to the day of the time of its line.
    - `startedAt` and `endedAt`: the first and the last time of the whole session (all its days), K8 times
      (Section 1), `startedAt` not after `endedAt`, and `day` from the date of `startedAt` to the date of
      `endedAt`.
    - `repository`: optional. The repository of the session's working folder (AIS-002, R-134): the `url` of
      the remote `origin` in the git configuration of the folder (of the main repository for a worktree),
      written `host/owner/name` with no scheme, user, password, token, port or `.git`. It is 1 to 200 bytes:
      a host of `[a-z0-9.-]`, then 2 to 4 segments of 1 to 100 bytes of `[A-Za-z0-9._-]`, none `.` or `..`,
      each after one `/`; the first segment is none of `Users`, `home`, `root`, `var`, `srv`, `opt`, `mnt`,
      `Volumes`, `tmp`, `private` and `data`, in any letter case. The name is never a folder path of a
      host (AU-5 security review): the daemon sends no `repository` for a local path, a `file://` URL, an
      scp-form remote (`host:path`) whose path starts with `/`, and a URL whose path breaks this rule.
      Absent when the folder has no such remote.
    - `prompts`: the prompts the operator typed on that day; `toolCalls`: the tool calls of the program on
      that day, sub-agents included.
    - `models` holds 0 to 16 entries with unique `model` names: the tokens of that day by model. `model` is
      1 to 64 bytes of `[0-9A-Za-z._:-]`; the daemon sends the name `other` for a model whose name breaks
      this rule, and for the smallest models when a day has more than 16. `inputTokens` are the input tokens
      not read from or written to the cache; `outputTokens` include reasoning; `cacheReadTokens` and
      `cacheWriteTokens` are the input tokens read from and written to the cache.
    - Every count (`prompts`, `toolCalls` and the four token counts) is a whole number from 0 to
      9 007 199 254 740 991 (2^53 − 1).
  - A batch whose `agentUsage` breaks one of these rules is `MALFORMED`. The daemon sends only values that
    pass them.
  - The daemon reads only the session files `~/.claude/projects/*/*.jsonl` (with the sub-agent files
    `~/.claude/projects/*/<id>/subagents/**/agent-*.jsonl`, at most 3 folders down) and
    `~/.codex/sessions/**/rollout-*.jsonl` (at most 4 folders down), and of the nearest folder at or
    above each working folder that has a `.git`, and is below the home folder, only its `.git` link, the
    `commondir` file of a worktree and the git `config` file (AIS-022). It never opens a credential or
    settings file of the programs. It opens each of these files without following a symbolic link
    (`O_NOFOLLOW`), and only a regular file. It follows the folder that a `gitdir:` line or a
    `commondir` file names only when that folder is below the home folder, outside the guarded folders
    below, and is the same folder when every link on its path is resolved (AU-5 security review);
    else the session has no `repository`. On macOS it reads no git file under `~/Documents`,
    `~/Desktop`, `~/Downloads` or `~/Library/Mobile Documents`, so the system asks the operator for no
    access; such a session has no `repository`.
  - Only a daemon that runs as the user of the operator reads the files: the launchd agent and the systemd
    user unit (AIS-024). A daemon whose `HOME` is not set, or is the state folder of the system unit
    (`/var/lib/anima-daemon`), or whose `ANIMA_DAEMON_HOME` is that folder, sends `system_unit`.
  - The operator's settings are the file `agent-usage.json` in the daemon's folder (MET-002):
    `{ "off": true, "excludedFolders": ["/Users/me/private"], "excludedRepositories": ["github.com/me/secret"] }`,
    each member optional, at most 64 KiB and at most 256 entries in each list. Each entry of
    `excludedRepositories` is written as `repository` is (above; letter case ignored) and does not end
    in `.git`. A session is not counted and not sent (AIS-008, AIS-023) when its working folder, or the
    main folder of its repository, each as written and with its links resolved when it exists, is under
    an excluded folder, as written or with its links resolved (by whole folder names; on macOS without
    letter case); or when its repository is excluded (letter case ignored); or when any repository is
    excluded and the name of its repository cannot be read (AU-5 security review). The daemon stops
    reading a file at the first line that names an excluded folder, and sends nothing of a file before
    a line of it has named its folder and the session is found not excluded. A settings file that
    cannot be read or parsed, that has a member not named here, or an `excludedRepositories` entry of
    another form, turns the function off (`turned_off`) and is logged once at start.
  - The daemon reads the files at start and every 15 minutes; it reads each file from where it stopped
    and skips a file whose size and modification time did not change. It sends `agentUsage` in its first batch
    after start (with its `state`, even with no session days), and each session day whose counts changed;
    at most 16 session days in one batch, the rest in the next batches; and the same until a batch is
    accepted, except after `MALFORMED`: then it drops the session days of that batch until their counts
    change. After a restart it sends every session day again, so the hub's upsert must be idempotent.
- `update`: optional (R-139). What the daemon says about its own update, in each batch (Section 8). Absent
  means a daemon that cannot update itself (ROL-044), and a hub older than R-139 ignores it.
- `keys`: optional (R-147). `{ "held": [ { "name", "version" } ] }`: the versions in the daemon's key cache, in
  each batch (Section 9). At most 256 entries, no name twice, each `name` a key name of K-E5 Section 2 and each
  `version` at least 1; any other value makes the batch `MALFORMED`. Absent means a daemon with no key cache: it
  is told no keys. A hub older than R-147 ignores it.

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
- with `agentUsage` present, in the same transaction (R-135; AIS-020, AIS-023): keeps its `state` as the
  device's usage state when it knows the state; and for each session day of a known source, upserts the
  session keyed by device, `source` and `id` (one session, never a copy: a later collector of the same
  session uses the same key, AIS-023), sets its start, end and repository to those of the entry (no
  `repository` clears it), and replaces the prompts, tool calls and model rows stored for that session
  and `day` with those of the entry. Sending the same entry again changes nothing. A session's model is
  the model of its rows with the most output tokens, the first by name on a tie. Absent `agentUsage`
  changes nothing. Deleting the device deletes its sessions and its usage state;
- keeps the `daemonVersion` of the newest accepted batch, or no version when that batch had none, and
  forgets it when it gives the device a new token (R-122). `Device.daemonVersion` serves it;
- with `update` present, keeps it as the update state of the device (Section 8), and forgets it when it gives
  the device a new token; with `update` absent, the device has no update function (ROL-044);
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
| 404 | `NO_KEY` | A fetch (Section 9) names a key that does not exist, has no active version, or is not granted to the token's device: one answer for the three | Drops that fetch; asks again at the next answer that names the key |
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

## 8. Update (ROL-008 to ROL-013, ROL-028 to ROL-032, ROL-040 to ROL-045; R-139)

The hub tells a daemon one thing: the version to have. The daemon gets the release record and its program
file from its hub, and the machine verifies them by itself (ROL-010). The fields of the release record and
its signature are not part of this contract: the release record has them (ROL-003, ROL-004; task U2 of
[daemon-and-hub-update-plan.md](../docs/plans/daemon-and-hub-update-plan.md)). To this contract the record is bytes.

### 8.1 What the daemon says

The `update` member of a `Batch`:

```json
{
  "update": {
    "target": "linux-x86_64",
    "switch": "on",
    "previous": "0.2.3",
    "busy": false,
    "last": { "version": "0.3.0", "result": "rolled_back", "cause": "no_check", "tries": 1 }
  }
}
```

- `target`: the target of the installed program: `linux-x86_64` or `darwin-universal` (OD-ROL-8). It is 1 to
  64 bytes of ASCII letters, digits, `-` and `_`. The hub reads a target of that form that it does not know as
  "no release for this machine".
- `switch`: `on` or `off`. `off` means that updates are set off at the machine (ROL-028). The hub cannot
  change it, and it tells no version to such a machine.
- `previous`: optional. The previous version that the machine keeps (ROL-017), with the rule of
  `daemonVersion` for its text. Absent means that the machine keeps none, so it cannot roll back.
- `busy`: true while the machine runs a job, a build or a job of a runner (ROL-028). A busy machine does not
  start an update; it starts when it is no longer busy.
- `last`: optional. The result of the newest try. Absent means that the machine did not try an update since
  its installation. `version` is the version that it tried. `result` is `installed`, `rolled_back` or
  `refused`. `tries` is the number of tries of that version since the operator last asked (1 or 2, ROL-032).
  `cause` is present exactly for `rolled_back` and `refused`, and is a fixed code, never text from the machine:

| Result | Cause | Meaning |
| --- | --- | --- |
| `refused` | `record_signature` | The signature of the record is not good (ROL-010) |
| `refused` | `record_mismatch` | The record names a different version, target or program (ROL-010, ROL-013) |
| `refused` | `file_mismatch` | The size or the SHA-256 of the file is not that of the record (ROL-010) |
| `refused` | `lower_version` | The version is lower than the installed one and is not the previous version (ROL-012) |
| `refused` | `not_replaceable` | The record says that the release cannot replace the installed version (ROL-041) |
| `refused` | `fetch_failed` | The record or the file did not arrive whole in 3 tries |
| `refused` | `no_space` | The machine cannot keep the new file and the previous version |
| `rolled_back` | `start_failed` | The new version failed to start 3 times (ROL-030) |
| `rolled_back` | `no_check` | 3 minutes passed with no good check (ROL-029, ROL-030) |
| `rolled_back` | `asked` | The hub told the previous version, and the machine has it again (ROL-031) |

  The hub reads a cause that it does not know as `failed`, so a newer daemon can talk to an older hub.
- A value that breaks these rules makes the batch `MALFORMED`.

### 8.2 What the hub says

The hub answers a push with 204 and no body when it has nothing to tell. When it wants the machine to have a
different version, and the batch had an `update` member, it answers 200 with `Accepted`:

```json
{ "update": { "version": "0.3.0", "again": false } }
```

- 200 and 204 mean the same for the batch: it is stored. A daemon that sent no `update` member always gets
  204, so a daemon older than R-139 keeps working.
- `version` is the only instruction (ROL-009). The hub never sends an address, a path, a command or a file
  name. The text has the rule of `daemonVersion`; a daemon ignores an `Accepted` that breaks it.
- `again`: true only after the operator pressed "Try again" (ROL-032). The machine then sets `tries` to 0 for
  that version. With `again` false, a machine that already tried the version 2 times does nothing.
- The hub repeats the same `Accepted` with each push until the daemon reports that version in
  `daemonVersion`, or reports a `last` for it with the result `refused`, or `rolled_back` with `tries` 2.
  Thus a lost answer costs nothing, and a hub that stopped while a machine was in its check gives the update
  one more time (ROL-045).
- A version higher than the installed one is an update. A version that equals `previous` is a rollback
  (ROL-031). The machine refuses each other lower version (`lower_version`).
- The hub tells a version only to a machine whose `switch` is `on`, that is not held, and whose step of the
  rollout started (ROL-023, ROL-028). A machine that connects after its rollout gets the version of the
  newest rollout that includes it (ROL-040). When the record of that release says that it cannot replace the
  installed version, the hub tells the releases between them in order (ROL-041).

### 8.3 How the daemon gets the record and the file

Request of both routes:

```json
{ "version": "0.3.0", "target": "linux-x86_64" }
```

`ReleaseRecord`:

```json
{ "record": "<base64 of the bytes of the record>", "signature": "<the text of the signature file>" }
```

- The hub gives the record and the file of a version only to a device to which it tells that version now, or
  whose `previous` it is. Each other request is `404 NO_RELEASE`. A daemon token is thus no way to read the
  release store.
- `record` is at most 64 KiB before base64. `signature` is at most 1 KiB of text.
- `POST /daemon/v1/update/file` answers 200 with `Content-Type: application/octet-stream`, a `Content-Length`,
  and the bytes of the file that the record names for the program `anima-daemon`. The request names no file:
  the hub selects it from the version and the target. The file is at most 256 MiB. The daemon reads at most
  the size that the record gives and stops there.
- The daemon asks for the record first, then for the file, at most 3 times each for one instruction, with the
  waits of Section 7. The answer's deadline of the file route is 10 minutes, not 10 seconds.
- The part of the daemon that makes these requests only stores the bytes. The second unit, which has no
  network access, verifies them and replaces the program (ROL-010, ROL-015). It trusts neither the hub nor
  the part that made the download.
- The limits of Section 6 apply. A file request holds the turn of its device and no turn on the database
  while the bytes go out. The hub serves at most 2 files at once; a third request is `503 UNAVAILABLE`.

New refusal:

| Status | Code | When | Daemon does |
| --- | --- | --- | --- |
| 404 | `NO_RELEASE` | The hub does not tell that version to this device, or does not hold that release for that target | Drops the instruction; waits for the next `Accepted` |

### 8.4 The states of a machine

The hub makes one state for each machine from its own facts and from the last `update` member. The words are
those of the client. This table is the order of the tests: the first row that is true gives the state.

| State | True when |
| --- | --- |
| Enrol again | The daemon token of the device ended (MET-013, ROL-043) |
| Needs a manual installation | The newest accepted batch had no `update` member (ROL-044) |
| Set off at the machine | `switch` is `off` |
| Held | The operator holds the machine (ROL-028) |
| Refused, with the cause | `last.result` is `refused` for the version that the hub tells |
| Rolled back, with the cause | `last.result` is `rolled_back` for the version that the hub tells, or for the version that it told last |
| Waiting: not connected | The hub tells a version, and the metrics of the device are stale (MET-018, ROL-039) |
| Updating | The hub tells a version, and `daemonVersion` is not that version |
| Checking | `daemonVersion` is the version that the hub tells, and the check of ROL-029 is not complete |
| Waiting for its step | A rollout includes the machine, and its step did not start |
| Update available | The hub holds a verified release for `target` that is higher than `daemonVersion` |
| Up to date | Each other case |

The check of ROL-029 is complete for the hub when it accepted one batch with the new `daemonVersion` and a
`last` with the result `installed` for that version. The machine writes `installed` only after its own check:
the new daemon started, `anima-daemon doctor` passed, and the hub accepted one push.

## 9. Operational keys (R-147, R-148; K-E5 Section 2.1)

The hub gives a daemon the operational keys that the operator granted to its device. The daemon proves its device
with its token on each call; to be on the network is not enough. K-E5 Section 2 owns the keys, their versions and
their grants; this section is the wire.

- **What the daemon says.** The `keys` member of each `Batch` (Section 4) lists the versions in its cache. The hub
  writes them to the consumer rows of the device (`held_version`, `held_seen_at`); a key that the list does not
  name holds no version.
- **What the hub says.** When the batch had a `keys` member and was stored, the hub answers `200` with `Accepted`:
  `{ "keys": { "current": [ { "name", "version" } ] } }`, each key granted to the device that is not removed, not
  past its expiry and has an active version, with that version. A key that is not in the list is not for this
  device now. A daemon that sent no `keys` member gets no `keys` member. When the hub cannot read the keys, it
  answers `204`: the batch is stored, and the daemon changes nothing in its cache. A daemon ignores an `Accepted`
  that it cannot read, or whose list breaks the rules of `held`.
- **The fetch.** `POST /daemon/v1/keys/fetch` with the body `{ "device": "<id>", "name": "<key name>" }`. The
  checks run in the order of a renewal: the bearer before the body, the size, the JSON (`MALFORMED`), the device
  (`WRONG_DEVICE`). The answer is `200` with `FetchedKey`, `{ "name", "version", "value" }` (`value` is the
  standard base64 of 1 to 16 384 bytes), and `Cache-Control: no-store`. A key that does not exist, has no active
  version, or is not granted to the device is `404 NO_KEY`, from one lookup. Each fetch that gives a value writes
  one audit entry (`target_type` `operational_key`, action `fetch`, the name, the version and the device; never
  the value) and `fetched_version` and `fetched_at` on the consumer rows. A refusal is written to the event log as
  any refusal of a daemon route (Section 6), without the key's name.
- **What the daemon does.** After each `Accepted` with `keys`, the daemon deletes from its cache each key that is
  not in `current` and each key whose version is not the current one, then fetches each current version that it
  does not hold, and stores the cache. A failed fetch is tried at the next such answer.
- **The cache.** The daemon keeps its cache where it keeps its token (Section 2): one Keychain item on macOS, or on
  Linux the file `keys` in its folder, mode `0600`, written to a temporary file and renamed. It keeps the time of the
  last `Accepted` with `keys`. When that time is more than 7 days old, when the hub refuses its token
  (`TOKEN_REFUSED`), on a new enrolment, and on `anima-daemon key forget`, it deletes the whole cache.
- **The local reader.** `anima-daemon key read <name>` writes the cached value to its standard output, only when that
  is not a terminal, and never to a log.
- No log line, event, error text or `Debug` output of either side holds a value (LOG-007). The Rust type of a value
  is `daemon_core::wire::KeyValue`: no `Debug`, `Display` or `Clone`, wiped on drop.

