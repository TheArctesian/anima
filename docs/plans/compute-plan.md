---
kind: Plan
state: In progress
evidence: Version 0.3 (2026-11-18) makes Level 1 agree with the work items C1 and C2, the rulings R-084, R-126 and R-131, the link and the code on `main` at `3dc5beae`; the record is [plan-reconcile-2026-11-18.md](../research/compute/plan-reconcile-2026-11-18.md). Version 0.3 approves nothing. Waits for the operator: the approval of Level 1 (Sections 1 to 9, 11 and 12, tasks L1-1 to L1-21), and OD-C1, OD-C7, OD-C8, OD-C9, OD-C10 and OD-C11 (Section 15). Built: the tool inventory (CAP-020 to CAP-031): the daemon, the hub, `anima capability list` and the MCP tool (commits 1cb43b06 to a0cf53e5). Its live checks are open ([need-human-testing.md](../need-human-testing.md), HT-002 to HT-006). Its client section has the words of approval of the operator and is not started (Section 11.1, CUI-010 to CUI-019). No other part is approved. OD-C5 is decided (R-131): a model request goes directly from device to device. Level 2 (Section 10, C4 to C6) is parked.
---

# Anima Compute Plan

Progress: the plan register in [work-plan.md](../work-plan.md) Section 9 holds the state of this plan (PLN-005).

| Item | Value |
| --- | --- |
| Document | Compute plan |
| Version | 0.3 (draft) |
| Date | 2026-11-18 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | The header of this document holds the state (PLN-020). History: For review (version 0.2, 2026-11-06). Version 0.3 (2026-11-18) changes Sections 1 to 9, 11, 12, 13 and 15 for Level 1: the model request goes directly to the compute node and the hub has no router (R-131); model work is a function of the one daemon (R-084, R-126); the network is the link, not the VPN with Headscale; the scope of [C1](../work-items/compute/C1.md) and [C2](../work-items/compute/C2.md) of 2026-11-07 is now text of this plan; Section 12.4 is the task table of Level 1; Section 15 gives each open decision as one question, and adds OD-C9 to OD-C11. Sections 10 and 14 did not change. Version 0.3 approves nothing: it waits for the operator (see the header). |
| Related | [design-spec.md](../spec/design-spec.md), [extensions-plan.md](extensions-plan.md), [ai-direction.md](../proposals/ai-direction.md), [jobs.md](../proposals/jobs.md), [hosted-mode.md](../proposals/hosted-mode.md), [one-click-plan.md](one-click/one-click-plan.md) |

This document obeys the writing rules in Section 0 of the design specification.

How to read a requirement of this version:

- A requirement of Level 1 is proposed. It is not approved until a ruling approves Level 1 (PLN-007). Build nothing from it before that ruling.
- The tool inventory (CAP-020 to CAP-031) and Section 11.1 (CUI-010 to CUI-019) have the words of approval of the operator. Their text did not change.
- Where version 0.3 changed a requirement, the line "Version 0.2:" gives the old text in short form and the source of the change.
- "Planning context" means text that helps a design and that is not a requirement.
- This plan owns the requirements of the model service (PLN-016). [C1](../work-items/compute/C1.md) and [C2](../work-items/compute/C2.md) point to it.

## 1. Purpose and scope

### 1.1 Purpose

This plan lets Anima use the AI compute of the devices of the operator. It has two levels.

**Level 1: the model service.** Each compute node runs a runtime that is validated for its hardware. The operator pulls models to each node, as with Ollama or Unsloth. Each computer of the operator has one model address on its loopback interface, in the form of the OpenAI API. The model address lists the models of all nodes and sends each request directly to the node that has the model (R-131). The hub keeps the list of models and the record of use. The hub carries no request. The Models, Chat and Compute pages come after the foundation.

**Level 2: pooled models (parked).** A model that is too large for one node runs across several nodes, one block of layers on each node. This needs a small framework of Anima's own (Section 10.3).

The hub decides where work can run. The compute nodes do the work. The hub never runs a model.

Level 1 has three layers ([C1](../work-items/compute/C1.md), scope of 2026-11-07):

1. Hardware signals and decisions: what a machine has, if a model fits, if a machine is permitted (Sections 3.1, 3.2 and 5.1).
2. Model setup on each machine: pulls, storage, runtimes and their supervision (Sections 3.3, 3.4 and 4).
3. One registry and the path of a request: the list of models, the model address, the choice of the node (Sections 6 and 7).

- **CMP-001.** Build only for the devices and models that the operator uses (EXT-000). Do not build a general model catalogue.
- **CMP-002.** A device takes model work only when it is permitted to. OD-C8 decides the switch and its default. Version 0.2: only when the operator sets compute on for that device; that text applies until OD-C8 is decided.
- **CMP-003.** Model work must stay on the private network of the hub: the link (LNK-001 to LNK-008 of the one-click plan), or the VPN for a hub that uses one (Section 4.1 of the spec). Anima must not join a public compute network, for example c0mpute. Version 0.2: "the private network (Section 4.1 of the spec)" only.
- **CMP-004.** Level 1 must be complete and in use before Level 2 starts (PH-C-001).
- **CMP-005.** The hardware signals of layer 1 must come from the daemon that R-084 gave to each machine. Do not add a second reader for a value that the daemon sends (CAP-032).
- **CMP-006.** The hub must keep three states apart for each variant and each node: catalogued (it can be pulled), downloaded (a copy is on the node), and validated (the copy ran on this runtime and this hardware and gave a correct answer). A variant that can be pulled is not evidence that it runs. The model address lists only validated copies (RTR-011).
- **CMP-007.** The first release serves text with streaming, and image input for a model with vision. Each other model family of LIBM-008 is a tag of the catalogue only. Before a family is called supported, its request, job, progress and result contract must be agreed, and one model of the family must be validated end to end on a runtime (source: C1).

### 1.2 Sources

This plan uses ideas from these projects. It copies no code from a project with an AGPL licence.

| Project | Licence | Idea that this plan uses |
| --- | --- | --- |
| [Ollama](https://github.com/ollama/ollama) | MIT | Pull a model by name and tag, list models, load on demand, unload when idle, the Ollama API |
| [unslothai/unsloth](https://github.com/unslothai/unsloth) | Apache 2.0 (core), AGPL 3.0 (`studio/`) | Hardware verdicts with a reason, memory fit estimate, offload planner, managed server flags, quantised GGUF and MLX model files |
| [leyten/shard](https://github.com/leyten/shard) | Apache 2.0 | Level 2: pipeline split over a WAN, speculative decoding, the `ModelRuntime` block contract |
| The 12 runtimes in Section 14 | Different | Runtimes for each hardware type, router designs, the single-site split (llama.cpp RPC) |

### 1.3 Out of scope

- Access to a model address or to a compute node from outside the private network of the hub (SEC-008).
- A key for each project, and a list of permitted models for each key. The first release has one shared key (RTR-004).
- The automatic choice of a model (SEL-004).
- Tensor parallelism across machines. It needs a fast link and one runtime on each side.
- Training across machines.
- Windows devices (Section 1.4 of the spec).

### 1.4 Changes to the design specification

Operator checkpoint 3 of the implementation plan applies. The operator must approve each change. [SPEC-C](../work-items/compute/SPEC-C.md) is the approval item; none of these changes is in the specification.

| Spec part | Change |
| --- | --- |
| 1.1 Purpose | Add a function: a private model service on the devices of the operator. |
| 3.1 Overview | Add model work to `anima-daemon` (Section 3.3) and the model address on each computer (Section 6). Version 0.2: a compute daemon and a router on the hub. |
| 4.1 NET-005 | No change, unless OD-C7 keeps the name `ai.anima.ing`. Version 0.2: add the DNS name `ai.anima.ing`, on the VPN only. |
| 4.2 VPN-004 | No change for a hub on the link. Version 0.2: ACL rules for compute traffic. They are necessary only if the operator wants Level 1 on a hub with the VPN (OD-C11). |
| 5.3 Navigation bar | Add the Models, Chat and Compute pages (Section 11). They are later delivery. |
| 6.6 Metrics, MET-002 and MET-020 | The daemon gets a fixed list of model work types, and with them it changes its own machine (OD-C9). Version 0.2: "the metrics collector stays daemonless; the compute daemon is a separate, optional program". R-084 replaced that text. |
| 9 Delivery phases | Add the compute phases (Section 12). |
| 13 Repository structure | Add the compute domain, the model work of the daemon and, in Level 2, the block adapters (Section 13). |

### 1.5 Terms

These terms add to Section 2 of the design specification.

| Term | Meaning |
| --- | --- |
| Compute node | A device that is permitted to take model work (CMP-002). |
| Daemon | `anima-daemon`, the one daemon of each machine (R-084, R-126). It reports the hardware and does the model work. Version 0.2 called it the compute daemon. |
| Runtime | An inference program, for example llama.cpp, MLX-LM or vLLM. |
| Backend | The hardware interface of a runtime: CUDA, ROCm, Metal, Vulkan, SYCL or CPU. |
| Model server | One runtime process that serves models through an OpenAI-compatible API. |
| Model | One named model in the library, for example `qwen3.8-27b`. |
| Variant | One file set of a model in one format and quantisation, for example GGUF `Q4_K_M` or MLX 4-bit. |
| Copy | One variant on the disk of one compute node. |
| Loaded copy | A copy that a model server holds in memory. |
| Validated combination | A variant, a runtime and the hardware of one node that passed the validation of CMP-006. |
| Alias | A name that the operator gives to a model for a job, for example `coder`. |
| Model address | The listener on the loopback address of a computer that answers model requests in the OpenAI form (R-131). |
| Router | The function behind the model address that finds the node for a request. It is in the Anima program on the computer of the caller. It is not in the hub. Version 0.2: a function of the hub at `ai.anima.ing`. |
| Model key | The one shared key that a caller sends to a model address (RTR-004). |
| Pin | The wish of the operator that a model stays loaded. It is a preference, not a reservation of memory (FIT-023). |
| Reservation | The memory that a loaded copy, a load in progress or an active request holds on a node (FIT-024). |
| Placement | The decision where a model runs. |
| Block, stage, coordinator, ring | Level 2 terms. Section 10 defines them. |

## 2. Architecture

```text
 A program on a computer of the operator (script, editor, AI agent, the Anima client)
        |
        |  http://127.0.0.1:<port>/v1/...  (OpenAI form; Ollama form in C2), with the model key
        v
+--- The Anima program on the same computer: the model address (R-131) ---+
|  asks the hub which node has the model          reports the use         |
+---------+-----------------------------------------------+---------------+
          | list of models, node, use (hub API)           | the model request, direct:
          v                                               |  same computer: loopback
+------------------ Hub ------------------+               |  other device: through the link
|  fleet/compute/                         |               |  (not built: task L1-12)
|  capabilities  library  placement (pure)|               v
|  copies  use                            |      +--------------------+
+---------+-------------------------------+      | Compute node       |
          ^  the daemon asks for model work      | anima-daemon       |
          |  and reports (OD-C9)                 | runtime on loopback|
          +--------------------------------------| model cache        |
                                                 +--------------------+
```

A node is, for example, a Mac with MLX-LM and llama-server, or a PC with an NVIDIA GPU and llama-server. Section 3.4 has the choice of a runtime.

- **CMP-010.** The hub must not load a model, run a layer or keep hidden states.
- **CMP-011.** The runtime choice, the fit estimate and the placement must be pure functions. They take the inventory, the measurements and the configuration as parameters (CQ-020).
- **CMP-012.** The hub must keep working when all compute nodes are off. Compute is optional.
- **CMP-013.** The hub must not carry a model request or its answer (R-131). The hub gives the list of models, tells a model address which node has a model, and keeps the record of use (RTR-037).

Facts of 2026-11-18 that this section depends on:

- The link carries streams from a device to its hub. Since HM-T11 and HM-T12 (R-143) the hub can also open a TCP connection to port 22, 5900 or 3389 of a computer on its link, when the person turned that port on at the computer (HMD-048, HMD-049). No path from one device to a different device exists. R-131 says that a spike must prove that path before a requirement depends on it. Task L1-12 is that spike.
- Thus the first parts of Level 1 use one machine only: the model address and the model server are on the same computer (tasks L1-1 to L1-11). This is also the first version of the fallback of [bots.md](../proposals/bots.md) (BOT-049).
- [ai-direction.md](../proposals/ai-direction.md) OD-AI1 asks which document owns the model address, the list and the record of use: this plan or the model gateway (Section 4.15 of the extensions plan). It is open. Until it is decided, this plan defines them for the models on the machines of the operator. If the operator gives them to the model gateway, RTR-001 to RTR-042 move there with their numbers.

## 3. Compute nodes

### 3.1 Capability inventory

- **CAP-001.** For each compute node, the hub must keep these values: the backends, the accelerator name, the dedicated memory, the unified memory, the system memory, the free disk space, the CPU features and the site.
- **CAP-002.** For each backend, the daemon must give a verdict: usable, or not usable with a reason and a repair hint. Example: "ROCm: not usable, the user is not in the `render` group".
- **CAP-003.** The daemon must report the installed runtimes and their versions.
- **CAP-004.** The daemon must report the power state (mains or battery), the thermal state and whether the operator is active on the device.
- **CAP-005.** On Apple silicon, the usable memory is the GPU working-set limit of macOS, not the total memory. The daemon must report this limit.
- **CAP-006.** The hub must keep the measured tokens per second of each loaded copy. The choice of a node can use it (RTR-034).
- **CAP-032.** The values of CAP-001 to CAP-005 must come from the readers of `anima-daemon`. The daemon already sends the GPUs of a machine (`gpus` of K8: index, name, vendor) and the drives. A value that the daemon does not send is a new optional member of the push batch of K8, as the tool inventory is (CAP-029). The hub gives the values in `capabilities` (CAP-030). Version 0.2 (PH-C-000): "the daemon gives its own hardware values".

#### Tool inventory

Approved for build (operator, 2026-11-15: "lets build the two buildable ones now"; ruling number to be assigned). The first build leaves out the Apple SDKs, the simulator runtimes and the running state of the container runtime (CAP-023). The first build probes `claude` and `codex` only; a harness of AGN-009 gets a probe when the operator selects it. The graphical client is Section 11.1 (CUI-010 to CUI-019); it is not built. An agent that runs as a system unit does not see the tools in the home folder of a user. On macOS, the agent does not run the placeholder programs of Apple in `/usr/bin` when the developer tools are absent. The operator said that an AI agent "will need to know what's installed so that it can either run commands or run agents". The placement of a job uses this inventory (JOB-012 to JOB-014 of [jobs.md](../proposals/jobs.md)). The tool inventory only reads. It needs no work route.

- **CAP-020.** The daemon must report a tool inventory of its machine to the hub: the tools, the build targets and the AI agent programs.
- **CAP-021.** The tool inventory must come from a fixed list of probes in the daemon. The hub must not be able to send a probe or a command line to the daemon.
- **CAP-022.** A probe must run a program only with fixed arguments that print a version or a list. A probe must not start a build, a model or a network request.
- **CAP-023.** The first probes must cover these items:

  | Group | Items |
  | --- | --- |
  | Source control | git |
  | Rust | rustc, cargo, each installed target of rustup, cargo-nextest, sccache |
  | JavaScript | bun, node |
  | Apple | the Xcode version, each installed SDK and simulator runtime |
  | Containers | the container runtime and whether it is running |
  | Nix | nix |
  | AI agent programs | claude, codex, and each harness of AGN-009 |

- **CAP-024.** Each line must give the name and a verdict, in the form of CAP-002: usable with the version, or not usable with a reason. The reason is a fixed code (timed out, exited, no version), never text from the device. The command and the MCP tool give a fixed repair hint for each reason.
- **CAP-025.** For an AI agent program, the line must say only "installed" and the version. The daemon must not read a credential file, an account name or a usage limit (LOG-007). The probe of such a program must run with an empty temporary home folder, so that it cannot read or change the folders of the account.
- **CAP-026.** The tool inventory must include the processor architecture and the operating system version.
- **CAP-027.** The daemon must send the tool inventory at start and when a probe result changes. It must check for changes each 15 minutes or less often. It must send an unchanged inventory again each 24 hours.
- **CAP-028.** The probe list and the parse of each probe output must be in `daemon-core`, with no I/O (CAG-007). The reader that runs each program is the I/O edge.
- **CAP-029.** The tool inventory must be an optional member of the push batch of K8, as `gpus` and `drives` are. K8 permits an optional member without a new version (its Section 1). Thus the daemon makes no new call and the daemon token does not change (MET-013). K8 is frozen (CON-001), so the addition carries the mark of this ruling.
- **CAP-030.** The hub must give the tool inventory of each device as `capabilities`. CAP-001 to CAP-004 join it when the model service is built. It must filter by tool name and by build target. The command is `anima capability list` and the MCP tool is `anima_capability_list` (CMD-030).
- **CAP-031.** Text in a tool inventory comes from a device. The MCP result must give it in named fields with a length limit (MCP-008).

### 3.2 Device policy

"Takes work" is the permission of a machine (USE-004 of [jobs.md](../proposals/jobs.md), proposed). The device policy says when a permitted machine takes model work (USE-009).

- **CAP-010.** The operator must set a policy for each compute node: on or off (OD-C8), a maximum memory share, a maximum disk share for models, and quiet hours.
- **CAP-011.** A portable computer must take compute work only on mains power. This is the default, and the operator can change it.
- **CAP-012.** If the operator is active on a client computer, the daemon can refuse new work. The operator sets this for each device.
- **CAP-013.** When a policy stops a node, the hub must drain it. The node gets no new requests. Current requests continue for 60 seconds or fewer.

### 3.3 Model work of the daemon

R-084 gave each macOS and Linux device one daemon, and said that the same daemon later does the compute work. R-126 named it `anima-daemon`. Its code is `crates/daemon` and `crates/daemon-core`. Today the daemon only reads values, connects only to the hub and listens on no port (MET-002, MET-020). Model work changes the machine: it installs a runtime, downloads files and starts processes. Thus it needs a ruling (OD-C9), as the update of the daemon did (R-139) and as the work route of the builds does (task W1 of [projects-and-builds-plan.md](projects-and-builds-plan.md)).

Version 0.2 of this section: a separate, optional Rust binary; the hub sends commands to a port of the daemon on the VPN; an enrolment and a node token of its own.

- **CAG-001.** Model work must be a function of `anima-daemon`. Do not make a second daemon binary and do not make a second pair of crates (R-084). The daemon starts with launchd on macOS and systemd on Linux, as it does today.
- **CAG-002.** The daemon must use the enrolment and the token that it has (K8). A daemon token must not permit model work by itself: the hub gives model work only to a device that CMP-002 permits.
- **CAG-003.** The daemon must ask the hub for model work (USE-005). It must not listen on a network port for it (MET-020). The work types are a fixed list (USE-006): report, install runtime, pull variant, delete copy, start server, stop server, load, unload, measure and drain. This requirement needs OD-C9.
- **CAG-004.** The daemon must reject work that is not in CAG-003. It must reject runtime flags that the hub owns (SEC-C-007).
- **CAG-005.** The daemon must supervise each runtime process. If a process stops, the daemon restarts it with exponential backoff and reports the event to the hub.
- **CAG-006.** The daemon must send the health and metrics of each runtime to the hub every 15 seconds. These are: state, loaded copies, requests in progress, queue depth, tokens per second and memory used. The contract of task L1-3 says if they are a member of the push batch or a report of the work route.
- **CAG-007.** The daemon logic (work validation, flag policy, backoff, cache choice) must be in `daemon-core`, with no I/O, so that tests can run it.
- **CAG-008.** The daemon can install a runtime only from the release sources in its configuration. It must verify each download.
- **CAG-009.** Model work can write only to the folders of the daemon: the runtimes, the model cache and its state. It must change nothing else on the machine.

### 3.4 Runtime choice

No runtime choice is settled (C1). The table below is planning context from the research of Section 14. A row becomes the default of a node only when its combination is validated on that node (CMP-006), after the operator gave the hardware (OD-C1).

| Hardware | Default runtime | Variant format | Also |
| --- | --- | --- | --- |
| Apple silicon | MLX-LM server | MLX | llama-server (Metal) for GGUF variants |
| NVIDIA, model fits in GPU memory | vLLM | Safetensors, AWQ, GPTQ, FP8 | llama-server (CUDA) for GGUF variants |
| NVIDIA, model needs offload | llama-server (CUDA) | GGUF | — |
| AMD with a usable ROCm verdict | llama-server (HIP) | GGUF | vLLM (ROCm) when the operator selects it |
| Other GPU | llama-server (Vulkan or SYCL) | GGUF | — |
| CPU only | llama-server (CPU) | GGUF | — |
| NVIDIA with a large system memory, mixture-of-experts model | KTransformers | Its own CPU format | Only when the operator selects it |

- **RTC-001.** The runtime choice must be a pure function of the capabilities (Section 3.1), the variant format and the validated combinations. The operator can override it for each node.
- **RTC-002.** A node can run more than one model server, for example MLX-LM and llama-server on one Mac.
- **RTC-003.** vLLM reserves a fixed share of GPU memory. The daemon must run one vLLM process for each GPU and set the share from the device policy.
- **RTC-004.** For a runtime that cannot unload a model (vLLM, KTransformers), the daemon must stop and start the process to change the model.
- **RTC-005.** llama-server must run in router mode, so that the daemon can load and unload GGUF models without a restart.
- **RTC-006.** Each model server must listen on the loopback interface only (SEC-C-001).
- **RTC-007.** The hub must keep the list of the combinations of variant, runtime and node with the state of each (CMP-006). The compatibility of a model with a runtime must be a record, not an assumption.

## 4. Model library and pulls

The library is the list of models that the operator uses. A pull puts a variant of a model on a node, as `ollama pull` does.

### 4.1 Library

- **LIBM-001.** The library must keep, for each model: the name, the family, the parameter count, the context length, the capability tags (LIBM-008), the licence and the source repositories. Version 0.2: the capabilities chat, tools, vision, embeddings and reasoning only.
- **LIBM-002.** The library must keep, for each variant: the model, the format, the quantisation, the file list with the size and the SHA-256 value of each file, and the source repository.
- **LIBM-003.** The source of a variant must be a Hugging Face repository in the approved list. The default list is `unsloth`, `mlx-community` and the model publisher. The operator can add an organisation.
- **LIBM-004.** The operator adds a model by a search of the approved organisations or by a repository name. The hub reads the file list and the metadata from the repository.
- **LIBM-005.** The hub must read the capabilities, the context length and the layer shape from the model files (GGUF metadata or `config.json`). It must not ask the operator for values that the files give.
- **LIBM-006.** The model name and the variant use the form `name:tag`, for example `qwen3.8-27b:q4_k_m`. This is the Ollama form.
- **LIBM-007.** The operator can give an alias to a model, for example `coder` or `fast`. A request can use an alias in place of a model name.
- **LIBM-008.** The capability tags of the catalogue are these (C1, checked there against the task catalogue of Hugging Face). A subtype is a tag, not a serving layer.

  | Family | Tags |
  | --- | --- |
  | Text | chat, coding, reasoning, translation, summarization, text classification, structured extraction and entity tagging (token classification) |
  | Image | generation, editing, upscaling, image classification |
  | Video | generation, editing, understanding, video classification |
  | Audio and music | music and sound-effects generation, audio generation, audio understanding, audio classification, audio-to-audio transformation |
  | Speech recognition | transcription, spoken translation |
  | Speech generation | text-to-speech, conversational voice |
  | Vision and documents | image understanding, OCR, document-field extraction, table and document question answering, visual document retrieval, object detection, image segmentation and background masks, depth estimation, keypoint and pose detection |
  | Search | embeddings, reranking |
  | Tabular | classification, regression |
  | Reinforcement learning | policies, as a tag of the catalogue only. This plans no training infrastructure. |
  | Later | 3D: object, mesh and scene generation |

- **LIBM-009.** A tag says what a model can do. It is not evidence that a runtime of Anima runs the model, and it selects no runtime and no dependency (CMP-006, CMP-007). Non-text generation can need a job with progress and a result file. Speech can need an audio or real-time contract. Search needs an embedding and a reranking contract. Do not assume that a text stream covers them.
- **LIBM-010.** A model with image understanding serves the extraction of the engine ([E0-5](../work-items/engine/E0-5.md)) and the Wardrobe and Library extensions through the same pulls, the same runtimes and the same model address. Do not build a model stack for one extension. Class A documents stay on the client ([XP-09](../work-items/extensions/XP-09.md)). The permission for a hosted model ([OD-E4](../work-items/decisions/OD-E4.md)) is not given here.

### 4.2 Pull

- **PULL-001.** The operator can pull a model to a node from the hub API, from the command `anima model pull` and from its MCP tool (CMD-030). The Models page (CUI-001) and the Ollama pull API (Section 6.4) follow. Version 0.2: the Models page, the Ollama pull API and the hub GraphQL API.
- **PULL-002.** If the operator does not select a variant, the hub must select one, and must show which. The variant must match the format of the runtime choice (RTC-001), and the fit estimate must pass (Section 5.1). The hub prefers the highest quality that fits. This requirement needs OD-C10: C1 says that no automatic selection is settled.
- **PULL-003.** If no variant fits a node, the hub must show the reason: the memory or the disk space that is missing.
- **PULL-004.** The daemon must download the files directly from the source repository to its model cache. The files do not go through the hub.
- **PULL-005.** The daemon must verify each file against the SHA-256 value in the library before it uses the file.
- **PULL-006.** A pull must continue after an interruption from the last complete file part.
- **PULL-007.** The command and the client must show the progress of each pull: bytes, speed and the time to completion.
- **PULL-008.** The daemon must keep the model cache within the disk share of the device policy. It must ask the hub before it removes a copy to make space.
- **PULL-009.** The operator can delete a copy from a node.
- **PULL-010.** A gated repository needs a Hugging Face token. The hub must keep the token as SEC-C-003 says and give it to the daemon only for that pull. No command output and no MCP tool gives the token (MCP-004).
- **PULL-011.** After a pull, the copy is downloaded and not validated. The validation of CMP-006 is a separate step: the daemon loads the copy on the runtime, a fixed request gets an answer, and the hub records the result for this node.

## 5. Placement

### 5.1 Fit estimate

- **FIT-001.** The fit estimate must be a pure function. Its inputs are the weight bytes, the KV bytes for each token, the context length, the fixed overhead for each device and the usable memory.
- **FIT-002.** KV bytes for each token are: 2 × layers × KV heads × head dimension × bytes for each element. The model files give these values.
- **FIT-003.** The fixed overhead for each runtime and backend must be a configuration value. Measurements replace the default values.
- **FIT-004.** If the model does not fit at the requested context, the estimate must give the largest context that fits. The context is rounded down to a multiple of 256.
- **FIT-005.** The fit estimate must give a verdict with a reason for each variant and each node. A GPU on a node is not evidence that a model fits (C1). The usable memory is the value of CAP-001 and CAP-005.

### 5.2 Single-node offload

This section is phase C3 ([C3](../work-items/compute/C3.md), tier 3a). Version 0.3 did not change it, and it has no row in Section 12.4.

Moving part of a model to system memory on one node is often faster than splitting it across machines. This idea comes from the Unsloth offload planner.

- **FIT-010.** If a model does not fit in the GPU memory of a node, the placement can move weights to system memory. It must never move the KV cache.
- **FIT-011.** The placement must rank offload choices by estimated milliseconds for each token, not by percent.
- **FIT-012.** If the model fits without offload, the placement must not offload.

### 5.3 Loading, unloading and memory

This section is phase C2 ([C2](../work-items/compute/C2.md), scope of 2026-11-07). Version 0.2 had FIT-020 to FIT-022 only: the hub loads on "the best node", unloads the least recently used copy, and unloads after 15 minutes, with "never" as the pin.

- **FIT-020.** A request can name a model that has no loaded copy. Then a compatible copy must load before the generation starts. The node is the node that the caller named (RTR-014) or the node of RTR-030. Anima must not select a different model.
- **FIT-021.** To make room for a load, a node can unload only loaded copies that are idle, the least recently used first. It must never unload a copy that has active work, never stop active work, and never delete a downloaded copy to free memory of a runtime. An unload frees the memory of the runtime and keeps the files.
- **FIT-022.** The idle time before an unload must be a setting, and the operator can set it for each model. There is no one fixed time for all models. 15 minutes is the proposed first value of the setting.
- **FIT-023.** The operator can pin a model. A pin is the wish to keep the model loaded. It is not a reservation: an idle pinned model can give its memory to a request that needs it. The node must remember that the pinned model was loaded, and must load it again after the other work is done and the memory permits. The return must not make a loop of loads and must not hold back a request that waits.
- **FIT-024.** Each node must count its memory reservations: the loaded copies, the loads in progress and the active requests. A new load must not start if the reservations with it are more than the usable memory.
- **FIT-025.** The hub and the model address must keep these states of a copy apart: downloaded, loading, loaded and ready, and active.
- **FIT-026.** When a model cannot load now, the answer must say which case applies: the memory is in use at this time, or the model is too large for this runtime and this hardware. In the first case the caller selects the behaviour: wait until a deadline, or fail at once (RTR-031). In the second case the request is refused with the reason. A machine that the caller named is never replaced.

Design work that the contract of task L1-15 must settle before the build: the unit of a pin (model, variant or copy), the rule for the return of a pinned model and its fairness, the limit of the queue, the names of the fields of the wait policy and their default limits, and what a timeout and a cancellation clean up. They are design outputs. One of them is a decision of the operator only if it changes a requirement of this section.

## 6. The model address

Version 0.2 of this section: a router in the hub at `https://ai.anima.ing`, with a name on Headscale DNS, a DNS-01 certificate and API keys with a list of permitted models. R-131 replaced the router in the hub. C1 replaced the keys.

### 6.1 Address and access

- **RTR-001.** The model address must be a listener on the loopback address of each computer of the operator that runs the Anima program (R-131). The contract of task L1-10 says which program holds the listener (the desktop application or `anima`) and how the port is fixed. The daemon does not hold it, unless a ruling changes MET-020. A DNS name for the address is OD-C7. Version 0.2: the router answers at `https://ai.anima.ing`, a name of Headscale DNS for the VPN address of the hub.
- **RTR-002.** The hub must have no route that carries a model request or an answer (CMP-013). No public DNS record must point at a model address or at a compute node. Version 0.2: no public record of `ai.anima.ing` points at the hub.
- **RTR-003.** Withdrawn for the loopback address: it needs no certificate. If OD-C7 keeps a name, this requirement returns with that decision. Version 0.2: a certificate of a public authority through the ACME DNS-01 challenge, with the DNS token as a Docker secret.
- **RTR-004.** The model address must require the model key and must refuse a request without it. The first release has one shared key for all models and all callers. The operator can rotate the key and can revoke it. The contract of task L1-9 must define what a caller sees in the change from the old key to the new key, before the build. A key for each project, a list of permitted models and an expiry come later (Section 1.3). The model key is only the key of the model address: it does not replace the daemon token, and it permits no model work. Version 0.2: each key has a name, a list of permitted models and an optional expiry.
- **RTR-005.** The model address must accept connections only from its own computer. A compute node must accept a model request only as SEC-C-002 says. Version 0.2: the permitted-network rule of the hub (NET-001, NET-002) applies to the router.

### 6.2 OpenAI API

- **RTR-010.** The model address must supply `/v1/models` and `/v1/chat/completions`, with a streamed answer and with image input for a model with vision (CMP-007). `/v1/completions`, `/v1/embeddings` and `/v1/responses` are planning context: each one is added when a model that needs it is validated. Version 0.2: all five from the first build.
- **RTR-011.** `/v1/models` must list each model that has a validated copy (CMP-006) on a node with a permitting policy, and each alias. It lists a model once, not once for each copy.
- **RTR-012.** Each entry of `/v1/models` must also give the Anima fields: the capability tags, the context length, the variants, and for each copy the node, the runtime and the state (FIT-025).
- **RTR-013.** A request can name a model, a model with a tag, or an alias. If the request has no tag, the router can use any validated variant of the model.
- **RTR-014.** A request can name a node with the header `Anima-Node`. The router then uses only that node. If the node cannot serve the model, the request is refused with the reason. The router never uses a different node in its place.
- **RTR-015.** The model address can forward the Anthropic Messages API (`/v1/messages`) to a model server that supports it.
- **RTR-016.** A refusal must give a fixed reason: the model is not known, no copy is validated, the node is off or its policy stops it, the model does not fit (FIT-026), the memory is in use, or the key is wrong. A refusal must never look like an answer of a model.

### 6.3 Routing

- **RTR-030.** When the request names no node, the router must select from the validated copies of the model on healthy nodes with a permitting policy. The rule of the selection needs OD-C10: C1 and C2 say that no automatic selection of a machine is settled. Version 0.2: the router prefers a loaded copy.
- **RTR-031.** If no copy is loaded, a load starts (FIT-020). An ordinary call in the OpenAI form or the Ollama form waits for its normal result, within a deadline. The caller can select the behaviour when the memory is in use: wait until a deadline, or fail at once. A caller that asks for it gets the status of the load before the result (RTR-039). The model address must never put loading text into the output of a model, must not make stream parts that the protocol does not have, and must not send extra HTTP answers. Version 0.2: the caller waits until the load is done or until a time limit of a setting.
- **RTR-032.** The model address must stream responses without a buffer.
- **RTR-033.** The router can send requests with the same conversation key to the same server, to use its prompt cache.
- **RTR-034.** Planning context, not a requirement of the first release: score each loaded copy by the expected wait (queue depth divided by measured tokens per second) and pick the better of two random candidates. It waits for OD-C10 and for more than one node with the same model.
- **RTR-035.** If a node fails 3 requests in a row, the router must stop sending to it for 30 seconds (circuit breaker).
- **RTR-036.** The router can retry a request on a different server only before the first byte goes to the caller, and never when the caller named the node (RTR-014).
- **RTR-037.** The hub must keep the record of use (R-131): the time, the model, the node, the computer of the caller, the token counts and the duration of each request. The model address reports them to the hub. No prompt, no output and no image is in the record (LOG-007). The owner of this record is OD-AI1 of [ai-direction.md](../proposals/ai-direction.md). Version 0.2: the router in the hub records each request with its key.
- **RTR-039.** The status of a request for a caller that asks for it has stages such as queued, loading, ready, running and failed. The contract of task L1-15 gives the names and the changes between them, the timeout, the cancellation and what each one cleans up. The background mode of the OpenAI Responses API (`background=true`, retrieve, poll, cancel) is the first candidate for the form. The wait until a deadline, the fail at once and the pin are local additions of Anima, not fields of OpenAI or Ollama. A non-text job uses the contract of its family (CMP-007).

The router design takes ideas from the SGLang model gateway, the LMDeploy proxy and the LocalAI SmartRouter (Section 14). Anima writes its own router in Rust.

### 6.4 Ollama API

This section is phase C2. Many tools can use an Ollama server. The model address can also speak the Ollama API, so that these tools work without a change.

- **RTR-040.** The model address can supply these Ollama endpoints: `/api/tags`, `/api/ps`, `/api/show`, `/api/chat`, `/api/generate`, `/api/embed` and `/api/pull`.
- **RTR-041.** `/api/pull` must use the hub selection of node and variant (PULL-002), unless the request names a node.
- **RTR-042.** The Ollama endpoints must use the same model key as the OpenAI endpoints.

## 7. Model selection for a job

The operator or the caller selects the model for each job. The hub gives the facts that they need.

- **SEL-001.** The Models page must show, for each model: the capability tags, the context length, the size, the nodes that have a copy, the state of each copy and the measured tokens per second. The page is later delivery (Section 11).
- **SEL-002.** The operator can sort and filter the models by capability, size, speed and node.
- **SEL-003.** The operator can point an alias at a different model at any time. Tools that use the alias then use the new model without a change.
- **SEL-004.** Anima must not choose a model for a request. Automatic model choice is out of scope for this version.
- **SEL-005.** The command `anima model list` and its MCP tool must give the facts of SEL-001 before the page exists (CMD-030).

## 8. Chat page

The Chat page is later delivery (task L1-21). It is not a condition for the acceptance of C1 or C2.

- **CHAT-001.** The client must have a Chat page that sends requests to the model address of its computer, with the model key. Version 0.2: to `ai.anima.ing` with a key that the hub gives to the client.
- **CHAT-002.** The Chat page must have a model selector that shows the models and aliases of `/v1/models` with their state.
- **CHAT-003.** The operator can change the model in a conversation. Each response must show the model, the node and the tokens per second.
- **CHAT-004.** Responses must stream. The operator can stop a response.
- **CHAT-005.** The operator can set a system prompt and the sampling values (temperature, top-p, maximum tokens) for each conversation.
- **CHAT-006.** The client must keep conversations on the client device only until the vault exists (OD-C6).
- **CHAT-007.** Text must render as Markdown with code blocks. Motion must use the helpers in `$lib/motion/`.

## 9. Security

Most runtimes have no authentication by default. Some listen on all interfaces. Thus Anima must put each runtime behind its own controls.

- **SEC-C-001.** Each runtime and model server must listen on the loopback interface only. The daemon must check this after each start.
- **SEC-C-002.** A compute node must open no network port for a model request. A request from a different device must come through the private network of the hub (CMP-003) to a loopback port of the node, and only when the node is permitted (CMP-002). The node must refuse a request that the hub did not permit for that caller. Task L1-12 proves the path, and the contract of task L1-13 gives the proof that a caller sends. The pattern is the path of HMD-048 and HMD-049: the computer holds the list of what it opens, and the hub cannot change that list. Version 0.2: the daemon listens only on the VPN address of the node and accepts only requests with a token of the hub.
- **SEC-C-003.** The hub must keep the model key and the Hugging Face token in the key vault of [E0-6](../work-items/engine/E0-6.md), which owns their storage, their distribution and the life of a cached copy (C1). The daemon token stays where K8 puts it; the three are separate credentials. Version 0.2: the hub keeps the router API keys, the node tokens and the Hugging Face token with the credentials domain (SEC-003).
- **SEC-C-004.** A hub on the link needs no ACL change for Level 1. For a hub on the VPN, the ACLs must permit only the model traffic that Section 6 needs and the compute nodes to the Hugging Face hosts, and each ACL change needs the operator's confirmation (VPS-005); Level 1 builds this only if OD-C11 says so. In Level 2, each stage reaches only the next stage of its ring. Version 0.2: the ACLs permit the hub to each daemon port, with the tag `tag:compute`.
- **SEC-C-005.** In Level 2, `rpc-server` must listen only on the VPN or site address of the node. The ACL must permit only the host of its pool. The daemon must refuse a llama.cpp build that a published advisory covers, for example CVE-2026-34159 (builds up to b7991).
- **SEC-C-006.** In Level 2, the wire of each ring must use a key that the hub makes when it forms the ring.
- **SEC-C-007.** The hub owns these runtime flags: model path, host, port, API key, parallel slots and remote code. The daemon must refuse them from any other source. This follows GW-012 and GW-013.
- **SEC-C-008.** No runtime must load remote code from a model repository (`trust_remote_code` or the same in a different runtime).
- **SEC-C-009.** Model files come only from the approved repositories (LIBM-003), with SHA-256 verification (PULL-005). The GGUF parsers have known faults, so do not load a model file from another source.
- **SEC-C-010.** No log, metric or event must contain a prompt, an output, an image or a hidden state (LOG-007).
- **SEC-C-011.** Do not use a remote access function of a runtime, for example LM Studio LM Link or a Cloudflare tunnel. SEC-008 applies.
- **SEC-C-012.** Work on the model work of the daemon, the model key, the path of a request between devices, the ACLs and the ring keys has the security tag. The security reviewer must accept it (AGT-004).

SEC-C-005 and SEC-C-006 are Level 2 and are parked. Version 0.3 did not change them.

## 10. Level 2: pooled models

Parked (tier 4, row 4.12 of the work plan). Version 0.3 did not change this section. Its words for the network (the VPN, an ACL, a site address) are the words of version 0.2.

Level 2 starts only after Level 1 is in use (CMP-004). The research found no runtime that splits one model across different runtimes. Thus Level 2 needs a small framework of Anima's own. It is a thin layer over existing runtimes, not a new inference engine.

### 10.1 Why only a pipeline split

A pipeline split gives each stage a whole block of layers. Only the hidden state of each token goes between stages. Thus each stage can use a different runtime, if all runtimes agree on a block contract. A tensor split needs data exchange in each layer, a fast link and one runtime.

| Term | Meaning |
| --- | --- |
| Block | A contiguous range of layers of one model. |
| Stage | A compute node that runs one block. |
| Coordinator | The node that holds the token embedding, the output head and the draft model. It holds no block. |
| Ring | The ordered stages and the coordinator that serve one pooled model. |
| Block contract | The interface that each stage obeys (Section 10.3). |
| Block adapter | The code that runs a block for one runtime behind the block contract. |

### 10.2 Single-site pool (llama.cpp RPC)

llama.cpp has one runtime with many backends. Its RPC mode sends blocks to `rpc-server` processes on other nodes. Thus a Mac, an NVIDIA PC and an AMD PC at one site can serve one GGUF model together. This needs no framework of Anima's own.

- **RNG-001.** The hub can form an RPC pool only from nodes at one site.
- **RNG-002.** The placement must split the layers in proportion to the usable memory of each node.
- **RNG-003.** Each `rpc-server` must obey SEC-C-005.
- **RNG-004.** The hub must show the RPC pool as experimental. The llama.cpp project describes RPC as a proof of concept.
- **RNG-005.** A pooled model must show in `/v1/models` like any other model.

### 10.3 Cross-runtime ring (the Anima framework)

A ring across sites and runtimes uses the Shard design. The coordinator runs a small draft model that proposes several tokens. The ring checks all of them in one pass. Thus one round trip commits several tokens, and the WAN delay matters less.

The framework has four parts:

| Part | Content | Source |
| --- | --- | --- |
| Block contract | `load(model, lo, hi)`, `reset()`, `forward(hidden, start_pos)`, `crop(pos)`, and the byte layout of hidden states | The Shard `ModelRuntime` interface |
| `MlxBlock` | The block adapter for Macs. MLX-LM models keep their layers in a Python list, and the KV cache has `trim()`. | New, easy to moderate |
| `TransformersBlock` | The block adapter for CUDA, ROCm and CPU. Models accept `inputs_embeds`, and `DynamicCache.crop()` exists. | New, moderate |
| Coordinator and transport | Speculative decoding, the ring, recovery from a failed stage | The Shard project, used as a dependency |

The other runtimes keep the KV cache inside their scheduler and have no layer-range interface. An adapter for them needs a fork. Do not build one.

- **RNG-010.** Each block adapter must obey the block contract. A contract test must run against each adapter.
- **RNG-011.** Hidden states must go between stages as fp16 or bf16, in the byte layout of the block contract.
- **RNG-012.** A block adapter must refuse a model with a layer type whose KV cache it cannot crop.
- **RNG-013.** The draft model must use the same tokenizer as the ring model.
- **RNG-014.** Anima must not write a second speculative decoding engine. It uses the Shard coordinator.
- **RNG-015.** Start with dense models. Add a mixture-of-experts model only after a dense model passes Section 10.5.

### 10.4 Ring planning

- **RNG-020.** The ring planner must be a pure function. Its inputs are the model shape and, for each node, the usable memory, the time for one layer and the site. It also uses the round-trip times.
- **RNG-021.** The planner must use as few stages as possible. A node with more memory comes first, because each stage adds a network hop.
- **RNG-022.** The planner must size each block by the measured time for one layer, within the memory limit of the node.
- **RNG-023.** The planner must order the stages to make the total ring round-trip time as small as possible.
- **RNG-024.** The planner must put the coordinator at the site of most stages.
- **RNG-025.** A node that joins does not change a running ring. If a stage fails, the hub replaces it with a spare node and continues from the committed tokens.

### 10.5 Numerical conformance

Stages with different runtimes and quantised files give slightly different numbers. Thus each ring must pass a conformance test before use.

- **RNG-030.** Block test: the same hidden state goes into the same block on each runtime. The outputs must agree within a tolerance.
- **RNG-031.** Ring test: a fixed prompt set runs on the ring and on one reference node with greedy decoding. Token agreement and perplexity must be within limits.
- **RNG-032.** Phase C4 sets the tolerance and the limits from measurements (OD-C3).
- **RNG-033.** The hub must refuse a ring that fails the test.

## 11. Client pages

The Models, Chat and Compute pages are later delivery (C1 and C2: "UI later"). They are the last step of CMD-030 for each function, after the hub API, the command and the MCP tool. They are not a condition for the acceptance of C1 or C2 (Section 12.3). Their tasks are L1-19 to L1-21.

- **CUI-001.** The Models page must show the library (SEL-001), the pull controls (Section 4.2), the pull progress and the aliases.
- **CUI-002.** The Chat page must obey Section 8.
- **CUI-003.** The Compute page must show each compute node with its backends, its verdicts (CAP-002), its runtimes, its loaded copies and its policy.
- **CUI-004.** The Compute page must show that the model key exists and when it was changed. The operator can rotate it and revoke it. The client shows a new key one time only. Version 0.2: the page shows the API keys of the router, and the operator makes and revokes keys.
- **CUI-005.** The device card must show a compute badge when the device is a compute node.
- **CUI-006.** If the hub cannot place or pull a model, the client must show the reason (PULL-003).

### 11.1 Tool inventory in the client

Approved for build (operator, 2026-11-16: "we should build this as a UI piece"; ruling number to be assigned). This is step 4 of CMD-030 for the tool inventory (CAP-020 to CAP-031). The hub function, the command and the MCP tool are on `main`. This section serves row 2b of Section 4 of the work plan. It needs no change to the hub and no change to a contract: it reads `Query.capabilities` (K1).

- **CUI-010.** The Overview tab of the Device page must show a Tools section after Storage (MET-029).
- **CUI-011.** On top, the section must show one summary line as text: the number of usable tools, the number of build targets, the architecture and the operating system version. Example: "12 tools · 3 build targets · aarch64 · macOS 27.0".
- **CUI-012.** Then the section must show one row for each tool, in the groups of CAP-023 and in their order. A row gives the name of the tool and its version in the monospace font.
- **CUI-013.** A tool that is not usable must show "Not usable", the reason as text and the repair hint (CAP-024). The client must not show this state with colour alone. The hint text must be the same as the text of `anima capability list`.
- **CUI-014.** After the tools, the section must show the build targets as a list in the monospace font. A device with no build target shows no list.
- **CUI-015.** The section must show when the daemon found the inventory, as a relative time with the full time in a tooltip.
- **CUI-016.** The loading and failed states must follow the Metrics section. The placeholder must be as tall as the loaded section of a typical device, so that nothing below moves (MOT-007).
- **CUI-017.** The section must give the reason when it has no inventory. For a device with no daemon: "The daemon of this device reports its tools", with the Install daemon action of MET-019. For a device whose daemon sent no inventory: "This daemon does not report tools. Update the daemon."
- **CUI-018.** The device list must have a Tools filter in its toolbar (DEV-008), after the tag filter. The operator selects one tool or types one build target. The list then shows only the devices that have it usable (`capabilities(tool, buildTarget)`). The filter must be in the address of the page, as the other filters are.
- **CUI-019.** Text in a tool inventory comes from a device (CAP-031). The client must show it as text only, never as markup, and must cut a value that is longer than its column with the full value in a tooltip.

Rules for the build:

- The logic is in `$lib/metrics/tools/`, beside `$lib/metrics/storage/`: the GraphQL document in the client of the metrics domain, a pure function from the query result to the rows and the summary line, and the section component. The route only composes.
- The values come from the design tokens and the motion helpers (design specification Sections 5.5 and 5.7). No new token is necessary.
- The fixture for the mock hub (CON-003) must hold: a Mac with all tools, a Linux device with one tool that is not usable for each reason, a device with a daemon and no inventory, and a device with no daemon.
- The tests must test what the operator sees: the summary line, the groups, each reason with its hint, each empty state, and the filter in the address.
- The interface reviewer must measure the section in WebKit and Chromium: tokens, layout shift, reduced motion and the accessible names.
- The builder must add an entry to [need-human-testing.md](../need-human-testing.md) for the section on a real hub, in the same commit as the section.

| Task | Content | State | Commits |
| --- | --- | --- | --- |
| T1 | The fixture, the GraphQL document and the pure function for rows and summary (CUI-011 to CUI-015, CUI-019) | Not started | — |
| T2 | The Tools section on the Overview tab, with its states (CUI-010, CUI-016, CUI-017) | Not started | — |
| T3 | The Tools filter of the device list (CUI-018) | Not started | — |
| T4 | The interface review, the corrections and the entry in the human testing list | Not started | — |

## 12. Delivery

### 12.1 Phases

- **PH-C-000.** Compute work must not start before Phase 1 of the spec is accepted. Phase 1 is accepted ([handoff.md](phase-3/handoff.md)). Level 1 uses the hardware readers of the daemon of Phase 3 (CMP-005). Version 0.2: "the daemon gives its own hardware values, so Level 1 does not need the Phase 2 metrics".
- **PH-C-001.** Level 2 (C4 and later) must not start before C2 is accepted and the operator uses the model service.
- **PH-C-002.** No task of Section 12.4 starts before a ruling approves Level 1 (PLN-007). The functions for AI agents and their use are built before the model service (R-132).
- **PH-C-003.** Each function of Level 1 is built in the order of CMD-030: the hub API, the `anima` command, the MCP tool, then the client. A page is later delivery.

| Phase | Level | Content | Depends on |
| --- | --- | --- | --- |
| C1 | 1 | The foundation: the hardware values, the fit estimate, the model work of the daemon, the library, the pull, the validation, the model key, the model address with the OpenAI form, image input, and a request to a different node. No page. | Spec Phase 1 (accepted), the approval of Level 1, OD-C1, OD-C8, OD-C9, OD-C10; for the request to a different node also the spike L1-12 and OD-C11 |
| C2 | 1 | Loading on demand, the idle unload as a setting, pins, reservations, the wait policy and the status of a request, the Ollama API. No page. | C1 |
| Pages | 1 | The Compute page, the Models page and the Chat page | The function of each page; OD-C6 for the Chat page |
| C3 | 1 | Single-node offload (Section 5.2). KTransformers on a large-memory node, if the operator has one. | C1 |
| C4 | 2 | Spikes and a decision record (Section 12.2). Parked. | C2 |
| C5 | 2 | Single-site pool with llama.cpp RPC (Section 10.2). Parked. | C4 go |
| C6 | 2 | Cross-runtime ring: block contract, adapters, ring planner, conformance, recovery. Parked. | C4 go |
| C7 | — | Jobs: fine-tuning with the Unsloth core library on the best node | C1 |

Version 0.2 of the rows C1 and C2: C1 held the compute daemon, the router at `ai.anima.ing` with API keys, and the Compute page; C2 held the Models page and the Chat page, and depended on OD-C7.

### 12.2 Phase C4 spikes

1. **Cross-runtime block test.** Use one dense model of 7B to 8B parameters. Put the first layers on a Mac with an MLX block and the other layers on a CUDA PC with a Transformers block. Drive them with the Shard coordinator. Compare the tokens and the perplexity with the same model on one node.
2. **Cross-site speed.** Run the same ring with the two nodes at different sites. Measure tokens per second with and without speculative decoding.
3. **RPC pool.** Run one GGUF model across two nodes at one site with llama.cpp RPC. Compare it with the best single node with offload.

- **PH-C-010.** The decision record must give the measured numbers, the conformance tolerances (RNG-032) and a go or no-go for C5 and C6.

### 12.3 Acceptance criteria

The rows C1 and C2 are the acceptance text of the work items of 2026-11-07, in the terms of this version. A page is not necessary for them. The rows C3 to C7 did not change.

| Phase | The phase is done when |
| --- | --- |
| C1 | The hub shows the decision if a machine can take a model (permitted, fits, compatible) apart from the metrics that it reuses. A Mac and a Linux PC each have a validated model. A pull that is cut continues and is verified. A runtime that stops is started again and the hub shows the event. `curl http://127.0.0.1:<port>/v1/models` with the model key, on a computer of the operator, lists both models with their nodes. A chat request to each model, with the node named and with no node named, streams a response; a request for a node that cannot serve the model is refused with the reason. A request without the key is refused. The one key works for all validated models. After a rotation the old key is refused as the contract says. A request with an image gets an answer from a model with vision. A test proves that each runtime listens on loopback only. A test proves that no prompt, output or image is in a log. |
| C2 | The idle time is a setting and a model can be pinned. An idle pinned model gives its memory to a larger request, its files stay, a copy with active work is never unloaded, and the pinned model loads again after the other work. With requests and loads at the same time, the reservations are never more than the usable memory and no other job is stopped. A caller that selected "wait until a deadline" waits, and a caller that selected "fail at once" gets the refusal; a model that is too large for the node gets a different, clear refusal. An ordinary call waits for a real result; a caller that asks for the status sees the load and the readiness; a timeout, a cancellation and a failure leave no reservation. No text that the model did not make is in an output. A return of a pinned model makes no loop of loads and holds back no request. An Ollama client lists and uses the models. An alias change takes effect for the next request. |
| C3 | A model 20 % larger than the GPU memory of a node runs on that node with offload. |
| C4 | The decision record exists (PH-C-010). |
| C5 | A model larger than any single node at one site runs across two nodes of different backends. The ACL refuses a connection to `rpc-server` from a node outside the pool. |
| C6 | A model runs on a ring with a Mac stage and a CUDA stage at different sites and passes the conformance test. When a stage stops in a request, the request completes on a spare node. |
| C7 | A fine-tune job runs on the node that the placement selects and returns an adapter file to the operator. |

Version 0.2 of the rows C1 and C2: C1 named `curl https://ai.anima.ing/v1/models` with a key; C2 named the Chat page, a fixed idle time and an unload after it.

### 12.4 Tasks of Level 1

No task is started. No task starts before a ruling approves Level 1 (PH-C-002). A session takes one row (PLN-022). Each row that adds a hub function builds its command and its MCP tool in the same task (CMD-030), unless the row says why not. A function that the gates cannot prove gets its entry in [need-human-testing.md](../need-human-testing.md) in the commit that builds it (PLN-014).

L1-1, L1-2 and L1-4 need no open decision: after the approval of Level 1 they can start at once. L1-1 is the first task. It only reads, it needs no work route and no path between devices, and what it reports is the inspected hardware list that OD-C1 asks for.

| Task | Content | Requirements | Needs | Check | State | Commits |
| --- | --- | --- | --- | --- | --- | --- |
| L1-1 | The hardware values of a machine: the daemon reads the memory values, the usable memory of Apple silicon, each backend with its verdict, the installed runtimes and the power state, and sends them as an optional member of the push batch of K8. The hub gives them in `capabilities`; `anima capability list` and `anima_capability_list` show them. | CAP-001 to CAP-005, CAP-032, CMP-005 | The approval of Level 1 (it permits the optional member of K8, CON-001). No open decision. | Tests with scripted readers for a Mac, an NVIDIA PC and a machine with no GPU; the mock hub has the fixture. A human check: the values of the Mac and of the Linux PC of the operator agree with what each system shows. | Not started | — |
| L1-2 | The fit estimate as a pure function, with its verdict and reason. | FIT-001 to FIT-005, CMP-011 | No open decision. | Unit tests against a table of known models and memory sizes, with the largest context that fits. | Not started | — |
| L1-3 | The contract of model work, `contracts/compute/model-work.md`: the work types, how the daemon asks for work and reports, the states of a copy, the report of the runtimes. It holds no secret. It uses the work route of task W1 of the builds plan if that contract exists, and says so if it does not. | CAG-002 to CAG-004, CAG-006, CAG-009, PULL-011 | OD-C9 | The security reviewer accepts it, and a ruling permits the new contract (CON-001). | Not started | — |
| L1-4 | The library in the hub: models, variants, the approved sources, the capability tags, aliases; the read of the file list and the metadata from a repository. Commands `anima model add`, `list`, `show`, `alias`; the MCP tools for list and show. | LIBM-001 to LIBM-009, SEL-003, SEL-005 | No open decision. | Tests against recorded answers of a repository: the metadata, the SHA-256 values, a source that is not approved is refused. | Not started | — |
| L1-5 | The device policy: the switch, the memory share, the disk share, quiet hours, mains power, the active operator, the drain. Hub API, command, MCP tool. | CMP-002, CAP-010 to CAP-013 | OD-C8 | Tests: a machine that is off gets no model work; a policy that stops a node drains it in 60 seconds or fewer. | Not started | — |
| L1-6 | The model work of the daemon: it asks for work, validates it, installs a runtime from a fixed source, starts and stops a model server on loopback, supervises it with backoff, and reports. The logic is in `daemon-core`. | CAG-001 to CAG-009, RTC-002 to RTC-006, SEC-C-001, SEC-C-007, SEC-C-008 | L1-3, OD-C9, OD-C1 (the first runtime of each machine) | Tests with a scripted hub and a scripted runtime: work that is not in the list is refused; a runtime that stops is started again; a test proves the loopback listener. A human check on the Mac and on the Linux PC. | Not started | — |
| L1-7 | The pull: the hub selects or takes the variant, the daemon downloads from the source, verifies, continues after a cut, keeps the disk share; delete a copy; the progress. Commands `anima model pull` and `anima model remove`; MCP tools. | PULL-001 to PULL-011, SEC-C-009 | L1-4, L1-6; OD-C10 for the variant that the hub selects | Tests with a local file server: a cut pull continues, a wrong SHA-256 value is refused, a full disk gives the reason. | Not started | — |
| L1-8 | The validation of a combination and the runtime choice: the daemon loads a copy, a fixed request gets an answer, the hub records the result for the node; the list of combinations. Command `anima model validate`; MCP tool. | CMP-006, RTC-001, RTC-007, PULL-011 | L1-7, OD-C1 | A human check: the Mac and the Linux PC each have one validated text model. | Not started | — |
| L1-9 | The model key: the hub makes it, keeps it in the key vault, gives it to a signed-in computer; rotate and revoke, with the change from the old key to the new key defined in the contract first. Command `anima model key`. No MCP tool: a tool must not return a secret (Section 7 of the command-line plan). | RTR-004, SEC-C-003 | The key vault of E0-6 on `main` (its contract K-E5) | Tests: the old key is refused after the time of the contract; no key is in a log or an error. | Not started | — |
| L1-10 | The contract of the model address, `contracts/compute/model-request.md`: the listener and its port, the Anima fields of `/v1/models`, the header `Anima-Node`, the reasons of a refusal, the report of use, the rule for the choice of a node. | RTR-001, RTR-002, RTR-005, RTR-010 to RTR-016, RTR-030, RTR-037, CMP-013 | OD-C10, OD-C7, and OD-AI1 of [ai-direction.md](../proposals/ai-direction.md) | A ruling permits the new contract (CON-001). | Not started | — |
| L1-11 | The model address on one computer: it lists the models from the hub, sends a request to a model server on the same computer, streams the answer, requires the model key, and reports the use to the hub. Hub API for the list, the node and the use; `anima model use`; MCP tool for the use. | RTR-001 to RTR-016, RTR-032, RTR-035 to RTR-037, CMP-013 | L1-8, L1-9, L1-10 | Tests with a scripted model server: a stream arrives with no buffer, a request with no key is refused, no prompt is in a log. A human check with `curl` on the Mac. This is also the server on the same device of BOT-049. | Not started | — |
| L1-12 | The spike: a connection from one device to a different device through the link, for a model request. A record in `research/compute/` with the measured delay, the direct and the relayed case, the failure cases and what the link program must change. It adds no requirement; the owner of the link ([one-click-plan.md](one-click/one-click-plan.md)) gets the requirement after the record. | SEC-C-002, CMP-003 (it proves that they can be built) | No open decision. It needs two machines on one hub. | The record exists, with a go or a no-go. With a no-go, the operator must decide the path again (R-131 names the spike as its condition). | Not started | — |
| L1-13 | A request to a different node: the contract of the proof that a caller sends, the list on the node of what it opens, and the path of L1-12 in the link program and the model address. | SEC-C-002, RTR-005, RTR-014, CMP-003 | L1-11, L1-12 with a go, OD-C11, and the requirement of the link in its owner | The acceptance of C1: a request from the Mac to the Linux PC and back streams; a device of a different hub is refused. A human check. | Not started | — |
| L1-14 | Image input for a model with vision, end to end, for the extraction of the engine. | CMP-007, LIBM-010, RTR-010, SEC-C-010 | L1-11 | A request with an image gets an answer; a test proves that no image is in a log. | Not started | — |
| L1-15 | The contract of loading and readiness: the states of a copy, the wait policy and its fields, the status of a request, the pin and its return, the queue limit, the timeout and the cancellation. An addition to `model-request.md` and `model-work.md`. | FIT-020 to FIT-026, RTR-031, RTR-039 | L1-10 | A ruling permits the contract change (CON-001). | Not started | — |
| L1-16 | Loading and memory on a node: load on demand, the reservations, the unload of idle copies, the idle time as a setting, the pin and its return. Command `anima model pin` and the setting; MCP tools. | FIT-020 to FIT-026 | L1-15, L1-6 | Tests with a scripted runtime and a test clock: the first five sentences of the acceptance of C2. | Not started | — |
| L1-17 | The wait policy and the status of a request at the model address. | RTR-031, RTR-039, RTR-016 | L1-15, L1-11 | Tests: wait, fail at once, too large, a timeout and a cancellation that leave no reservation; no output text that the model did not make. | Not started | — |
| L1-18 | The Ollama form at the model address. | RTR-040 to RTR-042 | L1-11; L1-7 for `/api/pull` | A human check: an Ollama client lists and uses the models. | Not started | — |
| L1-19 | The Compute page and the compute badge. Later delivery. | CUI-003 to CUI-006 | L1-5, L1-8, L1-9 | The interface review and a human check on a real hub. | Not started | — |
| L1-20 | The Models page. Later delivery. | CUI-001, SEL-001 to SEL-003, PULL-007 | L1-7, L1-16 | The interface review and a human check on a real hub. | Not started | — |
| L1-21 | The Chat page. Later delivery. | CUI-002, CHAT-001 to CHAT-007 | L1-11, OD-C6 | The interface review and a human check on a real hub. | Not started | — |

## 13. Repository changes

Version 0.2 named `crates/compute-daemon/`, `crates/compute-daemon-core/`, a `router/` folder in the hub, `contracts/compute/daemon-protocol.md` and `router-api.md`, and `deploy/headscale/` with `tag:compute` and a DNS record. None of them exists on `main`. The daemon is `crates/daemon` and `crates/daemon-core`; the tool inventory is `crates/daemon-core/src/inventory/` and `crates/hub/src/fleet/metrics/capabilities.rs`.

```text
crates/
├── hub/src/fleet/compute/
│   ├── capabilities/   Hardware values, verdicts, device policy
│   ├── library/        Models, variants, aliases, approved sources, capability tags
│   ├── placement/      Runtime choice, fit estimate, offload ranking, ring planner (pure)
│   ├── copies/         Copies and their states, validated combinations, pulls, model work
│   ├── use/            The record of use (OD-AI1 decides its owner)
│   └── rings/          Level 2: rings and conformance results
├── daemon/             anima-daemon: the model work at the I/O edge (runtimes, pulls)
├── daemon-core/        The logic of the model work with no I/O (CAG-007)
└── anima-cli/src/models/   The `anima model` commands and their MCP tools
contracts/compute/
├── model-work.md       The model work types and the reports of the daemon (L1-3)
├── model-request.md    The model address, the Anima fields of /v1/models, the Anima-Node header (L1-10)
└── block-contract.md   Level 2: block operations and the hidden-state byte layout
blocks/                 Level 2: MlxBlock and TransformersBlock (Python)
apps/client/src/routes/ models/, chat/, compute/ (later delivery)
link/                   The path from one device to a different device, after the spike L1-12
```

The program that holds the model address (RTR-001) gets its folder in the contract of task L1-10.

- **REPO-C-001.** The compute domain is under `fleet/` (REPO-E-001).
- **REPO-C-002.** The Python block adapters need their own gates: ruff, a strict type check and pytest. Add them to the gates in AGENTS.md before C6 starts.
- **REPO-C-003.** If the Shard project accepts the block adapters, keep them in Shard and remove `blocks/` from Anima (OD-C4).

## 14. Runtime research summary

Research date: 2026-11-06. Each runtime was checked against its repository and documentation.

| Runtime | Licence | Backends | Cross-machine split | Block adapter | Role in Anima |
| --- | --- | --- | --- | --- | --- |
| llama.cpp | MIT | CUDA, HIP, Metal, Vulkan, SYCL, CPU | RPC, one site, no authentication | Hard (fork) | Default model server; Level 2 single-site pool |
| MLX-LM | MIT | Metal; CUDA (new); CPU | MLX distributed, Mac only | Easy to moderate | Mac model server; `MlxBlock` |
| vLLM | Apache 2.0 | CUDA, ROCm, XPU, CPU; Metal by plug-in | Yes, same runtime, fast link | Hard (fork) | NVIDIA model server when the model fits |
| Transformers | Apache 2.0 | CUDA, ROCm, MPS, XPU, CPU | torch.distributed, fast link only | Moderate | `TransformersBlock` |
| KTransformers | Apache 2.0 | CUDA with CPU experts | No | Not practical | Large MoE on one large-memory node |
| ExLlamaV3 | MIT (TabbyAPI AGPL) | CUDA; ROCm (preliminary) | No | Hard (fork) | Optional NVIDIA model server |
| SGLang | Apache 2.0 | CUDA, ROCm, XPU, CPU; MLX (immature) | Yes, same runtime, fast link | Hard (fork) | Router design reference |
| LMDeploy | Apache 2.0 | CUDA, ROCm | Tensor split over Ray | Not practical | Router design reference |
| LocalAI | MIT | All, through other runtimes | Wraps llama.cpp RPC; own P2P overlay | Not possible | SmartRouter design reference |
| Jan | Apache 2.0 | Through llama.cpp and MLX | No | Not possible | Daemon supervision reference |
| TensorRT-LLM | Apache 2.0 | CUDA (GeForce best-effort) | Yes, data-centre links | Not practical | Not used |
| LM Studio | Closed | Through llama.cpp and MLX | No | Not possible | Not used (terms forbid change and service use) |

Findings:

1. For Level 1, three runtimes cover all hardware: MLX-LM on Macs, vLLM on NVIDIA when the model fits, and llama.cpp everywhere else.
2. No runtime splits one model across different runtimes. Each cross-machine mode needs one runtime on all nodes and a fast link.
3. llama.cpp RPC is the only ready split across different backends. It works at one site only.
4. A split across runtimes and sites needs the Anima framework of Section 10.3.

## 15. Open decisions

Each open row is one question that the operator can answer in one line. The last cell is the recommendation, or the result with its ruling (PLN-021, PLN-023). The rows are in the order of their numbers; the order of what they block is: OD-C9, OD-C1, OD-C8, OD-C10, OD-C7, OD-C11, OD-C6, then OD-C3 and OD-C4 (Level 2, parked). OD-AI1 of [ai-direction.md](../proposals/ai-direction.md) also blocks task L1-10; that document owns it.

| Number | Question | Options | Blocks | Recommendation |
| --- | --- | --- | --- | --- |
| OD-C1 | What is the hardware of each compute node? | The operator fills in the table of Section 15.1. Or: the operator approves task L1-1 first, the daemon reports the values, and the operator confirms the table that the hub shows. | L1-6, L1-8 (the first runtime and the first model of each machine). It does not block L1-1 to L1-5. | Build L1-1 first, then confirm the table from what the daemon reports. Until then, fill in only the site and "always on" columns, which no reader can give. |
| OD-C2 | One daemon binary or two? | — | Nothing | Decided for this plan (R-084, operator, 2026-11-07; the name by R-126): one daemon on each machine gives the metrics and later does the compute work. Thus CAG-001 makes no second binary. The other half of the old row, that `anima-daemon` stays separate from `anima-collector`, is not a decision of this plan: the collector is OD-E2 of the [extensions plan](extensions-plan.md). What the daemon is permitted to do for a model is OD-C9. |
| OD-C3 | Level 2, parked: which tolerances does the conformance test use? | No answer is necessary before phase C4. | C6 only | Set them from the C4 measurements (RNG-032). |
| OD-C4 | Level 2, parked: are the block adapters offered to the Shard project first? | Yes, and `blocks/` in Anima only if Shard does not accept them. Or: always in `blocks/`. | C6 only | Yes: offer them to the Shard project first. Keep them in `blocks/` only if Shard does not accept them. |
| OD-C5 | Where is the router? | — | Nothing | Decided (R-131, operator, 2026-11-17: "I like B"). A model request goes directly from the computer of the caller to the compute node. The Anima program on the computer of the caller listens on the loopback address and asks the hub which node has the model. The hub keeps the list of models and the record of use. It does not carry the request or the answer. This is the path of OD-E11 of the extensions plan. It needs a path from one device to a different device through the link. That path is not built and not verified: a spike must prove it before the plan depends on it (task L1-12). Version 0.3 changed Sections 2, 6 and 9 to this design. The recommendation before the ruling was: the router in the hub on the VPS (OD-9), with a second router at the home site if the delay is too high. |
| OD-C6 | Until the vault exists, are the conversations of the Chat page kept only on the client device? | Yes. Or: no Chat page before the vault. | L1-21 only | Yes: on the client device until the vault exists (extensions plan, Section 2.4). Then keep them sealed on the hub, so that all clients see them. |
| OD-C7 | Is the model address only `http://127.0.0.1:<port>` on each computer, with no name `ai.anima.ing`, no DNS record and no certificate? | (a) Yes, the loopback address only. (b) Also a name `ai.anima.ing`: then say what it resolves to; a name for a loopback address needs a public DNS record and gives no certificate that a client trusts without more work. | L1-10 (the contract names the address) | Yes, (a): the loopback address only. R-131 put the listener on each computer, so the name has no server to point at, and RTR-003 and the DNS token of KEYS-C are then not necessary. The old recommendation was: VPN only, with Headscale DNS and a DNS-01 certificate; the operator now uses the hosted hub and the link (R-130). The name can return later as a line of the documents and of `anima gateway setup`. |
| OD-C8 | Does a machine serve models by the one switch "takes work" (USE-004), with no second switch for compute? | (a) One switch. A machine that takes work serves the models that the operator pulled to it, within its device policy; a machine with no model serves nothing. (b) A second switch for compute, off by default on a client computer (the text of version 0.2). | L1-5 | Yes, (a): one switch. The work plan records OD-W10 (operator, 2026-11-12: "it should be on by default") and says that it changes CMP-002 and this row; it has no ruling number yet. R-153 already uses the same switch for the install of an agent (BOT-065). A pull is itself the act of the operator for each machine. |
| OD-C9 | Can `anima-daemon` do model work on its machine (install a runtime, pull a model, start and stop a model server) as a fixed list of work types that it asks the hub for? | (a) Yes, now, as narrow work types of their own, which the work route of task W1 takes over later (the pattern of `agent_install`, R-153). (b) Yes, but only after the work route of W1 is built. (c) No: the operator installs each runtime by hand, and the daemon only reports. | L1-3, L1-6, L1-7 and each task after them: most of Level 1 | Yes, (a): now, as narrow work types. It changes MET-002 ("the daemon must only read") and it is the command channel that R-115 did not take, so it needs a ruling. The daemon still listens on no port (MET-020) and takes only work of the fixed list (CAG-003, CAG-004). |
| OD-C10 | When a request names no machine, or a pull names no variant, can Anima select one by a fixed rule that it shows? | (a) Yes. Variant: the highest quality that fits (PULL-002). Machine: a node with a loaded copy first, else the node with the most free memory; no scores (RTR-034 waits). (b) No: the request is refused with the list of the choices, and the caller names the machine and the variant. | L1-10; the automatic part of L1-7 | Yes, (a): a fixed rule that Anima shows. A program in the OpenAI form cannot name a machine, so with (b) each such program needs the header `Anima-Node`. Anima still never selects the model (SEL-004) and never replaces a machine that the caller named (RTR-014). C1 and C2 say that no automatic selection is settled, so this needs the word of the operator. |
| OD-C11 | Does Level 1 build the path between devices for the link only (the hosted hub), and not for a hub on the VPN? | (a) The link only. (b) The link and the VPN (ACL rules with `tag:compute`, each change with VPS-005). | L1-13 | Yes, (a): the link only. The operator said that he will use only the hosted hub (R-130; no code follows from that sentence, so this row asks). A hub on the VPN can get its task later; SEC-C-004 keeps its rule. |

### 15.1 The hardware table of OD-C1

The operator fills in each cell that has a question mark and corrects each cell that is wrong. A value with a source is what the documents record; none of them was read from a machine. No output of `anima capability list` is recorded in the documents (its checks HT-002 to HT-006 are open), so this table has no inspected value.

| Machine | Chip or GPU | Memory | GPU memory | Operating system | Site | Always on? | Source |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Mac (M4); its name in Anima: ? | Apple M4; the variant (M4, Pro, Max): ? | 32 GB (unified) | The same memory; the usable share: ? | macOS: ? | ? | ? | [C1](../work-items/compute/C1.md): reported by the operator on 2026-11-07, not inspected |
| Mac (M5), "this Mac" of the session of 2026-11-07; its name in Anima: ? | Apple M5; the variant: ? | 48 GB (unified) | The same memory; the usable share: ? | macOS: ? | ? | ? | [C1](../work-items/compute/C1.md): reported, not inspected |
| `pop-os` (desktop) | NVIDIA A6000, the exact model: ? (RTX A6000 or RTX 6000 Ada: C1 says not to assume one). Processor: Intel i9, the model: ? | System memory: ? | ? (C1: do not assume a value) | Pop!_OS 24.04, x86_64 | ? | ? | [C1](../work-items/compute/C1.md): reported, not inspected. The system and the host name: [operator-changes.md](phase-3/operator-changes.md), 2026-11-07. The architecture: [g1-supervisors.md](../research/watchdog/g1-supervisors.md) |
| A different machine that must serve models: ? | ? | ? | ? | ? | ? | ? | — |

The hosted hub is not in this table: the hub never runs a model (CMP-010). A rented computer or GPU is in [cloud-machines.md](../proposals/cloud-machines.md), not here.
