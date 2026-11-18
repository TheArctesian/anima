# Deferred work

This document is a record, not a plan (PLN-010, work plan Section 9). It holds work that is decided and left for later. It replaces GitHub issue #1 (R-119, which replaces R-007).

| Item | Value |
| --- | --- |
| Document | Deferred work: the list |
| Version | 1.1 |
| Date | 2026-11-16 |
| Status | In use. Add a deferred item here, in the commit that defers it. |
| Origin | GitHub issue #1: made 2026-11-04, last changed 2026-11-13, with 27 comments. Copied on 2026-11-16. Only the links were changed. Not written in STE yet. |
| Order | Version 1.1 puts the lists in areas (operator, 2026-11-16). The text of each list did not change. Each heading keeps the package and the date of the list. The line "Revisit" of each area is a proposal; the operator has not approved it. |

Rules:

- An item here is not a requirement. Before a build, put the requirement in the plan that owns it (work plan Section 5).
- Tick an item in the commit that does the work, or delete it with a reason in the commit message.
- Add a new item to the area of its subject, in the list of its package. If there is no such list, add a heading with the package and the date. Give the requirement identifiers.
- The handoff of 2026-11-12 says that some items were not posted to the issue and were only in a scratchpad of the orchestrator (`issue1-pending.md`). They are not in this document. Rebuild them from the Outcome sections of the package logs if they are necessary.

## Areas

| Area | Lists | Items with a box | Revisit |
| --- | --- | --- | --- |
| 1. Releases, signing and installation | 1 | 4 | Before the first public release (tasks R5 to R7 of projects-and-builds-plan.md). |
| 2. Later phases and proposals | 9 | 27 | When the proposal or the phase of the item gets a ruling. Each proposal has a row in the work plan Section 9. |
| 3. Ideas from other products | 2 | 17 | When the plan that owns the subject is changed next. |
| 4. Hub: access, logs and security | 7 | 24 | Before a hub is hosted for a different person, and at the next security review of access. |
| 5. Hub: devices, checks and metrics | 9 | 20 | When the domain of the item is changed next. |
| 6. Sessions and the gateway | 6 | 23 | With the native gateway work (S2), or when sessions change next. |
| 7. Deployment, VPN, network and the agent | 5 | 25 | Before the next change to the VPS, and before the hosted launch. |
| 8. Client | 19 | 57 | With the redesign of the interface (ui-redesign-plan.md). |
| 9. Desktop shell | 2 | 9 | With the packaging of the one-click work. |
| 10. Quality, gates and tests | 6 | 6 | Before the Phase 3 acceptance run, and at the next work on the gates. |
| 11. Lists that mix many packages | 2 | 70 | Split each list into the areas above when its items are read next. They are not split in this version. |
| 12. Notes on the sources | 3 | 0 | These lines came with the lists. They hold no item. |

## 1. Releases, signing and installation

Revisit: Before the first public release (tasks R5 to R7 of projects-and-builds-plan.md).

### Releases and signing

Added 2026-11-04 to 2026-11-13, top of the issue.

- [ ] **iOS TestFlight release pipeline** (Phase 5, spec Section 10). Build and sign the iOS app, upload to TestFlight with an App Store Connect API key, run it in the release workflow on push to `main` (R-005 allows only deployment workflows). iOS updates come from TestFlight / the App Store, not the Tauri updater (UPD-001).
- [ ] **macOS Developer ID signing and notarization.** Releases start with ad-hoc signing (spec 3.9). Add the Apple secrets (`APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_API_KEY`, `APPLE_API_ISSUER`); the release workflow switches to real signing, `notarytool submit --wait` and stapling when all are present.
- [ ] **Windows desktop build** (out of scope, spec 1.4). NSIS installer, `windows-x86_64` feed entry, optional Authenticode signing. Needs a spec change first.
- [ ] **Web client hosting** (Phase 3). Where the web build is served on the private network and how it is deployed.

## 2. Later phases and proposals

Revisit: When the proposal or the phase of the item gets a ruling. Each proposal has a row in the work plan Section 9.

### Later phases (from spec Section 9)

Added 2026-11-04 to 2026-11-13, top of the issue.

- [ ] Phase 2: RDP, VNC, web interfaces, Wake-on-LAN, SFTP, metrics.
- [ ] Phase 3: web client, notifications, Prometheus endpoint, session sharing and recording.
- [ ] Phase 4: Proxmox integration.
- [ ] Phase 5: iOS client.
- [ ] Phase 6: Proxmox later functions, site agents, traffic data (Pi-hole DNS logs and router `conntrack`) as input for a future memory system (spec 4.3).

### Open decisions to verify

Added 2026-11-04 to 2026-11-13, top of the issue.

- [ ] OD-10: Headscale has no Tailnet Lock. Re-check before the VPN goes live (WP-03).

### Extensions (docs/plans/extensions-plan.md)

Added 2026-11-04 to 2026-11-13, top of the issue.

- [ ] **Extensions** are tracked as work items (they were issues with the `extensions` label before R-119): engine [E0-1](work-items/engine/E0-1.md)–[E0-8](work-items/engine/E0-8.md), extensions [XP-01](work-items/extensions/XP-01.md)–[XP-14](work-items/extensions/XP-14.md), decisions [SPEC](work-items/extensions/SPEC.md)–[OD-E3](work-items/decisions/OD-E3.md), accounts and keys [KEYS](work-items/extensions/KEYS.md), seams [SEAMS](work-items/extensions/SEAMS.md). Plan: `docs/plans/extensions-plan.md`. Start only after the device manager build is finished (PH-E-000).

### Proposals after Phase 1 (docs/proposals)

Added 2026-11-04 to 2026-11-13, top of the issue.

- [ ] **First-run setup in the app** (R-057, `first-run-setup.md`): `setupRequired`, a setup code in a root-only file, account and TOTP set up in the app. Needs an SEC-002 exception and a K3 change.
- [ ] **In-app enrolment** (R-055, `in-app-enrolment.md`): a root-owned broker on the VPS that issues pre-auth keys.
- [ ] **Add this machine by machine ID** (R-058, `add-this-machine.md`): HMAC of the OS machine ID, `machineId` on devices, `deviceByMachineId`. Replaces R-060's "same VPN address" check.
- [ ] **Open the VPN path when a machine is added** (R-061, `add-this-machine.md` "Opening the VPN path"): the R-055 broker writes one `device-<id>` grant from tag:hub on tcp 22/5900/3389 only. Needs a ruling that changes vpn.md Section 11.

- [ ] **SSH in a real terminal** (R-062, `ssh-terminal.md`): the hub opens SSH with russh and relays bytes over a ticketed `/terminal` WebSocket; the app renders with xterm.js (OSC 52 off, links confirmed). guacd stays for RDP/VNC. Fixes guacd 1.6.0 printing OSC 7/133 payloads. Needs RA-011, RA-040, RA-041 changes and K4 Section 8.

### R-055: in-app VPN enrolment

Added 2026-11-05.

**R-055: in-app VPN enrolment (operator request, after Phase 1)**
The operator wants joining the VPN to need no separate install and no terminal. The proposal is `docs/proposals/in-app-enrolment.md`.
- [ ] Spec change: a setup screen; invite and enrolment requirements; Settings "Add a computer" and "Add a site"
- [ ] Decision (a): embed tsnet/libtailscale in the Tauri shell (userspace VPN, app traffic only; check iOS)
- [ ] Decision (b): a root-owned VPS broker with three verbs (personal key, site router key, approve a site's own route), called by the hub with TOTP and an audit entry. This relaxes vpn.md §11 "no Headscale API keys"
- [ ] Re-enrolment before the 180-day expiry (VPN-003)
- [ ] Site options: a router or NAS with Tailscale, the Phase 6 site agent, or the app acting as router

### R-057: first-run setup in the app

Added 2026-11-06.

**R-057: first-run setup in the app (operator request, after Phase 1)**
No sign-up page; the single operator (OD-2) stays. A first-run "Set up Anima" screen replaces the SSH `anima-hub operator set` / `operator totp` steps, unlocked by a one-time, 1-hour setup code from the hub. The proposal is `docs/proposals/first-run-setup.md`. It pairs with R-055.
- [ ] Spec change: a setup-screen requirement; a SEC-002 exception for `setupRequired` and the setup mutation; setup-code handling under LOG-007.
- [ ] Ruling: where the setup code appears (stdout once, or a root-only file in the secrets folder).
- [ ] K3 change: `setupRequired` query and the setup mutation (access surface).
- [ ] Build: hub setup state, code and `anima-hub setup-code`; client setup screen with TOTP QR enrolment and a confirming code.

### R-057 update

Added 2026-11-06.

R-057 update: the operator ruled that the setup code is written only to a root-only file in the deploy secrets folder (0400), never to stdout or logs. The ruling checkbox above is done.

### R-058: "Add this machine" after sign-in

Added 2026-11-06.

**R-058: "Add this machine" after sign-in (operator request, after Phase 1)**
After sign-in the desktop app offers to add the computer it runs on, unless a device already has its machine ID. The ID is the OS hardware ID (macOS IOPlatformUUID, Linux /etc/machine-id, Windows MachineGuid) sent only as HMAC-SHA256 with a fixed Anima key, so it is stable across networks. MAC addresses were rejected because of per-network private addresses. Proposal: `docs/proposals/add-this-machine.md`.
- [ ] Spec change: desktop prompt requirement, device `machineId` field and `deviceByMachineId` query, hashing rule.
- [ ] Contract change: devices model and API.
- [ ] Build: Tauri shell reads and hashes the ID; modal pre-fills the Add device form (name, VPN address, services that answer); "Not now" and "Don't ask on this machine".
- [ ] Depends on WP-13 (Add device form).

### From R-060 (pre-filled Add device for this machine)

Added 2026-11-06.

**From R-060 (pre-filled Add device for this machine)**
- [ ] "Already added" matches only by VPN address, so a device saved with a DNS name for this Mac is not recognised and the Mac is offered again. The hashed machine ID in R-058 replaces this after Phase 1.
- [ ] Linux: when SSH does not answer on this computer, nothing explains why. R-059 Remote Login settings is Mac only.

## 3. Ideas from other products

Revisit: When the plan that owns the subject is changed next.

### Ideas from Coolify Sentinel (agent, K8)

Added 2026-11-04 to 2026-11-13, top of the issue.

Read from `coollabsio/sentinel` (Apache-2.0, Rust) on 2026-11-12. Sentinel itself is not adopted: it takes remote commands, listens on a port and is Linux and Docker only. These details are worth taking for `anima-agent`.
- [ ] **Push status in the agent** (MET-014, MET-018): keep the last attempt, last success, last error with its full cause chain (DNS, TLS and refused connection look the same in reqwest's own message) and the count of consecutive failures. First check what `crates/agent/src/report.rs` already gives.
- [ ] **Agent version on every push** (MET-017): a request header, so the hub can show an outdated agent. Needs a ruling on whether a header changes frozen K8.
- [ ] **`Retry-After` on a refusal** (K8 Section 6): let the hub tell an agent of a deleted device to wait up to 24 hours, in place of a retry every 10 minutes. A K8 change.
- [ ] **Virtual network interfaces** (MET-002): where the hub or the device card selects or adds interfaces, leave out bridges, veth, bonds, tunnels and VPN interfaces so traffic is not counted twice. Sentinel's list has 22 name prefixes. First check how the hub uses the per-interface series.
- [ ] **Container metrics** (spec 6.4 Docker container check, PVE-040 Docker hosts): calculate rates from the counters of the previous sample, keyed by container ID and not name; write no row on the first sample or after a counter reset; cache the inspect result until the container state changes.

### Ideas from Coolify (hub)

Added 2026-11-04 to 2026-11-13, top of the issue.

Read from `coollabsio/coolify` (Apache-2.0, PHP) on 2026-11-12. Coolify is a deployment platform, so Anima takes designs and read-only shell commands from its server-management layer, not code. File paths are in the Coolify repository.
- [ ] **Host patch status through SSH** (MET-002, spec 6.4 new check type): pending updates, security updates, held packages and "restart required" for apt, dnf, zypper and pacman, with read-only commands. Reference: `app/Actions/Server/CheckUpdates.php` (apt phased updates, dnf4 and dnf5 differences). Needs a new check identifier (CHK-009) and obeys CHK-012.
- [ ] **Container status rules** (CHK-004, spec 6.4 Docker container check, PVE-040): one priority order that makes one status from many containers, for example running + exited = Degraded. Reference: `app/Services/ContainerStatusAggregator.php`.
- [ ] **Notification claim row** (NOT-005): one row for each resource and notification that the hub claims atomically before it sends, with a limit of one reminder each day while a failure continues. Reference: `app/Models/NotificationThrottle.php`.
- [ ] **Notification event list** (NOT-001, NOT-006): compare our events with Coolify's: host key changed, high disk usage and recovered, unreachable and reachable, API token expiry, TLS expiry.
- [ ] **Pending host key** (SEC-006, SEC-007): keep a changed host key as pending, notify, and let the operator approve it; ignore the same SSH error text when a program on the device printed it. Needs a spec change to SEC-007.
- [ ] **One SSH connection for each device** (MET-001, SSH fallback): keep and reuse the connection between collections, with a health check and cleanup. Reference: `app/Helpers/SshMultiplexingHelper.php`.
- [ ] **Hostinger API endpoints** (PRJ-008, PRJ-003): base URL `https://developers.hostinger.com`, `/api/vps/v1/virtual-machines`. Reference: `app/Services/HostingerService.php`. Use the read endpoints only.
- [ ] **Sensitive-read scope for API keys** (INT-003): a scope apart from read for values such as secrets and credentials. Needs a spec change.
- [ ] **Check the tunnel token holder again during a session** (API-009, RA-034): Coolify checks a live terminal session again at intervals, not only at the start.
- [ ] **Scheduler locks** (CHK-001, CHK-010): one overlap lock for each schedule type so that a slow type does not delay the others, and one timestamp for all items in a run.
- [ ] **Image and package scan** (later check type for Docker hosts): Grype and Syft, with a digest notification.
- [ ] **Automatic security updates procedure** (VPS-004): `unattended-upgrades` or `dnf-automatic` limited to security origins, as a documented procedure that the operator runs (VPS-005).

## 4. Hub: access, logs and security

Revisit: Before a hub is hosted for a different person, and at the next security review of access.

### WP-05 T5 audit hardening: accepted redaction gaps (LOG-007 backstop, D-3; merged at 873c7bc)

Added 2026-11-04.

**WP-05 T5 audit hardening: accepted redaction gaps (LOG-007 backstop, D-3; merged at 873c7bc)**

- [ ] JWTs behind `%20` or `\n` escapes, with trailing dots, the five-part JWE form, and non-compact headers.
- [ ] A no-break-space gap after a scheme word; `Bearer abc%2B…`; `token <value>` with no colon.
- [ ] Unterminated or escaped quotes, and ODBC `{…}` values.
- [ ] `auth` reached through a JSON-patch or field label.
- [ ] Acronym plurals (`PSKs`, `PINs`).
- [ ] Object keys are never redacted.

### Deferred from WP-05 T6 (TOTP), merged at 6f6bd7a

Added 2026-11-04.

- [ ] Audit entries for `operator totp`, `operator totp --disable` and `operator set`
- [ ] Column-level grants on the TOTP columns
- [ ] Zeroize the secret printed to stdout; turn off core dumps
- [ ] Record the unopenable-secret path's two events on one connection (strict timing parity)
- [ ] `events::record` publishes to the bus before the transaction commits, so a failed commit leaves a published event that never happened (predates T6)
- [ ] WP-05 T8: signed-out login field leak (from T8 security review)

### Deferred from WP-05 T2 (events API), merged at 1669eae

Added 2026-11-04.

- [ ] `events.message` and `audit_log.change` have no size bound; a page over 2 MiB ends the socket with no close reason (today's pages are about 0.6 MB or less)
- [ ] Per-socket WebSocket Ping rate limit
- [ ] Per-address limit on sockets before `connection_init` (T4 area)
- [ ] A revoked token can still read the event feed for up to 15 s

### WP-05 Access and logs

Added 2026-11-05.

**WP-05 Access and logs (merged into phase-1 at 5d86152): deferred items**
- [ ] Site subnets join the permitted networks automatically (R-052). Today the operator lists them in `ANIMA_PERMITTED_NETWORKS`.
- [ ] Document the order: accepting connections continues for up to 5 s while sessions end
- [ ] No overall deadline on the answer body. A permitted peer that sends one byte every 29 s keeps its slot, within the 64-per-peer and 1024-total caps.
- [ ] The once-a-minute refusal log repeats if the clock goes back
- [ ] Flush the dropped-refusals count on a timer
- [ ] Reload the permitted list one at a time
- [ ] `events::record` publishes before its transaction commits (seen by WP-09)
- [ ] VPS: the operator's Docker pool 192.168.0.0/16 could one day give a new network the same /24 as a site subnet. Check when site subnets are chosen (WP-03).

### WP-41 (hub groundwork)

Added 2026-11-08.

- T1 (should): hub tests `devices::fields::batching` api_005 (statement count) and `sessions::terminal::tests::wire::refusals::log_007` are flaky under machine load.
- T2 (should): web interface mutations read the device name on a second pool connection while their transaction is open; add a `devices` face function on `&mut PgConnection`.
- T2 (K3 deviations, recorded): `Trail::from_context` returns `async_graphql::Result`; `write(self, ..)` takes self by value and returns `events::Error`; extra `Trail::new(bus, (operator, client), now)` for the files tunnel; `AuditTarget::WebInterface` not added (WP-30 D-3).

### WP-43 T4 / WP-47 (note)

Added 2026-11-10.

- access::throttled and record_failure check before they record, so concurrent joinShare failures can pass 5 in a burst (42 of 50 in a probe). Accepted, because share tokens are 32 random bytes.

### WP-43 T5 rotation

Added 2026-11-10.

- SHOULD-1: two concurrent sign-ins with the same cookie leave an orphan token, which expires after 7 days.
- totp::log_007 has a real test weakness. Its needle "999999" matched a slow-statement elapsed float in the sqlx log ("elapsed_secs":1.6555874959999999). Use a needle that cannot appear in a float.

## 5. Hub: devices, checks and metrics

Revisit: When the domain of the item is changed next.

### WP-06 Hub devices

Added 2026-11-04.

**WP-06 Hub devices: deferred (merged into phase-1 at 5cdc99c)**

- [ ] D-9: `Site.devices` loads every device of the requested sites and pages them in memory. Revisit if a site grows past a few hundred devices.
- [ ] C3: four ID parsers with copied messages.
- [ ] C4: loader types in two places (the domain face and `devices::graphql`).
- [ ] C5: K7 `UNAVAILABLE` is unused; an unreachable database gives INTERNAL across the hub.
- [ ] C6: the `deleteDevice` audit entry does not record that a secret was removed (LOG-005).

### Deferred from WP-09 (checks and maintenance), merged at 2163f06

Added 2026-11-04.

- [ ] A shared text-rule leaf, so checks, maintenance and devices name/text rules are not written three times
- [ ] STATUS/UPTIME device sorts load every matching device in memory; make it an SQL join (cursors are positional)
- [ ] `push_token` stored in plaintext: seal it, keep an HMAC for lookup
- [ ] Custom secret header names beyond Authorization and Cookie
- [ ] `events::record` publishes before its transaction commits (WP-05)
- [ ] C-a (CHK-001): one unreadable maintenance window row stops all recording
- [ ] C-b: a device deleted mid-push gives 500 instead of 404
- [ ] C-c: a check deleted between tick and attempt logs at different levels
- [ ] `regroup` writes one `update_check` per child (capped at 100)
- [ ] No deterministic test proves the `FOR UPDATE` row locks in the checks write paths
- [ ] `Device.heartbeats(last:)` in K1, so the Devices page need not merge per-check heartbeats (R-044)

### WP-30 (hub web interfaces)

Added 2026-11-08.

- [ ] The T1 security mutation run hit its 20-minute limit and returned no results. It covered the URL rule, fingerprint parse, origin and pin drop, and the suggestion filter.
- [ ] Web interface events have device `None`. Filling it needs an import from `devices`.
- [ ] Add an `AuditTarget::WebInterface` kind. Today the entries target the device (H-3).
- [ ] Accepting or dismissing a suggestion reads the check outside the caller's transaction. Swap to `checks::http_urls` (H-1, WP-29).

### WP-28 (hub metrics)

Added 2026-11-08.

- GPU discovery and GPU metrics (MET-006 to MET-010) are not built. The tables and the `gpus` face exist (R-073).
- Narrow the metrics public surface to `pub(crate)` where nothing outside the crate uses it.
- Add a metrics test that a deny-listed device gets no samples.
- Rename the `met_005_*` subscription tests and fix one stale doc comment.
- Finish the guard and filter mutation run on `metricsAdded`.
- Process: one shared Cargo target directory across worktrees can leave stale test binaries.

### WP-29 (hub check types)

Added 2026-11-08.

- Operator decision (should): the sealed SNMP secrets are bound to the check, the version and the user name, but not to the device host. Anyone who can write to the database and changes `devices.host` gets the secrets sent to the new host. D-5 accepted this risk. Binding the host fixes it, but then every operator host edit needs the secrets entered again.
- The client SNMP editor (WP-25) does not check three hub limits: a v3 password of at least 8 characters, a user name of at most 32 bytes, and an OID of at most 128 arcs (WP-29 H-1).
- "SSH off" and "no credential" record Down. Unknown would be truer.
- The link error is a free-form string, not a typed reason.
- The scheduler's catch-all match arm could list each check type.
- The two seal-and-open files repeat each other.
- Some hash temporaries are not wiped, and a malformed SNMP datagram ends the attempt instead of being dropped.
- There is no audit test for setting only a v2c community.
- Several checks files are near the 300-line limit.
- Live: Docker and systemd need a real SSH device; SNMP v3 needs a real SHA-256 and AES agent.

### WP-51 vendor retry

Added 2026-11-10.

- A vendor left pending by an unfinished read and then failing in a full look is dropped, which takes two problems in a row.
- A pending Intel costs a read of up to 3 s each round.
- AMD still drops known cards when grep prints nothing (documented).
- Known flakes to add: access/listen sec_001 "a client that keeps reading gets the whole answer", and "a client that stops reading frees its place at the write stall".
- deploy/headscale/tests/ holds 27 files, over the 12-per-folder rule. quality:limits does not count folders under deploy/. Needs a folder split by the deploy owner.

### WP-51 final (open risks)

Added 2026-11-10.

- Hardware not proven:
  - S4: an AMD APU's memory carve-out reads as VRAM.
  - Intel's `--preserve-status` exit code is unproven on a real machine.
  - The ioreg layout is unproven on a real machine.
  - nvidia-smi has no timeout on the device.
- GPU numbering:
  - C1: numbering is not reloaded from metrics.gpus after a hub restart.
  - nvidia-smi renumbers its GPUs after one drops off the bus.
- C3: Vram and shared_memory_bytes are unused until the agent lands.
- C4: labels accept bidi and format characters.
- C5: the comment in migration 0700 is stale.
- C6: metrics/ and collector/tests/ are at the 12-file limit.
- C7: every AMD card is named "AMD GPU".
- gpu-core-count is not read.

### WP-46 R-102 amended

Added 2026-11-10.

- Should, accepted as a gap by the orchestrator: the hour floor counts from `last_beat_at`. An RDAP query that leaves no beat behind can therefore be followed by a second query within the hour. That happens when a check is paused while a query is in flight, or when a write fails and is followed by a pause and resume, an update or a hub restart. It costs at most one extra query and needs an operator action. The fix stores the time of the last RDAP query, which needs a migration.
- Coulds:
  - the module doc line at schedule.rs:2 is 125 characters;
  - in needs_new_due, a local `floor` shadows the function `floor`;
  - CreateRetries::to_value ties the schema default to CheckSettings::default().

### Hub

Added 2026-11-12.

- Same shape as the d_25 fix (WP-53 T9): the error paths in `builds/jobs.rs` (`report`, `uploaded`, `claim`) and `notifications` `claimed.rs` drop a transaction instead of ending it. A dropped sqlx transaction only queues its rollback, so a `FOR UPDATE` lock can outlive the call.
- T9b coulds: note the runtime flavour the hold tests need; add a test for the Retry arm.

## 6. Sessions and the gateway

Revisit: With the native gateway work (S2), or when sessions change next.

### Deferred from WP-08 (guac-protocol)

Added 2026-11-04.

- [ ] `ArgValues` keeps secrets in plain `String`s that are never zeroed. Use `zeroize`/`secrecy` for defence in depth (SEC-005).

### Deferred from WP-10 T3 (host keys), coulds

Added 2026-11-04.

- [ ] Refuse 6to4 and local-use NAT64 addresses that carry a refused IPv4
- [ ] Keep a trailing dot as part of an observed host name
- [ ] Document hosts that offer only SHA-1 `ssh-rsa`
- [ ] DNS lookup can tie up a blocking thread after its timeout
- [ ] russh does not re-check the host key on rekey

### Deferred from WP-10 (sessions), merged at 1f75518

Added 2026-11-04.

- [ ] R-030 deny list for sessions: the target rule still allows the hub's Docker networks (bridge gateway 172.17.0.1:22, the VPS public address, other containers)
- [ ] `ArgValues` copies of secrets are not zeroed (live in the handshake up to about 20 s)
- [ ] russh's password `String` and the GraphQL body/JSON of a `testConnection` credential are not zeroized (with WP-06)
- [ ] H-10: compare a real guacd `args` with the fixtures; refuse an `args` list without `host-key`
- [ ] Multi-key-type test against a real guacd
- [ ] Blocking DNS lookups; no concurrency bound on `testConnection` probes or open tunnels
- [ ] Port oracle through `testConnection` outcomes (operator-only)
- [ ] A client that sends a byte after the request headers and leaves is seen only at the handshake wait
- [ ] Head-of-line stall in the single-task relay between the two writes
- [ ] F3: the tunnel uses the first checked address while the probe tries several (fails closed)
- [ ] F4: an unsaved `testConnection` against an endpoint a saved device pins shows no "changed" warning
- [ ] F5: the `finish` retry can publish an event twice (events publish before commit)
- [ ] An `endSession` between a start's commit and its ticket insert can leave the row live until `close_stale`

### WP-21 (hub remote access settings)

Added 2026-11-08.

- [ ] Clearing a wake relay automatically writes no audit entry of its own.
- [ ] A cleared relay's event has no device link.
- [ ] `devices/graphql/standing/interfaces.rs` is a name a newcomer would not look for.
- [ ] Nothing checks that `metricsSource: SSH` has SSH turned on.

### WP-27 (SSH link, SFTP, wake on LAN)

Added 2026-11-08.

- Add a DNS-rebinding test and a test for the per-address bound. There is no cheap loopback blackhole for the second.
- If a create is still pending past `CLOSE_WAIT`, the file can stay on the device.
- `russh-sftp` decodes non-UTF-8 file names lossily.
- A single directory listing can reach about 60 MB.
- `MAX_PENDING` for files tickets is 32 across the whole hub, not per operator.
- `cancel` is ignored while the file is still opening.
- The `TimeLimit` sentence says "10 seconds" twice.
- The host-key-changed wording differs between wake, files and the terminal.
- The terminal could open through the link.
- The link-error to GraphQL-code mapping is written in two places.
- Programme: add a `clippy.toml` `disallowed-methods` rule banning `Box::leak`, `String::leak` and `Vec::leak` (CHK-012).

### WP-22 (hub RDP and VNC sessions)

Added 2026-11-08.

- The password copy in `ArgValues` is not zeroed.
- A woken session has no cap of its own; only the ticket cap applies.
- The terminal timeout message is vague after a 120 s wake.
- The certificate-mode tests could be one table-driven test.
- The `vnc.rs` banner check has no property test.
- The lagged branch of the `sessionChanged` stream is untested.
- Old plan IDs remain in comments written before WP-22.
- Live: Windows RDP with NLA (resolution, clipboard, Ctrl+Alt+Delete, a changed certificate ending 4003); the `sha256:` format of `cert-fingerprints`, worked out from the FreeRDP source and not yet checked on a real server; VNC to macOS Screen Sharing and to a Linux desktop; wake through a real site relay; Hyper-V may fail closed with 503.
- The wake line's icon nudges 2 px sideways when it changes.
- The ended-panel button widens when its text changes between Reconnect and "Check certificate".
- WP-34: the ended text on a 4003 close may read "You ended the session." Check it against the real hub.

## 7. Deployment, VPN, network and the agent

Revisit: Before the next change to the VPS, and before the hosted launch.

### Deferred from WP-04 stage A (hub deployment), merged at decc2e4

Added 2026-11-04.

- [ ] Pin the five base images by digest
- [ ] Egress filter for guacd
- [ ] Certificates readable without root
- [ ] C-4: a oneshot unit that waits for the VPN address before compose starts
- [ ] make-secrets: check all secrets before writing any (all or nothing)
- [ ] Run the SIGINT prompt test under dash
- [ ] Automated SELinux test
- [ ] Make the smoke test's TLS check strict once WP-05 H-2 (TLS from tls.pem) lands

### Deferred from WP-03 (VPN and network), merged at bdfc785

Added 2026-11-04.

- [ ] Alerts
- [ ] Automated node inventory
- [ ] `_acme-challenge` delegation
- [ ] Offline copy of the home-PC repository
- [ ] Verify packs before prune
- [ ] Link check from the guide to the scripts
- [ ] DHCPv6 handling
- [ ] Lock the Docker major version
- [ ] T7 C7: Docker networks outside 172.16/12, and IPv6 Docker networks
- [ ] T7 C8: any bridge named `br-*` passes the guard (provider bridges)
- [ ] T7 C9: macvlan networks are not shown in the inventory
- [ ] Confirm Tailnet Lock (OD-10) behaviour upstream; checked only locally against the 0.29.7 image

### WP-23 (gateway, compose and VPN policy)

Added 2026-11-08.

- [ ] The Dockerfile is checked only as text.
- [ ] Images are pinned by tag, not by digest.
- [ ] The Windows machine needs its own RDP grant once it has a name and an address.
- [ ] `deploy/README.md` needs the two new variables.
- [ ] The SSH session target check does not read `ANIMA_PROBE_DENY` yet.

### WP-54 T4 agent binary (should/could)

Added 2026-11-10.

- Parent-folder check tests only "others cannot write unless sticky". OpenSSH's StrictModes refused umask-002 homes, so this check is deliberately looser. Revisit.
- The agent finds its uid through a temp file, because there is no safe portable call without a new dependency.
- Could: the `next_at(now, None)` mutant survives. A device with no GPU looks every 5 minutes, not every 24 hours.
- Could: `EnrolError::Folder` is not in the LOG-007 message list.
- macOS code is not compiled on Linux (D-8). The live proof is WP-53 on the Mac.

### Deploy tests

Added 2026-11-12.

- Bun 1.3.11 `spawnSync` can busy-wait on a child that has already exited. T8 moved `deploy/test` to an async `finish` helper; `spawnSync` remains in the headscale, dns and `broker-build.ts` tests. Upgrade Bun or move them too.
- `deploy/test` holds 33 files, over the 12-per-folder limit.
- `secrets-token`: check that the token path is a file, not just that it exists.

## 8. Client

Revisit: With the redesign of the interface (ui-redesign-plan.md).

### From WP-02 (design tokens and fonts)

Added 2026-11-04 to 2026-11-13, top of the issue.

- [ ] Semantic shadow-colour token so `elevation-2`/`elevation-3` stop referencing `nord-0` directly; needed for a light theme (UI-005, TOK-011; a K5 change).
- [ ] `size-adjust` / `ascent-override` on `@font-face` so the font swap shifts layout by exactly 0 in Chromium (measured CLS 0.002; WebKit 0) (MOT-007).
- [ ] Route the remaining renderer guards in `design/src` through `TokenSourceError` with token paths (CQ-022 typed errors).
- [ ] Verify the `prefers-reduced-transparency` block in WebKit; only Chromium can emulate it today (MAT-006, MV-001).

### Deferred from WP-11 (client shell), merged at bb575fd

Added 2026-11-04.

- [ ] Lint does not refuse a dynamic `import()` of `motion` or `graphql-ws`
- [ ] Test that drives a non-instant motion to natural arrival and checks `finished` settles without a timer
- [ ] Nav-bar resize gesture (5.7.4, MOT-030+) and a compact nav below `bp-compact` (23 px overflow at 360 px)
- [ ] Modal backdrop blur (MOT-006 ambiguity)
- [ ] A 502 HTML answer sends no status signal (F7)
- [ ] Hub socket: pin the 4000/4999 fatal-close boundaries in tests (G1); `subscribe` during a retry wait after 4403 waits up to 30 s (G3); re-probe after a fatal 1011 close (G4)
- [ ] The hub address in the banner wraps mid-word in the 192 px nav
- [ ] `lib/hub/socket/fake.ts` imports vitest beside production code (D7)

### From WP-13 E-2 (R-048)

Added 2026-11-04.

- [ ] K1 `HttpConfig.basicUsername: String` (non-secret), so editing a stored BASIC HTTP check does not force the operator to retype the username

### Deferred from WP-14 (SSH tab), merged at 1446b54

Added 2026-11-04.

- [ ] WebKit logs the tunnel URL (with the single-use, 30 s ticket) to the console when a WebSocket fails
- [ ] Paste guard: guacd 1.6.0 has no bracketed paste, so multi-line clipboard text runs at once; confirm multi-line or large pastes
- [ ] Vendor boundary lint can be bypassed by `new URL(...vendor...)`, `import.meta.glob` or a directory symlink
- [ ] No keyboard reorder of session tabs
- [ ] Terminal pane takes Tab; add an escape chord or document it (WCAG 2.1.2), with WP-16 window shortcuts
- [ ] The strip's first appearance is one layout move (CLS 0.018); reserve its height
- [ ] At 360 px the session pane is 120 px wide; collapsing nav (WP-11) and wrapping panel row (UI-009)
- [ ] Host key modal is a `div` with a focus trap; consider `<dialog>`
- [ ] RDP (RA-004/005) is Phase 2

### WP-15 Settings and sign-in

Added 2026-11-05.

**WP-15 Settings and sign-in (merged into phase-1 at ae8ec3d): deferred items**
- [ ] Browser sign-in: there is no token provider outside the desktop app
- [ ] Settings overflows at 360 px width
- [ ] The `--border` token's contrast is 1.69:1
- [ ] Add a control-height token
- [ ] A desktop-app login test against a real hub
- [ ] WP-09 tests that depend on the time of day (`heartbeat_cost::api_005_*` 00:00–01:40 UTC, `daily::db_005` 23:00–00:00 UTC). The fix is with WP-09.

### WP-12 Devices page

Added 2026-11-05.

**WP-12 Devices page (merged into phase-1 at bbd9415): deferred items**
- [ ] `Device.heartbeats` in K1, to replace the per-check heartbeat read (E-2)
- [ ] Folder cycle: devices screen → events panel → devices toolbar (`Menu`, `toggled`)
- [ ] `lib/events/panel/` is at the 12-file limit
- [ ] Compact nav at narrow widths: at 390 px the card is 150 px wide and the panel filters stack in five rows
- [ ] Chromium load CLS of 0.0005–0.002 at the font swap (check again after R-042's font-display: block)
- [ ] Status changes are not announced to screen readers; the status dot's contrast relies on its label
- [ ] Heartbeats: O(n²) dedupe in `latestBeats`; a blank cell mid-spring; `responseTimeMs` is fetched but unused
- [ ] Menu material on the panel in real WebKit
- [ ] Logs panel: the 8 px edge strip has no grip (under UI-013's 24 px); a truncated device name has no `title`; the empty page scrolls 89 px; the `innerHeight` initial value is redundant; stale indentation
- [ ] Constants that tests repeat (`PAGE_SIZE`, `VELOCITY_WINDOW_MS`) are not exported
- [ ] WP-17: a 200-device page with real heartbeats; panel frame time and blur in WKWebView and WebKitGTK; resizing the window while holding the pointer in WKWebView

### From WP-13 T5 (device detail page)

Added 2026-11-06.

**From WP-13 T5 (device detail page)**
- [ ] Per-device live update documents (`deviceStatusChanged(deviceId)`, `heartbeatAdded(deviceId)` filtered on the hub). Phase 1 reuses the all-device subscriptions filtered by `deviceId` in the client (accepted deviation).
- [ ] Card-to-detail flight runs only when the device loads within the 200 ms page cross-fade; show the header from the card data first so the flight always runs.
- [ ] Unsaved-changes cue when leaving the Settings tab with edits (C3).
- [ ] Reserve panel height while the loading skeleton is gated, to avoid layout shift (C5).

### WP-24 (client RDP and VNC tabs)

Added 2026-11-08.

- [ ] The `if (!closed)` guard in `read()` in `view/certificate/CertificateModal.svelte` has no test.
- [ ] The `target === client` and `rules.paste` guards are redundant. They survive mutation.

### WP-25 (client device form)

Added 2026-11-08.

- [ ] Phase 1 patterns to fix everywhere in one change:
  - 13 px checkboxes (UI-013)
  - hover contrast of 1.17:1 under more contrast (MAT-006)
  - primary button contrast (UI-004)
  - the press role animates background colour (MOT-006)
- [ ] Comments and test titles cite decision IDs (COM-002).
- [ ] The port and URL checks are duplicated.
- [ ] `sameJson` depends on key order.
- [ ] Changing an SNMP check's destination gives no warning.
- [ ] Saving an unchanged web interface row gives no feedback.
- [ ] `hub-pending.ts` still holds the device and web interface read types. This waits on `lib/devices/api.ts`.

### WP-24 (client RDP and VNC tabs, final)

Added 2026-11-08.

- [ ] Microphone input and the shared drive in the client.
- [ ] The `onclient` test seam in `RemoteView`.
- [ ] Two exports that only tests use.
- [ ] No test cites RA-030.
- [ ] Check in the end-to-end run whether WebKit in the macOS app refuses device-to-system clipboard writes, because the write does not follow a user action.

### WP-33 (client detail view and charts)

Added 2026-11-08.

- Add a K5 opacity token for the chart min-max band, which uses `fill-opacity: 0.2` today.

### WP-32 (client files, wake and live sessions)

Added 2026-11-08.

- A download cancelled after the hub has read every byte, but before the client has received them all, shows as cancelled on the client and is audited as done on the hub.
- The files panel's busy guards are pinned only through the link's busy refusal.
- Downloads are held in memory as a Blob.
- The files panel has no compact-width layout.
- In the Tauri shell, wry saves a download under its suggested name in ~/Downloads. Check the save path in the app.
- Revoking the blob URL at once may fail a WebKit download. Check it in the app.
- WP-34: a failed Stop in the live-sessions list shifts the rows once (MOT-007).
- WP-34: check a real download inside the Tauri app.
- WP-34: add a test for the live-sessions popover's reduced-motion form.
- The ping and silence timer code is duplicated between the files link and the terminal link. Lift it into one shared file.
- File names that are not valid UTF-8 come through with their bad bytes replaced, so the client cannot send them back.
- The file panel shows that upload or download is turned off only after the hub refuses an attempt.
- Downloads have no size warning.

### WP-33 (final)

Added 2026-11-08.

- Host uptime (`UPTIME_SECONDS`) is fetched but not shown. Add a fact row if wanted.
- The chart skeleton draws four cells, but a device can have 5 to 9 or more charts. The skeleton could reserve the last-known chart count.
- Time labels jump one step on a chart update while the line slides. A label cross-fade could be added.
- Weekday tick labels are fixed English. Use `Intl.DateTimeFormat`.
- When the clocks go back, an hourly tick in the repeated hour gets one label, not two.
- WP-34: confirm the D-2 reading of 5.7.3 "moves left continuously", which is one spring slide per point (MOT-011).
- Live points update only the 1-hour range.
- A stale hub `readings` answer can briefly replace a newer live reading during a load.
- Add a test for colour wrap-around past six series.
- Under `prefers-contrast: more`, the chart grid stays `chart-grid` instead of `text-muted` (MAT-006). WP-34 should check this.
- The Sessions Start and End timestamps wrap at 600 px.
- `MetricsSection.svelte:5` has a four-space indent on one import.
- H-2: show `Session.clientAddr` in the Sessions tab (`DeviceSessions`; D-10, RA-034).
- One shared `endTransitionsAtOnce` fixture under `lib/motion/`.
- `routes/devices/[id]/` is at the 12-file limit. The next file there goes into a subfolder.

### WP-26 (client device list and header)

Added 2026-11-08.

- Should (WEB-010): an open web tab keeps the device status from when it opened, so a device that goes Down mid-tab does not show the Down page.
- WP-34: button borders are 1.36:1 against the card. Check them against the spec's contrast rule for UI components.
- At a 1280 px window the device table is 1506 px wide, so the Connect column is off-screen until you scroll.
- There is no Wake button in the table row or the header.
- `DeviceActions` restyles `WakeButton` internals through `:global(.status)`.
- The warning wording exists twice.
- The DEV-001 200-card test bound is 60 s under load.
- A 1024 px window gets compact header rows.
- `lib/sessions/web/open/` sits inside WP-31's folder.

### WP-34 (acceptance fixes)

Added 2026-11-08.

- F5 (should): the compact-header `767.98px` media query literal is not tied to the `bp-compact` token by a test; a token change would not fail any test.
- F5 (should): the narrow-header test runs in jsdom, which cannot evaluate the media query; a browser test should prove the wrap and icon-only badge at 390 px.
- F3 (could): in Chromium only, the settled close of the last session tab logs one layout-shift entry (0.0198) from the hidden `.stage`; nothing visible moves.
- F7 (should): at 390 px the Overview chart skeleton is 1382.5 px against 1432 px for the data, because the legend row wraps in one column; the area grows 49.5 px at the bottom (CLS 0, nothing below it).
- F7 (could): `rememberChartShape` then `chartShapeOf` round-trip through the shapes map (`routes/devices/[id]/overview/metrics.svelte.ts:79-82`); pass the shape directly.

### WP-35 (one-button screen sharing, R-079)

Added 2026-11-08.

- Could: a 0.0197 layout shift in Chromium when the session strip appears late after setup; it belongs to the session shell.
- Could: the status panel is placed twice on mount.
- Could: no visible Cancel while testing (Escape works).
- Could: the "+ VNC" button is 57 px against 56 px for VNC.
- Could: `normalizeHost` would sit better in a shared hosts module.
- Operator: try the flow on this Mac with Screen Sharing off, then on, and on one Linux VNC device.

### WP-42 (client groundwork)

Added 2026-11-08.

- T1 (could): the error text in `quality/src/hub-modules.ts` for `{"domains":[]}` changed wording (same class); the `octet()` helpers in `deploy/localapi` and `deploy/dns` duplicate each other on purpose (separate workspaces); no test for an invalid `graphql` value.
- T2 (could): import order in `MetricsSkeleton`; `DEGRADED_ABOVE` could move to `thresholds.ts`; three import styles for units; `phase-2-acceptance.md:25` cites the old `charts.test.ts` path.
- T3 (could): `Capture` reads stdout after exit (document it); the `try_wait` error reap path is untested (cannot be provoked); `name.rs` poll 10 → 20 ms. Behaviour: `sharing_settings_open` now has the 3 s opener wait limit.
- T3 (should): hub test `api_005_interfaces_and_suggestions` (`web_interfaces.rs:133`) is load-sensitive (14 > 12 statements under load).
- T4 (could): `devices/form/form` is an awkward carried-over name; `RemoteLoginButton` uses a relative import and its tests still live in `desktop/system-settings.svelte.test.ts`.
- T5 (could): `web-interfaces/tab/api.ts` (AcceptWebCertificate) and `web-interfaces/api.ts` are two GraphQL clients in one domain; fold into one. `open/` ↔ `sessions` type-only edges are worth watching; `shell.ts` re-exports make `events` and `fingerprint` reachable two ways.

### WP-50 final package review round 2 (shoulds)

Added 2026-11-10.

- S2: three copies of `isMissingContext`.
- S4: HubRows and SignOutRow diverge.
- S2b: `retry()` is copied in seven places.
- Files near the 300-line limit: lib/checks/api.ts (296), lib/devices/api.ts (297), DeviceForm.svelte (298).
- Domain-retries coulds:
  - Advanced opens for a hub `retries` error on a domain form;
  - no test covers a `retryIntervalSeconds` hub error;
  - one import is out of order.
- WP-50: the 4-line status band from T12; the shell banner moves the page at 390; the device error banner shifts the layout; token contrast for `--status-down` text and `--border` needs a programme decision.
- WP-50 check-row: at 390 the type labels "Domain expiry" and "Tailscale ping" are cut with an ellipsis (title is set).

### Client

Added 2026-11-12.

- T7c coulds (see `WP-53.md`).
- Spec gap: the chart's visible number waits for a chart header change (G-11).

## 9. Desktop shell

Revisit: With the packaging of the one-click work.

### Deferred from WP-16 (Tauri shell), merged at 8960ca4

Added 2026-11-04.

- [ ] Operator-watched checks (H-4): Tab in WKWebView (WP-11 H-6); running without `core:default`; running without `'unsafe-inline'`; Keychain prompt repeats
- [ ] The web client has no token provider (C2); `address.ts` may move to `$lib/hub/` (C4)
- [ ] TypeScript cannot refuse a `TokenProvider` that ignores its `address` argument (lint rule idea)
- [ ] Export `AddressResult` from `$lib/hub/hub`; unexport `AccessTokenProblem` (C-c)
- [ ] Default panic hook prints key-store thread panics to stderr (C-b)
- [ ] Linux Secret Service not exercised on a real desktop; macOS legacy Keychain ACL trusts only the creating app
- [ ] CLI-001 and CLI-004 rest on review evidence; make them tests
- [ ] x86_64 Linux build; AppImage launch with a real compositor
- [ ] Pop-out windows (RA-002), built to WP-14's port (device identity, `openSession` in the new window)

### WP-31 (desktop web view)

Added 2026-11-08.

- Add a test that a certificate challenge for a closed tab is cancelled.
- Add a load-failure hook, so an unreachable device shows the error page at once instead of after the 30 s timeout.
- Emit an event when the operator closes a popped-out window.
- Revisit the navigation filter, which refuses `about:`, `data:` and `blob:` frames that some device pages may need.
- Move the system trust check for tabs with no pin off the main thread.
- Run the shell commands in tests.
- Skipped: the separate whole-package final review (Section 3.2 step 4). Each task had an Opus security review, and T5 also had a Sonnet interface review.
- Operator: run the OD-7 spike on Linux (`apps/desktop/src-tauri/spikes/od7`, `xvfb-run -a cargo run`). Try a real device with an untrusted certificate (first visit, accept, changed certificate).

## 10. Quality, gates and tests

Revisit: Before the Phase 3 acceptance run, and at the next work on the gates.

### WP-20 (skeleton and contract gate)

Added 2026-11-08.

- [ ] `host_port` accepts `[]:4822` and port 0.
- [ ] The fixture test does not check field names.
- [ ] The contract test has duplicated code.
- [ ] A blank line splits a table in `hub-domains.md`.
- [ ] The MV-008 allow-list does not exist.

### Programme

Added 2026-11-08.

- [ ] Three tests fail under load:
  - `k4_8_5` (WP-22 is fixing it);
  - the deploy test DNS-003, for the `make-secrets` interrupt timing;
  - `checks::api::batching::api_005_status_last_heartbeat…`. With the 10 ms loader window under load, the run took 24 statements against a limit of 20, and it passed 3/3 alone. WP-29, which owns checks, should give the test schema a longer batch window.

### CQ-050-P2 (Phase 2 code-quality SPEC items)

Added 2026-11-08.

- Non-SPEC audit items Q2-05 to Q2-16 in `docs/research/code-quality/phase-2.md` (R-076).
- T3 (could): `Detail.applyCheck` could be private; `pauseCheck` and `saveCheckValues` are tested only through `page.test.ts`; `DeviceMissing.svelte` carries an empty script block for svelte-check.
- T1 (could): the `token_half` mutant survives (`checks.rs:253-257`); `Vault::seal` draws its own nonce, so it cannot be passed in for tests (`credentials/vault.rs:68-72`); `rand::rng()` at `checks/scheduler.rs:48` still panics on a random-source failure (startup jitter only).
- T2 (should): in WebKit the opener focus return misses conditionally mounted callers (S1, as before).
- T2 (should): after opening, the host key surface grows 96 px and the credential surface 32 px in one frame (MV-004, MOT-007; as before).
- T2 (should): on `--surface-raised`, modal fields and the Cancel button share the dialog fill and are separated only by a 1.2:1 border (shell-wide, also AddDeviceModal; UI-001, MAT-001).
- T2 (could): the literal-opacity rule skips `stop-opacity`, `flood-opacity` and `style:` bindings; the "Save password" checkbox is 12 px (UI-013); prompt heights differ from before by -22 to +10 px from the shell gap; one stack-test mutant hangs instead of failing; the CredentialPrompt `.content` wrapper is unused.

### WP-40 (skeleton and contract gate)

Added 2026-11-08.

- T5 (could): NET-004 (`design-spec.md` line 289) still says the nav bar shows the hub address; it should be desktop-only. UI-005 holds several rules in one requirement.
- T1b (could): unexport `BareProbeConfig`; comment that the `default` branch in `withConfig` is the exhaustiveness check.
- WP-42 T6 (could): adapters in `lib/desktop` keep their own `isTauri` guards; a folder-level type import `lib/desktop` → `lib/platform`; the `schedule.races` "cannot be loaded" test simulates a use-time failure, not a cached failed import. Handoff to WP-44: `platform.ts` and `client-shell.ts` still pull `@tauri-apps/api/core` and `/event` into the browser bundle; a build-time switch in `platform.ts` is the place to cut it.

### Load flakes still open (all pass alone)

Added 2026-11-12.

- `devices` `api_005` batching window: replace the timing test.
- `met_017`.
- sessions tunnel `PoolTimedOut` under full-suite load.
- desktop `process_log` `log_008`.
- client `web_004`.

### Acceptance

Added 2026-11-12.

- The motion audit file reads "NOT MET as worded" until the acceptance run re-runs it.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

## 11. Lists that mix many packages

Revisit: Split each list into the areas above when its items are read next. They are not split in this version.

### From Phase 3 (WP-43 to WP-54)

Added 2026-11-08.

**From Phase 3 (WP-43 to WP-54), reviews up to `phase-3` 595c624c, 2026-11-08**

Spec references are to `docs/spec/design-spec.md`; rulings are in `docs/rulings.md`. Items come from accepted review rounds, not from a fresh audit.

**Access and API keys (WP-43; R-100)**
- [ ] Argument-level API-key rules: let a WRITE key rename or tag a device through `updateDevice` without changing its address or certificate rules (R-100 refuses `updateDevice` and `deleteDevice` for keys outright).
- [ ] Notification channels stay open to WRITE keys; consider refusing a key that points a channel at a new outside target (R-100 (d)).
- [ ] `operator_for_token` updates `last_used_at` before its login-only filter.
- [ ] `origin_header` checks for a non-empty authority, not a host (fails safe, config validates the origin).
- [ ] `identity::resolve` fails closed silently for an unknown stored token kind; log it.
- [ ] Browser and desktop sign-ins write the same `login` audit action; tell them apart.
- [ ] API-key revoke is audited as `delete`; `AuditAction` has no `Revoke`. Pruning expired keys writes no audit entry.

**Web client serving (WP-43 T1)**
- [ ] S-3: `/index.html/` serves the copy on disk, not the copy read at start (fails safe under the CSP hashes).
- [ ] A missing `index.html` is logged once and every path then answers 404; consider failing at start.
- [ ] The strictest fallback CSP cannot be reached; `pub mod web_client` could be private.
- [ ] `index.html` is read once at start, so a new web build needs a hub restart: say so in the operator docs.
- [ ] C-2: symlinks under the web root are followed out of the root (documented in the module only).

**Web client (WP-44)**
- [ ] Extract `Shell.svelte` from the layout; move route classification out of `guard.ts`.
- [ ] The loopback `http:` branch is present in production builds.
- [ ] WebKit preconnects to the real `hub.anima.ing` during the browser harness.
- [ ] Sign-in status line: the idle reserve is 67 px; shorten the COOKIE_KEPT/TOKEN_KEPT copy and reserve 2 lines. In forced colours the empty status bar draws black (`border-style: none` while `:empty`).
- [ ] TextField error line, CheckForm focus after a refused save, ConfirmModal space reserve.

**Settings and contrast (WP-50; D-27)**
- [ ] Contrast below target, held by the programme: field error text at 12 px (2.11:1 dark, 3.03:1 light), FormButton danger label (2.23:1 / 3.11:1), ConfirmModal failure text (2.11:1 dark), invalid input border (2.46:1 dark).
- [ ] Status text at 4.5:1 sites outside WP-50; dark pressed `accent-deep` with `--bg` is 3.10:1 (UI-004, also WakeButton); FormButton `.danger` hover puts the status-down label on the hover fill.
- [ ] Primary FormButton has no hover change (as WakeButton).
- [ ] FormButton fade still runs under reduced motion.
- [ ] Nested landmarks: SignOutRow, TypefaceRows, UpdatesRow and LogFolderRow keep an inner `section aria-label`.
- [ ] Menu `.option:hover` is surface-raised on a surface-raised panel (1.02:1).
- [ ] MOT-007: with 2 or more API keys the Updates section still shifts about one row per extra key; cap the list with its own scroll, or move Updates above API keys.
- [ ] Linux PRIMARY selection: check by hand in the Tauri app on pop-os.

**Checks (WP-46)**
- [ ] `createCheck` and `updateCheck` report one out-of-range bound at a time.
- [ ] An unreachable parse path in the domain-expiry input.

**Notifications (WP-45)**
- [ ] hub_005: partition `notifications.deliveries` by time like the other history tables.
- [ ] A CHECK tying a delivery's `code` to its `reason`; round-trip tests for the K2 columns.
- [ ] K3 `Observation.site` and `check_type` are unused; no query-count test for `for_checks`; Unknown and maintenance beats could skip the settings read.
- [ ] SEC-C4: the SMTP `security` setting is outside the target binding. SEC-C5: a new path on the same origin keeps the header values.
- [ ] Move the copies of `Opaque` and `SecretText` to one shared leaf (touches checks).
- [ ] The unreadable-channel warning repeats on every list request; the InternalErrors masking claim is untested.

**Metrics and GPUs (WP-51)**
- [ ] Intel discovery needs `intel_gpu_top` privileges; reading sysfs vendor 0x8086 is an alternative. Intel exit status after the interrupt is unproven on real hardware.
- [ ] Reading GPU values can wake a runtime-suspended GPU (laptop power).
- [ ] Intel Macs report their ioreg accelerators as vendor Apple.
- [ ] `nvidia-smi` renumbering moves a GPU's index; after a hub restart discovery numbers GPUs again in vendor order, so series history can mix.
- [ ] A vendor that always hangs is read every round; a driver ready more than 5 minutes after boot waits 24 h.
- [ ] GPU core count; Intel power reading unproven.
- [ ] The hung-tool test waits 5 s of real time (the per-command limit is a const).

**Agent (WP-54)**
- [ ] A push `Receipt` is not tied to the backlog that issued it.
- [ ] The SHA-256 hasher leaves stack residue of the token.

**Deploy (WP-52)**
- [ ] The "only the build output" Dockerfile test matches the text `--from=web`, so `COPY --from=0`, `RUN --mount=…,from=web` and `install -d -m 0777` get past it; no test forbids `--platform` on the rust stage; the web stage's `--platform=$BUILDPLATFORM` is optional in the test regex.
- [ ] No test for `(unreachable)`; exit 142 exists only for the test shim; docker failing in the middle of the loop reads as "blocked".
- [ ] C-4: `harden.sh check` cannot check a running container.

**Gates and machine (R-101)**
- [ ] The global `~/.cargo/config.toml` `target-dir` makes worktrees share one build folder, which can run another worktree's stale test binary; the gates now set their own `CARGO_TARGET_DIR`. Consider making `bun run gates` set it itself.
- [ ] Flaky under load: `ra_031`, `ra_032`, `api_005`, `log_007`, `k4_8_5`, desktop `log_008`/`web_004`, snmp `d_5_*`, the deploy localapi endless-body test and the deploy `secrets-token` DNS-003 test.

### Phase 3 follow-ups, 2026-11-09 (up to phase-3 3088cb32)

Added 2026-11-10.

**Checks and retention (WP-46 final package review)**
- [ ] C1: `checks/store/retention.rs` `first_kept_day` copies `events/retention.rs`, and is pure logic inside a store file.
- [ ] C3: migration 0900 leaves INSERT and DELETE on `retention.periods` to `anima_runtime`; a deleted row makes every `keep` run fail as Unreadable (0321 already revokes them on `checks.defaults`).
- [ ] C4: wrong IDs in merge messages: ddcc9644 and 180348d0 cite CHK-019 (means INT-001); 1652899f cites CHK-011 (means DB-011).
- [ ] C5: duplicated tests: retention bounds in-crate and over GraphQL; "a missing row is Unreadable" three times; a second private `database_now` in tests/it/checks/exposition.rs.
- [ ] C6: `setRetention` logs under `Source::Hub`, `setCheckDefaults` under `Source::Check`; pick one rule for settings changes.
- [ ] The INT-002 domain-days test drops `checks_domain_not_after_check` in its test database; a unit test of `Check::domain_days_left` would be cleaner.

**Settings (WP-50 close-out)**
- [ ] `missing_context` string match in the policy, channel and defaults readers; pass calls as props from AddDeviceModal and SettingsPanel.
- [ ] HubRows and SignOutRow duplicate `leaveHub` and the button CSS; T12's 4-line status band.
- [ ] Layout shift at 390 px: the shell "Not on the network" banner (~79 px) and the device error banner (36 px). The R-096 CLS margin on /settings is thin (0.0102 vs 0.01 with the skeleton removed).
- [ ] Token contrast: `--status-down` text 2.11–3.36:1; `--border` 1.17:1 in dark.
- [ ] subscribe.test.ts:203 could pin TOKEN_UNREADABLE by identity.

**GPU read side (WP-51 T4)**
- [ ] The readings tests sit in tests/gpu_labels.rs; they belong in gpu_reads.rs.
- [ ] No two-device test covers the metrics.gpus join's `device_id` condition.
- [ ] The chart ORDER BY uses the stored label, so GPU 10 sorts before GPU 2 (nothing relies on it).
- [ ] The live.ts doc comment needs reflowing.

**Gates and flakes**
- [ ] New load flakes: `metrics::collector::tests::gpu_rounds::d11_gpu_reads_stop_at_the_gpu_window_and_the_collection_is_stored`; `k4_5_a_client_write_stalled_past_the_idle_wait_is_4002_timeout`; the deploy test step hung about 58 minutes once during gate 7 and exited by itself.

**Metrics parsers (WP-51 final package review)**
- [ ] S4: an AMD APU's memory carve-out shows as VRAM (`readings/gpu/amd.rs:45`, `discovery.rs:215`); needs a check on real hardware.
- [ ] C1: GPU numbering does not survive a hub restart; `metrics.gpus` and `metrics.discovery` are written but never read back. Load them on the first collection.
- [ ] C2: three GPU vendor enums (`readings`, `wire`, the hub); keep one in agent-core.
- [ ] C3: `readings::Vram` and `GpuReading.shared_memory_bytes` have no consumer until the agent lands.
- [ ] C4: a GPU `label` allows bidi and format characters such as U+202E, so a name can spoof its chart label (the client escapes it; not an injection). Refuse `Cf` characters.
- [ ] C6: `crates/hub/src/metrics/` and `metrics/collector/tests/` are at the 12-file limit.
- [ ] C7: every AMD card is named "AMD GPU"; read `product_name`.

## 12. Notes on the sources

Revisit: These lines came with the lists. They hold no item.

Work that is decided but deliberately left for later. Agents add new deferred items here (programme ruling R-007) instead of leaving them only in chat or in a package plan. Spec references are to `docs/spec/design-spec.md`.

### Phase 2 coulds for issue #1

Added 2026-11-08.

These items are not blocking. The programme orchestrator collects them here and posts them to GitHub issue #1 in batches.

### Phase 3 should and could items (for issue #1)

Added 2026-11-08.

Items found during Phase 3 reviews that are not fixed in Phase 3. Tagged by package and task.

### Phase 3: WP-53 acceptance preparation and WP-55 gate speed (up to 25b3c64c)

Added 2026-11-12.

Deferred items from these packages. Full detail is in `docs/plans/phase-3/WP-53.md`, `WP-55.md` and `phase-3-acceptance.md`.
