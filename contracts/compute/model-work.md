# Model work: the work types of the model service, the copies and the runtime report

Status: **Draft, waits for a ruling (R-NEW-modelwork) and the security review (SEC-C-012).** Version 0.3,
2026-11-20 (version 0.3 takes the answers of the operator to OD-C22 to OD-C25: R-NEW-sandboxrisk,
R-NEW-hubhash, R-NEW-mlxcommunity, R-NEW-agentmodelwork; each stricter form is a setting at the machine,
Section 10.1. Version 0.1, 2026-11-20, was the text of the first security read; version 0.2 has the corrections of
its 14 plain findings, and the safe default of its 4 findings that wait for the operator, marked "until the
operator decides"; [security-findings.md](../../docs/security-findings.md), the section of the model work).
This contract has no K number: no ruling gave one. Task L1-3 of [compute-plan.md](../../docs/plans/compute-plan.md) Section 12.4. No code may be built
from this text before the ruling (CON-001, AGT-004). Change it only through a programme ruling.

Requirements: CAG-002 to CAG-006, CAG-008, CAG-009, PULL-004 to PULL-011, RTC-002 to RTC-006, SEC-C-001,
SEC-C-003, SEC-C-007 to SEC-C-010 of the compute plan; USE-004 to USE-009 of [jobs.md](../../docs/proposals/jobs.md);
MET-002, MET-014, MET-020, LOG-007 of the design specification. Decisions: OD-C9 (R-165: the model work types
join the fixed list of the work route), OD-C8 (R-165: one switch), R-160 (the work route).

This file holds no secret, no token, no key and no example value of a secret.

## 0. What this contract uses, and what it changes

- **The work route exists.** Contract K10, [work-route.md](../work-route.md), is the contract of task W1 of
  [projects-and-builds-plan.md](../../docs/plans/projects-and-builds-plan.md). It is accepted (R-209, version 0.3;
  version 0.5 is in force by R-251). Its database rows are K2 range 0980–0999 (task W3). The socket of task W0
  (R-172) is in K8 Section 1. Thus this contract does not make a narrow route of its own, as OD-C9 option (a) and
  the pattern of `agent_install` (R-153) would need without K10. It adds the model work types to the fixed list of
  K10, with a new kind `model`. K10 Section 13, point W1-P5, says: "Each is one more type of the fixed list, added
  with a ruling." R-NEW-modelwork is that ruling.
- **Changes to K10 that wait for R-NEW-modelwork.** The text of this contract is the change. K10 itself is not
  edited before the ruling. The changes are: the list of work types of K10 Section 1 and Section 6 gets the ten
  types of Section 2 below; the kind `model` (Section 1 below); the purpose `hf_token` of K10 Section 8
  (Section 7 below); the causes of refusal of Section 5 below; the order of the claim (Section 3 below).
- **Changes to K8 that wait for R-NEW-modelwork.** The optional member `runtimes` of the `Batch` of K8 Section 4
  (Section 8 below). It is an optional member, so it needs no `/daemon/v2/` (K8 Section 1).
- **A change of the design specification that waits for R-NEW-modelwork.** MET-020 says that the daemon connects
  only to the hub and sends no request to a TCP port of its own computer. A pull (PULL-004), a runtime install
  (CAG-008) and a measure or a validation (PULL-011) need more. Section 4 gives the rule; open point MW-P1 asks
  the operator for the text.
- **What this contract does not hold.** The model request and its address (task L1-10, `model-request.md`); the
  wait policy, the pin and the readiness of a request (task L1-15); the device policy itself (task L1-5); the
  library and the choice of a variant (tasks L1-4 and L1-7); the model key (task L1-9).

## 1. Terms

| Term | Meaning |
| --- | --- |
| Model item | An item of K10 whose kind is `model`. |
| Kind `model` | The daemon does the steps itself, with its own user. No job code runs. Only the model work types have this kind. |
| Runtime | A model server program of a fixed list in the daemon: `llama_server`, `mlx_lm`, `vllm`. The list grows only with a new version of this contract. |
| Server | One running process of a runtime, with its own loopback port. A node can have more than one (RTC-002). |
| Variant | A set of model files in one format and one quantisation (LIBM-002). |
| Copy | The files of one variant on one node, in the model cache. |
| Owned flag | A flag of a runtime that only the daemon sets (SEC-C-007). |

## 2. The work types (CAG-003)

The list is fixed in the daemon program. The hub cannot add a type. Each model item has the kind `model`, no
member `repository`, `project` null, and `secrets` empty, except `model_pull` (Section 7), and the member `actor` of Section 3. The daemon refuses an
item that breaks this (`member_unknown`).

Each `parameters` object has exactly the members that its row names. The daemon refuses each other member
(`member_unknown`), as K10 Section 5 says. Identifiers are UUIDs that the hub makes. A name of a runtime, a
format or a backend is a word of a fixed list in the daemon. An `Item` never holds a path, an address, a port,
a command, an argument, a flag or an environment variable.

| Type | `parameters` | What the daemon does |
| --- | --- | --- |
| `model_report` | `{}` | Sends the full model state of the node as the result: runtimes and versions, copies and their states, servers (Section 8). Changes nothing. |
| `runtime_install` | `{ "runtime": "llama_server", "version": "b8100", "backend": "metal" }` | Installs that runtime from its release source in the daemon configuration (Section 6). |
| `model_pull` | `{ "copy": "<uuid>", "variant": "<uuid>", "source": { "organisation": "unsloth", "repository": "Qwen3-8B-GGUF", "revision": "<40 hexadecimal characters>" }, "format": "gguf", "files": [ { "name": "Qwen3-8B-Q4_K_M.gguf", "sizeBytes": 5027783488, "sha256": "<64 hexadecimal characters>" } ], "gated": false }` | Downloads the files to the model cache and verifies them (Section 7). |
| `copy_delete` | `{ "copy": "<uuid>" }` | Unloads the copy if a server has it, then deletes its folder in the model cache (PULL-009). |
| `server_start` | `{ "server": "<uuid>", "runtime": "llama_server", "version": "b8100", "backend": "metal", "settings": { … } }` | Starts one server of an installed runtime on loopback and checks the listener (Section 9). |
| `server_stop` | `{ "server": "<uuid>" }` | Stops the server: it asks it to end, waits at most 10 seconds, then ends the process. |
| `model_load` | `{ "server": "<uuid>", "copy": "<uuid>", "settings": { … } }` | Loads a copy on a server. For a runtime that cannot unload (vLLM), the daemon stops and starts the process with the new copy (RTC-004). llama-server runs in router mode and loads with no restart (RTC-005). |
| `model_unload` | `{ "server": "<uuid>", "copy": "<uuid>" }` | Unloads the copy from the server, or stops the process when the runtime cannot unload (RTC-004). |
| `model_measure` | `{ "server": "<uuid>", "copy": "<uuid>", "probe": "text_short" }` | Sends a fixed request of the daemon to the server and reports the result (PULL-011, CMP-006). |
| `model_drain` | `{ "seconds": 60 }` | Sends no new request to each server, waits for the requests in progress for at most `seconds` (at most 60, CAP-013), then reports. It does not stop a server. |

- `version` is the version of a release of the runtime, by the pattern of that runtime in the daemon (for
  `llama_server`, `b` and digits). `backend` is `metal`, `cuda`, `rocm`, `vulkan` or `cpu`.
- `source.organisation` and `source.repository` have only the characters that Hugging Face permits for a name:
  letters, digits, `-`, `_` and `.`, at most 96 characters, not `.` or `..`. `revision` is a commit of the
  repository, never a branch name: a branch can change after the hub read its file list.
- `files[].name` is a path inside the repository: segments of the characters above and `/`, no segment `.` or
  `..`, no leading `/`, at most 256 characters, at most 64 files. `sizeBytes` is above 0. The daemon makes each
  address from a fixed pattern (Section 7).
- `probe` is a word of a fixed list in the daemon. Today: `text_short` (one fixed prompt, a fixed answer check)
  and `vision_short` (one fixed image in the daemon program). The hub sends no prompt.

### 2.1 `settings`: what the hub may set, and what the daemon owns (SEC-C-007, CAG-004)

`settings` is an object of typed members. It is never a list of flags. The daemon makes the command line of
the runtime from its own code, these members, and the paths that it makes itself.

| Member | Type and limit | Runtimes |
| --- | --- | --- |
| `contextTokens` | integer, 512 to 1,048,576 | all |
| `gpuLayers` | integer, 0 to 999, or the string `all` and no other string | `llama_server` |
| `memoryShare` | number, 0.05 to the memory share of the device policy (RTC-003) | `vllm` |
| `gpu` | integer, an index of a GPU in the inventory that the daemon reports (one vLLM process for each GPU, RTC-003); another index is refused (`gpu_unknown`) | `vllm` |
| `batchTokens` | integer, 16 to 65,536 | all |
| `kvCacheType` | `f16`, `q8_0` or `q4_0` | `llama_server` |

The daemon owns, and refuses from the hub and from each other source: the model path, the host, the port, the API
key, the parallel slots and remote code (SEC-C-007). The daemon also owns the log path, the chat template file,
each address, each file path and each environment variable of the process. A member of `settings` that is not in
the table, or that the table does not permit for the runtime, is refused (`flag_owned` when it names one of the
owned values, as `host`, `port`, `apiKey`, `parallel`, `modelPath`, `trustRemoteCode`; else `member_unknown`).

The values that the daemon sets:

- Host: `127.0.0.1` only (RTC-006, SEC-C-001). Never `0.0.0.0`, `::` or a name.
- Port: a free port that the daemon selects from the range in its configuration, default 41000 to 41999.
- API key: a random key of 32 bytes that the daemon makes for each start, gives to the process in its
  environment (never on its command line), and holds in memory only. It is not the model key of L1-9.
- Parallel slots: from the memory share of the device policy and the fit estimate of the copy, by a pure function
  in `daemon-core`.
- Remote code: off. For vLLM the daemon passes no `--trust-remote-code` and sets the refusal of remote code in
  each form that the runtime has. A copy whose format needs remote code cannot be loaded (`remote_code`)
  (SEC-C-008). For vLLM the daemon also sets `--load-format safetensors` and `--tokenizer` to the folder of the
  copy, and sets no `--download-dir` and no `--allowed-local-media-path`. A media input is accepted only as a
  `data:` URL; the daemon sets the runtime so that it fetches no address (Section 4).
- Logs (SEC-C-010): llama-server with `--log-disable` and the lowest verbosity, `--no-slots`, and no slot save
  path; vLLM with `--disable-log-requests`. The standard output and the standard error of each runtime go to a
  file of mode 0600 in the state folder, a ring of at most 1 MiB, that the daemon never sends to the hub and
  never copies into a log line. A runtime that has no such flag in its version is refused
  (`runtime_not_configured`).
- Environment (SEC-C-001, SEC-C-007): the daemon starts each runtime with an empty environment and only these
  variables: `HOME` and `TMPDIR` set to the temporary folder of the process (Section 10), `PATH` set to the `bin`
  folder of the runtime only, `HF_HUB_OFFLINE=1`, `TRANSFORMERS_OFFLINE=1`, `VLLM_NO_USAGE_STATS=1`,
  `DO_NOT_TRACK=1`, and the variable that holds the API key. No proxy variable, no `HF_TOKEN`, no `DYLD_*`, no
  `LD_PRELOAD`, no `PYTHONPATH` reaches a runtime.

### 2.2 The files that a copy may have (SEC-C-008, SEC-C-009)

A model file can hold code: a pickle file runs code when it is read, a `.py` file of a repository runs through
`auto_map`, and a chat template is a program of Jinja. Thus each format has a fixed list of file names in the
daemon, and the daemon refuses each other file of `files` (`file_type`) before it downloads a byte.

| Format | Permitted files |
| --- | --- |
| `gguf` | `*.gguf` |
| `safetensors`, `mlx` | `*.safetensors`, `*.safetensors.index.json`, `config.json`, `generation_config.json`, `tokenizer.json`, `tokenizer_config.json`, `special_tokens_map.json`, `*.model` (a SentencePiece file), `chat_template.jinja`, `preprocessor_config.json` |

- Each `.bin`, `.pt`, `.pth`, `.pkl`, `.py`, `.npz`, `.ckpt` and `.h5` file is refused (`file_type`), also when
  its name is in a sub-folder.
- Before a load, the daemon reads `config.json` and `tokenizer_config.json` of the copy with a JSON parser. It
  refuses the copy (`remote_code`) when one of them has the member `auto_map` or `custom_pipelines`, or names a
  class that is not in the list of the runtime.
- A chat template comes only from `tokenizer_config.json`, `chat_template.jinja` or the GGUF metadata of the
  copy, and is at most 64 KiB. The daemon gives no template of the hub and no template file from outside the
  copy. The runtime renders it in its own sandboxed Jinja form; a runtime version that renders a template with
  no such sandbox is not in the list of release sources. The runtime and its parsers still read files that the
  hub chose, so the isolation of Section 9 is the limit for a fault of a parser.

## 3. How the daemon asks and reports (CAG-002, CAG-003)

- The transport, the token and the routes are those of K10 Section 2: `POST /work/v1/claim`, `/work/v1/report`
  and `/work/v1/secret`, with the daemon token of K8 Section 2. The daemon asks; the hub never opens a
  connection to a machine. The daemon listens on no port for work (MET-020, USE-005). The model servers listen on
  loopback for model requests; that is the model address of task L1-10, not a route for work.
- A daemon token alone permits no model work (CAG-002). The hub gives a model item only to a device that takes
  work (CMP-002, USE-004) and whose device policy permits it now (CAP-010 to CAP-012). The machine checks the
  switch and the policy again before each item (K10 Section 3: `switch_off`, `policy`). `server_stop`,
  `model_unload`, `model_drain`, `copy_delete` and `model_report` are never refused for `policy` or `switch_off`:
  they only reduce the work of the machine.
- **Who may ask (R-NEW-agentmodelwork).** The hub makes each of the ten types for an API key or an AI agent,
  with the MCP state of its tool, as R-153 does for `agent_install`. Each model item has the member `actor`:
  `User`, `Agent` or `Hub` (LOG-012). The setting `agentTypes` at the machine (Section 10.1) lists the types
  that the machine takes from an item whose `actor` is `Agent`; by default it has the ten types. The daemon
  refuses another (`actor_not_permitted`). The hub writes `actor`, so the setting limits an honest hub only.
- The `Claim` gets `model` in `kinds` and the ten types in `types` when the daemon has the model function. The
  model function is on when the switch is on; there is no second switch (OD-C8).
- Model items do not use the job slot of K10 (`free.slots` counts jobs). The hub counts its own leases: one
  machine has at most one model item that changes the machine (`runtime_install`, `model_pull`, `copy_delete`,
  `server_start`, `server_stop`, `model_load`, `model_unload`) and one `model_measure` at a time. A `model_report`
  and a `model_drain` are not counted. One claim gets at most one item. The order: `model_drain`, `server_stop`,
  `model_unload`, then each other model item the oldest first, then a job.
- A model item has the states, the reports, the lease, the stop and the refusals of K10 Section 7 and Section 11.
  `preparing` is the check of the item and of the disk; `running` is the step; `finishing` is the clean-up of
  partial files. A `model_pull` holds its lease by its reports, each 15 seconds.
- `result` of a model item has the members of K10 Section 7 and one more optional member, `model`. Its members
  are fixed codes, numbers, identifiers and SHA-256 values, never text from a runtime or a file:

```json
{ "outcome": "succeeded", "cause": null, "seconds": 412,
  "model": { "copy": "<uuid>", "copyState": "downloaded", "bytes": 5027783488, "files": 1 } }
```

- The log of a model item holds only the lines of the daemon: the step, the sizes, the codes. It never holds the
  output of a runtime, a prompt, an answer or an image (SEC-C-010, LOG-007), and never a token (Section 7).
- Progress of a pull (PULL-007): each report of a running `model_pull` has `progress`:
  `{ "bytes": 1073741824, "totalBytes": 5027783488, "bytesPerSecond": 52428800 }`. The hub computes the time to
  completion. `progress` is an optional member of the `Report` of K10 for the kind `model` only.

## 4. The connections that model work makes (MET-020)

| Connection | Who makes it | Rule |
| --- | --- | --- |
| The hub | The daemon | K8 and K10 only. |
| Hugging Face | The fetch process (below) | HTTPS only, to `huggingface.co` and the file hosts that the daemon holds in a fixed list. It follows a redirect only to a host of that list. |
| Release source of a runtime | The fetch process | HTTPS only, to the address of the daemon configuration (Section 6). Redirects only to a host of the fixed list for that runtime. |
| A model server | The probe process (below) | `http://127.0.0.1:<port>` of a server that the daemon started, for `model_measure` and the check of a start only. Before each request it confirms that the listener of that port belongs to the process tree of that server (Section 9). |
| Each other address | A runtime | None. A runtime makes no outgoing connection: its environment (Section 2.1) turns off each fetch of Hugging Face and each report of use, it gets no media address, and the isolation of Section 9 refuses the network outside loopback where the system can do it. |

Rules of the fetch process (both sources):

- TLS with the root certificates of the system or of the daemon program only. No root certificate comes from
  the hub, from the configuration of a runtime source, or from an environment variable.
- No proxy. No `http`. No redirect to `http`, to an address written as an IP address, or to a host outside the
  list. At most 5 redirects.
- The hosts of Hugging Face are a fixed list in the daemon program: `huggingface.co`, `cdn-lfs.hf.co`,
  `cdn-lfs-us-1.hf.co`, `cas-bridge.xethub.hf.co`. A change of that set comes with a release of the daemon.
- It reads at most `sizeBytes` of a file, and at most the `sizeBytes` of a release source entry (Section 6).

The fetch process and the probe process are child processes of the daemon, made from the same program file with a
fixed mode. They get no daemon token, no hub origin and no proxy variable. They take their instruction on their
standard input as JSON from the daemon, and give back fixed codes and numbers. The part of the daemon that speaks
to the hub makes no other connection. This follows K10 point W1-P1. MET-020 must get this exception in its text
(open point MW-P1).

**For task L1-6 only (R-NEW-fetchprocess).** The build of task L1-6 downloads a runtime archive in the daemon
process, with a client of its own that holds no token, no hub origin and no proxy, and verifies the SHA-256 before it
opens the archive. This is permitted for `runtime_install` of task L1-6 only. The child fetch process of this section
is built before task L1-7 (task L1-6b of the compute plan), and task L1-7 does not start without it.

## 5. What the daemon checks, and the causes of a refusal

The daemon checks each model item before `preparing`, with a pure function in `daemon-core` (CAG-007). It uses
the causes of K10 Section 5, and these:

| Cause | When |
| --- | --- |
| `type_unknown` | The type is not in the list of Section 2 (CAG-004) |
| `flag_owned` | `settings` names a value that the daemon owns (SEC-C-007) |
| `runtime_not_configured` | The daemon configuration has no release source for that runtime, version, operating system, architecture and backend (Section 6) |
| `runtime_not_installed` | `server_start` names a runtime and version that the node does not have |
| `runtime_advisory` | The version is in the list of versions with a published advisory that the daemon holds (SEC-C-005) |
| `source_not_permitted` | The organisation is not in the approved list of the machine (Section 7) |
| `copy_unknown` | The copy, the server or the variant is not on this node |
| `copy_not_ready` | `model_load` or `model_measure` names a copy that is not `downloaded` or `validated` |
| `remote_code` | The copy needs remote code to load (SEC-C-008) |
| `disk_share` | The copy does not fit in the disk share of the device policy after the removal that the hub permitted (PULL-008) |
| `memory_share` | The load does not fit in the memory share of the device policy |
| `file_type` | `files` names a file that the format does not permit (Section 2.2) |
| `gpu_unknown` | `gpu` is not a GPU of the inventory of the machine |
| `runtime_too_old` | The version is lower than the lowest version of that runtime that the daemon program holds |
| `actor_not_permitted` | `actor` is `Agent` and the setting `agentTypes` does not name the type |
| `revision_too_new` | The commit of the source is newer than the lowest age that the machine holds (Section 7) |

Causes after `preparing`, reported with the state `failed`: `checksum` (a SHA-256 is different), `size` (a file is
larger than `sizeBytes`), `source_failed` (the source refused or did not answer after the retries), `listener`
(Section 9), `isolation_failed` (the daemon cannot make the isolation of Section 9), `limit_memory` (Section 9),
`load_crash` (the server ended while it loaded a copy), `hash_mismatch` (Section 7), `start_failed` (the server did not become ready in 120 seconds), `probe_failed` (the fixed request got
no correct answer), `limit_disk`, `program_mismatch` (the SHA-256 of a runtime file), `archive_rejected` (a
`runtime_install` whose archive breaks a rule of the unpack of Section 6, or has no server program at the path of its
release source entry; R-NEW-archiverejected), `containment_failed` (Linux: the daemon cannot make the cgroup of the
server, Section 9; R-NEW-containment).

## 6. The install of a runtime (CAG-008)

- The daemon configuration holds the release sources: one entry for each runtime, version, operating system,
  architecture and backend, with the HTTPS address of the archive and its SHA-256. The operator of the machine
  writes it at the machine. The hub cannot write it. A daemon release has a default file of entries that the
  release signs (K8 Section 8); a value of the operator replaces a default entry.
- Each entry also has `sizeBytes`, the largest size of the archive. The fetch process reads no more.
- The archive is a `.tar.gz` (or `.tgz`) or a `.zip`, by the end of the path of its address; another form is
  `runtime_not_configured` (R-NEW-archivezip). Each rule of the unpack below applies to both forms in the same way;
  a `.zip` entry that is a symbolic link, or that is encrypted, is refused, and the declared size of an entry
  counts in the limit before a byte is read. A break of a rule fails the item with `archive_rejected`.
- The daemon downloads the archive with the fetch process and verifies the SHA-256 before it opens the archive.
  It unpacks it into a new temporary folder, then renames that folder to `runtimes/<runtime>/<version>-<backend>/`
  of its runtime folder (Section 10). It accepts only regular files and folders: it refuses an entry that is a
  symbolic link, a hard link, a device, a FIFO, an absolute path or a path with `..`, and two entries whose names
  are equal when they are in lower case. It sets the mode of each folder and of each program file to 0755 and of
  each other file to 0644; it clears each setuid, setgid and sticky bit. It stops at 8 GiB of unpacked bytes or at
  50,000 entries (`limit_disk`).
- The daemon program holds the lowest version of each runtime that it accepts, and a list of versions with a
  published advisory (SEC-C-005). It refuses a lower version (`runtime_too_old`) and a listed version
  (`runtime_advisory`). Only a signed release of the daemon (K8 Section 8) changes the two lists; the
  configuration of the machine and the hub cannot lower them.
- No shell runs at any step. The daemon starts a runtime by the absolute path of the program file in that folder,
  never by a name that the system looks up in `PATH`. It runs no install script of the archive, and no package
  manager (`pip`, `brew`, `apt`).
- A runtime that needs Python (`mlx_lm`, `vllm`) is installed only from an archive that holds its environment.
  How such an archive is made is open point MW-P4.
- The daemon reports each installed runtime and version in the runtime report (Section 8; CAP-003).

## 7. The pull (PULL-004 to PULL-010, SEC-C-009)

- **The source.** The daemon downloads each file directly from Hugging Face to its model cache. The files do not
  go through the hub (PULL-004). The address is
  `https://huggingface.co/<organisation>/<repository>/resolve/<revision>/<file>`, made by the fetch process from
  the parameters; each part is encoded as a path segment.
- **The approved organisations.** The hub holds the approved list (LIBM-003). The machine holds its own list too,
  and refuses a pull from an organisation that is not in it (`source_not_permitted`). By R-NEW-mlxcommunity, the default list of the machine (the setting `organisations`, Section 10.1) is `unsloth`,
  `mlx-community` and the publishers that the release of the daemon names; the owner of the machine changes it
  at the machine. The hub cannot write it. See open
  point MW-P2.
- **A second source of each hash (R-NEW-hubhash; off by default).** The SHA-256 of an item comes from the hub,
  so it proves nothing against a hub that is taken over. The check against the hub value is always done. When
  the setting `secondHash` is on (Section 10.1), the fetch process also reads, from Hugging Face,
  the SHA-256 that the repository publishes for each LFS file at that `revision` (the file list of the commit),
  and refuses a file whose two values differ (`hash_mismatch`). A file that is not an LFS file is checked only
  against the hub value and must be at most 16 MiB. When the setting `revisionAgeDays` is above 0
  (default 0), the machine refuses a `revision` whose commit date at Hugging Face is newer (`revision_too_new`). The owner of the
  machine can change the age at the machine.
- **The checks.** The daemon writes each file to `<cache>/partial/<copy>/` and computes the SHA-256 as the bytes
  arrive. It moves the file to `<cache>/copies/<copy>/` only when the size and the SHA-256 are those of the item
  (PULL-005). A file that is larger than `sizeBytes` stops at once (`size`). A different SHA-256 deletes the
  file and fails the item (`checksum`).
- **Resume (PULL-006).** After a cut, the daemon continues each file from the last complete part with an HTTP
  `Range` request. A part is 16 MiB. The SHA-256 state is kept with each complete part, so the daemon does not
  read the file again. When the source answers 200 in place of 206, the daemon starts that file again from 0.
  The partial folder survives a restart of the daemon; the next `model_pull` of the same copy continues it.
  The bytes of `partial/` count in the disk share and in `cacheBytes`. The daemon deletes a partial copy that no
  pull touched for 7 days, and at `copy_delete` of that copy. It holds at most 4 partial copies; a fifth pull
  is refused (`disk_share`).
- **The disk share (PULL-008).** Before the start, the daemon checks that the copy fits in the disk share of the
  device policy with what the cache holds. If it does not fit, it refuses (`disk_share`) and the hub asks the
  operator which copy to remove. The daemon never removes a copy by itself: only `copy_delete` removes one.
  While the pull runs, the daemon stops it (`limit_disk`) when the free disk drops below the reserve of the
  daemon configuration (default 10 GiB).
- **The copy is downloaded, not validated (PULL-011).** A pull that succeeds makes the state `downloaded`. Only
  `model_measure` with a correct answer makes `validated` (Section 11).
- **The token of a gated repository (PULL-010, SEC-C-003).** When `gated` is true, `secrets` of the item is
  `["hf_token"]`, a new purpose of K10 Section 8 for `model_pull` only, with the life of the item. The hub reads
  the token from the key vault of E0-6 (K-E5) when the holder asks for it, and not before. The daemon asks for it
  in `preparing`, gives it to the fetch process on its standard input, and to no other process. It writes it to
  no file, no log, no command line and no environment variable. The operator must give a fine-grained token of
  Hugging Face with read access to gated repositories only; the hub says so where he enters it. The fetch process sends it only in the
  `Authorization` header of a request to `huggingface.co`, and drops it at a redirect to a different host. The
  Rust types that hold it implement neither `Debug`, `Display` nor `Clone`, and wipe on drop. The hub writes
  each release to the audit log without the value (K10 Section 8). No command, MCP tool or query gives it
  (MCP-004).
- **The key vault on dev.** The operational keys of K-E5 Section 2 are built on `dev` and wait for the human
  check HT-035; the `credentials` domain of the hub holds them (R-147). The purpose `hf_token` reads one key of
  that store by its name. The token is not granted to the device as an operational key, because a grant puts it
  in the cache of the daemon for longer than one pull. Until the store has the Hugging Face token, or if the
  vault is not on the hub, the hub makes no `model_pull` item with `gated` true and tells the operator why;
  pulls from public repositories work.

## 8. The runtime report (CAG-006)

The runtime report is an optional member `runtimes` of the `Batch` of K8 Section 4. It is not a report of the
work route.

Reason: a server runs when no item is held, so the work route has no item to report on. K10 ends an attempt that
holds an item with no end (K10 Section 7, `expired`), and a report of K10 belongs to one item. The push batch is
already sent each 10 seconds by each enrolled daemon, is kept in the buffer while the hub cannot be reached
(MET-014, MET-022), and adds no new route. The daemon sends the member in one batch each 15 seconds at most
(CAG-006); because batches go each 10 seconds, a value is never older than 15 seconds, and the member is left out
of the batches between.

```json
"runtimes": {
  "at": "2026-11-20T10:00:10Z",
  "installed": [ { "runtime": "llama_server", "version": "b8100", "backend": "metal" } ],
  "servers": [
    { "server": "<uuid>", "runtime": "llama_server", "state": "ready", "restarts": 0,
      "loaded": ["<copy uuid>"], "inProgress": 1, "queued": 0, "tokensPerSecond": 41.5,
      "memoryBytes": 6442450944, "tempBytes": 0 }
  ],
  "copies": [ { "copy": "<uuid>", "state": "validated", "bytes": 5027783488 } ],
  "cacheBytes": 5027783488
}
```

- `state` of a server is `starting`, `ready`, `draining`, `restarting`, `failed` or `stopped`.
- `tokensPerSecond` is the mean of output tokens over the last 15 seconds, or null. `memoryBytes` is the resident
  memory of the process and, for a GPU with its own memory, its use of that memory.
- The member holds no prompt, no output, no file name of a copy and no address (SEC-C-010). At most 16 servers,
  64 copies and 16 installed runtimes.
- A hub that does not know the member ignores it (K8 Section 1).
- `model_report` gives the same object as its `result.model`, for the hub to read the whole state at once.

## 9. Start, the isolation, the loopback check and the supervision (SEC-C-001, CAG-005)

- **The isolation (R-NEW-sandboxrisk; off by default).** A runtime reads files that the hub chose, and its parsers
  have faults (SEC-C-009). With no isolation, a malicious model file can take the account of the user of the
  daemon; the operator accepted that risk. When the setting `sandbox` is on (Section 10.1), the daemon starts
  each runtime in an isolation that denies each thing by default:
  on macOS a `sandbox-exec` profile of the daemon program; on Linux Landlock and seccomp, or `bubblewrap` where
  it is installed. The runtime can read its runtime folder and its copies, write only its temporary folder,
  and listen and connect only on loopback. It cannot read the daemon folder (the token and the configuration),
  the home folder of the user or the other folders of the daemon. When the daemon cannot make the isolation, the
  start fails (`isolation_failed`); there is no start with less isolation. With the setting off, the environment
  and the flags of Section 2.1 still apply, and the loopback check below is the only control of the network.

- **The tree of a server, and what the loopback check binds (R-NEW-containment).** Without the sandbox, the
  loopback check below constrains an honest runtime only: a runtime that is not honest can make outgoing
  connections and can hide a listener for the time between two checks.
  - On Linux the tree of a server is one cgroup v2 below the cgroup of the daemon, which its unit delegates. The
    server joins that cgroup before its program runs: the daemon starts the daemon program in the fixed mode
    `__model-launch`, which writes its own process id to `cgroup.procs`, sets the parent-death signal (`SIGKILL`) as a
    backstop, checks that the daemon is still its parent, and then runs the runtime with `exec`. The loopback check
    reads the sockets of each process of `cgroup.procs`, in the network tables of each network namespace that a
    process is in, merged. A `cgroup.procs` that does not read, a process of the tree whose files do not read, or
    an empty tree while the server runs, counts as a listener that is not on loopback (it fails closed). A stop and
    an end use `cgroup.kill` and a signal to each process, then remove the cgroup; a cgroup that does not empty is
    logged and left, and each start has a cgroup of its own name. When the `pids` controller is delegated, the
    daemon sets `pids.max` of the cgroup to 512 (a placeholder value, to measure). When the cgroup cannot be made
    (no cgroup v2, or no delegation to the user of the daemon), the start fails (`containment_failed`). There is no
    start with less containment.
  - On macOS the tree is the walk of the parent processes from the server (`/bin/ps`), and the daemon records each
    process of the tree at each poll, so a child that a double fork gives to `launchd` stays in the tree once it was
    seen. This is best effort unless the server runs under the sandbox.
  - Before each signal the daemon checks the identity of the process: its id and its start time. It never signals
    a process id that the system gave to a different program.
  - The daemon writes the identities and the cgroup of each server to a record in the folder of that server in the
    state folder at each poll. When the daemon starts, it ends the processes and the cgroups that each record names
    and removes the records (the sweep), because it starts no server by itself (MW-P5). The sweep ends only a
    cgroup named `anima-model-` directly below the cgroup of the daemon, because the runtime can write the record
    when the sandbox is off.
- **How the socket table is read (R-NEW-archivezip).** On macOS the daemon runs `/usr/sbin/lsof` by its absolute
  path, with an empty environment, for the processes of the tree, with a limit of 5 seconds. A read that does not
  end in time or does not read counts as a listener that is not on loopback (it fails closed). `proc_pidinfo` would
  need code that the workspace forbids (`unsafe`).
- **Ready (R-NEW-archivezip).** A server is `ready` when the socket table shows a TCP listener of its tree on
  loopback on its port. The daemon sends no request to the server for this. The HTTP probe, with the owner check
  of the listener right before it, comes with task L1-9 or `model_measure` (task L1-8).
- **The port.** The daemon starts the runtime with port 0 where the runtime permits it and reads the port from
  the socket table; else it selects a free port of its range. Before it sends the API key in a probe, it checks
  that the listener of that port belongs to the process tree of the server; if not, the cause is `listener`.
- **The loopback check.** When a server is ready, after each restart, after each `model_load`, and in each cycle
  of 15 seconds, the daemon reads the sockets of the process tree of the server from the system (on Linux
  `/proc/<pid>/net/tcp`, `tcp6`, `udp` and `udp6` with the socket inodes of each process of the tree; on macOS
  `proc_listchildpids` and `proc_pidinfo` with `PROC_PIDFDSOCKETINFO`). It does not connect to find them. Each
  listening TCP socket and each bound UDP socket must be on `127.0.0.1`, `::1` or `::ffff:127.0.0.1`; no other
  address that maps IPv4 counts as loopback. If one is not, the daemon ends the process tree at once, reports
  `failed` with the cause `listener`, and does not start that server again until a new `server_start`. Between
  two checks a listener can live for at most 15 seconds; only the isolation closes that time.
- **Memory and disk.** The daemon ends a server whose memory is above its share of the device policy and 10 %
  (`limit_memory`). On Linux it also sets `memory.max` of a cgroup of the server, or `RLIMIT_AS` where no cgroup
  is available. The temporary folder of a server has a limit of 2 GiB; its bytes count in the report
  (`tempBytes`), and a server above the limit is ended (`limit_disk`). The disk reserve of Section 7 is checked
  for each write of model work, also in the runtime and temporary folders.
- **The supervision (CAG-005).** When a server process ends and no item asked for it, the daemon starts it again
  with the same settings after a wait of 1, 2, 4, 8, 16, 32, 64, 128, 256 seconds, then each 300 seconds. The
  daemon counts each end that no item asked for, also of a server that was `ready` for a long time. After 5 in one
  hour or 20 in 24 hours, the server stays `failed` until a new `server_start`; only a new `server_start` that
  then stays `ready` for 10 minutes sets the counts to 0. When a server ends during a `model_load` or in the 60
  seconds after it, the copy of that load is `failed` with the cause `load_crash`, and the daemon does not load
  it again by itself. Each restart and each `failed` is an event of the
  runtime report (`restarts`) and an entry in the log of the machine. The function of the waits is pure, in
  `daemon-core`, with a test clock (CAG-007).
- On a stop of the daemon, the daemon stops each server. At its start it starts no server by itself; the hub
  sends `server_start` again from its record. See open point MW-P5.

## 10. The folders that model work writes (CAG-009, MET-002)

Model work writes only in these folders, and below them. It changes nothing else on the machine: no system
setting, no file of a user, no package, no service unit.

| Installation | Runtimes | Model cache | State |
| --- | --- | --- | --- |
| macOS, user (launchd agent) | `~/Library/Application Support/anima-daemon/runtimes/` | `~/Library/Application Support/anima-daemon/models/` | `~/Library/Application Support/anima-daemon/model-state/` |
| Linux, user (systemd user unit) | `$XDG_DATA_HOME/anima-daemon/runtimes/` (default `~/.local/share/anima-daemon/runtimes/`) | `$XDG_DATA_HOME/anima-daemon/models/` | `$XDG_STATE_HOME/anima-daemon/model-state/` (default `~/.local/state/anima-daemon/model-state/`) |
| Linux, system unit | `/var/lib/anima-daemon/runtimes/` | `/var/lib/anima-daemon/models/` | `/var/lib/anima-daemon/model-state/` |
| `ANIMA_DAEMON_HOME` set | `<home>/runtimes/` | `<home>/models/` | `<home>/model-state/` |

- The configuration of the daemon (release sources, the approved list of the machine, the port range, the disk
  reserve) stays in the daemon folder of K8 Section 2 (`config/home.rs`). The hub cannot write it.
- The daemon makes each folder with mode 0700 and refuses to use one that a different user owns, or that a link
  replaces. The system unit adds the three folders to its `ReadWritePaths` and nothing else.
- On Linux the user folders are below `XDG_DATA_HOME`, not `XDG_CONFIG_HOME`, because a model cache is large and
  is not configuration. A macOS folder below `~/Library/Caches` is not used, because the system can delete it.
- A runtime process writes only to a temporary folder below the state folder, which the daemon empties at each
  start of that process (limit: Section 9).
- The daemon makes, moves and deletes each file and folder of model work relative to an open handle of its
  folder (`openat`, `renameat`, `unlinkat`), with `O_NOFOLLOW`. `copy_delete` and each clean-up follow no
  symbolic link.
- MET-002 gets one more exception, which R-165 decided (OD-C9): "the daemon does the model work of the fixed list
  of CAG-003, and with it changes only its own folders (CAG-009)". The text of MET-002 changes with
  R-NEW-modelwork.

### 10.1 Settings at the machine

The file `model-work.json` in the daemon folder of K8 Section 2. Only the owner of the machine writes it; the hub
cannot write it, and an update of the daemon does not change it (as the switch, K10 Section 3, SF-007). Each
stricter value is permitted; the defaults are the answers of the operator.

| Setting | Default | Stricter form |
| --- | --- | --- |
| `sandbox` | `false` (R-NEW-sandboxrisk) | `true`: Section 9, the isolation |
| `secondHash` | `false` (R-NEW-hubhash) | `true`: Section 7, the hash of Hugging Face |
| `revisionAgeDays` | `0` (R-NEW-hubhash) | Days, for example 7: Section 7 |
| `organisations` | `unsloth`, `mlx-community`, the publishers of the release (R-NEW-mlxcommunity) | A shorter list |
| `agentTypes` | The ten types (R-NEW-agentmodelwork) | A shorter list, for example `model_report`, `server_stop`, `model_unload`, `model_drain` |

The daemon reports the values in `model_report`, so the hub can show them. The plain controls (Section 2.2, the
hash of the hub, Sections 4, 6 and 10) are not settings.

## 11. The states of a copy (CMP-006, PULL-011)

The daemon keeps the state of each copy in its state folder. The hub keeps the record (CMP-006).

```text
            model_pull                       model_measure (correct)
(none) ──────────────► downloading ──► downloaded ──► validating ──► validated
                           │                ▲              │
                           │                └──────────────┤ (probe_failed, start_failed)
                           ▼                               ▼
                         failed ◄──────────────────────── failed
            copy_delete from each state ──► (none)
```

- `downloading`: the files are in `partial/`. `downloaded`: each file is verified and the copy was not validated
  on this node. `validating`: a `model_measure` runs. `validated`: the fixed request got a correct answer on this
  runtime and this hardware. `failed`: the copy cannot be used; `cause` says why. A `model_pull` of a `failed`
  copy starts it again.
- A `validated` copy goes back to `downloaded` when the runtime version or the backend that validated it changes.
- **Hook for task L1-15 (not defined here).** The states `loading`, `loaded` and `active` of a copy on a server,
  and the change between them, are for the contract of loading and readiness (task L1-15, FIT-020 to FIT-026).
  This version reports only `loaded` (the list `servers[].loaded` of Section 8). L1-15 adds its states as members
  of `servers[]`, not as new states of a copy in this diagram.

## 12. Open points of this text

| Point | Question | Proposal |
| --- | --- | --- |
| MW-P1 | MET-020 permits the daemon only the hub. A pull, an install and a measure need Hugging Face, a release host and loopback | A child fetch process and a child probe process, with no token and fixed hosts (Section 4); MET-020 gets that exception in its text with R-NEW-modelwork |
| MW-P7 | The SHA-256 comes from the hub (F4) | Decided (R-NEW-hubhash): the hub hash only; `secondHash` and `revisionAgeDays` are settings at the machine, off |
| MW-P8 | Is `mlx-community` in the default list? (F5) | Decided (R-NEW-mlxcommunity): yes; `organisations` is a setting at the machine |
| MW-P9 | A runtime runs with the rights of the user of the daemon (F2) | Decided (R-NEW-sandboxrisk): no required isolation; `sandbox` is a setting at the machine, off |
| MW-P2 | Does the machine hold its own list of approved organisations, or does it trust the list of the hub? | The machine holds its own list (Section 7), as the list of repositories of R-185 |
| MW-P3 | Is the runtime report a member of the push batch or a report of the work route? | The push batch (Section 8) |
| MW-P4 | How is an archive of `mlx_lm` or `vllm` with its Python environment made and signed? | Anima builds it with the release (task L1-6 decides the recipe); until then only `llama_server` from its upstream releases |
| MW-P5 | Does the daemon start its servers again by itself after its own restart? | No: the hub sends `server_start` again, so that the machine runs nothing that the hub does not know |
| MW-P6 | Can an AI agent through an API key make a model item? (F15) | Decided (R-NEW-agentmodelwork): each type; `agentTypes` is a setting at the machine |

## 13. What a person who controls the hub can do with this contract

On a machine that takes work, a person who controls the hub can: install a runtime from a release source that the
machine holds; download files of the permitted types from an approved organisation of the machine (by default
also `mlx-community`), at a commit of their choice and with their own hash (R-NEW-hubhash), and fill the disk
share; load such a file in a runtime, whose parser can have faults (SEC-C-009), with the rights of the user of
the daemon when `sandbox` is off (R-NEW-sandboxrisk), and so take that account; use memory and power
within the device policy; stop each model server. The person cannot name a command, a flag that the daemon owns,
an address, a path or a host outside the lists of the machine. The security review (SEC-C-012) reads this
section.
