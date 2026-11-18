# GH-2: Tamper-evident audit log: a hash chain so a change to an old entry shows

Progress: the index [README.md](../README.md) holds the state of this item (PLN-006). The plan register in [work-plan.md](../../work-plan.md) Section 9 lists the index.

| Item | Value |
| --- | --- |
| Identifier | GH-2 |
| Kind | Work item |
| Owner | [design-spec.md](../../spec/design-spec.md) Section 3.8 (LOG-005, LOG-006) |
| Serves | Work plan Section 4, tier 4 (hash chain for the audit log) |
| Version | 1.0 |
| Date | 2026-11-16 |
| Origin | GitHub issue #2: made 2026-11-05, last changed 2026-11-13, open at import. Labels: enhancement, security, design, priority: later. |
| Text | Copied from the issue on 2026-11-16 (R-119). Only the links were changed. Not written in STE yet. "This issue" in the text means this document. |
| Note | Only phase L0 stays (OD-W9). |

## Tasks of phase L0

The requirements come first (PLN-013). The build follows CMD-030.

| Task | Content | State | Commits |
| --- | --- | --- | --- |
| L0-T1 | The requirements LOG-021 to LOG-028 in the design specification Section 3.8.2, and the ruling R-136 | Done. Approved by the operator, 2026-11-17 (R-136) | 33295a96 and the commit that records R-136 |
| L0-T2 | The hub: the migration, the chain on each audit write, the check of the chain, the GraphQL field (K1, K2, K3) | Built: migration 0506, `events/chain`, `Query.auditChain`, the daily check. Waits for HT-023 ([need-human-testing.md](../../need-human-testing.md)) on the real hub | 9113ee8d, a14a9a56 |
| L0-T3 | The `anima audit` command and the MCP tool for the check | Done. `anima audit verify` and `anima_audit_verify` | 9113ee8d |
| L0-T4 | The client: show if the chain is whole | Done by ruling: no new screen; a fault shows as an Error event in the Logs panel (R-136, LOG-028) | R-136 |

## Scope made smaller — 2026-11-12

The operator decided on 2026-11-12 that Anima keeps **one home with copies that a policy places**, not a copy of the data on each node that syncs between nodes ("lets go with the one in the side chat as the every machine has alocal copy doesn't really make sense as different computer have different capacities"). The operator then selected "Narrow it" for this issue.

**What stays: phase L0 only.** Every operation gets a stable ID, a writer and a schema version, and the audit log becomes a hash chain in PostgreSQL, so that a change to an old entry shows (LOG-005, LOG-006). It needs no copies on other machines.

**Withdrawn: phases L1 to L5.** Node identities and signed per-writer chains, the sync engine, direct sync between nodes when the hub is down, authorisation policy on the ledger, quorum checkpoints, and the Headscale watcher. The "Related direction" of 2026-11-05 at the end of this issue is replaced by the decision above.

What replaces them is in the draft proposals on the branch `claude/work-plan-integration` (not yet merged): `docs/proposals/coordinator-standby.md` (standby copies of the database, a new home only after the operator confirms, a read-only recent copy on each client) and `docs/proposals/bulk-data-placement.md` (files and backups as encrypted pieces on the operator's machines).

---

## Previous scope and original planning context

## Goal

**A compromised machine must not be able to change the data or history of any other machine.**

Anima will grow from one hub into a network of nodes that all belong to one operator: the always-on hub (the *anchor*), one site agent per site, and the operator's desktop apps. Each node will hold a local copy of the data and sync changes, Linear style. When the hub is down, nodes sync directly over the VPN mesh (see *Related direction* below).

Once data lives on many machines, the trust model has to change. Today the hub is trusted for everything. This issue plans a **signed, hash-linked, replicated ledger** (a permissioned "blockchain" in the loose sense) so that trust is spread across the nodes.

This is a plan for a later phase. It is not Phase 1 work: the contracts are frozen (CON-001).

## Non-goals

- No proof of work or proof of stake, no token, no public chain, no smart-contract VM. All nodes belong to one operator, so there is nothing to mine and no one to pay.
- No global consensus on every event. Most data has a single writer and needs none (see *What goes on the ledger*).
- No schema migration or back-migration of replicated data yet (the operator deferred this; see *Deferred* below).

## Threat model

| Compromised node | What it can still do | What the ledger must stop |
| --- | --- | --- |
| One operator desktop (stolen or malware) | Read the replica it already holds; sign low-risk changes with its own key until revoked | Changing credentials, device hosts or ports, keys or policy **alone**; rewriting or deleting anyone's history; pretending to be another node |
| One site agent | Lie about its own site's heartbeats (it is the sensor); read its own site's secrets | Touching other sites' data or secrets; changing config; hiding that it showed different histories to different peers |
| The anchor (hub) | Withhold or delay changes (censorship); read the data it holds | Forging other nodes' changes; finalising history alone; rewriting finalised history |
| The operator root key | Everything | Out of scope. Keep it offline, on a hardware key, with a written recovery procedure |

## Design

### 1. Identity

- Every node has its own signing key, created on the node and kept in the OS key store (Keychain, Secret Service, or TPM where available). Use Ed25519, or P-256 where the key must live in the Secure Enclave.
- An **operator root key**, kept offline on a hardware key, signs a *node certificate* for each node: public key, role (`anchor`, `site-agent:<site>`, `operator-device`), the time it was issued, and an optional expiry.
- The first ledger entry (*genesis*) holds the root public key and the first certificates. Every later key change is a ledger entry signed by the root key or by a quorum (see §4).
- Revocation is a ledger entry. Peers reject anything a revoked key signs after the revocation's causal position.

### 2. Ledger structure: per-writer chains joined into a Merkle DAG

Every node writes only to its **own** append-only chain. An entry holds:

```text
writer      node public key
seq         1, 2, 3 … (no gaps)
prev        BLAKE3 hash of this writer's previous entry
deps        hashes of the latest entries this writer had seen from other writers (causal links)
hlc         hybrid logical clock (for display and tie-breaks, never for authority)
schema      payload schema version (present from day one, even before migrations exist)
payload     a typed operation (see §3)
sig         signature over all of the above
```

All the chains together form a Merkle DAG. An entry's hash covers its whole causal past, so changing one old entry changes every hash after it. Peers would see that at once.

### 3. What goes on the ledger

| Data | On the ledger | Why |
| --- | --- | --- |
| Config changes (devices, checks, windows, tags, sites) | Every operation | Low volume, and it is what an attacker most wants to change |
| Audit entries (LOG-005) | Every entry | The audit log must be tamper-evident across nodes, not only through database permissions (LOG-006) |
| Key, role and policy changes | Every operation, with quorum (§4) | They control everything else |
| Heartbeats | **Only commitments**: one signed Merkle root per check per hour, written by the site agent that ran the check | The raw volume (about 43,000 rows per check per month) is too large for the ledger. Raw rows stay in the site's store and can be checked against the commitment on demand |
| Event log | Commitments, like heartbeats | Same reason |
| Credentials | **Never in plain text.** Encrypted with HPKE to the site agent of the device's site and to the operator devices; the ledger holds only the ciphertext and its hash | A compromised node may expose only its own site's secrets, never the whole network's (SEC-003, SEC-005, CLI-004) |
| Live session content | Never (RA-008) | — |

Every node turns the DAG into its own read model: SQLite on clients and agents, PostgreSQL on the anchor. That read model is what the Linear-style sync engine and the UI read from.

### 4. Authorisation is data, checked by every peer

The policy lives on the ledger. Every peer checks each operation at its causal position and gets the same answer, so there is no central gatekeeper:

- `site-agent:<site>` may write heartbeat commitments and events only for checks assigned to its site, and nothing else.
- `operator-device` may make low-risk config changes: names, tags, check intervals, maintenance windows.
- **High-risk operations need M-of-N signatures**, for example two operator devices, or one device plus the root hardware key. These include:
  - a credential change;
  - a change of a device's host or port, or of its SSH host key (SEC-006/007);
  - adding, revoking or changing the role of a key;
  - a policy change;
  - a change of the probe deny list or the permitted networks (NET-001).
- `anchor` may propose checkpoints (§6) and order changes for the sync engine. It cannot authorise config.

Prior art: Ink & Switch's Keyhive (local-first access control), Matrix state resolution, and Tailscale's Tailnet Lock (a quorum of signing nodes).

### 5. Spotting a node that lies

- Peers gossip their chain heads, as Certificate Transparency logs do.
- Two different entries with the same `(writer, seq)` are **proof of equivocation**. Either one is enough to prove it, and every peer can check it.
- A peer that sees such proof spreads it, quarantines the key (stops accepting its new entries), raises an Error event, and waits for the operator to revoke the key or clear it.
- A site agent that reports false heartbeats cannot be stopped by signatures, so detect it instead: the anchor sometimes probes the same devices through the VPN and flags results that disagree with the agent's.

### 6. Finality checkpoints

- Every N minutes the anchor proposes a checkpoint: the Merkle root of every writer's current head.
- The checkpoint is **final** when a quorum co-signs it, for example the anchor plus a majority of online site agents. Nodes refuse any fork that would rewrite history before the last final checkpoint.
- An operator desktop that was offline for a long time syncs from the newest final checkpoint instead of replaying everything.
- This is the only place the design needs agreement among several nodes. It runs in the background, so it does not slow down normal writes.

### 7. Retention without breaking immutability

Retention (DB-009, LOG-006) drops data, but the ledger never drops entries. So expired raw data is deleted while its hashes and commitments stay. You can still prove what existed, even though the data is gone.

### 8. The VPN control plane (OD-10)

Headscale has no Tailnet Lock, so whoever controls it can add a node (vpn.md §11). Two steps:

1. **Detect.** A watcher compares Headscale's node list, routes and policy with the ledger's authorised set, and raises an Error event for anything not on the ledger.
2. **Prevent (later).** Clients accept a peer only if its node key is certified on the ledger, which is a self-hosted equivalent of Tailnet Lock. This needs work in the client or the coordinator, and should be evaluated separately.

## Phases

| Phase | Delivers | Depends on |
| --- | --- | --- |
| L0 (cheap, do early) | Every operation gets a stable ID, a writer and a schema version. The audit log becomes a hash chain in PostgreSQL | — |
| L1 | Node identities and certificates; signed operations; the Linear-style sync engine with central ordering by the anchor | L0, the sync engine |
| L2 | Per-writer chains, the Merkle DAG, gossip, equivocation proofs; direct peer sync over the mesh when the hub is down | L1, site agents |
| L3 | Authorisation policy on the ledger; M-of-N for high-risk operations | L2 |
| L4 | Quorum checkpoints and finality | L2 |
| L5 | The Headscale watcher, then peer-key certification | L1 |

## Open questions

- The quorum size for checkpoints when only a few nodes exist, for example an anchor and one site agent. The hardware root key could act as a tie-breaker.
- Merkle-DAG CRDT on its own versus a small BFT engine (CometBFT, HotStuff) for checkpoints only. BFT needs 3f+1 nodes to tolerate f faulty ones.
- Choice of library: Automerge, cr-sqlite or a custom operation log for the read models; how hypercore- or Secure Scuttlebutt-style feeds map to Rust.
- How the operator approves an M-of-N operation from a second device without friction: a push notification and one tap, or a hardware key.
- Recovery when the root key is lost: a social or offline backup scheme.

## Deferred

The operator wants these left until later, while Anima is in early development:

- **Schema migrations across replicas**, including **back-migrating** data on nodes that were offline during an upgrade. Ops carry `schema` from L0, so this can be added later without a format break.

## Related direction (operator, 2026-11-05)

- **Linear-style sync engine:** a local copy on every client and agent, optimistic edits, and a durable change feed with sync IDs from the always-on anchor.
- **Keep working when the hub is down:** sites keep checking, operators keep reading and editing locally, and peers sync over the mesh until the hub returns.
- One always-on server stays: it is the anchor, the archive and backup, and the outside observer that can report that a whole site is down.
- Related: R-055 (in-app enrolment), [#1 Deferred work](../../deferred.md).

🤖 Generated with [Claude Code](https://claude.com/claude-code)
