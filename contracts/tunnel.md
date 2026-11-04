# K4 — Remote access tunnel

Status: **frozen for Phase 1** (change only through a programme ruling, CON-001).

The remote access stream does not use GraphQL (API-009). The client gets a ticket from the `startSession` mutation (K1), then opens one WebSocket to the hub. The hub relays Guacamole protocol instructions between that socket and `guacd` (GW-011). Owners: hub side WP-10 (`sessions`), protocol codec WP-08 (`guac-protocol`), client side WP-14.

## 1. Ticket

| Property | Value |
| --- | --- |
| Token | 32 bytes from the OS random source, base64url without padding (43 characters) |
| Storage | The hub keeps only the SHA-256 of the token, in memory, never in the database or a log (LOG-007) |
| Lifetime | 30 seconds from issue (`expiresAt` in the ticket) |
| Use | Single use. The hub removes the token when a tunnel request presents it, valid or not |
| Binding | Operator ID, session ID, client address. A request from another client address is refused |
| `tunnelPath` | Always `/tunnel` in Phase 1 |

`startSession` does these steps before it returns the ticket: checks that the device exists and SSH is enabled; checks that a credential is stored (or the gateway will ask, GW-015); inserts a `sessions.sessions` row with `started_at`; writes an audit entry `session.start`.

## 2. Tunnel request

```text
GET /tunnel?token=<token>  HTTP/1.1
Upgrade: websocket
Sec-WebSocket-Protocol: guacamole
```

- The client uses `guacamole-common-js` `Guacamole.WebSocketTunnel(wssBase + "/tunnel")` and calls `client.connect("token=" + token)`. The library appends the connect data as the query string.
- The route sits behind `access::permitted_networks_layer` (NET-002). It does not use the bearer token: the ticket is the authentication (SEC-002).
- Refusals happen **before** the upgrade, with HTTP status and no body detail:

  | Case | Status |
  | --- | --- |
  | No token, unknown, used or expired token | 403 |
  | Client address differs from the ticket | 403 |
  | Subprotocol `guacamole` missing | 400 |
  | Gateway unreachable | 503 (and the session ends with `GATEWAY_ERROR`) |

## 3. Handshake (hub ↔ guacd, GW-010)

The hub, not the client, does the handshake with the `guac-protocol` `Handshake` state machine:

1. `select` with `ssh`.
2. Read `args`. The first arg is the protocol version (`VERSION_1_5_0` or later). The hub answers with `VERSION_1_5_0`, the newest version it implements, not the version guacd sends (R-018).
3. Send `size` (width, height, DPI from `DisplayInput`), `audio` (empty), `video` (empty), `image` (`image/png`, `image/jpeg`, `image/webp`), `timezone` (from `DisplayInput` when it is an IANA zone name, else UTC; sites have no zone in Phase 1, R-032), `name` (operator username).
4. Send `connect` with one value per name in `args`, in the same order. Values come only from the database and the credential store (GW-012). Unknown names get an empty value. The client never supplies `hostname`, `port`, credentials or any path (GW-013).
5. Read `ready`. Its argument is the gateway connection ID. The hub keeps it with the session and never sends it to the client (GW-014).

Phase 1 SSH argument values:

| Argument | Value |
| --- | --- |
| `hostname`, `port`, `username` | From `devices.devices` and `devices.ssh_settings` |
| `password` or `private-key` + `passphrase` | From `credentials::load_ssh` |
| `host-key` | From `sessions.host_keys` (SEC-006). Empty only before the first accepted connection |
| `color-scheme` | Nord scheme built from `crates/theme` (RA-010) |
| `font-name`, `font-size` | `OpenDyslexicM Nerd Font Mono`, `12` (RA-011) |
| `scrollback` | `5000` |
| `terminal-type` | `xterm-256color` |
| `server-alive-interval` | `30` |
| `enable-sftp`, `recording-path`, `typescript-path` | Empty (Phase 1) |

## 4. Relay

After `ready`, the hub sends to the client, in this order:

1. The internal instruction `0.,36.<tunnel-uuid>;` (empty opcode, one arg: a new UUID). `guacamole-common-js` needs it to mark the tunnel open. The UUID is not the gateway connection ID.
2. Every gateway instruction, unchanged except that characters outside the Basic Multilingual Plane become U+FFFD (lengths then agree in code points and UTF-16 units, R-046), as one WebSocket text message per complete instruction (the `Decoder` gives complete instructions only), except `error`: the hub relays its status code with the fixed text `"Aborted. See logs."`, so no gateway error detail reaches the client (GW-014, LOG-007, R-035).

From the client, the hub:

- answers the internal instruction `0.,4.ping,<n>.<ts>;` with the same instruction, and does not forward it;
- forwards only allow-listed instructions (R-035), each with at least the argument count guacd's handler reads: `key` (2), `mouse` (3), `touch` (7), `size` (2–3), `clipboard` (2), `blob` (2), `end` (1), `ack` (3), `sync` (1), `nop`, `disconnect`, `argv` (3);
  - `size` is rebuilt from plain decimal numbers and forwarded only when width and height are 1..8192 and DPI, when present, is 48..480 (R-018);
  - `blob` and `end` only on streams the client opened;
  - `argv` only for `color-scheme`, or once per gateway `required` for `username`, `password`, `passphrase` or `domain` (GW-013 before GW-015);
- closes the tunnel with close code 1002 (`PROTOCOL_ERROR`) if an instruction does not decode (GW-011) or is not allow-listed.

The hub sends a `nop` to the client when the gateway has sent nothing for 5 seconds. If the client sends nothing for 15 seconds (no ping, no input), the hub ends the session with `TIMEOUT`.

## 5. End of session

| Cause | WebSocket close code sent to client | `endReason` |
| --- | --- | --- |
| Client closes, or `endSession` mutation | 1000 | `OPERATOR` |
| Gateway sends `disconnect`, or the remote host closes | 1000 after relaying `disconnect` | `REMOTE_CLOSED` |
| Gateway sends `error` with status 0x0300–0x03FF (upstream) | 1011 after relaying `error` | `GATEWAY_ERROR` |
| Gateway `error` with status 0x0301 / 0x0303 (unauthorized / forbidden) | 1011 after relaying `error` | `AUTH_FAILED` |
| Host key mismatch | 4001 after the hub sends `error` `"Host key changed"` `0x0303` | `HOST_KEY_MISMATCH` |
| Client sends an instruction that does not decode (GW-011) | 1002 | `PROTOCOL_ERROR` |
| Idle timeout | 4002 | `TIMEOUT` |
| Hub shutdown | 1001 | `HUB_SHUTDOWN` |

On every end the hub: closes both sockets; sets `ended_at` and `end_reason` on the session row; publishes an `events::Signal::Event` (`Source::Session`, severity Info; Error for `HOST_KEY_MISMATCH`; Warning for `AUTH_FAILED`, `GATEWAY_ERROR` and `PROTOCOL_ERROR`; R-032, R-034); writes the audit entry `session.end`. The hub never records session content (RA-008).

## 6. Host keys (SEC-006, SEC-007)

- The connection test (`testConnection`) reads the host key before any session. If none is recorded, the result has `hostKey` set and `hostKeyChanged: false`; the client shows the fingerprint and calls `acceptHostKey`.
- `startSession` refuses with `CONFLICT` when no host key is accepted yet.
- Before the gateway handshake, the hub reads the device's SSH host key itself (the same no-auth key exchange `testConnection` uses), because guacd reports a mismatch only as a generic `error` 0x0200 (R-034). When the hub reads a key that differs from the recorded key, it sends `error` `"Host key changed"` `0x0303` and the session ends with `HOST_KEY_MISMATCH`, an Error event is written and the client shows the warning modal. guacd still pins `host-key`, so a key that changes between the hub's read and guacd's connect fails closed as `GATEWAY_ERROR`. Only a new `testConnection` + `acceptHostKey` replaces the key, and that writes an audit entry.

## 7. Tests the owners must have

- Ticket: expired, reused, wrong client address → 403 (property-based on the clock with `FixedClock`).
- Handshake: a recorded `args` list from guacd 1.5 and 1.6 gives the expected `connect` values; a forbidden name sent by a fake client never reaches `connect`.
- Relay: a fake guacd (in-process TCP listener) and a fake client run one full session, including `ping`, an invalid instruction (close 1002) and each end cause in Section 5.
