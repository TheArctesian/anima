# S5. Access rules by device key: task list

Plan item S5 of `docs/one-click-plan.md` (Section 4.5), built on S1 (`s1-link-tasks.md`).
Security tag: each task gets the `reviewer` and the `reviewer-security` agent.

## 1. What the hub does today

- **NET-001, NET-002.** `fleet/access/layer.rs` (`Edge`) refuses a TCP peer whose address is not in
  `access.permitted_networks`, at accept and again for each request. A request that passes carries
  its `ClientAddr`.
- **NET-003.** `fleet/access/listen/bind.rs` lets the hub listen only on loopback, private and VPN
  addresses.
- **BLD-024.** `fleet/builds/builders.rs` keeps the `ClientAddr` of the enrolment and refuses a
  builder token from another address.
- **Agent token path (K8).** `fleet/metrics/ingest` limits requests and refusal events for each
  `ClientAddr`.
- Sign-in limits, session tickets and the audit log also use `ClientAddr`.

With the S1 link, every device reaches the hub from `127.0.0.1`. Thus each of these rules sees one
client. A builder token would work from each device, and one device could use the request share
of all agents.

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
| E-5 | With a socket target the header is always sent. `-name-device` stays for a TCP target | A hub on the socket cannot work without the header. |
| E-6 | The link checks, at each connection, that the socket and its folder belong to its user | A socket in a shared folder could be another user's program that takes sign-ins. |
| E-7 | Loopback stays permitted in the link mode | The application and `anima` on the hub computer use it. Sign-in is still needed. |
| E-8 | New variable `ANIMA_LINK_SOCKET`. The table in K7 (`contracts/conventions.md`) is not changed | K7 is frozen. `ANIMA_VPN_BROKER` is the same case. Proposed row: Section 6. |
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
| T7 | Hub, as a process: requests through the socket carry the device; refusals; BLD-024 and the agent limits for each device | Integration tests |
| T8 | Documents: `docs/setup/link.md`, this file's results, the list for S6 | — |
| T9 | Full gates on the branch with `main` merged in | Gate record |

`main.rs` gets the wiring of the socket (T6). `schema.rs`, `contracts/` and `quality/` do not change.

## 5. Not in S5

- **The agent's own transport.** K8 Section 1 (frozen) says that the agent uses HTTPS on port 443.
  An agent that reaches the hub through the link uses HTTP to a loopback port of `anima-link`. That
  is a change to K8 and to MET-020, and it needs a programme ruling. Proposed text: "In the link
  mode the agent's hub origin is `http://127.0.0.1:<port>`, the local port of `anima-link device`.
  The link authenticates the hub by its key." The hub side of the agent path is built here (T7).
- The hub API, the command and the screen for invites and the device list (S6; needs K1).
- Starting `anima-link` and the hub as services (S4).
