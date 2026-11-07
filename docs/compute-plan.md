# Anima Compute Plan

| Item | Value |
| --- | --- |
| Document | Compute plan |
| Version | 0.2 (draft) |
| Date | 2026-11-06 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | For review |
| Related | [design-spec.md](design-spec.md), [extensions-plan.md](extensions-plan.md), [implementation-plan.md](implementation-plan.md) |

This document obeys the writing rules in Section 0 of the design specification.

## 1. Purpose and scope

### 1.1 Purpose

This plan lets Anima use the AI compute of the devices of the operator. It has two levels.

**Level 1: the model service (first priority).** Each compute node runs the runtime that suits its hardware best. The operator pulls models to each node, as with Ollama or Unsloth. One OpenAI-compatible endpoint, `ai.anima.ing`, lists all models on all nodes and answers requests from them. A Chat page in the client uses the same endpoint.

**Level 2: pooled models (later).** A model that is too large for one node runs across several nodes, one block of layers on each node. This needs a small framework of Anima's own (Section 10.3).

The hub decides where work runs. The compute nodes do the work. The hub never runs a model.

- **CMP-001.** Build only for the devices and models that the operator uses (EXT-000). Do not build a general model catalogue.
- **CMP-002.** Each device takes compute work only when the operator sets compute on for that device.
- **CMP-003.** Compute work must stay on the private network (Section 4.1 of the spec). Anima must not join a public compute network, for example c0mpute.
- **CMP-004.** Level 1 must be complete and in use before Level 2 starts (PH-C-001).

### 1.2 Sources

This plan uses ideas from these projects. It copies no code from a project with an AGPL licence.

| Project | Licence | Idea that this plan uses |
| --- | --- | --- |
| [Ollama](https://github.com/ollama/ollama) | MIT | Pull a model by name and tag, list models, load on demand, unload when idle, the Ollama API |
| [unslothai/unsloth](https://github.com/unslothai/unsloth) | Apache 2.0 (core), AGPL 3.0 (`studio/`) | Hardware verdicts with a reason, memory fit estimate, offload planner, managed server flags, quantised GGUF and MLX model files |
| [leyten/shard](https://github.com/leyten/shard) | Apache 2.0 | Level 2: pipeline split over a WAN, speculative decoding, the `ModelRuntime` block contract |
| The 12 runtimes in Section 14 | Different | Runtimes for each hardware type, router designs, the single-site split (llama.cpp RPC) |

### 1.3 Out of scope

- Access to `ai.anima.ing` from outside the VPN (SEC-008).
- Tensor parallelism across machines. It needs a fast link and one runtime on each side.
- Training across machines.
- Windows devices (Section 1.4 of the spec).

### 1.4 Changes to the design specification

Operator checkpoint 3 of the implementation plan applies. The operator must approve each change.

| Spec part | Change |
| --- | --- |
| 1.1 Purpose | Add a function: a private model service on the devices of the operator. |
| 3.1 Overview | Add the compute agent on compute nodes (Section 3) and the router on the hub (Section 6). |
| 4.1 NET-005 | Add the DNS name `ai.anima.ing` for the router. It resolves only on the VPN (Section 6.1). |
| 4.2 VPN-004 | Add ACL rules for compute traffic (SEC-C-004). A compute node can be a client computer. |
| 5.3 Navigation bar | Add the Models, Chat and Compute pages (Section 11). |
| 6.6 Metrics | The metrics collector stays agentless. The compute agent is a separate, optional program. |
| 9 Delivery phases | Add the compute phases (Section 12). |
| 13 Repository structure | Add the compute domain, the agent crates and the block adapters (Section 13). |

### 1.5 Terms

These terms add to Section 2 of the design specification.

| Term | Meaning |
| --- | --- |
| Compute node | A device on which the operator set compute on. |
| Compute agent | The Anima program on a compute node (`anima-agent`). It reports the hardware and controls the runtimes. |
| Runtime | An inference program, for example llama.cpp, MLX-LM or vLLM. |
| Backend | The hardware interface of a runtime: CUDA, ROCm, Metal, Vulkan, SYCL or CPU. |
| Model server | One runtime process that serves models through an OpenAI-compatible API. |
| Model | One named model in the library, for example `qwen3.8-27b`. |
| Variant | One file set of a model in one format and quantisation, for example GGUF `Q4_K_M` or MLX 4-bit. |
| Copy | One variant on the disk of one compute node. |
| Loaded copy | A copy that a model server holds in memory. |
| Alias | A name that the operator gives to a model for a job, for example `coder`. |
| Router | The hub function that answers requests at `ai.anima.ing` and sends each one to a model server. |
| Placement | The decision where a model runs. |
| Block, stage, coordinator, ring | Level 2 terms. Section 10 defines them. |

## 2. Architecture

```text
 Any device on the VPN (scripts, editors, agents, the Anima client)
        |
        |  https://ai.anima.ing/v1/...   (OpenAI API, optional Ollama API)
        v
+--------------------------------- Hub ---------------------------------+
|  fleet/compute/                                                        |
|    library   capabilities   placement (pure)   servers   router (HTTP) |
+------+--------------------------------------------+-------------------+
       | agent protocol (VPN)                        | model requests (VPN)
       v                                             v
+----------------+     +----------------+     +----------------+
| anima-agent    |     | anima-agent    |     | anima-agent    |
| Mac (Metal)    |     | PC (CUDA)      |     | PC (ROCm)      |
| mlx-lm server  |     | vLLM           |     | llama-server   |
| llama-server   |     | llama-server   |     |                |
| model cache    |     | model cache    |     | model cache    |
+----------------+     +----------------+     +----------------+
```

- **CMP-010.** The hub must not load a model, run a layer or keep hidden states.
- **CMP-011.** The runtime choice, the fit estimate and the placement must be pure functions. They take the inventory, the measurements and the configuration as parameters (CQ-020).
- **CMP-012.** The hub must keep working when all compute nodes are off. Compute is optional.

## 3. Compute nodes

### 3.1 Capability inventory

- **CAP-001.** For each compute node, the hub must keep these values: the backends, the accelerator name, the dedicated memory, the unified memory, the system memory, the free disk space, the CPU features and the site.
- **CAP-002.** For each backend, the agent must give a verdict: usable, or not usable with a reason and a repair hint. Example: "ROCm: not usable, the user is not in the `render` group".
- **CAP-003.** The agent must report the installed runtimes and their versions.
- **CAP-004.** The agent must report the power state (mains or battery), the thermal state and whether the operator is active on the device.
- **CAP-005.** On Apple silicon, the usable memory is the GPU working-set limit of macOS, not the total memory. The agent must report this limit.
- **CAP-006.** The hub must keep the measured tokens per second of each loaded copy. The router uses it (RTR-034).

### 3.2 Device policy

- **CAP-010.** The operator must set a policy for each compute node: on or off, a maximum memory share, a maximum disk share for models, and quiet hours.
- **CAP-011.** A portable computer must take compute work only on mains power. This is the default, and the operator can change it.
- **CAP-012.** If the operator is active on a client computer, the agent can refuse new work. The operator sets this for each device.
- **CAP-013.** When a policy stops a node, the hub must drain it. The node gets no new requests. Current requests continue for 60 seconds or fewer.

### 3.3 Compute agent

The design specification uses SSH for metrics and needs no agent. A model service needs a process on the device that installs and starts runtimes, so it needs an agent. The agent is optional.

- **CAG-001.** The compute agent must be a separate Rust binary (`anima-agent`). It must start with launchd on macOS and systemd on Linux.
- **CAG-002.** The agent must enrol with the hub with a single-use enrolment code. The hub then gives it a node token. Node tokens must follow VPN-003 for expiry.
- **CAG-003.** The agent must accept commands only from the hub. The commands are: report, install runtime, pull variant, delete copy, start server, stop server, load, unload, measure and drain.
- **CAG-004.** The agent must reject a command that is not in CAG-003. It must reject runtime flags that the hub owns (SEC-C-007).
- **CAG-005.** The agent must supervise each runtime process. If a process stops, the agent restarts it with exponential backoff and reports the event to the hub.
- **CAG-006.** The agent must send the health and metrics of each runtime to the hub every 15 seconds. These are: state, loaded copies, requests in progress, queue depth, tokens per second and memory used.
- **CAG-007.** The agent logic (command validation, flag policy, backoff, cache choice) must be in a library with no I/O, so that tests can run it.
- **CAG-008.** The agent can install a runtime only from the release sources in its configuration. It must verify each download.

### 3.4 Runtime choice

The agent installs the runtime that suits the hardware best. The research in Section 14 gives this table.

| Hardware | Default runtime | Variant format | Also |
| --- | --- | --- | --- |
| Apple silicon | MLX-LM server | MLX | llama-server (Metal) for GGUF variants |
| NVIDIA, model fits in GPU memory | vLLM | Safetensors, AWQ, GPTQ, FP8 | llama-server (CUDA) for GGUF variants |
| NVIDIA, model needs offload | llama-server (CUDA) | GGUF | — |
| AMD with a usable ROCm verdict | llama-server (HIP) | GGUF | vLLM (ROCm) when the operator selects it |
| Other GPU | llama-server (Vulkan or SYCL) | GGUF | — |
| CPU only | llama-server (CPU) | GGUF | — |
| NVIDIA with a large system memory, mixture-of-experts model | KTransformers | Its own CPU format | Only when the operator selects it |

- **RTC-001.** The runtime choice must be a pure function of the capabilities (Section 3.1) and the variant format. The operator can override it for each node.
- **RTC-002.** A node can run more than one model server, for example MLX-LM and llama-server on one Mac.
- **RTC-003.** vLLM reserves a fixed share of GPU memory. The agent must run one vLLM process for each GPU and set the share from the device policy.
- **RTC-004.** For a runtime that cannot unload a model (vLLM, KTransformers), the agent must stop and start the process to change the model.
- **RTC-005.** llama-server must run in router mode, so that the agent can load and unload GGUF models without a restart.
- **RTC-006.** Each model server must listen on the loopback interface only (SEC-C-001).

## 4. Model library and pulls

The library is the list of models that the operator uses. A pull puts a variant of a model on a node, as `ollama pull` does.

### 4.1 Library

- **LIBM-001.** The library must keep, for each model: the name, the family, the parameter count, the context length, the capabilities (chat, tools, vision, embeddings, reasoning), the licence and the source repositories.
- **LIBM-002.** The library must keep, for each variant: the model, the format, the quantisation, the file list with the size and the SHA-256 value of each file, and the source repository.
- **LIBM-003.** The source of a variant must be a Hugging Face repository in the approved list. The default list is `unsloth`, `mlx-community` and the model publisher. The operator can add an organisation.
- **LIBM-004.** The operator adds a model by a search of the approved organisations or by a repository name. The hub reads the file list and the metadata from the repository.
- **LIBM-005.** The hub must read the capabilities, the context length and the layer shape from the model files (GGUF metadata or `config.json`). It must not ask the operator for values that the files give.
- **LIBM-006.** The model name and the variant use the form `name:tag`, for example `qwen3.8-27b:q4_k_m`. This is the Ollama form.
- **LIBM-007.** The operator can give an alias to a model, for example `coder` or `fast`. A request can use an alias in place of a model name.

### 4.2 Pull

- **PULL-001.** The operator can pull a model to a node from three places: the Models page, the Ollama pull API (Section 6.4) and the hub GraphQL API.
- **PULL-002.** If the operator does not select a variant, the hub must select one. The variant must match the format of the runtime choice (RTC-001), and the fit estimate must pass (Section 5.1). The hub prefers the highest quality that fits.
- **PULL-003.** If no variant fits a node, the hub must show the reason: the memory or the disk space that is missing.
- **PULL-004.** The agent must download the files directly from the source repository to its model cache. The files do not go through the hub.
- **PULL-005.** The agent must verify each file against the SHA-256 value in the library before it uses the file.
- **PULL-006.** A pull must continue after an interruption from the last complete file part.
- **PULL-007.** The client must show the progress of each pull: bytes, speed and the time to completion.
- **PULL-008.** The agent must keep the model cache within the disk share of the device policy. It must ask the hub before it removes a copy to make space.
- **PULL-009.** The operator can delete a copy from a node.
- **PULL-010.** A gated repository needs a Hugging Face token. The hub must keep the token with the credentials domain (SEC-003) and give it to the agent only for that pull.

## 5. Placement

### 5.1 Fit estimate

- **FIT-001.** The fit estimate must be a pure function. Its inputs are the weight bytes, the KV bytes for each token, the context length, the fixed overhead for each device and the usable memory.
- **FIT-002.** KV bytes for each token are: 2 × layers × KV heads × head dimension × bytes for each element. The model files give these values.
- **FIT-003.** The fixed overhead for each runtime and backend must be a configuration value. Measurements replace the default values.
- **FIT-004.** If the model does not fit at the requested context, the estimate must give the largest context that fits. The context is rounded down to a multiple of 256.

### 5.2 Single-node offload

Moving part of a model to system memory on one node is often faster than splitting it across machines. This idea comes from the Unsloth offload planner.

- **FIT-010.** If a model does not fit in the GPU memory of a node, the placement can move weights to system memory. It must never move the KV cache.
- **FIT-011.** The placement must rank offload choices by estimated milliseconds for each token, not by percent.
- **FIT-012.** If the model fits without offload, the placement must not offload.

### 5.3 Load on demand

- **FIT-020.** A request can name a model that has no loaded copy. Then the hub must load a copy on the best node. The best node has free memory, a permitting policy and the highest measured speed.
- **FIT-021.** If the node does not have enough free memory, the hub can unload the least recently used loaded copy on that node.
- **FIT-022.** The hub must unload a loaded copy after an idle time. The default is 15 minutes. The operator can set it for each model, and "never" pins the copy.

## 6. The endpoint `ai.anima.ing`

### 6.1 Name and access

- **RTR-001.** The router must answer at `https://ai.anima.ing`. The name must resolve to the VPN address of the hub, through Headscale DNS only.
- **RTR-002.** No public DNS record of `ai.anima.ing` must point at the hub. The hub must not listen on a public interface (NET-003).
- **RTR-003.** The certificate of `ai.anima.ing` must come from a public certificate authority through the ACME DNS-01 challenge. Thus clients trust it without a pinned certificate. The DNS token must be a Docker secret (DB-002 pattern).
- **RTR-004.** The router must require an API key. Each key must have a name, a list of permitted models and an optional expiry.
- **RTR-005.** The permitted-network rule (NET-001, NET-002) applies to the router.

### 6.2 OpenAI API

- **RTR-010.** The router must supply `/v1/models`, `/v1/chat/completions`, `/v1/completions`, `/v1/embeddings` and `/v1/responses`. It forwards each request to a model server.
- **RTR-011.** `/v1/models` must list each model that has a copy on a node with a permitting policy, and each alias. It lists a model once, not once for each copy.
- **RTR-012.** Each entry of `/v1/models` must also give the Anima fields: the capabilities, the context length, the variants, and for each copy the node, the runtime and the state (loaded or on disk).
- **RTR-013.** A request can name a model, a model with a tag, or an alias. If the request has no tag, the router can use any variant of the model.
- **RTR-014.** A request can name a node with the header `Anima-Node`. The router then uses only that node.
- **RTR-015.** The router can forward the Anthropic Messages API (`/v1/messages`) to a model server that supports it.

### 6.3 Routing

- **RTR-030.** The router must select from the copies of the model on healthy nodes with a permitting policy. It prefers a loaded copy.
- **RTR-031.** If no copy is loaded, the router must start a load (FIT-020). The caller waits until the load is done or until a time limit passes. The limit is a setting.
- **RTR-032.** The router must stream responses without a buffer.
- **RTR-033.** The router can send requests with the same conversation key to the same server, to use its prompt cache.
- **RTR-034.** The router must score each loaded copy by the expected wait: queue depth divided by measured tokens per second. It must pick the better of two random candidates ("power of two choices").
- **RTR-035.** If a server fails 3 requests in a row, the router must stop sending to it for 30 seconds (circuit breaker).
- **RTR-036.** The router can retry a request on a different server only before the first byte goes to the caller.
- **RTR-037.** The router must record the time, the key, the model, the node, the token counts and the duration of each request. It must not record prompts or outputs (LOG-007).

The router design takes ideas from the SGLang model gateway, the LMDeploy proxy and the LocalAI SmartRouter (Section 14). Anima writes its own router in Rust.

### 6.4 Ollama API

Many tools can use an Ollama server. The router can also speak the Ollama API, so that these tools work without a change.

- **RTR-040.** The router can supply these Ollama endpoints: `/api/tags`, `/api/ps`, `/api/show`, `/api/chat`, `/api/generate`, `/api/embed` and `/api/pull`.
- **RTR-041.** `/api/pull` must use the hub selection of node and variant (PULL-002), unless the request names a node.
- **RTR-042.** The Ollama endpoints must use the same API keys as the OpenAI endpoints.

## 7. Model selection for a job

The operator selects the model for each job. The hub gives the facts that the operator needs.

- **SEL-001.** The Models page must show, for each model: the capabilities, the context length, the size, the nodes that have a copy, the state of each copy and the measured tokens per second.
- **SEL-002.** The operator can sort and filter the models by capability, size, speed and node.
- **SEL-003.** The operator can point an alias at a different model at any time. Tools that use the alias then use the new model without a change.
- **SEL-004.** The hub must not choose a model for a request. Automatic model choice is out of scope for this version.

## 8. Chat page

- **CHAT-001.** The client must have a Chat page that sends requests to `ai.anima.ing` with a key that the hub gives to the client.
- **CHAT-002.** The Chat page must have a model selector that shows the models and aliases of `/v1/models` with their state.
- **CHAT-003.** The operator can change the model in a conversation. Each response must show the model, the node and the tokens per second.
- **CHAT-004.** Responses must stream. The operator can stop a response.
- **CHAT-005.** The operator can set a system prompt and the sampling values (temperature, top-p, maximum tokens) for each conversation.
- **CHAT-006.** The client must keep conversations on the client device only until the vault exists (OD-C6).
- **CHAT-007.** Text must render as Markdown with code blocks. Motion must use the helpers in `$lib/motion/`.

## 9. Security

Most runtimes have no authentication by default. Some listen on all interfaces. Thus Anima must put each runtime behind its own controls.

- **SEC-C-001.** Each runtime and model server must listen on the loopback interface only. The agent must check this after each start.
- **SEC-C-002.** The agent must listen only on the VPN address of the node. It must accept only requests that carry a valid hub-issued token.
- **SEC-C-003.** The hub must keep the router API keys, the node tokens and the Hugging Face token with the credentials domain (SEC-003).
- **SEC-C-004.** The VPN ACLs must permit only: the hub to each agent port, the compute nodes to the Hugging Face hosts, and, in Level 2, each stage to the next stage of its ring. Tag compute nodes `tag:compute`. Each ACL change needs the operator's confirmation (VPS-005).
- **SEC-C-005.** In Level 2, `rpc-server` must listen only on the VPN or site address of the node. The ACL must permit only the host of its pool. The agent must refuse a llama.cpp build that a published advisory covers, for example CVE-2026-34159 (builds up to b7991).
- **SEC-C-006.** In Level 2, the wire of each ring must use a key that the hub makes when it forms the ring.
- **SEC-C-007.** The hub owns these runtime flags: model path, host, port, API key, parallel slots and remote code. The agent must refuse them from any other source. This follows GW-012 and GW-013.
- **SEC-C-008.** No runtime must load remote code from a model repository (`trust_remote_code` or the same in a different runtime).
- **SEC-C-009.** Model files come only from the approved repositories (LIBM-003), with SHA-256 verification (PULL-005). The GGUF parsers have known faults, so do not load a model file from another source.
- **SEC-C-010.** No log, metric or event must contain a prompt, an output or a hidden state (LOG-007).
- **SEC-C-011.** Do not use a remote access function of a runtime, for example LM Studio LM Link or a Cloudflare tunnel. SEC-008 applies.
- **SEC-C-012.** Work on the agent, the ACLs, the router keys, the DNS token and the ring keys has the security tag. The security reviewer must accept it (AGT-004).

## 10. Level 2: pooled models

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

- **CUI-001.** The Models page must show the library (SEL-001), the pull controls (Section 4.2), the pull progress and the aliases.
- **CUI-002.** The Chat page must obey Section 8.
- **CUI-003.** The Compute page must show each compute node with its backends, its verdicts (CAP-002), its runtimes, its loaded copies and its policy.
- **CUI-004.** The Compute page must show the API keys of the router. The operator can make and revoke a key. The client shows a new key one time only.
- **CUI-005.** The device card must show a compute badge when the device is a compute node.
- **CUI-006.** If the hub cannot place or pull a model, the client must show the reason (PULL-003).

## 12. Delivery

### 12.1 Phases

- **PH-C-000.** Compute work must not start before Phase 1 of the spec is accepted. The agent gives its own hardware values, so Level 1 does not need the Phase 2 metrics.
- **PH-C-001.** Level 2 (C4 and later) must not start before C2 is accepted and the operator uses the model service.

| Phase | Level | Content | Depends on |
| --- | --- | --- | --- |
| C1 | 1 | Compute agent, capability inventory, runtime choice, model servers, library, pull, router at `ai.anima.ing` with the OpenAI API and API keys, Compute page | Spec Phase 1, OD-C1, OD-C7 |
| C2 | 1 | Models page with search and aliases, Chat page, Ollama API, load on demand and idle unload | C1 |
| C3 | 1 | Single-node offload (Section 5.2). KTransformers on a large-memory node, if the operator has one. | C1 |
| C4 | 2 | Spikes and a decision record (Section 12.2) | C2 |
| C5 | 2 | Single-site pool with llama.cpp RPC (Section 10.2) | C4 go |
| C6 | 2 | Cross-runtime ring: block contract, adapters, ring planner, conformance, recovery | C4 go |
| C7 | — | Jobs: fine-tuning with the Unsloth core library on the best node | C1 |

### 12.2 Phase C4 spikes

1. **Cross-runtime block test.** Use one dense model of 7B to 8B parameters. Put the first layers on a Mac with an MLX block and the other layers on a CUDA PC with a Transformers block. Drive them with the Shard coordinator. Compare the tokens and the perplexity with the same model on one node.
2. **Cross-site speed.** Run the same ring with the two nodes at different sites. Measure tokens per second with and without speculative decoding.
3. **RPC pool.** Run one GGUF model across two nodes at one site with llama.cpp RPC. Compare it with the best single node with offload.

- **PH-C-010.** The decision record must give the measured numbers, the conformance tolerances (RNG-032) and a go or no-go for C5 and C6.

### 12.3 Acceptance criteria

| Phase | The phase is done when |
| --- | --- |
| C1 | A Mac and a Linux PC each run the runtime from Section 3.4. The operator pulls a model to each node. `curl https://ai.anima.ing/v1/models` with a key lists both models with their nodes. A chat request to each model streams a response. A request without a key is refused. A test proves that each runtime listens on loopback only. A test proves that no prompt or output is in a log. |
| C2 | An Ollama client lists and uses the models. The Chat page streams a response and shows the node and the speed. A model with no loaded copy loads on the first request and unloads after the idle time. An alias change takes effect for the next request. |
| C3 | A model 20 % larger than the GPU memory of a node runs on that node with offload. |
| C4 | The decision record exists (PH-C-010). |
| C5 | A model larger than any single node at one site runs across two nodes of different backends. The ACL refuses a connection to `rpc-server` from a node outside the pool. |
| C6 | A model runs on a ring with a Mac stage and a CUDA stage at different sites and passes the conformance test. When a stage stops in a request, the request completes on a spare node. |
| C7 | A fine-tune job runs on the node that the placement selects and returns an adapter file to the operator. |

## 13. Repository changes

```text
crates/
├── hub/src/fleet/compute/
│   ├── capabilities/   Inventory, verdicts, device policy
│   ├── library/        Models, variants, aliases, approved sources
│   ├── placement/      Runtime choice, fit estimate, offload ranking, ring planner (pure)
│   ├── servers/        Model servers, copies, loads and pulls
│   ├── router/         The ai.anima.ing endpoint: OpenAI and Ollama APIs (I/O edge)
│   └── rings/          Level 2: rings and conformance results
├── compute-agent/      anima-agent binary: enrolment, supervision, pulls, proxy
└── compute-agent-core/ Agent logic with no I/O (CAG-007)
contracts/compute/
├── agent-protocol.md   Hub to agent commands and reports
├── router-api.md       The Anima fields of /v1/models and the Anima-Node header
└── block-contract.md   Level 2: block operations and the hidden-state byte layout
blocks/                 Level 2: MlxBlock and TransformersBlock (Python)
apps/client/src/routes/ models/, chat/, compute/
deploy/headscale/       tag:compute, the ACL rules and the ai.anima.ing DNS record
```

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
| Jan | Apache 2.0 | Through llama.cpp and MLX | No | Not possible | Agent supervision reference |
| TensorRT-LLM | Apache 2.0 | CUDA (GeForce best-effort) | Yes, data-centre links | Not practical | Not used |
| LM Studio | Closed | Through llama.cpp and MLX | No | Not possible | Not used (terms forbid change and service use) |

Findings:

1. For Level 1, three runtimes cover all hardware: MLX-LM on Macs, vLLM on NVIDIA when the model fits, and llama.cpp everywhere else.
2. No runtime splits one model across different runtimes. Each cross-machine mode needs one runtime on all nodes and a fast link.
3. llama.cpp RPC is the only ready split across different backends. It works at one site only.
4. A split across runtimes and sites needs the Anima framework of Section 10.3.

## 15. Open decisions

| Number | Decision | Recommendation |
| --- | --- | --- |
| OD-C1 | Hardware of each compute node | The operator gives the list: each Mac, each GPU, memory and site. |
| OD-C2 | One agent binary or two | Keep `anima-agent` separate from `anima-collector`. They run on different devices. Share the enrolment code in a library crate. |
| OD-C3 | Conformance tolerances | Set from the C4 measurements (RNG-032). |
| OD-C4 | Where the block adapters live | Offer them to the Shard project first. Keep them in `blocks/` only if Shard does not accept them. |
| OD-C5 | Router location | The router runs in the hub on the VPS (OD-9). Each request then goes caller to VPS to node. Measure the added delay in C1. If it is too high, run a second router at the home site. |
| OD-C6 | Where conversations are kept | On the client device until the vault exists (extensions plan, Section 2.4). Then keep them sealed on the hub, so that all clients see them. |
| OD-C7 | Access to `ai.anima.ing` | VPN only, with Headscale DNS and a DNS-01 certificate (Section 6.1). Confirm that the operator controls the DNS zone of `anima.ing`. Public access is out of scope (SEC-008). |
| OD-C8 | Device policy default for client computers | Off by default. The operator sets each client computer on. |
