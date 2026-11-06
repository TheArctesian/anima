# K4 — Remote access tunnel

Status: **frozen for Phase 1** (change only through a programme ruling, CON-001). R-062 split SSH onto the terminal tunnel and added Section 8.

The remote access stream does not use GraphQL (API-009). The client gets a ticket from the `startSession` mutation (K1), then opens one WebSocket to the hub. Owners: hub side WP-10 (`sessions`), protocol codec WP-08 (`guac-protocol`), client side WP-14; the terminal tunnel WP-18.

There are two tunnels (R-062):

- **SSH** uses the terminal tunnel (Section 8): the hub runs the SSH connection itself and relays terminal bytes; the client draws the terminal with xterm.js (RA-011). No SSH session goes to `guacd`.
- **RDP and VNC** (Phase 2) use the Guacamole tunnel: the hub relays Guacamole protocol instructions between the socket and `guacd` (GW-011). Sections 2 to 4 describe it. Since only SSH exists in Phase 1, the hub does not serve `/tunnel` until a protocol uses it (WP-18 D-1).

Sections 1, 5 and 6 apply to both tunnels.

## 1. Ticket

| Property | Value |
| --- | --- |
| Token | 32 bytes from the OS random source, base64url without padding (43 characters) |
| Storage | The hub keeps only the SHA-256 of the token, in memory, never in the database or a log (LOG-007) |
| Lifetime | 30 seconds from issue (`expiresAt` in the ticket) |
| Use | Single use. The hub removes the token when a tunnel request presents it, valid or not |
| Binding | Operator ID, session ID, client address. A request from another client address is refused. A ticket is valid only on the path its `tunnelPath` names |
| `tunnelPath` | `/terminal` for SSH (Section 8). `/tunnel` for RDP and VNC (Phase 2) |

`startSession` does these steps before it returns the ticket: checks that the device exists and SSH is enabled; checks that a host key is accepted (Section 6); inserts a `sessions.sessions` row with `started_at`; writes an audit entry `session.start`.

## 2. Tunnel request (Guacamole, RDP and VNC)

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
  | Ticket whose `tunnelPath` is not this path | 403 |
  | Client address differs from the ticket | 403 |
  | Subprotocol `guacamole` missing | 400 |
  | Gateway unreachable | 503 (and the session ends with `GATEWAY_ERROR`) |

## 3. Handshake (hub ↔ guacd, GW-010; RDP and VNC)

The SSH table below is historical: it is what the hub sent guacd before R-062, and no SSH session uses it now. Phase 2 replaces it with the RDP and VNC values (spec Section 6.3.2).

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

## 4. Relay (Guacamole, RDP and VNC)

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

The Guacamole tunnel's causes; Section 8.6 gives the terminal tunnel's causes for the same reasons and codes.

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
- The terminal tunnel (Section 8) pins the key in its own SSH connection: the hub compares the key the device presents in the key exchange with the accepted key and sends no credential unless they are equal (Section 8.3).
- Before the gateway handshake (Guacamole tunnel), the hub reads the device's SSH host key itself (the same no-auth key exchange `testConnection` uses), because guacd reports a mismatch only as a generic `error` 0x0200 (R-034). When the hub reads a key that differs from the recorded key, it sends `error` `"Host key changed"` `0x0303` and the session ends with `HOST_KEY_MISMATCH`, an Error event is written and the client shows the warning modal. guacd still pins `host-key`, so a key that changes between the hub's read and guacd's connect fails closed as `GATEWAY_ERROR`. Only a new `testConnection` + `acceptHostKey` replaces the key, and that writes an audit entry.

## 7. Tests the owners must have

- Ticket: expired, reused, wrong client address → 403 (property-based on the clock with `FixedClock`).
- Handshake: a recorded `args` list from guacd 1.5 and 1.6 gives the expected `connect` values; a forbidden name sent by a fake client never reaches `connect`.
- Relay: a fake guacd (in-process TCP listener) and a fake client run one full session, including `ping`, an invalid instruction (close 1002) and each end cause in Section 5.
- Terminal (Section 8): a fake SSH server (a `russh` server in the test) and a fake client run one full session: bytes both ways, `resize` reaching the server as a window change, `ping` both ways, and each end cause in Section 8.6. A changed host key ends `HOST_KEY_MISMATCH` and the fake server sees no authentication request; a refused credential ends `AUTH_FAILED`; a ticket that is expired, reused or from another address gets 403 and one without the subprotocol gets 400. Also: a first message that is not `resize`, or binary before it; no first `resize` within the wait; a message over 64 KiB (also across fragments) and a text message over 256 bytes; keyboard-interactive and its round and prompt limits; a key that does not decode; a WebSocket ping keeps the session alive; client silence, a stalled send, a stalled device write (`GATEWAY_ERROR`, with hub pings continuing meanwhile) and lost keep-alives or a dropped connection, all with the waits shortened through the sessions module's internal router constructor (the K3 `terminal_router` always uses the Section 8.5 values); `endSession` and the client leaving during the opening; channels the device opens are refused; a ticket on the wrong path is refused (a pure check while only SSH exists); a client that does not read leaves the output waiting in the device, not the hub; the fake server sees only `pty-req`, `shell` and `window-change` on one channel; no secret in any frame to the client and no token or secret in the log. The control-message parser is property-tested: no input panics, and only the messages in Section 8.4 are accepted. The Guacamole relay tests issue SSH tickets with `sessions::start` as their fixture and call the `/tunnel` router directly; it is not mounted.

## 8. Terminal tunnel (SSH, R-062)

The hub opens the SSH connection with `russh`, as the connection test does (Section 6, R-034). The client gets bytes, not pictures, and draws them with xterm.js (RA-011).

### 8.1 Request

```text
GET /terminal?token=<token>  HTTP/1.1
Upgrade: websocket
Sec-WebSocket-Protocol: anima-terminal
```

- The client opens `new WebSocket(<hub address, https→wss, http→ws> + tunnelPath + "?token=" + token, "anima-terminal")`. The token appears only in that query.
- The route sits behind `access::permitted_networks_layer` (NET-002); the ticket is the authentication (SEC-002). Neither the URI nor the token reaches a log, a span or an error (LOG-007).
- Refusals before the upgrade, with HTTP status and no body detail. The token is spent before the subprotocol is checked (Section 1):

  | Case | Status |
  | --- | --- |
  | No token, unknown, used or expired token | 403 |
  | Ticket whose `tunnelPath` is not this path | 403 |
  | Client address differs from the ticket | 403 |
  | Subprotocol `anima-terminal` missing | 400 (the session ends `PROTOCOL_ERROR`) |
  | Device gone, SSH off, no accepted key, host does not resolve or fails the target rule (R-032), device unreachable, or no SSH key exchange | 503 (the session ends `GATEWAY_ERROR`) |

### 8.2 Opening (before the upgrade)

1. Load the device; SSH must be on and a host key accepted.
2. Resolve the host; every address must pass the target rule (R-032). Connect to a checked address, never to the name again.
3. Key exchange with the host key order the connection test uses, so the device presents the same key it presented to `testConnection`. Pin the key (Section 8.3).
4. Authenticate as `devices.ssh_settings.username` with the stored credential from `credentials::load_ssh`, one attempt per method:
   - a password: `password`; when the device offers `keyboard-interactive` and not `password`, each prompt of a round is answered with the password, for at most 4 rounds of at most 4 prompts each (more is `AUTH_FAILED`);
   - a private key: `publickey` with the key and passphrase (RSA with the SHA-2 hash the device advertises in `server-sig-algs`; `ssh-rsa` (SHA-1) when it advertises none, as the connection test does).

   The secret is decrypted in the hub, used only for this, and dropped (zeroed) after it; it never reaches the client (OD-9, SEC-005). No stored credential, a key or passphrase that does not decode, or a refusal: the opening goes on to the upgrade and the session ends `AUTH_FAILED` (Section 8.6). SSH certificates are not used in Phase 1.

   GW-015 (the gateway asks for a missing credential) does not apply to SSH: the terminal tunnel has no prompt for a missing credential (WP-18 E-1).
5. SSH keep-alive every 30 seconds (spec Section 6.3.1); three unanswered keep-alives lose the connection (`GATEWAY_ERROR`).
6. The device's pre-authentication banner is not read or sent to the client.

The whole opening has the connection test's limit (8 seconds). `endSession`, hub shutdown or the client leaving during the opening ends the session (`OPERATOR`, or `HUB_SHUTDOWN`) and closes the SSH connection, before any credential is sent when it comes first. The PTY and the shell wait for the client's first `resize` (Section 8.4).

### 8.3 Host key

The hub compares the key the device presents in the key exchange with the accepted key in `sessions.host_keys` (byte equality of the key blob). It sends no authentication request unless the two are equal and the exchange signature verified with that key. When they differ, the hub sends no authentication request: it upgrades, sends `end` `HOST_KEY_MISMATCH` and closes 4001 (Section 8.6). A certificate, or a key `russh` decodes but the hub cannot store, counts as a different key. A key that does not decode at all fails the key exchange: 503 and `GATEWAY_ERROR` (Section 8.1), also with no authentication request. Only a new `testConnection` + `acceptHostKey` replaces the key (Section 6).

### 8.4 Messages

One WebSocket per session. Frames are of two kinds:

- **Binary:** terminal bytes, both ways, unchanged. Client → hub: what the operator types or pastes, written to the shell's standard input. Hub → client: the shell's output (its standard output and standard error, in the order they come), as the channel delivers it. The hub does not read or change these bytes and keeps none of them (RA-008).
- **Text:** one JSON object per message, from a closed set: exactly the keys listed, each once; `cols` and `rows` are integer literals with no fraction, exponent or sign; whitespace outside strings is allowed. Any other `type`, a missing, extra or repeated key, another JSON value (an array, a string), or a value out of range is a protocol error.

| Direction | Message | Meaning |
| --- | --- | --- |
| Client → hub | `{"type":"resize","cols":C,"rows":R}` | The terminal size in cells, `1 ≤ C ≤ 1000`, `1 ≤ R ≤ 1000`. The first one opens the PTY (`TERM=xterm-256color`) at that size and starts the login shell; later ones change the PTY size (SSH `window-change`, pixel sizes 0) |
| Client → hub | `{"type":"ping"}` | The client is alive. The hub does not answer it |
| Hub → client | `{"type":"ping"}` | The hub is alive. The client does not answer it |
| Hub → client | `{"type":"end","reason":"<SessionEndReason>"}` | The session ends with this K1 `SessionEndReason`; the close follows |

- The client's **first message must be `resize`**. Any other first message is a protocol error.
- The client sets no other PTY value: the terminal type, the command (the login shell) and the environment are the hub's (GW-013). On the one session channel the hub sends `pty-req` (`xterm-256color`, the size, pixel sizes 0, no terminal modes) and then `shell`; it sends no `exec`, `env`, `subsystem`, agent, X11 or port-forwarding request, opens no other channel, and refuses every channel the device opens. `DisplayInput` carries no terminal value for SSH.
- The client sends `ping` when it has sent nothing for 5 seconds. The hub sends `ping` when it has sent nothing for 5 seconds.
- "Message" means a whole WebSocket message after reassembly. Any message from the client, WebSocket ping and pong included, counts as the client being alive; only text and binary messages are Section 8.4 messages (a ping or pong is not a first `resize`).
- The hub sends its `ping` and watches client silence independently of any write in flight to the device or the client; a stalled write never stops them. (Handing a PTY, shell or window-change request to the SSH connection is not such a write: it waits only if the connection has stopped, and the 15-second limit in Section 8.5 ends it. While a client write waits on the device, the client-silence clock pauses.)
- When the client has received nothing for 15 seconds, it closes the socket and shows the session as lost (RA-007).
- The reason the client shows: the recorded `Session.endReason` (from `endSession`, which returns an ended session unchanged) is authoritative. Until it arrives, the client uses `end` when it came, else the close code: 1000 shows as closed, 1011 as a failed connection, the other codes as their single reason in Section 8.6.

### 8.5 Limits

| Limit | Value |
| --- | --- |
| Largest client message (binary or text) | 64 KiB; larger is a protocol error |
| Largest text message | 256 bytes; larger is a protocol error |
| Wait for the first `resize` after the upgrade | 10 seconds; then `TIMEOUT` |
| Client silence (no message of any kind) | 15 seconds; then `TIMEOUT`. The time the hub waits for the device to take client input does not count |
| One send to the client | 15 seconds; then `TIMEOUT` |
| One write of client input to the device, or a PTY, shell or window-change request | 15 seconds; then `GATEWAY_ERROR` |
| SSH channel buffer in the hub | 4 channel messages (each an SSH data packet of at most 32 KiB) |
| Text that is not UTF-8 | Protocol error |

Backpressure: the hub reads the next output from the SSH channel only after the previous binary frame was sent, and the channel buffer is bounded (above). A slow client therefore stops the hub reading the device's connection, so the device stops sending, instead of a buffer growing in the hub. The hub reads the next client message only after the previous one was written to the channel, under the channel's window.

### 8.6 End of session

The reasons, events and audit are those of Section 5. The first cause wins. When the hub ends the session, it first sends `end` with the reason (best effort), then closes; it waits at most 2 seconds for each peer while it closes.

| Cause | Close code | `endReason` |
| --- | --- | --- |
| Client closes, or `endSession` mutation | 1000 | `OPERATOR` |
| The shell exits, or the device closes the channel or sends an SSH disconnect | 1000 | `REMOTE_CLOSED` |
| The hub's SSH connection fails after the opening (a TCP drop with no SSH disconnect, a transport or protocol error, lost keep-alives), the PTY or shell request is refused, or a write to the device stalls (Section 8.5) | 1011 | `GATEWAY_ERROR` |
| The device refuses the credential, none is stored, or the stored key does not decode | 1011 | `AUTH_FAILED` |
| Host key differs from the accepted key | 4001 | `HOST_KEY_MISMATCH` |
| Client message that is not in Section 8.4, over a limit in Section 8.5, or not UTF-8 | 1002 | `PROTOCOL_ERROR` |
| Client silence, no first `resize`, or a send that stalls (Section 8.5) | 4002 | `TIMEOUT` |
| Hub shutdown | 1001 | `HUB_SHUTDOWN` |

On every end the hub closes the SSH channel and connection and the WebSocket, and writes the row, event and audit entry as in Section 5. Event messages are fixed sentences; none holds device text, an address, a value or session content (LOG-007, RA-008).

### 8.7 What the client terminal may do

The remote host controls the bytes the client draws, so the client sets each of these on purpose:

- OSC 52 (write to the clipboard): off. A remote host must not fill the operator's clipboard.
- OSC 8 hyperlinks and detected links: open only after a confirmation that shows the full URL; only `http` and `https`.
- Window title (OSC 0 and 2): shown in the tab after removing control, format and line or paragraph separator characters (Unicode Cc, Cf, Zl, Zp; this removes bidi overrides), at most 80 characters.
- No sixel or other image protocols.
- Copy and paste go through the operator's own actions only (RA-003): selection and the copy shortcut, the paste event or the Keys menu. Paste uses bracketed paste when the shell asks for it.
