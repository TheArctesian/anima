# K8 — Agent push: enrolment, push and renewal

Status: **frozen for Phase 3** (CR-15, R-092; change only through a programme ruling, CON-001). Written by
WP-54 T2. Spec: MET-011 to MET-020, LOG-007, NET-002, NET-003, P3-004. Rust types: `agent_core::wire`.

This contract is the whole interface between `anima-agent` and the hub. The agent makes no other request.

## 1. Transport

- HTTPS to the hub's public origin (`ANIMA_PUBLIC_ORIGIN`, `https://hub.anima.ing`), tcp 443, at the hub's VPN
  address (MET-020). The agent verifies the certificate against the web PKI roots and the host name; there is
  no pin in Phase 3.
- The agent sends no cookie, follows no redirect (a 3xx is a failed request), ignores proxy variables
  (`HTTPS_PROXY`, `ALL_PROXY`), and listens on no port.
- Every route below sits behind the NET-002 layer like any other route, and is logged by its template only.
- Bodies are UTF-8 JSON, `Content-Type: application/json`. Member names are camelCase. Unknown members are
  ignored by both sides, so either side can add an optional member without a new version.
- Times are RFC 3339 UTC with `Z` and whole seconds (`2026-11-08T10:00:10Z`).
- A breaking change adds `/agent/v2/`; the hub keeps `v1` for at least one phase.

| Route | Credential | Body limit | Success |
| --- | --- | --- | --- |
| `POST /agent/v1/enrol` | Enrolment code, in the body | 4 KiB | 200, `Enrolled` |
| `POST /agent/v1/push` | Agent token, `Authorization: Bearer` | 512 KiB (MET-017) | 204, empty |
| `POST /agent/v1/renew` | Agent token, `Authorization: Bearer` | 4 KiB | 200, `Renewed` |

The hub reads at most the limit and answers 413 without reading further.

## 2. Secrets

| Value | Format | Hub keeps | Lifetime |
| --- | --- | --- | --- |
| Enrolment code | `anima_enrol_` + 22 base64url characters (16 bytes, 128 bits, MET-012) | SHA-256 only | 15 minutes, one use |
| Agent token | `anima_agent_` + 43 base64url characters (32 bytes, MET-013, P3-004) | SHA-256 only | 365 days |

- Bytes come from the OS random source. Only canonical unpadded base64url of exactly 16 or 32 bytes is accepted,
  so each value has one spelling and no lookup runs for junk.
- The prefixes let a scanner find a leaked value. An API key (`anima_` + 43) never has the length of an agent
  token. The access guard refuses, before any lookup, a bearer spelled exactly as an agent token or an
  enrolment code: the prefix, then exactly 43 or 22 base64url characters. Any other text that starts with
  `anima_agent_` or `anima_enrol_` goes through the guard's usual length rule: it is read as an API key only
  when it is `anima_` and 43 characters, and is refused otherwise. So an agent token never reaches GraphQL, a
  session or a read (MET-013).
- No log, event, metric, error message or response other than `Enrolled` and `Renewed` holds a code or a token
  (LOG-007, MET-013). The Rust types that hold them implement neither `Debug`, `Display` nor `Clone`, and wipe
  on drop.
- The agent keeps the token in the macOS Keychain, or on Linux in a file with mode `0600` owned by the user that
  runs the agent, written to a temporary file in the same folder and renamed (MET-013).

## 3. Enrolment (MET-012)

The operator makes a code with `enrolAgent(deviceId)` (K1, signed-in operator only, never an API key, INT-003).
Making a new code for a device makes its earlier unused codes useless.

Request:

```json
{ "code": "anima_enrol_…" }
```

`Enrolled`:

```json
{ "device": "0192f5c4-…", "deviceName": "pop-os", "token": "anima_agent_…", "expiresAt": "2027-11-08T10:00:00Z" }
```

- The hub marks the code used in the transaction that looks it up, before it makes the token. A second use
  fails, even when the first use failed later or its response was lost (MET-012).
- `deviceName` is the device's name: 1 to 64 characters (so at most 256 bytes) with no control
  character. That is the hub's own rule for a device name (`agent_core::wire::DEVICE_NAME_CHARS`), so the
  agent keeps every name the hub can send, and a name it would not keep never costs a code (MET-012).
- A successful enrolment replaces any agent token the device had, sets the device's metrics source to `agent`
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
  "gpus": [ { "index": 0, "name": "Apple M3 Pro", "vendor": "apple", "hasVram": false } ]
}
```

- `device`: the device ID from `Enrolled`. It must equal the token's device (MET-017).
- `series`: 1 to 256 entries, no two equal. `kind` is a `series.kind` text (`cpu_percent`, `memory_percent`,
  `memory_bytes`, `disk_percent`, `disk_bytes`, `network_in_bps`, `network_out_bps`, `uptime_seconds`,
  `load_1`, `load_5`, `load_15`, `gpu_percent`, `vram_percent`, `vram_bytes`, `gpu_temperature_c`,
  `gpu_power_w`, `gpu_memory_bytes`). `gpu_memory_bytes` is the memory in use by a unified-memory GPU (Apple);
  it is never a VRAM value (MET-007, R-092 D-6). `label` is present exactly for the disk kinds (the mount point) and the network kinds (the
  interface name): 1 to 128 bytes, no control character. `gpu` (0 to 63) is present exactly for the GPU kinds.
  The hub ignores a series whose kind it does not know, and the values in its column, so a newer agent can
  talk to an older hub.
- `points`: 1 to 360 entries (one hour), `at` strictly increasing, each `at` a multiple of 10 seconds. A point
  covers the 10 seconds that end at `at`. `mean[i]` is the mean of the 2-second samples of `series[i]` in that
  window, or `null` when the window has none. `max`, when present, has the same length; only `cpu_percent`
  and `gpu_percent` columns may be non-null, and each is at least its mean. Every number is finite and at
  least 0; percent kinds are at most 100; a `gpu_temperature_c` is at most 1000 (°C) and a `gpu_power_w` at
  most 10 000 (W), and a batch with a value past either is `MALFORMED` (MET-002, CR-17). The agent drops such a
  2-second sample before it makes a mean, so it never sends one.
- `gpus`: optional. The agent sends it in its first batch after start and after each GPU discovery (at start
  and every 24 hours, MET-006), and again with the next batch until one is accepted, except after `MALFORMED`:
  then it waits for the next discovery, so a list the hub refuses cannot sink every later batch. Absent means
  "unchanged". `vendor` is `nvidia`, `amd`, `intel` or `apple`;
  `name` is 1 to 128 bytes, no control character; `hasVram` is false for `intel` and `apple` (MET-007).

The hub:

- stores each non-null mean as the sample of its series at `at` (10-second resolution, MET-015); the series
  key follows the SSH collector's rule for the same kind, label and GPU;
- folds `max` into the `max` column of the minute, hour and day aggregates; the samples hold the mean;
- skips a point whose series and `at` are already stored, so sending a batch twice adds nothing twice;
- keys a GPU series by its index: its label is the index as text. A GPU series with a `label` is `MALFORMED`;
- lets a device create at most 1024 series in any hour, counted by when the hub created each series, never
  by the times of its points: a batch whose new series would pass that is `MALFORMED` and stores none of it.
  One agent has at most 519 series (at most 256 in one batch) (R-098 e, CR-20);
- with `gpus` present, replaces the device's GPU set with it; `[]` means the device has no GPUs now, and an
  absent `gpus` leaves the set as it is. A GPU that leaves the set keeps its series and their history
  (R-098 b);
- records the time of the last accepted batch for MET-018 (R-098 a). Any accepted batch resets it, even one
  whose series the hub all ignores; a refused batch does not. 30 seconds after the last accepted batch the
  device's metrics are stale: the hub writes one event (source `metrics`, severity warning) and serves
  `Device.metricsStale` true. The next accepted batch clears the mark and writes one event (severity info).
  An agent that has never pushed is not marked, and `metricsStale` is true for it. Stale changes no check:
  a stale agent is never a Down check.

## 5. Renewal (MET-013)

The agent calls renew with its current token when fewer than 30 days remain, at most once an hour. The body is
`{}`.

`Renewed`:

```json
{ "token": "anima_agent_…", "expiresAt": "2027-11-08T10:00:00Z" }
```

- The hub keeps the new token's hash beside the current one. The current token keeps working until the new one
  is first used, for a push or a renewal; then the current one stops working (MET-013).
- A renewal with the current token while an unused new token exists replaces that new token, so a lost response
  costs nothing.
- The agent stores the new token (Section 2) before it uses it. If it cannot store it, it keeps the current one.

## 6. Refusals

The body of every refusal is `{ "error": "<CODE>" }` and nothing else. It never echoes a value from the request.

| Status | Code | When | Agent does |
| --- | --- | --- | --- |
| 400 | `MALFORMED` | The body is not a valid `Batch` or `{}` by Section 4 or 5 | Drops that batch; logs the code |
| 401 | `TOKEN_REFUSED` | The token is missing, malformed, unknown, expired or revoked, or the device's metrics source is no longer `agent` (CR-18) | Stops pushing; keeps sampling and buffering; logs once; tries again every 10 minutes and after the token store changes |
| 401 | `ENROLMENT_REFUSED` | Section 3 | `anima-agent enrol` exits 1 with a fixed sentence |
| 403 | `WRONG_DEVICE` | `device` is not the token's device | As `TOKEN_REFUSED` |
| 413 | `TOO_LARGE` | The body exceeds its limit | Drops that batch; sends smaller batches |
| 422 | `OUT_OF_RANGE` | A point's `at` is more than 1 hour before or more than 60 seconds after the hub's clock | Drops that batch |
| 503 | `UNAVAILABLE` | The hub cannot store now, or is busy (below) | Keeps the points; tries again |

The hub runs the push checks in this order and stops at the first refusal: the bearer (before it reads the body), the size,
the JSON, the device, the shape, the times. Every push refusal is written to the event log (source `metrics`,
severity warning) with its code, the client address and, when the token was valid, the device, and never with
the token (MET-017). A refusal with no valid token is written at most once a minute per client address, so a
flood cannot fill the event log. A refusal with a valid token, on any agent route, is written at most once a
minute per device (CR-21). Only the event is held back: the refusal and its answer are the same.

The hub answers at most 4 agent requests at once, and at most one push or renewal per device at once
(MET-017). A request past either is `UNAVAILABLE` at once, before it waits for the database: an enrolment
after its body is read, a push or renewal after its bearer is read, and a second push or renewal of a device
after the token is looked up. This answer is not written to the event log. The agent keeps the batch and
tries again after its wait (Section 7), so one device, or any number of requests with one token, cannot hold
the connections the rest of the hub needs.

## 7. Agent buffer (MET-014)

- The agent samples every 2 seconds and makes one point per 10-second window, ending on a multiple of 10
  seconds of its clock.
- It pushes every 10 seconds: the oldest unsent points first, at most 60 points per batch, then the next batch
  at once while points remain.
- While the hub cannot be reached (a network error, a timeout of 10 seconds, a 5xx), it keeps up to 360
  points (1 hour), dropping the oldest first, and drops any point older than 59 minutes before it sends.
- After a failure it waits 10 seconds, doubling to at most 60 seconds, then resumes the 10-second rhythm.
