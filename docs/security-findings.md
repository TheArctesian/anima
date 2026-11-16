# Security Findings

| Item | Value |
| --- | --- |
| Document | The list of security findings and the state of each |
| Date | 2026-11-16 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Kind | Record. It adds no requirement (PLN-010). |

This document lists each security weakness that a person or an agent found in Anima.ing. It is
not a GitHub issue list. A requirement that corrects a finding goes in the plan or the proposal
that owns the area.

## How to use this list

- Each entry has an identifier `SF-nnn`. Do not use an identifier again. `SEC-nnn` identifiers
  are requirements of the design specification. Do not use them here.
- An entry gives: what the weakness is, where it is (files), the effect and who can cause it,
  the mitigation that is present now, the state, and a link to its commit or its human test
  (`HT-nnn` in [need-human-testing.md](need-human-testing.md)).
- The states are Open, Accepted and Corrected:
  - Open: the weakness is present, or a correction is built but a person did not test it.
  - Accepted: the operator accepts the weakness as it is. The entry gives the reason.
  - Corrected: the correction is merged and its tests and its human test passed.
- Only the operator sets an entry to Accepted or Corrected. If the operator tells an agent the
  decision, the agent writes it in the entry with the date.
- When a correction is built, write "built" and the commit. Do not write "corrected" until the
  operator closes the entry (`AGENTS.md`, section "Human testing").
- Do not delete an entry. Move a closed entry to "Closed" at the end of this document.

## Open

| Entry | Subject | Area | State |
| --- | --- | --- | --- |
| SF-001 | "This machine" can show and probe the address of another overlay network | Desktop app | Open |
| SF-002 | The web view chooses the login server that the desktop shell trusts | Desktop app | Open |

### SF-001: "This machine" can show and probe the address of another overlay network

Requirements: ADR-005
([cve-watch-and-network-awareness.md](programme/proposals/cve-watch-and-network-awareness.md)),
R-060 ([programme/programme.md](programme/programme.md)).

What it is: the desktop app found its own address on the Anima VPN as the lowest IPv4 address
of this computer in `100.64.0.0/10`. Headscale, Tailscale and NetBird all give addresses in
that range. On a computer that also runs NetBird or another overlay network, the app could
select an address that is not an Anima node.

Where: `apps/desktop/src-tauri/src/this_machine.rs` (`read()`) and
`apps/desktop/src-tauri/src/this_machine/address.rs` (`vpn_address_among`, now removed).

Effect: "This machine" in Add device showed the wrong address, and the SSH check (TCP port 22)
connected to it. The device that the operator added then had a host on a different network.
"Already added" (R-060), the device `/32` in the VPN policy (R-061) and the VPN match
(VPN-011) used the same wrong address. Nobody must attack to cause it: a second overlay network
on the same computer is sufficient. An administrator of that other network controls the node
that receives the probe and, later, the sessions of the operator.

Mitigation now: built and merged to `main` (fast-forward of `claude/vpn-own-address`). No
person tested it yet.

- bdb321f1: the desktop shell asks the Tailscale client for its own address
  (`tailscale status --json`, `Self.TailscaleIPs`). It accepts the address only when the client
  runs and is logged in to the Anima login server (`tailscale debug prefs`, `ControlURL`). It
  does not read the interfaces. Each question to the client has a limit of 1 second.
- 71bfcbdb: the hub is the one source of the login server (`Query.vpnLoginServer`).

State: Open, until the human test [HT-017](need-human-testing.md#ht-017-this-machine-with-the-anima-vpn-and-netbird-together)
passes.

### SF-002: the web view chooses the login server that the desktop shell trusts

Requirements: ADR-005, R-060. Found while SF-001 was built (71bfcbdb).

What it is: the shell command `this_machine_read` takes the login server URL as an argument
from the web view (`loginServer`). The shell accepts the VPN address of a Tailscale client that
is logged in to that URL. The shell does not check the URL against the hub.

Where: `apps/desktop/src-tauri/src/this_machine.rs` (`this_machine_read`),
`apps/desktop/src-tauri/src/this_machine/vpn_client.rs` (`signed_in_to`),
`apps/client/src/lib/desktop/this-machine.ts` (`readThisMachine`).

Effect: script that runs in the web view, for example after a cross-site scripting fault, can
send a different URL. The shell then reports, and probes with a TCP connection to port 22, the
own address of this computer on a tailnet that the script chooses. The address can be the
address on a tailnet that is not Anima's, so the operator can add the wrong host. Only script in
the web view can cause it.

Mitigation now:

- The shell accepts only an IPv4 address inside `100.64.0.0/10`.
- The shell accepts only an address that the local Tailscale client reports for this computer.
  The script cannot name a different computer.
- Each question to the Tailscale client has a limit of 1 second, and the SSH check has a limit
  of 300 ms. The script cannot make the shell wait for a long time.
- No secret crosses: the shell sends no credential, token or key to the address. The SSH check
  only opens and closes a TCP connection.

Options to close it (not built):

- The shell compares the URL with an allow list, for example the login server of the hub that
  the app signed in to, kept by the shell.
- The shell takes the login server only from the hub, with its own request, and the web view
  passes nothing.

State: Open.

## Closed

None.
