# The installed runtimes: the text for the second approval (R-NEW-runtimelist)

Status: waits for the approval of the operator. No code sends this list before the approval.

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
    { "runtime": "llama_server", "version": "6040" },
    { "runtime": "mlx_lm", "state": "no_version" }
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
  `mlx_lm.server`, macOS only) and `vllm` (the program `vllm`). The hub drops an entry of any other name, so a
  newer daemon can talk to an older hub.
- The daemon reads each runtime with the fixed-argument probe of the tool inventory: the program from the fixed
  tool folders, the one argument `--version`, 5 seconds at most, no shell, no model started, the first version
  on standard output (K8 Section 4, CAP-022, CAP-023). It reads them again at each probe of the inventory
  (each 15 minutes).
- Before a model server runs, the daemon sends `runtimes` with `installed` only (`servers` and `copies` empty,
  `cacheBytes` 0): after start, each time the list changes, and once every 24 hours when unchanged, as the
  tool inventory (CAP-027). When a server runs, the 15-second rule above applies.

Change "At most 16 servers, 64 copies and 16 installed runtimes." to "At most 16 servers and 64 copies."
(the limit of `installed` is in its item).

## The reference in daemon-push.md Section 4

After the list of the tool names, the text is now: "the model runtimes are not tools: they have their own list,
R-NEW-runtimelist". After the approval it becomes:

> The installed model runtimes are not tools. They are in the optional member `runtimes`, which
> [model-work.md](compute/model-work.md) Section 8 defines (R-NEW-runtimelist).

## The points the operator decides with this approval

1. `ollama` is not in the list. The compute plan selects llama-server, MLX-LM and vLLM (RTC-001); Ollama is
   a separate server that the model service replaces (L1-18 gives the Ollama form at the model address). The
   daemon of task L1-1 reported it; this text does not.
2. The member `backend` of an installed runtime is removed. The backend verdicts are in the K8 member `hardware`
   (CAP-004), and a runtime build does not say its backend in its `--version` output in a form that the daemon
   reads without free text.
