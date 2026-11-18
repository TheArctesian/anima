# Record: where the compute plan did not agree with its sources (2026-11-18)

| Item | Value |
| --- | --- |
| Document | A record of one comparison. It adds no requirement (PLN-010). |
| Date | 2026-11-18 |
| Compared | [compute-plan.md](../../plans/compute-plan.md) version 0.2 (2026-11-06) against: the work items [C1](../../work-items/compute/C1.md) and [C2](../../work-items/compute/C2.md) (scope of 2026-11-07), [rulings.md](../../rulings.md), the link and the hosted hub, and the code on `main` at `3dc5beae` |
| Result | Version 0.3 of the compute plan. The column "Version 0.3" says what it did. "Open" means that the plan did not choose: the row is an open decision of its Section 15. |
| Language | ASD-STE100 Simplified Technical English (STE) |

The operator asked for this comparison on 2026-11-18 (the choice "B: revise first" for row 4.5 of the work plan). The session that made it approved nothing.

## A. The plan and the work items C1 and C2

Each of the two work items said: "Where they do not agree, this text applies until the plan is changed".

| Number | Subject | Compute plan 0.2 | C1 or C2 | Version 0.3 |
| --- | --- | --- | --- | --- |
| A1 | The keys of the endpoint | RTR-004 (Section 6.1): "Each key must have a name, a list of permitted models and an optional expiry." CUI-004: the operator makes and revokes keys. | C1, "First-release endpoint authentication": "Use one shared API key for all models"; allowlists "are deferred and must not appear in v1 acceptance"; rotation and revocation stay, with the transition "explicitly defined before implementation". | RTR-004 and CUI-004 have the text of C1. Section 1.3 puts keys for each project out of scope. |
| A2 | Where the keys are kept | SEC-C-003 (Section 9): "with the credentials domain (SEC-003)". | C1: "the central VPS vault in E0-6 owns key storage/distribution and its protected-cache lifecycle". | SEC-C-003 names the key vault of E0-6. Task L1-9 thus waits for that vault. |
| A3 | The pages | Section 12.1: C1 has the Compute page; C2 has the Models page and the Chat page. Section 12.3: C2 is done when "The Chat page streams a response". | C1: screens "are later delivery, not foundation acceptance requirements". C2 title: "(UI later)"; "Their completion is not a gate". | Sections 8, 11 and 12 mark the pages as later delivery (tasks L1-19 to L1-21). The acceptance of C1 and C2 needs no page. |
| A4 | The choice of a runtime | Section 3.4: a table of default runtimes. Section 14, finding 1. | C1: "No exact runtime choice or automatic model-selection policy has been settled"; "Older runtime defaults ... remain planning context". | Section 3.4 calls the table planning context. A row is a default only after its combination is validated (CMP-006, RTC-007). |
| A5 | The capabilities of a model | LIBM-001 (Section 4.1): "chat, tools, vision, embeddings, reasoning". | C1, "Desired model families": text, image, video, audio, speech, vision and documents, search, tabular, reinforcement learning, 3D later; they are tags, not proof that a model runs; three states to keep apart (catalogued, downloaded, validated). | LIBM-008 and LIBM-009 hold the tags. CMP-006 holds the three states. CMP-007 says that the first release serves text and image input. |
| A6 | The idle unload and the pin | FIT-022 (Section 5.3): "The default is 15 minutes ... 'never' pins the copy." | C2: "Do not impose one universal fixed idle timeout"; "pin means a keep-warm preference, not an unevictable memory reservation". | FIT-022 and FIT-023. |
| A7 | Making room for a load | FIT-021: "the hub can unload the least recently used loaded copy on that node". | C2: "Track live memory reservations"; "Never unload a model with active work, silently kill active jobs or delete downloaded copies"; restore the pinned model after. | FIT-021, FIT-023, FIT-024, FIT-025. |
| A8 | What a caller sees during a load | RTR-031 (Section 6.3): "The caller waits until the load is done or until a time limit passes. The limit is a setting." | C2, "Progress is not generated content": the caller selects wait until a deadline or fail at once; status only for a caller that asks; "Do not inject fake loading text". | RTR-031, RTR-039, FIT-026. The field names are work of the contract (task L1-15). |
| A9 | The choice of a machine | FIT-020: the hub loads a copy "on the best node". RTR-030, RTR-034: the router selects and scores. | C1: "a caller's chosen compatible model and machine. Validate the selection or return a clear ... reason". C2: "Do not infer an automatic machine/model selection policy". | Open: OD-C10. RTR-014 and RTR-016 hold the explicit choice and the reasons. RTR-034 is planning context. |
| A10 | The variant of a pull | PULL-002 (Section 4.2): "the hub must select one". | C1: no automatic selection is settled; compatibility "must be explicit". | Open: OD-C10. |
| A11 | The hardware values | PH-C-000 (Section 12.1): "The daemon gives its own hardware values, so Level 1 does not need the Phase 2 metrics." | C1, layer 1: "Reuse GPU inventory/readings from Phase 3 WP-51/WP-54 and its shared agent/agent-core foundation." | CMP-005, CAP-032, PH-C-000. |
| A12 | Vision for other parts of Anima | No text. | C1, "Shared vision capability": private image-input requests through the same pulls, runtimes and router, for the extraction of E0-5. | LIBM-010, CMP-007, task L1-14. |
| A13 | The API of the endpoint | RTR-010 (Section 6.2): five OpenAI endpoints from the first build. | C1: "Older ... API capability lists below remain planning context". Streaming text is part of serving. | RTR-010: models and chat first, the others when a model that needs them is validated. |
| A14 | The acceptance | Section 12.3, rows C1 and C2. | C1 and C2, "Acceptance" (the boundary of the decisions, validated models, rotation, reservations, wait policy, no page). | Section 12.3, rows C1 and C2. |
| A15 | The hardware | OD-C1: the operator gives the list. | C1: "a Mac M4 32 GB, this Mac M5 48 GB, and a Pop!_OS desktop with an A6000 and unspecified i9. These are not inspected inventory". | Section 15.1 has the table with these values and their source. Still open. |

## B. The plan and the rulings

| Number | Subject | Compute plan 0.2 | Ruling | Version 0.3 |
| --- | --- | --- | --- | --- |
| B1 | The path of a model request | Section 2: the router is in the hub. RTR-001: "The router must answer at `https://ai.anima.ing`." C1 says the same ("`ai.anima.ing` exposes the model registry and private authenticated serving"). | R-131 (2026-11-17): "A model request goes directly from the computer of the caller to the compute node. The Anima program on the computer of the caller listens on the loopback address ... The hub ... does not carry the request." | Sections 1.1, 2, 6 and 9: the model address on each computer; CMP-013; RTR-001, RTR-002, RTR-005, RTR-037. |
| B2 | One daemon | Section 3.3: "The daemon is optional"; CAG-001: "a separate Rust binary". Section 1.4: "The metrics collector stays daemonless." OD-C2: one binary or two. | R-084 (2026-11-07): metrics come from one agent on each device, and "the same agent later becomes the compute agent". R-126: its name is `anima-daemon`. | Section 3.3, CAG-001, CAG-002; Section 1.4; OD-C2 is decided for this plan. |
| B3 | The daemon listens | SEC-C-002 (Section 9): "The daemon must listen only on the VPN address of the node." CAG-003: the hub sends commands. | MET-020 of the design specification (R-084): "The daemon must not listen on any port". MET-002: the daemon "must only read values on the device". R-115 did not take "a command channel". | CAG-003: the daemon asks for work and listens on no port. The permission to change the machine is open: OD-C9. |
| B4 | The enrolment | CAG-002: "a single-use enrolment code ... a node token. Node tokens must follow VPN-003". | R-084 and K8: the daemon has its enrolment and its token; R-126: the prefix `anima_daemon_`. | CAG-002. |
| B5 | The order of the AI functions | Section 1.1: "Level 1: the model service (first priority)". | R-132 (2026-11-17): the functions for AI agents and their use are built first; the model service follows. | PH-C-002. |
| B6 | The record of use | RTR-037: the router in the hub records each request. | R-131: the hub keeps the record of use and carries no request. OD-AI1 of ai-direction.md (open): which document owns the record. | RTR-037: the model address reports the use. The owner is OD-AI1. |
| B7 | The command and the MCP tool | PULL-001: a pull from the Models page, the Ollama API and the GraphQL API. No `anima` command in the plan for a model. | CMD-030 (operator, 2026-11-12, AGENTS.md): hub API, command, MCP tool, then client. | PULL-001, SEL-005, PH-C-003, and each row of Section 12.4. |
| B8 | The switch of a machine | CMP-002: only when the operator sets compute on. OD-C8: off by default. | No ruling. The work plan Section 7 records OD-W10 (operator, 2026-11-12: "it should be on by default") and says that it "needs a ruling number"; R-153 uses the "takes work" switch for the install of an agent. | Open: OD-C8. CMP-002 keeps the old text until then. |
| B9 | The approval of the tool inventory | Sections 3.1 and 11.1: "ruling number to be assigned". | rulings.md has no ruling that names CAP-020 to CAP-031 or CUI-010 to CUI-019 (PLN-021). | Not changed. The session that keeps the rulings must give the two approvals their numbers. |

## C. The plan and the network

| Number | Subject | Compute plan 0.2 | The network now | Version 0.3 |
| --- | --- | --- | --- | --- |
| C1 | The name and the certificate | RTR-001: "through Headscale DNS only". RTR-003: a DNS-01 certificate. OD-C7: "VPN only, with Headscale DNS". KEYS-C: a Headscale DNS record and a DNS token. | R-129, R-130: a hosted hub has no VPN address; the operator "will use only the hosted version". A device reaches its hub through the link (`link/`, one-click plan S1). | RTR-001; RTR-003 is withdrawn for the loopback address. Open: OD-C7 (a name or not). |
| C2 | The ACLs | SEC-C-004: "The VPN ACLs must permit only: the hub to each daemon port ... Tag compute nodes `tag:compute`." Section 13: `deploy/headscale/`. | The link has no ACL of Headscale. The computer holds the list of the ports that it opens (HMD-049). | SEC-C-004. Open: OD-C11 (the link only, or the VPN also). |
| C3 | The permitted network | RTR-005: NET-001 and NET-002 apply to the router. CMP-003: "the private network (Section 4.1 of the spec)". Section 1.3: "outside the VPN". | The same. | CMP-003, RTR-005, Section 1.3: "the private network of the hub". |
| C4 | A path from one device to a different device | OD-C5 (after R-131): "That path is not built and not verified." | HM-T11 and HM-T12 (R-143) are on `main`: the hub link can open a TCP connection to port 22, 5900 or 3389 of a computer, when the person turned the port on (HMD-048, HMD-049; `link/devicelink/presence.go`). A path from one device to a different device still does not exist. | Section 2 states the fact. Task L1-12 is the spike. Tasks L1-1 to L1-11 use one machine and do not wait for it. |

## D. The plan and the code on `main`

| Number | Subject | Compute plan 0.2 | Code at `3dc5beae` | Version 0.3 |
| --- | --- | --- | --- | --- |
| D1 | The crates of the daemon | Section 13: `crates/compute-daemon/` and `crates/compute-daemon-core/`. | They do not exist. The daemon is `crates/daemon` and `crates/daemon-core`. | Section 13. |
| D2 | The compute domain of the hub | Section 13: `crates/hub/src/fleet/compute/` with `router/`. | The folder does not exist. The tool inventory is `crates/hub/src/fleet/metrics/capabilities.rs` and `crates/daemon-core/src/inventory/`. | Section 13: no `router/`; the folder is still to be made. |
| D3 | The contracts | Section 13: `contracts/compute/daemon-protocol.md`, `router-api.md`. | `contracts/` has no `compute/` folder. | Section 13: `model-work.md` (task L1-3) and `model-request.md` (task L1-10). |
| D4 | The endpoint | Sections 1 and 6: `ai.anima.ing`, `tag:compute`. | No file outside `docs/` has `ai.anima.ing` or `tag:compute`. | See B1 and C1. |
| D5 | The daemon | Section 3.3: the daemon accepts commands and supervises runtimes. | The daemon opens no network listener, and no work type is in the code (no `agent_install`, no work route). | See B3; OD-C9. |
| D6 | The hardware values in the hub | CAP-001 to CAP-005. | The push batch of K8 has `gpus` (index, name, vendor, `hasVram`), `drives` and `inventory` (`contracts/daemon-push.md`). `Query.capabilities` gives the tool inventory only (`contracts/schema.graphql`). | CAP-032; task L1-1 adds what is missing. |
| D7 | The pages | Section 13: routes `models/`, `chat/`, `compute/`. | `apps/client/src/routes/` has none of them. `$lib/metrics/tools/` (Section 11.1, tasks T1 to T4) does not exist. | Later delivery. Section 11.1 did not change. |
| D8 | The tool inventory | CAP-020 to CAP-031, built. | On `main`, as the plan says; its checks HT-002 to HT-006 are open. | No change. |

## What version 0.3 did not change

Section 10 (Level 2) and Section 14 (the runtime research), SEC-C-005 and SEC-C-006, the tool inventory (CAP-020 to CAP-031) and Section 11.1 (CUI-010 to CUI-019).
