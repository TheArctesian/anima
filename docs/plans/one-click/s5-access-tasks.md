---
kind: Task breakdown: access rules by device key
state: In progress
evidence: T1 to T9 have commits on `main` (merge `f367f819`); T10, the daemon's link origin, has its commit. Open: the K7 row of its Section 6.3 and one test of daemon, link and hub together. Its parent plan is [one-click-plan.md](one-click-plan.md).
---

# S5. Access rules by device key: task list

Progress: the plan register in [work-plan.md](../../work-plan.md) Section 9 holds the state of this plan (PLN-005).

Plan item S5 of `docs/plans/one-click/one-click-plan.md` (Section 4.5), built on S1 (`s1-link-tasks.md`).
Security tag: each task gets the `reviewer` and the `reviewer-security` daemon.

## 1. What the hub does today

- **NET-001, NET-002.** `fleet/access/layer.rs` (`Edge`) refuses a TCP peer whose address is not in
  `access.permitted_networks`, at accept and again for each request. A request that passes carries
  its `ClientAddr`.
- **NET-003:** `fleet/access/listen/bind.rs` lets the hub listen only on loopback, private and VPN
  addresses.
- **BLD-024:** `fleet/builds/builders.rs` keeps the `ClientAddr` of the enrolment and refuses a
  builder token from another address.
- **Daemon token path (K8).** `fleet/metrics/ingest` limits requests and refusal events for each
  `ClientAddr`.
- Sign-in limits, session tickets and the audit log also use `ClientAddr`.

With the S1 link, every device reaches the hub from `127.0.0.1`. Thus each of these rules sees one
client. A builder token would work from each device, and one device could use the request share
of all daemons.

## 2. Design

### 2.1 Each permitted key has one link address

The hub link gives each device one address when it permits the device's key (LNK-002):
`fd41:4e49:4d41::/48` and 80 bits of a SHA-256 hash of that key. The address is in the device list.
It stays the same when the device renews its key (LNK-005), and it leaves the list with the device
(LNK-004). Two devices cannot have the same address: the list refuses the second one.

The address is not a network address. No packet is sent to it. It is the name of a device key in
the form that the hub's rules, its database (`inet` columns, K2) and its audit log already use.
Thus the hub rules become key rules with no change to a frozen contract.

### 2.2 The link reaches the hub through a Unix socket

- The hub listens on a Unix socket (`ANIMA_LINK_SOCKET`) as well as on loopback. The socket's
  folder must be the hub user's, with no access for other users. The socket has mode `0600`.
- `anima-link hub -target /path/to/socket` connects each stream to that socket. It first sends a
  PROXY protocol header (version 2) whose source address is the device's link address.
- The hub reads the header, refuses a connection that has none or that names an address outside
  the link range, and takes the source address as the request's `ClientAddr`.
- Only a process of the hub's own user can open the socket. Thus only the link can name a device.

### 2.3 The link mode of the hub

`ANIMA_LINK_SOCKET` set means the link mode:

- `ANIMA_LISTEN` must be a loopback address (NET-003 in the link mode; acceptance criterion 4).
- `ANIMA_PERMITTED_NETWORKS` must not be set. The permitted networks are loopback (this
  computer) and the link range.
- Every other rule is the same code as today. Without `ANIMA_LINK_SOCKET` nothing changes
  (acceptance criterion 6).

### 2.4 What the rules become in the link mode (proposed spec delta, not applied)

- **NET-001 (link).** The list of permitted clients is the list of permitted device keys of the
  link, and this computer.
- **NET-002 (link).** The link refuses a key that is not in the list before any stream reaches the
  hub. The hub refuses, and writes to the event log, a link connection that does not name a
  permitted device.
- **NET-003 (link).** The hub listens only on loopback and on the link's socket.
- **BLD-024 (link).** The hub refuses a builder request from a device other than the device that
  enrolled the builder.
- **K8 (link).** The limits "per client address" count for each device.

## 3. Decisions

| ID | Decision | Reason |
| --- | --- | --- |
| E-1 | A device is named to the hub by a link address, not by its raw key | `ClientAddr` and the `inet` columns stay. No change to K1 or K2. |
| E-2 | The address comes from the key at enrolment and does not change at renewal | A builder and a session stay bound to the device for longer than 180 days. |
| E-3 | The hub does not read the device list. The link is the one place that permits a key | One list, one owner (S1). The socket lets only the link speak. |
| E-4 | The PROXY header no longer has the key field `0xE0` of S1 | The hub does not use it. The address names the device. |
| E-5 | A socket target always gets the header. A loopback target never gets it; the option `-name-device` of S1 is removed (security review) | Each local user can connect to a loopback port and write a header, so a header there proves nothing. |
| E-6 | The link checks, at each connection, that the socket and its folder belong to its user | A socket in a shared folder could be another user's program that takes sign-ins. |
| E-7 | Loopback stays permitted in the link mode | The application and `anima` on the hub computer use it. Sign-in is still needed. |
| E-8 | New variable `ANIMA_LINK_SOCKET`. The table in K7 (`contracts/conventions.md`) is not changed | K7 is frozen. `ANIMA_VPN_BROKER` is the same case. Proposed row: Section 6. |
| E-10 | A key or an address that left the list is never permitted again (security review) | The hub keeps what it bound to an address. An old key must not get it back. |
| E-9 | Reviews: one `reviewer` and one `reviewer-security` pass for the link tasks, and one of each for the hub tasks | The tasks of one side change the same files. |

## 4. Tasks

| Task | Content | Proof |
| --- | --- | --- |
| T1 | `link/devices`: link address of each device; unique; kept at renewal; shown by `devices.list` | Go tests |
| T2 | `link/hublink`: socket target, header with the link address, owner check of the socket (E-4 to E-6) | Go tests; a header file that the hub's tests also read |
| T3 | Link rules through the socket, from end to end: LNK-002, LNK-003, LNK-004, LNK-005 | One Go test for each rule |
| T4 | Hub configuration: `ANIMA_LINK_SOCKET` and the rules of Section 2.3 | Unit tests |
| T5 | Hub: PROXY header reader (pure) | Unit tests with the header file of T2 |
| T6 | Hub: the link socket and its accept path; refusal events; wiring in `main.rs` | Unit tests |
| T7 | Hub, as a process: requests through the socket carry the device; refusals; BLD-024 and the daemon limits for each device | Integration tests |
| T8 | Documents: `docs/procedures/setup/link.md`, this file's results, the list for S6 | — |
| T9 | Full gates on the branch with `main` merged in | Gate record |
| T10 | Daemon: the link origin `http://127.0.0.1:<port>` (K8 Section 1, MET-020; operator's ruling of 2026-11-15) | Unit tests of the origin rule; one push through the client |

`main.rs` gets the wiring of the socket (T6). `schema.rs`, `contracts/` and `quality/` do not change.

## 5. Not in S5

- **The daemon's own transport: built after S5 (T10).** The operator ruled on 2026-11-15 that the
  daemon may use HTTP to a loopback port in the link mode. K8 Section 1 and MET-020 have the text.
  `anima-daemon enrol --hub http://127.0.0.1:<port>` now works: the origin rule accepts `http` only
  for `127.0.0.1` with a port from 1024 (`crates/daemon/src/config/origin.rs`). The token stays bound
  to that origin, port included, so `anima-link device` must use a fixed `-listen` port on a
  computer that runs the daemon. Known remainder: a program of any local user of that computer that takes
  the port while the link is stopped receives the daemon token with the next push, and nothing
  detects it. The planned correction is a Unix socket between the daemon and the link (the HTTP
  client and the link can both do it); it needs a new origin form in K8 and a ruling. The security
  review (accept, nothing to correct before the merge) also found that MET-023 (`doctor`, not
  built) speaks of TLS on port 443 and needs a clause for the link mode before it is built. Not proved here:
  one test with a real daemon, a real link and a real hub together (Rust and Go in one test); the
  daemon half is proved against a local HTTP server, the hub half by the tests of T7.
- The hub API, the command and the screen for invites and the device list (S6; needs K1).
- Starting `anima-link` and the hub as services (S4).

## 6. What was built

| Task | Result | Commit |
| --- | --- | --- |
| T1 to T3 | Link address in the device list; socket target with the header; tests of LNK-002 to LNK-005 by what a hub on the socket sees | `9142c7e9`, and the corrections after review |
| T4 to T7 | `ANIMA_LINK_SOCKET` and the link mode; the header reader; the socket and its accept path; tests of the whole hub through the socket | `84340c05`, and the corrections after review |
| T8 | `docs/procedures/setup/link.md` Section 7; this section | — |
| T9 | Full gates on `5d353164` (the branch with `main` at `bb4c00d6` merged in): 20 of 20 pass; Rust 4731 tests passed, 1 skipped | — |

### 6.1 Tests for each rule

| Rule | Test |
| --- | --- |
| LNK-002 | `link/hublink`: `TestLNK002TheHubsSocketGetsEachDeviceByItsOwnAddressAndNoOtherKey`. Hub: `lnk_002_a_request_through_the_socket_has_the_named_device_as_its_peer` |
| LNK-003 | `link/devicelink`: `TestLNK003ADeviceThatJoinedIsSeenByTheHubUnderItsOwnAddress` (the second use of the invite reaches neither the socket nor the list) |
| LNK-004 | `link/devicelink`: `TestLNK004ARevokedDevicesSessionEndsAtTheHubAndItReachesTheHubNoMore`. `link/devices`: `TestLNK004AKeyThatLeftTheListIsNeverPermittedAgain` |
| LNK-005 | `link/devicelink`: `TestLNK005TheHubSeesTheSameDeviceAfterAKeyRenewal`, `TestLNK005AnExpiredKeyDoesNotReachTheHubsSocket` |
| NET-001 (link) | `config::link`: `net_001_in_the_link_mode_the_clients_are_this_computer_and_the_links_devices` |
| NET-002 (link) | Hub: `net_002_a_socket_connection_that_names_no_device_gets_no_answer_and_is_logged`, `net_002_a_header_names_no_address_outside_the_link_range` |
| NET-003 (link) | `config::link`: `net_003_in_the_link_mode_the_hub_listens_on_loopback_only`. Hub: `net_003_a_folder_other_users_can_reach_gets_no_socket` |
| BLD-024 (link) | Hub: `bld_024_a_builder_token_works_only_from_the_device_that_enrolled_it` |
| K8 (link) | Hub: `k8_an_daemon_refusal_is_logged_for_each_device_of_the_link` |

The link's test and the hub's tests read one file, `link/hublink/testdata/device-header.bin`, so
the two programs agree on the header.

### 6.2 What the security reviews changed

| Finding | Correction |
| --- | --- |
| Medium, link: after a revocation, a key that once named the device (also the key that a renewal replaced) could join with a new invite and get the revoked device's address, and so its builder and its history | E-10: `retired.json` holds each key and address that left the list; none is permitted again |
| Low, link: a device in its renewal could name a revoked key as its next key | The same record refuses it |
| Low, link: a socket path through a symbolic link in a folder that others can write passed the check | Such a link is refused |
| Note, link: a header on a loopback port proves nothing, because each local user can write one | E-5: `-name-device` is removed |
| Medium, hub: clients of the loopback port could take every connection place, so that no device was served (on Linux, where a user can connect from many loopback addresses) | The link's devices have their own places |
| Low, hub: on macOS an inherited access-control entry can give another user access to a socket of mode `0600` | The hub serves a socket connection only when the connecting process has the hub's user |
| Low, hub: a race through a folder above the socket's folder | The same check of the connecting user |

Not corrected, by decision: each local user of the hub computer can reach the loopback port and
must sign in (E-7). The check of the connecting user has no test: a test cannot connect as
another user.

### 6.3 Proposed row for K7 (not applied)

| Variable | Meaning | Default |
| --- | --- | --- |
| `ANIMA_LINK_SOCKET` | Path of the Unix socket for `anima-link`; set means the link mode: the hub listens on loopback and this socket only, and `ANIMA_PERMITTED_NETWORKS` must not be set | empty: the hub is not in the link mode |

## 7. What S6 (first run) and S4 (service) need from S5

1. **Start order and user.** The hub and `anima-link hub` run as one user. The hub makes the
   socket; the link connects at each stream, so either can start first. S4 owns the two service
   units; S5 changed neither.
2. **The socket's folder.** A folder of that user with mode `0700` and a short path (a socket
   path has at most about 100 characters). The link's state folder is a good place: it already
   has these properties.
3. **The target.** Start the link with the socket as `-target`, never with the hub's loopback
   port (see `docs/procedures/setup/link.md` Section 7).
4. **Environment of the hub.** `ANIMA_LINK_SOCKET` and a loopback `ANIMA_LISTEN`; no
   `ANIMA_PERMITTED_NETWORKS`; no TLS files (loopback serves plain HTTP).
5. **`ANIMA_PUBLIC_ORIGIN`.** The hub compares a browser's `Origin` with it, and each device
   reaches the hub at `http://127.0.0.1:<its local port>`. S6 must decide the origin rule of the
   link mode: one fixed local port on each device, or a rule for loopback origins. S5 did not
   change it. The desktop client and `anima` send a bearer token and are not affected.
6. **Invites and the device list in the hub.** The operator makes an invite, sees the devices
   with their addresses and revokes one through `anima-link ctl` today. S6 needs these in the hub
   API, the `anima` command and the client (CMD-030). That is a change to K1; the control socket's
   verbs (`invites.make`, `devices.list`, `devices.revoke`) then become a contract between the
   hub and the link.
7. **Names for addresses.** The audit log and the list of builders show a link address. The
   client should show the device's name beside it, from `devices.list`.
8. **A device that joins again.** A revoked device needs a new key before it can join again. The
   join flow of S6 must make a new key when the hub answers `key_retired`.
9. **The daemon.** The metrics daemon can use the link only after the ruling on K8 (Section 5).
10. **Recovery.** A lost hub key means a new hub address and new invites for each device; the
    hub's bindings to link addresses stay valid only if the link's state folder (`devices.json`,
    `retired.json`) is restored with the hub's database. The backup (S9) must hold both.
