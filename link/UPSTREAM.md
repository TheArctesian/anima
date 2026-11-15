# The Tailcat library

`anima-link` uses `github.com/tailscale/tailcat`. Only the `tunnel` package imports it.

## The pin

| Item | Value |
| --- | --- |
| Version | `v0.7.1-0.20261105151022-e00cdde6ff78` (branch `main`, 2026-11-05) |
| Why not a release | The last release, `v0.7.0`, does not have `Server.AllowClient`, `Server.PeerKey`, `Server.DisconnectClient` or `KeySet`. |
| Move to a release | When a release has these functions (expected `v0.8.0`). Change `go.mod`, run the gates, read the changelog. |

Tailcat is before version 1.0. Its interface changed between `v0.7.0` and this commit.

## Behaviour that the link relies on

Each line has a test in `tunnel/pinning_test.go` or `tunnel/tunnel_test.go`. An update that
changes one must fail there.

| Behaviour | Why the link needs it |
| --- | --- |
| A device that `AllowClient` refuses gets no answer. A device that is admitted later connects with no hub restart. | LNK-002 |
| `PeerKey` gives the key that the tunnel authenticated for an accepted stream. | The hub closes the streams of one device at revocation (LNK-004). |
| `DisconnectClient` stops the traffic of a device and does not close its streams. | The link closes the streams itself, before the call (LNK-004). |
| The same key, pre-shared key and relay region give the same address. | LNK-001 |
| `Status` shows a direct or a relayed path for each device, with the relay's name. `DiscoPing` shows it on the device. | LNK-006 |

## Known faults of the library (spike K-A, 2026-11-14)

| Fault | Answer in the link |
| --- | --- |
| A client registers with the server only at its first use. After a server restart, a running client is unknown and gets no answer. | The device role registers again in its keepalive. |
| A link that is idle for 150 s or more stalls for 1 to 5 s at the next request. | The keepalive. |
| The first connection of a new process sometimes takes 5 to 10 s. The cause is not known. | Not answered yet. The second-machine check decides. |

No upstream issue is open for these. Opening one is the operator's decision.

## How to follow upstream

Before each release of Anima, and when Tailcat makes a release: read its `CHANGELOG.md` from the
pinned commit to the new one, change the pin on a branch, and run `bun run gates`.
