---
kind: Plan
state: In progress
evidence: Its status: the operator accepted the direction and the spikes on 2026-11-14 (its Section 9). It serves Section 4, row 4.16. It is the parent plan of the task breakdowns and the spec delta proposals in `research/one-click/` below.
---

# Anima One-Click Install Plan

| Item | Value |
| --- | --- |
| Document | One-click install plan: Tailcat link, hub in the application, native remote display, stay awake, many hubs, shares and teams |
| Version | 0.3 (operator rulings of 2026-11-14 folded in, Section 9) |
| Date | 2026-11-14 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | Direction and spikes accepted by the operator, 2026-11-14 (Section 9). The work items wait for the spike reports. |
| Base | Branch `main` at `baa7eeeb`. All paths and requirement IDs in this plan are on that branch. |
| Related | [design-spec.md](../../spec/design-spec.md) Sections 3, 4.1, 4.2, 7, 13; [vpn-plan.md](../vpn-plan.md); `docs/procedures/setup/vpn.md`; <https://github.com/tailscale/tailcat>; <https://github.com/No-Instructions/Relay> |

This document obeys the writing rules in Section 0 of the design specification.

## 1. Purpose and scope

### 1.1 Purpose

Today an operator must set up a VPS, Headscale, a domain and the Tailscale client before Anima
works. This plan gives a second way to run Anima: the operator installs one application and
creates a hub on that computer. The operator installs nothing else.

The plan does not remove the current way. Headscale and the VPS stay as an install mode, beside
the link (ruling OR-1).

### 1.2 Target flow

1. The operator installs the Anima application (DMG or Linux package).
2. On the first start, the operator selects "Create a hub on this computer".
3. The operator makes the operator account: password and TOTP.
4. The hub shows an invite: a QR code and a text.
5. On each other computer, the operator installs Anima, selects "Join a hub" and gives the invite.

### 1.3 The three install modes

The three modes use the same hub and the same link. Only the location of the hub changes.

| Mode | Hub location | Relay | Operator pays |
| --- | --- | --- | --- |
| Free | In the application, on the operator's computer | Tailscale public relays (rate limit) | Nothing |
| Self-host | Docker Compose on the operator's server or home lab | Public relays, or the operator's relay | Their own server |
| Hosted | One isolated hub for each account, on the shared pool of Anima (HM-R4, 2026-11-16; before that: one VPS for each customer) | Anima relay, no rate limit | Nothing below a usage line, then a subscription (HM-R3) |

The hosted mode is the default of the first run (HM-R1). Its plan is
[hosted-mode.md](../../proposals/hosted-mode.md). The prices and the costs are in the business
vault.

### 1.4 What "one click" cannot remove

- The operator must make an account (password and TOTP).
- macOS shows the Local Network permission prompt when the hub first connects to a LAN device.
- The hub computer must stay on. Section 4.8 (stay awake) helps with this.

### 1.5 Ideas that this plan does not take

| Idea | Reason |
| --- | --- |
| Tailscale (`tsnet`) or Headscale in each application | A control server needs a public HTTPS address. A hub behind NAT has none without port forwarding. |
| A Tailcat client in the browser | All traffic goes through the relay until Tailcat has WebRTC. vpn-plan.md Section 1.3 gives the same reason. |
| RDP and VNC as a function of the paid mode only | A Linux hub has them through `guacd`. The source is open. The gap is a platform limit, not a tier. |
| `guacd` in a Linux virtual machine in the application | Large and fragile. |

## 2. Rulings and constraints

| ID | Source | Effect on this plan |
| --- | --- | --- |
| Spec 4.1 | design-spec.md | "Anima does not supply remote access from the public internet." The link keeps this: the hub opens no inbound port. The text of 4.1 needs a delta. |
| NET-001, NET-002 | spec 4.1 | The hub permits clients by network address. With the link, the hub must permit clients by device key. Needs a spec delta. |
| NET-003 | spec 4.1 | The hub must not bind to a public interface. In the new mode the hub binds to loopback and the link only. |
| DNS-001, DNS-002 | spec 4.2.1 | `hub.anima.ing` and its certificate are not used in the new mode. The link authenticates the hub by its key. |
| VPN-003 to VPN-014, CHK-009 | spec 4.2 | They need Headscale and the LocalAPI proxy. They do not apply in the new mode. |
| VPN-006 | spec 4.2 | A site agent is already planned for Wake-on-LAN. Item S7 extends it. |
| VPS-005 | spec 4.2 | A relay on the VPS is a VPS change. It needs operator confirmation in chat. |
| BLD-024 | spec 3 | A builder is bound to its VPN address. In the new mode it must be bound to its device key. |
| Spec 1.4 | design-spec.md | Anima has one operator. Shares (S14) and team hubs (S15) need members and a permission check on each operation. Needs a spec delta. |
| LOG-005, INT-003 | spec 3, 6 | The audit log and API keys assume one operator. With members, each record and each key must name its member. |
| R-062 | `docs/rulings.md` | The hub already runs SSH itself. Item S2 does the same for RDP and VNC. |
| R-076 | `programme.md` | No Windows work. RDP checks run against a second Mac. |
| MET-012, MET-013, MET-019 | spec 6 | The pattern for invites (single use, short life) and for a background service (LaunchAgent, systemd unit). |
| CON-001, K1, K2, K4 | `contracts/` | The schema, the database and the tunnel are frozen contracts. Each change needs a programme ruling. |
| Limits | AGENTS.md | 300 code lines for each file, 12 files for each folder. New domains are `link`, not `tailcat`, so that names are operator nouns. |

## 3. Facts

Items 1 to 6 are from the Tailcat README and source at version `v0.7.0`. Items 7 to 10 are from
`main`. Items marked "not checked" must be checked in the spikes.

1. Tailcat is the Tailscale data plane without a control server: WireGuard, NAT traversal, DERP
   and a userspace TCP stack. It needs no account, no administrator rights and no route change.
2. Tailcat is a Go library and a CLI. The licence is BSD-3-Clause. It is before version 1.0.
3. A server has an address that holds its public key, its relay and a pre-shared key. A saved key
   gives a stable address. An address can be in a DNS TXT record.
4. The library has `Server.AllowClient`, `Server.PeerKey` and `Server.DisconnectClient`. Thus an
   application can keep its own list of permitted keys and revoke a key.
5. Tailcat carries TCP and UDP. It does not carry ICMP.
6. Tailscale's public Tailcat relays have a rate limit and keep metadata logs. A relay cannot read
   the traffic. The operator can use a relay of their own (`derper`). The rate is not checked.
7. The hub uses PostgreSQL through `sqlx`. About 148 hub source files contain SQL calls. The amount
   of PostgreSQL-only SQL is not checked.
8. `guacd` runs RDP and VNC. It has no official macOS build. The client draws remote displays from
   the Guacamole protocol (GW-011, K4).
9. The hub is Rust. Tailcat is Go. The hub already uses a sidecar pattern (the LocalAPI proxy).
10. It is not checked whether the Headscale DERP on `hs.anima.ing` accepts a key that is not in
    the tailnet.

### 3.1 Lessons from Relay

Relay is a plugin for Obsidian (MIT licence). Each person keeps a private vault. A "relay" is a
named space that holds shared folders. A person can be in many relays. These facts are from its
README and its self-host guide.

| Relay fact | Lesson for Anima |
| --- | --- |
| The unit of sharing is a named space, not the vault. The owner adds folders to it. A member selects which shared folders to add. | A share is a named set of devices, not the hub. The owner adds devices. The member accepts the share. |
| One share key lets each holder join. The README says a removed user can join again with the key. | Do not use a key that many people hold. Each invite is for one person and one use (LNK-003). Each device has its own key, so removal is final (LNK-004). |
| The owner can destroy a relay. A member can leave and remove the local copy. | The owner revokes a share. A member can leave a share. Both remove the devices from the member's list. |
| Relay's control plane does login and permissions. A self-hosted server trusts tokens that the control plane signs. The guide says the control plane can thus make a token and reach a server on the public internet. | For the free mode and the self-host mode, the Anima account directory must only find hubs. It must never be able to give access. The hub alone decides who is permitted. For a hosted hub this changed on 2026-11-16 (HM-R2 of [hosted-mode.md](../../proposals/hosted-mode.md), OR-5): a sign-in to the account is sufficient for a device to join, so the directory gives access to a hosted hub. A second factor on each account is proposed (HM-D4), not ruled. |
| The guide tells self-hosters to keep the server on a private network (it recommends Tailscale). | The link gives this by default: the hub has no public port. |
| "We don't charge for privacy": self-hosting is in each tier. The paid tiers add users, devices for each user, storage, roles and single sign-on. The team tier has a price for each user. | The hosted mode sells capacity and team functions, not privacy. A team plan can have a price for each member. |
| Relay is local-first. It works offline and merges later. | A client with many hubs must show each hub's last data and its age when that hub is not available (NET-004). |
| The plugin and the server are open source. The login, permissions and billing server is not. | A possible split for Anima: the application and the hub are open; the account directory and billing are not. This is a business decision. |

## 4. Work items

### 4.1 S1. The link (Tailcat transport)

A small Go program, `anima-link`, is in the application bundle, the hub image and the CLI. On the
hub it listens and sends each stream to the hub on loopback. On a client it gives a local port
that goes to the hub.

Proposed requirements (prefix `LNK`, free on `main`):

- **LNK-001.** The hub must have one saved link key. The hub address must stay the same after a restart.
- **LNK-002.** The hub must accept a link only from a device key in its list of permitted keys.
- **LNK-003.** An invite must hold the hub address and a code. The code must be 128 random bits,
  expire after 15 minutes and work only once (as MET-012).
- **LNK-004.** The operator must be able to see each permitted device and revoke it. Revocation
  must close the open link of that device.
- **LNK-005.** A client key must expire after 180 days (as VPN-003). The client must renew it
  while the key is valid.
- **LNK-006.** The hub and the client must show if the link is direct or relayed (as VPN-005).
- **LNK-007.** The operator must be able to set a relay of their own, and a domain name that holds
  the hub address. A name only finds the hub; LNK-002 still applies.
- **LNK-008.** The invite, the keys and the pre-shared key must not be in logs or command lines (LOG-007).

### 4.2 S2. Remote display without `guacd`

The hub connects to RDP and VNC devices itself, as it does for SSH. The hub keeps the Guacamole
protocol to the client, so that K4 and the client do not change. `guacd` stays behind the same
interface as a fallback, for each protocol and platform, until the native path is proved.

The hub on a Mac must ship with remote display (OR-7). It can ship with one protocol first, RDP
or VNC, the one that spike K-B shows to work best. The second protocol follows.

Risks: RDP coverage of the library (IronRDP); Apple Screen Sharing uses its own login and
encodings; the hub must encode screen updates well; a parser fault must not be in the process
that holds credentials, so sessions run in a worker process.

### 4.3 S3. Database without a separate server

The hub in the application cannot ask the operator to install PostgreSQL. The application starts
a bundled PostgreSQL itself (OR-6). The hub SQL does not change. SQLite is not used.

The work is: the bundle and its size, the data folder and its location, start and stop with the
hub service (S4), an upgrade of the data folder between PostgreSQL versions, and the backup (S9).

### 4.4 S4. The hub as a background service

The hub must run when the application window is closed and must start at login. Use the
LaunchAgent and systemd user unit pattern of the metrics agent (MET-019). The Tauri shell
installs and removes the service.

### 4.5 S5. Access rules by device key

Replace address rules with key rules in the new mode: NET-001, NET-002, BLD-024 and the agent
token path. The hub listens on loopback and the link only. This item has a security tag.

### 4.6 S6. First run

The first start shows three choices: "Sign in" (hosted), "Join a hub" and "Create a hub on this
computer". It then makes the operator account and shows the invite. It must also give recovery
for a lost hub key and a lost TOTP device. One DMG holds all three choices.

### 4.7 S7. Site agent

A hub at one site cannot reach the LAN of a different site without a subnet router. The Anima CLI
in site mode is a link server that permits only its site subnet. It runs ping checks and
Wake-on-LAN inside the site and reports to the hub (fact 5, VPN-006). A device then has a site and
a LAN address, not a VPN name.

### 4.8 S8. Stay awake

The hub does no checks while its computer sleeps. The application gets a "Stay awake" control
with two levels.

Proposed requirements (prefix `AWK`, free on `main`):

- **AWK-001:** The control must have three states: off, "Keep awake" and "Keep awake with the lid closed".
- **AWK-002:** "Keep awake" must stop idle sleep of the computer. The display can sleep.
- **AWK-003:** "Keep awake with the lid closed" must keep the computer on when the operator
  closes the lid.
- **AWK-004:** The control must be in Settings and in the menu bar or tray item of the hub.
- **AWK-005:** When the computer is the hub and the control is off, the application must tell the
  operator that checks stop during sleep.
- **AWK-006:** On battery power, the application must be able to turn the control off below a
  battery level that the operator sets.
- **AWK-007:** When the application or the hub service stops, the computer must return to its
  normal sleep behaviour.

Platform notes (not checked; check in the S8 task):

| Platform | Keep awake | Lid closed |
| --- | --- | --- |
| macOS | A power assertion (as `caffeinate`). No administrator rights. | Needs a system power setting (`pmset disablesleep`) and thus administrator rights once. A closed laptop also gets hot. |
| Linux | A `systemd-logind` sleep inhibitor. | A `handle-lid-switch` inhibitor. The desktop policy can refuse it. |

Both levels are in the first version (OR-8). AWK-003 is the only item in this plan that can need
administrator rights. It must be one short set-up step that the operator starts; it is never a default.

### 4.9 S9 to S12. Operation

- **S9. Hub update and backup.** The hub in the application updates with the application. The
  operator selects a backup location (a folder or R2).
- **S10. Export and import.** A hub can move between the three modes with its devices, checks,
  history and keys.
- **S11. Relay and domain settings.** The Settings rows for LNK-007.
- **S12. Packaging.** Sign and notarize `anima-link` and the hub in the macOS bundle (UPD-011).
  Make the Linux package.

### 4.10 S13 to S15. Many hubs, shares and team hubs

Each person keeps their own hub. Hubs never connect to each other. The application joins them
for the person who uses it. This plan does not take hub-to-hub federation.

**S13. One application, many hubs.** The client holds a link to each hub that the person can use.
The CLI already has contexts for this.

Proposed requirements (prefix `MHB`, free on `main`):

- **MHB-001.** The client must be able to hold more than one hub. Each hub has its own link key, account and session.
- **MHB-002.** The device list must show the devices of all hubs, with a group and a filter for each hub.
- **MHB-003.** The client must show the state of each hub: connected, relayed, or not available with the age of its data.
- **MHB-004.** A search, a notification and a session must name the hub that it comes from.
- **MHB-005.** The client must never send data or credentials of one hub to a different hub.

**S14. Shares.** A hub owner gives a named set of devices to a person who is not a member of the hub.

Proposed requirements (prefix `SHR`, free on `main`):

- **SHR-001.** A share must have a name, a list of devices and one level: view, use sessions, or manage.
- **SHR-002.** The owner must invite one person to a share with an invite as LNK-003. The invite must work once.
- **SHR-003.** A person with a share must see only the devices of that share, and only the data of those devices.
- **SHR-004.** A person with a share must never get a device credential. The hub opens each session (R-062).
- **SHR-005.** The owner must be able to change a share, remove a person and revoke a share. Open sessions of that person must close.
- **SHR-006.** A person must be able to leave a share. The devices must go from their list.
- **SHR-007.** The audit log must name the person for each action through a share (LOG-005).
- **SHR-008.** A share can have an end date. After that date the hub must refuse the person.

**S15. Team hub.** More than one person is a member of one hub. Each member can also have a
personal hub in the same application (S13).

Proposed requirements (prefix `MBR`, free on `main`):

- **MBR-001.** A hub must have members. Each member has a role: owner, admin or member.
- **MBR-002.** Each member must have their own password, TOTP and device keys.
- **MBR-003.** The hub must check the role of the member for each operation.
- **MBR-004.** Only an owner can add or remove an admin, export the hub, or delete the hub.
- **MBR-005.** A hub must always have one owner or more.
- **MBR-006.** Each API key and each audit record must name its member (INT-003, LOG-005).

**Hosted mode and shares (OR-9).** The business position is that a hosted hub does not store the
credentials for interactive sessions; the client supplies them for each session ("split
custody"). SHR-004 needs the hub to hold the credential. For now, the plan is that a device on a
hosted hub cannot be in a share. The operator will look at this again when the result of the
accelerator applications is known. Shares on a free or self-host hub are not affected.

S14 and S15 use one permission model: a person, a scope and a level. A share is a scope of some
devices. A role is a scope of the hub. Build the model once, in S14.

## 5. Spikes first

Three spikes decide if the plan is possible. Each spike is a throwaway branch and a short report.

| Spike | Question | Stop condition |
| --- | --- | --- |
| K-A. Link | How often is the link direct? How does a relayed RDP session feel? Does the Headscale DERP accept link keys (fact 10)? | Relayed sessions cannot be used, and a relay of our own does not correct this. |
| K-B. Remote display | Can the hub show Apple Screen Sharing and one RDP host through the native path, with clipboard and resize? Which protocol works best first (OR-7)? | Neither protocol is acceptable. Then bundle `guacd` for macOS instead. |
| K-C. Database | Does a bundled PostgreSQL start, stop and upgrade cleanly on macOS and Linux, with no administrator rights? What is its size in the bundle? | A bundled PostgreSQL is not practical. Then the operator must rule again on SQLite. |

## 6. Order and dependencies

```text
K-A, K-B, K-C (parallel)
   └─> S1 link ─┬─> S5 access rules ─> S6 first run ─┐
       S3 db  ──┴─> S4 hub service ──────────────────┼─> Free mode, one site
       S2 remote display (Linux hub can ship with guacd first)
       S8 stay awake (no dependency; can ship at any time)
   Free mode, one site ─> S7 site agent ─> S9 to S12 ─> Self-host package ─> Hosted mode
   S1 link ─> S13 many hubs ─> S14 shares (permission model) ─> S15 team hub ─> Hosted team plan
```

S13 needs only the link. It changes the client and no hub rule. S14 is the first item that ends
the one-operator rule of spec 1.4.

S8 has no dependency on the link. It is useful today for a computer that runs the metrics agent
or a builder.

## 7. Risks

| Risk | Effect | Answer |
| --- | --- | --- |
| Tailcat is before version 1.0 | Its interface can change | Pin one version. Keep `anima-link` small. |
| Tailscale changes or stops the free relays | The free mode has no relay | LNK-007. Document a relay of our own. |
| The key list replaces tested Headscale rules | A fault opens the hub | Security review of S1 and S5. Tests for each rule of LNK-002 to LNK-005. |
| Two transports to maintain | More test cost | Accepted (OR-1). Keep the hub unaware of the transport where possible. |
| The hub is on a laptop | Gaps in checks | S8, and AWK-005 tells the operator. |
| Members and shares end the one-operator rule | Each operation needs a permission check; one missed check shows data to the wrong person | One permission model (Section 4.10). A test for each operation with each level. Security review of S14 and S15. |
| A member's client holds links to many hubs | A fault can mix data of two hubs | MHB-005. Each hub has its own key, session and cache. |
| Relayed traffic on hard networks (carrier NAT, mobile) | Slow remote display | Show the path (LNK-006). Offer the hosted relay. |

## 8. Acceptance criteria

1. On a Mac with no Tailscale client, the operator installs Anima, creates a hub and adds a
   device on the same LAN. The operator installs nothing else.
2. A second computer on a different network joins with an invite and opens an SSH session.
3. A second use of the invite fails. A revoked device cannot connect, and its open link closes.
4. The hub has no open inbound port on a public interface.
5. With "Keep awake" on, the hub writes check results for 8 hours with no gap while the computer is idle.
6. The current install mode (VPS and Headscale) passes its existing gates with no change.
7. One client shows the devices of two hubs in one list. When one hub is off, its devices show their age.
8. A person with a share opens a session to a shared device and cannot see its credential or a
   device outside the share.
9. After the owner removes that person, their open session closes and a new connection fails.

## 9. Operator rulings (2026-11-14)

| ID | Question | Ruling |
| --- | --- | --- |
| OR-1 | Does the link replace Headscale, or stay beside it? | Beside it. |
| OR-2 | Is this a new phase in spec Section 9? | Yes. |
| OR-3 | May the three spikes start? | Yes. |
| OR-4 | Is S13 (many hubs) in the first free version? | Yes. |
| OR-5 | Does Anima get an account directory at `anima.ing` for "Sign in"? | Yes, for the hosted mode. It must only find hubs (Section 3.1). Changed on 2026-11-16 (HM-R2 of [hosted-mode.md](../../proposals/hosted-mode.md)): for a hosted hub, a sign-in to the account is sufficient for a device to join. The free mode and the self-host mode keep this ruling. The directory and billing code are not open source. |
| OR-6 | Database: bundled PostgreSQL or SQLite? | Bundled PostgreSQL. |
| OR-7 | Does the Mac hub ship without RDP and VNC first? | No. It ships with remote display: RDP or VNC, the one that works best first. |
| OR-8 | Is "Keep awake with the lid closed" in the first version? | Yes, both levels, with an easy set-up step. |
| OR-9 | Shares and split custody on a hosted hub | For now, no shared devices on a hosted hub. Look again after the accelerator results. |

## 10. Open items

| ID | Item | Owner |
| --- | --- | --- |
| OI-1 | The three spike reports (Section 5) | Spike sessions |
| OI-2 | The spec delta for a new phase (OR-2): Sections 4.1, 4.2 and 9, and the `LNK`, `AWK`, `MHB`, `SHR` and `MBR` requirements | After the spike reports |
| OI-3 | Split custody for the hosted mode: the size is about one large work package. It is not planned in this document. | Operator, with the business session |
| OI-4 | macOS: does "Keep awake with the lid closed" need administrator rights? (Section 4.8) | S8 task |
