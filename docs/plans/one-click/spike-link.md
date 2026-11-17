# Spike K-A: the link (Tailcat transport)

Date: 2026-11-14. Plan: `docs/plans/one-click/one-click-plan.md`, Sections 4.1 (S1) and 5. Code: `spikes/link/`.
This is a spike: throwaway code and a report. It changed no contract, no hub code and no live system.

## 1. Verdict

**The stop condition is not met on the evidence of one machine. S1 can continue, with five
changes (Section 8). The two questions that decide the stop condition for remote display need a
second machine (Section 7).**

The stop condition is: "Relayed sessions cannot be used, and a relay of our own does not correct this."

| Question | Answer |
| --- | --- |
| Does the link work as S1 describes? | Yes. A stable address, a list of permitted keys, revocation that closes open sessions, HTTP and WebSocket, and a path report all work. Three of them need code in `anima-link` that the library does not supply (Sections 3.2, 3.3, 4). |
| Can a relayed session be used? | For the hub's API, the interface and SSH: yes, in a model. The link added less than 6 ms to the round trip of the relay. For remote display: only at a low bit rate. Through a relay, bulk transfer fell to 4 to 9 Mbit/s when the relay added 40 ms, and to 0.85 Mbit/s when it added 160 ms. |
| Does a relay of our own correct it? | In part. It removes the shared rate limit, it can be near the hub (speed is in proportion to 1 / round trip), and a deeper queue on the relay gave 1.6 to 2 times the speed. It cannot reach direct-path speed: one queue of 32 packets is a constant in the library. |
| Does the Headscale relay accept link keys (fact 10)? | Not checked on the VPS. The repository sets `verify_clients: true` for it, so the expected answer is no (Section 5). |

The spike also found faults that are not in the plan: the release `v0.7.0` does not have the
functions that fact 4 names; a running device does not connect again after a hub restart; and an
idle link stalls for 1 to 5 seconds. Each has a tested answer.

## 2. Set-up

| Item | Value |
| --- | --- |
| Machine | One Mac, Apple M5 Pro (18 cores), macOS 27.0.1. Hub side, device side, relay and test hub all ran on it. |
| Tailcat | `github.com/tailscale/tailcat` at `v0.7.1-0.20261105151022-e00cdde6ff78` (`main`, 2026-11-05) |
| Tailscale library | `tailscale.com v1.105.0-pre.0.20261013225543-fb79793d99f5` |
| Go | 1.27.4 |
| Test hub | `anima-link testhub`: HTTP and a WebSocket echo on loopback. The real hub was not run. Its GraphQL and its tunnel use the same two transports over one TCP port. |
| Own relay | `derper` from the same Tailscale version, on loopback, plain HTTP |
| Distant relay model | `anima-link slowrelay`: a TCP proxy in front of the relay. It adds a fixed delay and an optional rate limit. Both sides connect through it, so a delay of 10 ms adds 40 ms to the round trip. It has no loss and no jitter. |
| Forced relay | `TS_DEBUG_ALWAYS_USE_DERP=1` on both sides |

## 3. Function results

### 3.1 Stable address after a restart (LNK-001): works, with one condition

- Own relay: the hub link stopped and started with the same state file. The address was the same
  (compared by SHA-256). The state file and the address file have mode 0600.
- Tailscale's relay map: the same result, but only after a change. The address contains the
  relay. At the first start the library selects the nearest relay region. The address does not
  hold that region's number in the map, so the hub cannot save "the number". A first version that
  saved the number failed at the restart. The hub must save the whole relay region with the key
  and the pre-shared key.
- The address was 201 characters with an own relay and 232 with a Tailscale region.
- Effect: if the operator sets a different relay (LNK-007), the address changes. Each device then
  needs the new address.

### 3.2 A running device after a hub restart: a fault in the first version

The library registers a device with the hub only at its first use. A hub that restarts has
forgotten each device. A device process that stayed running then failed on each request and did
not recover.

Answer tested: after a failed dial, the device calls `Client.Ping` (this registers it again) and
dials once more. With this, the first request after the hub restart still failed after 15 s, and
the second request worked. S1 needs a keepalive that registers again (Section 3.6).

### 3.3 Permitted keys and revocation (LNK-002, LNK-004)

| Test | Result |
| --- | --- |
| Device key not in the list | Refused. The hub sends no answer. The request failed after 13 s. The hub's peer list did not show the key. |
| The same key, added to the list while the hub runs | The next request worked in 3 ms. |
| Revocation with the library only (`KeySet.Remove`, then `DisconnectClient`) | Traffic stopped at once and new requests failed. **The open session did not close**: the device's WebSocket was still open after 100 s, and the hub's stream to the test hub was still open 6 minutes later. |
| Revocation where `anima-link` first closes the streams of that key, then calls `DisconnectClient` | The device's open WebSocket ended 69 ms after the command. A new request failed. A second device was not affected. |

So LNK-004 is possible, but `anima-link` must do it: record each accepted stream by
`Server.PeerKey`, and on revocation remove the key, close its streams, wait until the close
reaches the device (500 ms in the spike), then call `DisconnectClient`. The library documents
that `DisconnectClient` "does not reset the client's connections".

A refused device and a hub that is off look the same to the device: no answer. The client can
only say "the hub did not answer".

### 3.4 HTTP and WebSocket (item 3 of the task)

Through the link, on a direct path: `GET`, a 300 MiB download, a 300 MiB upload and 200 WebSocket
echo round trips on one connection all completed with correct byte counts. No change to the HTTP
server was necessary: the hub side sends each stream to the loopback port.

### 3.5 First connection: 10 ms, or 4 to 10 s

The first request of a new device process took about 10 ms in most runs against a hub link that
was already running (new key: 9 of 9 runs; same key, new process: 24 of 37 runs). In the other
13 runs it took 5.0 to 5.3 s, and once 10.3 s. The slow state stays with one device key: 8 of 8
restarts were slow, and for a different key 4 of 4. It ended when the hub forgot the key
(`DisconnectClient`, 4 of 4 runs fast after it).

When a new hub link and a new device link started together on an own relay, the first request
took 4.0 to 4.7 s in 6 of 6 runs. Of this, 3 s is the spike's own wait before it registers again
(Section 3.2). With Tailscale's relay map the same start took 1.0 to 1.3 s (one run: 7.3 s).

The delay agrees with the WireGuard handshake retry time (5 s). The cause was not found. It is
possible that it occurs only when both ends are on one machine. The second machine must check it
(Section 7).

### 3.6 Idle link: a stall of 1 to 5 s

One device made a request, waited, and made the next request (own relay, direct path permitted).

| Idle time | Next request |
| --- | --- |
| 20 s, 45 s, 90 s | 2 to 15 ms |
| 150 s | 1.0 s |
| 240 s | 5.1 s |

A second device made one request each 15 s for 10 minutes: 40 requests, median 2.8 ms, maximum
42 ms. A keepalive each 15 s removes the stall.

## 4. Direct or relayed (LNK-006)

| Side | Source | Result |
| --- | --- | --- |
| Hub | `Server.Status()`: each peer has `CurAddr` (direct) or `Relay` (relayed) | Works with no extra traffic. It showed `direct`, `relayed` and the relay name correctly in each run. A peer that was disconnected stays in the list with no path; `anima-link` must remove it. |
| Device | `Client.DiscoPing()`: the answer has `Endpoint` (direct) or a relay region | Works, but it is an active ping, not a status call. It also starts the search for a direct path. The client has no passive status call. |

The device cannot show the relay's name: the address does not hold the region's name, and the
library shows a number. The hub can show it.

With Tailscale's relay map and both ends on this machine, the path was direct at the first check
(about 2 s after the first request).

## 5. Fact 10: the Headscale relay and link keys

Not checked on the VPS (VPS-005). Two findings from this machine:

1. `deploy/headscale/config/config.yaml` sets `derp.server.verify_clients: true`, and WP-03
   decision D-7 gives the reason. With this setting the relay accepts only keys that are nodes
   of the tailnet. A link key is not one.
2. A local relay that refuses each key (`derper -verify-client-url` with an answer of "not
   allowed") showed the effect. The relay's log showed each refusal. **The hub link started with
   no error and wrote an address.** The device's request failed after 13 s. So a refused relay
   gives no error to the operator.

Thus the Headscale relay cannot be the link's relay as it is configured. The choices are a
separate `derper` for the link, or `verify_clients: false`. Each is a VPS change that needs the
operator's confirmation.

## 6. Measurements

All on one machine. Latency is 200 WebSocket echo round trips of 64 bytes on one connection,
5 ms apart. Bulk speed is one HTTP download and one HTTP upload. "Echo in download" is the same
echo test during the download.

| Path | Echo p50 / p95 (ms) | Download / upload (Mbit/s) | Echo in download p50 / p95 (ms) | Bulk size |
| --- | --- | --- | --- | --- |
| No link (loopback to the test hub) | 0.14 / 0.53 | 36 300 / 14 300 | 0.15 / 0.25 | 500 MiB |
| Link, direct path, same machine | 0.60 / 0.90 | 249 / 192 | 16.5 / 22.2 | 300 MiB |
| Link, own relay on loopback | 0.71 / 1.21 | 644 / 723 | 0.99 / 1.77 | 300 MiB |
| Link, own relay + 40 ms | 45.9 / 50.9 | 4.0 / 4.7 | 47.7 / 59.3 | 100 MiB |
| Link, own relay + 40 ms, relay queue 1024 | 44.9 / 47.0 | 6.5 / 9.5 | 44.2 / 47.0 | 100 MiB |
| Link, own relay + 160 ms | 165.1 / 173.9 | 0.85 / 0.85 | 164.2 / 170.7 | 50 MiB |
| Link, own relay + 40 ms, 8 Mbit/s limit | 44.2 / 44.5 | 6.8 / 6.1 | 44.3 / 517 | 10 MiB |
| Control: the delay proxy only, no link, + 20 ms | 22.4 / 22.9 | 2 960 / 2 860 | 22.3 / 22.6 | 100 MiB |

What the numbers show:

- **The link adds little latency.** On a relayed path the echo time was the relay's round trip
  plus 4 to 6 ms, also during a bulk transfer with no rate limit.
- **A relayed path has low bulk speed, and the speed falls with the round trip.** The control
  row shows that the proxy is not the limit. The relay's counters showed the cause: it dropped
  packets at its queue for each client (`queue_head`, 1.19 MB dropped). The default depth is 32
  packets. With `TS_DEBUG_DERP_PER_CLIENT_SEND_QUEUE_DEPTH=1024` on our relay the drops were 0
  and the speed was 1.6 to 2 times higher. The library's own queue to the relay is a constant of
  32 packets (`derpWriteQueueDepth`); an own relay cannot change it.
- **With a rate limit that the session fills, latency becomes bad**: echo p95 517 ms, maximum 2.3 s.
- A run with a limit of 2 Mbit/s did not complete in 10 minutes and was stopped. It gives no number.
- The "direct, same machine" row is not a result for a real network. Both ends share one
  processor, and the relayed row on loopback is faster than it. Use it only as a floor: the link
  costs no more than this on one machine.

For comparison, spike K-B measured 38 Mbit/s from Apple Screen Sharing for a large Mac screen
(116 Mbit/s as a simple Guacamole stream). That does not go through a relay at 4 to 9 Mbit/s.
A relayed remote display needs a low-rate mode, or a direct path.

Other measurements:

| Item | Result | How |
| --- | --- | --- |
| Hub link start, Tailscale relay map | 1.4 s | Time from process start to the address file |
| First request, Tailscale relay map | 1.0 s and 1.3 s (one run: 7.3 s) | `curl` through a new device process |
| Echo through Tailscale's shared relay, nearest region | best 27 to 107 ms; p50 144 to 307 ms; p95 559 to 939 ms (4 runs of 60 to 100 echoes) | Forced relay. **Not a valid relay result:** at the same time a TCP connect from this machine to an internet host took 91 to 313 ms, so the machine's own connection was unstable. No bulk transfer was run. |
| Binary size, stripped, with Tailcat's build tags | macOS arm64 15.1 MB (5.8 MB gzip); macOS x86_64 16.4 MB; Linux x86_64 16.6 MB (6.5 MB gzip); Linux arm64 15.5 MB | `go build -trimpath -ldflags="-s -w" -tags <build-tags.txt>` |
| Binary size, stripped, no tags / not stripped | 18.8 MB / 28.5 MB (macOS arm64) | The same, without tags / plain `go build` |
| Administrator rights on macOS | Not necessary. All runs used a standard user (uid 501), no TUN device, no route change, no `sudo`. | See the note below |

Note on the macOS firewall. The firewall on this Mac is on. After the runs, its application list
has the entry "anima-link: Allow incoming connections". The spike did not add it by command. It
is not known if macOS showed a prompt and the operator accepted it, or if macOS added it with no
prompt. Without that entry the link still works through the relay, but a direct path is less
probable. The answer matters for S1 and for a signed application bundle (Section 7, step 7).

## 7. Not measured here, and the steps to measure it

Not measured:

1. **The direct-connection rate across real networks.** One machine always finds a direct path.
2. **How a relayed RDP or Screen Sharing session feels.** The model gives latency and speed, not feel.
3. **If the Headscale relay on the VPS accepts link keys** (Section 5 gives the expected answer).
4. Speed and latency of Tailscale's shared relays (Section 6: the one run is not valid).
5. If the slow first connection (3.5) and the idle stall (3.6) occur between two machines.
6. The macOS firewall prompt for a new machine.

Not in the spike: invites (LNK-003), key expiry (LNK-005), a domain name for the address
(LNK-007), UDP through the link.

Steps for the operator with a second Mac (B). Mac A is this one. Go 1.27 is necessary on a Mac
that builds; or build on A and copy `bin/anima-link` to B.

1. On A and B: `cd spikes/link && go build -o bin/anima-link .` Set `RUN` to a private directory
   outside the repository on each Mac.
2. On B: `bin/anima-link pubkey -state $RUN/device.json -out $RUN/device.pub`. Copy `device.pub`
   to A as `$RUN/allow.txt`.
3. On A: `bin/anima-link testhub -listen 127.0.0.1:18080 &` then
   `bin/anima-link serve -state $RUN/hub.json -allow $RUN/allow.txt -target 127.0.0.1:18080 -addr-file $RUN/hub.addr -control 127.0.0.1:18082`.
   Copy `hub.addr` to B by AirDrop. It is a secret: do not put it in chat or a log.
4. On B: `bin/anima-link connect -state $RUN/device.json -addr-file $RUN/hub.addr -listen 127.0.0.1:18081 -status 127.0.0.1:18083`.
   Then `curl http://127.0.0.1:18081/health`. Record the time of this first request.
5. **Direct rate.** On B, run `curl http://127.0.0.1:18083/path` each 2 s for 30 s. Record
   `direct` or `relayed` and the time to `direct`. Do this for each network pair: the same LAN;
   B on a telephone hotspot; B on a guest or office network; A on the hotspot and B at home.
   Stop and start both processes between pairs. When the path is `direct`, run
   `bin/anima-link bench -url http://127.0.0.1:18081 -mib 200 -label <pair>`. Do not use `-mib`
   when the path is `relayed`.
6. **Relayed remote display.** On A, turn on Screen Sharing and start a second hub link with
   `-target 127.0.0.1:5900`, a new `-state` and `-addr-file`, and no `-control`. Start both sides
   with `TS_DEBUG_ALWAYS_USE_DERP=1`. On B, open `vnc://127.0.0.1:18081` in Screen Sharing. Work
   for 5 minutes: type, drag an application, scroll. Check `/path` shows `relayed`. Record if it can be
   used. Then remove the variable, start both sides again and compare with the direct path. This
   uses Tailscale's shared relay for a normal session, not for a speed test.
7. **Firewall.** On B, note if macOS shows "accept incoming network connections" at step 4, and
   if "Allow" asks for an administrator password. On A, tell this session if a prompt appeared today.
8. **Faults 3.5 and 3.6.** On B, stop and start `connect` 10 times and time the first `curl`
   each time. Then leave the link idle for 4 minutes and time the next `curl`.
9. **Headscale relay (the operator only, VPS-005).** On A, start a hub link with
   `-relay <the Headscale host>:443` and a new `-state` and `-addr-file`. Connect from B. If the
   request fails after 13 s and the Headscale log shows a refused client, the answer to fact 10
   is "no". This step only connects to the relay as a client. It changes nothing on the VPS.
10. **A relay of our own on a real network** needs a host that both Macs can reach. That is a
    cloud or VPS change. It was not planned here and needs the operator's confirmation.

## 8. Changes for plan Section 4.1 (S1) and Section 3

1. **Fact 4 is wrong for `v0.7.0`.** `Server.AllowClient`, `Server.PeerKey`,
   `Server.DisconnectClient` and `KeySet` are only on `main` (after 2026-09-19). `v0.7.0` has
   `AllowedClients` and `AddAllowedClient`, with no way to remove a key. S1 must pin a commit of
   `main` until the next release. This is the risk "Tailcat is before version 1.0" in practice:
   the interface changed in under two months.
2. **LNK-001: add what the hub must save.** The key, the pre-shared key and the whole relay
   region. Add a rule for a relay change (LNK-007): the address changes, so the hub must give the
   new address to each permitted device before it changes.
3. **LNK-004: say who closes the session.** `anima-link` records streams by device key and closes
   them before `DisconnectClient`. The order is: remove the key, close the streams, disconnect.
   A test must hold an open session and see it end.
4. **New requirement (proposed LNK-009): the link must stay up.** The device link sends a
   keepalive each 15 s that also registers the device again. This answers the hub restart (3.2)
   and the idle stall (3.6). The acceptance value needs the second-machine result (3.5): for
   example "a request after a hub restart works in 20 s with no user action".
5. **LNK-006: say where the path comes from.** The hub reads it with no traffic. The device must
   ping. The relay name is known only on the hub. The hub must drop disconnected peers from its list.
6. **LNK-002: add the refused case.** A refused device gets no answer. The client message for
   "refused" and for "hub off" is the same. Each failed attempt takes about 13 s.
7. **LNK-007 and fact 10.** The Headscale relay is set to refuse link keys. A relay for the link
   is a separate `derper`. Set its queue depth higher than 32. A hub link that starts on a relay
   that refuses it gives no error, so S1 needs a start check (for example a ping to itself
   through the relay).
8. **LNK-007, a domain name for the address.** The address holds the pre-shared key. Tailcat's
   documents say that an address in DNS is public. Then the list of permitted keys is the only
   protection. State this in the requirement.
9. **Section 7, risk "Relayed traffic on hard networks".** Add the numbers: about 4 to 9 Mbit/s
   at 40 ms and under 1 Mbit/s at 160 ms. S2 needs a low-rate display mode for relayed paths.
   The client should warn before a remote-display session on a relayed path.
10. **Tailscale's shared relays.** Tailcat's own speed test refuses to run through them. Do not
    plan the free mode's remote display on them without the result of Section 7, step 6.
11. **Size.** `anima-link` adds about 15 to 17 MB for each platform (about 6 MB compressed). A
    universal macOS binary is about 31 MB (the sum of the two; not built).
12. From spike K-C: a parent that is killed leaves its child process running on macOS. The
    application must stop an old `anima-link` at start.

## 9. Left on this Mac

- The macOS firewall list has an entry for `anima-link` (Section 6). Removal needs an administrator.
- Go modules in the user's module cache.
- No process is running. Keys and addresses were only in the session's scratch directory, not in
  the repository.
