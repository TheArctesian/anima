# Draft issues for the Tailcat project

Status: **drafts. Not filed.** Filing an issue is public, so it is the operator's decision. Each
draft is written for `github.com/tailscale/tailcat` and holds no key, address or host name of ours.

All were seen with Tailcat `main` at `e00cdde6ff78` (2026-11-05), Go 1.27, macOS 27 on arm64,
with the Go library (not the CLI). Reproductions use a relay on loopback from
`tailscale.com/tstest/integration.RunDERPAndSTUN`.

## 1. A running client is not reachable after the server restarts with the same key

**What happens.** A `Client` sends its announce ("meow") only at its first use (`Client.up`
returns at once when `upDone` is set). A `Server` that restarts with the same key and pre-shared
key has an empty client table. A client process that stayed running then gets no answer: each
`DialTCPPort` waits until its context ends. The client does not recover by itself.

**Expected.** The client announces again when the server stops answering, or the library
documents that the application must call `Client.Ping` again.

**To reproduce.** Start a server with a saved key and a `Listen` on one port. Dial from a client
and exchange bytes. Close the server and start a new `Server` with the same `Key`,
`PresharedKey` and `Region`. Dial again from the same `Client`: the dial times out.

**What we do now.** We call `Client.Ping` in a keepalive when a `DiscoPing` fails. After that, the
first dial can still take about 10 to 15 seconds.

## 2. The first connection of a client is sometimes 5 or 10 seconds late

**What happens.** For some client keys, the first `DialTCPPort` of a new client process takes
5.0 to 5.3 seconds, sometimes about 10 seconds, in place of about 10 milliseconds. The state
belongs to one key: each new process with that key is slow, until the server forgets the key
(`Server.DisconnectClient`); then it is fast again. Other keys on the same server are fast at the
same time. The delay matches the WireGuard handshake retry time. It was seen with both ends on
one machine; we have not yet tested two machines.

**Expected.** A first connection in the time of one handshake.

**To reproduce.** Not exact. Start a server. In a loop: start a client process with a saved key,
dial, exchange bytes, stop the process. On our machine, about one key in three goes into the
slow state and stays in it.

## 3. A server gives no error when its relay refuses its key

**What happens.** With a relay that accepts only known keys (`derper -verify-client-url` with
an answer of "not allowed"), `Server.Start` returns no error and `TailcatAddr` gives an address.
Clients then time out. The relay's log shows the refusal; the server shows nothing.

**Expected.** `Start` (or a status call) reports that the relay connection was refused.

**What we do now.** At start we announce from a second key in the same process and wait for the
server's answer through the relay.

## 4. An idle link stalls for seconds at the next use

**What happens.** A client that made a request, then sent nothing for 150 seconds or more,
waits 1 to 5 seconds at its next dial (150 s idle: about 1 s; 240 s idle: about 5 s). With one
request each 15 seconds there is no stall.

**Expected.** No stall, or a documented keepalive that the application must send.

## 5. A note for the documents: `AllowClient` runs before the client has proved its key

`AllowClient` is asked when the announce arrives. The announce is authenticated by the relay
only, not by a WireGuard handshake. An application must therefore not change its own state in
`AllowClient` (for example "this key connected, so drop the old key"). A sentence in the
documents of `AllowClient` would prevent this mistake; we made it once.
