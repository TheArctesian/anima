# The installed runtimes: the text for the second approval (R-NEW-runtimelist)

Status: the operator approved the list with two changes (R-NEW-runtimelist): "Approve, add Ollama" and "Add the
field". This text has both. He then approved the wording of `backend` with one fix: "Approve, one fix
(Recommended)". The text is in model-work.md Section 8 on the branch compute/l1-3.

The operator selected "Separate list" (R-NEW-runtimelist): the model runtimes are not names of the K8
tool inventory. They have one list of their own, defined once, in the runtime report of the model-work
contract (`contracts/compute/model-work.md` Section 8, on the branch compute/l1-3, which is not merged).
The text below replaces the `installed` part of that section. `contracts/daemon-push.md` Section 4 gets
only the reference in the last part of this file.

## The text for model-work.md Section 8

Replace the example line `"installed": [ { "runtime": "llama_server", "version": "b8100", "backend": "metal" } ],`
with:

```json
  "installed": [
    { "runtime": "llama_server", "version": "6040", "backend": "metal" },
    { "runtime": "mlx_lm", "state": "no_version", "backend": "metal" },
    { "runtime": "ollama", "version": "0.12.6", "backend": null }
  ],
```

Add these items after the list of `state` of a server:

- `installed` is the list of the installed model runtimes (CAP-003, R-NEW-runtimelist). It is the only list of
  the runtimes; the tool inventory of K8 Section 4 does not hold them. It holds at most 16 entries with unique
  names. Each entry has a `runtime` and either a `version` of 1 to 64 bytes of `[0-9A-Za-z.+_-]` or a `state`,
  never both. The `state` is a fixed code, never text from the device: `timed_out`, `exited` or `no_version`,
  with the meanings of the tool inventory (CAP-024); the hub reads a `state` it does not know as `failed`. A
  runtime the daemon did not find is not listed.
- The runtimes the hub knows are `llama_server` (the program `llama-server`), `mlx_lm` (the program
  `mlx_lm.server`, macOS only), `vllm` (the program `vllm`) and `ollama` (the program `ollama`). Ollama is
  listed only so that a machine that has it shows up; it is not a runtime that the model service uses, and
  task L1-18 still replaces it (the Ollama form at the model address). The hub drops an entry of any other name, so a
  newer daemon can talk to an older hub.
- The daemon reads each runtime with the fixed-argument probe of the tool inventory: the program from the fixed
  tool folders, the one argument `--version`, 5 seconds at most, no shell, no model started, the first version
  on standard output (K8 Section 4, CAP-022, CAP-023). It reads them again at each probe of the inventory
  (each 15 minutes).
- Each entry has a `backend`: the backend the runtime uses on this machine, one of the backend names of the K8
  member `hardware` (`metal`, `cuda`, `rocm`, `vulkan`, `cpu`), or null when the daemon cannot find it. The
  member is always present. The daemon finds it with fixed rules, never from free text:
  - `mlx_lm`: `metal` (MLX runs only on Metal).
  - `llama_server`: a second fixed-argument probe, `llama-server --list-devices`, with the same limits. The first
    device line that starts with `Metal`, `CUDA`, `ROCm` or `Vulkan` gives `metal`, `cuda`, `rocm` or
    `vulkan`; output with no such line gives `cpu`; a probe that fails gives null.
  - `vllm`: the build tag after `+` in the version that it prints: `rocm…` gives `rocm`, `cpu` gives `cpu`,
    `cu…` gives `cuda`; no tag or any other tag gives null.
  - `ollama`: Ollama selects its backend when it loads a model, so the daemon takes it from the hardware
    verdicts: on macOS `metal`; on Linux the first usable of `cuda`, `rocm` and `cpu`; null when none is usable.
  - A runtime with a `state` (not usable) still has a `backend` when the rule finds one, else null.
- When `backend` and the hardware verdict disagree, the hardware verdict wins: a runtime whose `backend` is not
  usable in `hardware` is taken as not able to use that backend, and the hub and the placement read the
  verdict. The daemon still sends the `backend` that it found, so the operator sees the disagreement.
- Before a model server runs, the daemon sends `runtimes` with `installed` only (`servers` and `copies` empty,
  `cacheBytes` 0): after start, each time the list changes, and once every 24 hours when unchanged, as the
  tool inventory (CAP-027). When a server runs, the 15-second rule above applies.

Change "At most 16 servers, 64 copies and 16 installed runtimes." to "At most 16 servers and 64 copies."
(the limit of `installed` is in its item).

## The reference in daemon-push.md Section 4

After the list of the tool names, the text is now: "the model runtimes are not tools: they have their own list,
R-NEW-runtimelist". After the approval it becomes:

> The installed model runtimes are not tools. They are in the optional member `runtimes`, which
> model-work.md (a link to `compute/model-work.md`) Section 8 defines (R-NEW-runtimelist).

## The answers of the operator

1. "Approve, add Ollama": `ollama` is in the list of known runtimes, with the note that task L1-18 still replaces
   it.
2. "Add the field": each entry has a `backend`, with the rules above.
3. "Approve, one fix (Recommended)": he approved the wording of `backend` with one fix in the rule of `vllm`
   (no tag gives null, not `cuda`).
