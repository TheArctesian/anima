# Phase 3 delta: metrics push agent (R-084)

Status: **Approved by the operator on 2026-11-07.** The text is in `docs/design-spec.md` (R-084).

Operator decision: 2026-11-07. Ruling: R-084. Contract change: CR-8. New package: WP-54.

## 1. §6.6 Resource metrics

### 1.1 Replace the "Metrics sources" list with

Metrics sources, in order of preference:

- **Anima agent (push).** A small headless program, `anima-agent`, runs on the device. It
  reads the values on the device and pushes them to the hub over the VPN. Use the agent on
  each macOS and Linux device that can run software.
- **SSH (agentless).** This is the fallback for devices that cannot run software, for example
  routers and NAS appliances. The hub connects with SSH and reads system values. On Linux,
  the hub reads `/proc`. On macOS, the hub uses `vm_stat`, `sysctl` and `df`.
- **Proxmox API.** The hub reads the node, virtual machine and container status from the
  Proxmox VE API.

The agent and the SSH collector parse the same command output and files with the same code
(MET-016).

### 1.2 Replace the GPU table heading and the Apple silicon row

GPU metrics sources (agent, or SSH as the fallback):

| GPU type | Command or file | Notes |
| --- | --- | --- |
| Apple silicon (macOS) | Agent: IOKit `IOAccelerator` performance statistics (Device Utilization %, In use system memory, `gpu-core-count`); IOReport for GPU power where macOS gives it. SSH: `ioreg -r -d 1 -c IOAccelerator` (GPU usage only). | The GPU shares system memory (unified memory). Do not show a VRAM value (MET-007). The agent shows GPU usage and the GPU's share of memory in use as a number, not as a VRAM bar. |

The NVIDIA, AMD and Intel rows stay as written. The agent may read NVIDIA values through NVML
in place of `nvidia-smi`. The values must be the same.

### 1.3 Changes to existing requirements

- **MET-001 (changed).** The SSH collector must collect metrics every 30 seconds. The agent
  rules are MET-014 and MET-015.
- **MET-002 (changed).** The SSH collector and the agent must only read values on the device.
  Neither must change the device. The agent may write only its own configuration, token and
  buffer files.

MET-003 to MET-010 stay as written. They apply to readings from every source.

### 1.4 New requirements

- **MET-011.** A device's metrics source must be one of `agent`, `ssh`, `proxmox` or `none`.
  When a device has a working agent, the hub must stop SSH metric collection for it. If the
  agent is removed or its token is revoked, the operator can switch the device back to `ssh`.
- **MET-012. Enrolment.** The operator makes an enrolment code for one device on the Device
  page ("Install agent"). The code must:
  - be 128 random bits;
  - expire after 15 minutes;
  - work only once. A second use must fail, even when the first use failed later in the
    exchange;
  - be kept by the hub only as its SHA-256.

  `anima-agent enrol <code>` swaps the code for the device's agent token. Only a signed-in
  operator can make a code. An API key must not make one (add `enrolAgent` to the list in
  INT-003).
- **MET-013. Agent token.**
  - The format is `anima_agent_` followed by 43 base64url characters. The hub keeps only its
    SHA-256.
  - A token is bound to one device. It allows only two calls: push readings for that device,
    and renew itself. It must not call the GraphQL API, open a session or read anything.
  - It expires after 365 days. The agent renews it during the last 30 days with the current
    token. The old token stops working when the new one is first used.
  - The operator can revoke it on the Device page. Revocation takes effect on the next push.
  - The agent keeps it in the macOS Keychain. On Linux, it keeps it in a file with mode
    `0600`, owned by the user that runs the agent.
  - No log, event, metric or error message must contain a token or an enrolment code
    (LOG-007).
- **MET-014. Sampling and push.**
  - The agent samples every 2 seconds.
  - Each 10 seconds, it makes one point from the samples: the mean, plus the maximum for CPU
    and GPU usage.
  - It pushes the new points every 10 seconds, in one batch.
  - While the hub cannot be reached, it keeps up to 1 hour of points in memory, oldest
    dropped first. It sends them when the hub comes back.
- **MET-015. Stored resolution.** The hub must store agent points at 10-second resolution.
  The existing aggregates and retention (D-25, CR-6) apply to every source.
- **MET-016. One parser.** The parsers for `/proc`, `vm_stat`, `sysctl`, `df`, `nvidia-smi`,
  `amdgpu` sysfs, `intel_gpu_top -J` and `ioreg` must be pure functions in `agent-core`. The
  hub's SSH collector and the agent must both call them.
- **MET-017. Ingest checks.** The hub must refuse a batch, and write the refusal to the event
  log without the token, if any of these is true:
  - the token is unknown, expired or revoked;
  - the batch names a device other than the token's device;
  - a point is more than 1 hour old or more than 60 seconds in the future;
  - the batch is larger than 512 KiB.

  The ingest endpoint must follow NET-002 and NET-003. It must not be reachable from outside
  the VPN.
- **MET-018. Stale readings.** If the hub gets no batch from an agent for 3 push intervals
  (30 seconds), the hub must mark the device "metrics stale". The device card and the detail
  view must show the last values in the muted colour with the label "Stale". The hub must
  write one event when a device becomes stale and one when it recovers. A stale agent is not
  a Down check.
- **MET-019. Agent install.**
  - The Device page shows "Install agent" with the install command and a fresh enrolment code.
  - The desktop app can install the agent on the computer it runs on. The Tauri shell does the
    OS work: it copies the binary and writes the LaunchAgent or systemd unit. The client's
    `lib/metrics/` holds the logic.
  - On Linux, the agent runs as a systemd user or system unit. On macOS, it runs as a
    LaunchAgent.
- **MET-020. Agent network.**
  - The agent must connect only to the hub, with HTTPS to `hub.anima.ing` on tcp 443 at the
    hub's VPN address.
  - The agent must not listen on any port in Phase 3.
  - The VPN policy must allow agent devices to reach only that port of the hub (VPS-005 for
    each live change).

## 2. §6.9 Integrations: INT-003

Add `enrolAgent` and `revokeAgentToken` to the list of calls an API key must not make.

## 3. §9 Delivery phases

Phase 3 row, "Content": append "Metrics push agent for macOS and Linux (`anima-agent`), with
GPU usage on Apple silicon."

## 4. §13 Repository structure

Under `crates/`, after `hub/`:

```text
│   ├── agent/                 The metrics agent binary (anima-agent): enrolment, sampling, push, OS readers
│   ├── agent-core/            Agent logic with no I/O: parsers (MET-016), points, batching, buffer, wire types
```

Change the `metrics/` line to:

```text
│   │   │   ├── metrics/       Collection (agent ingest, SSH, GPU, Proxmox), aggregates, retention
```

`hub` depends on `agent-core` for the parsers and wire types. `agent-core` depends on no other
Anima crate.

## 5. Compute plan alignment (no change made now)

`docs/compute-plan.md` is on branch `claude/distributed-compute-devices-25e110`. It names the
crates `compute-agent` and `compute-agent-core` (CAG-001, CAG-007). It also has the hub open
connections to an agent port (SEC-C-002, SEC-C-004).

Under R-084 those become the same `agent` and `agent-core` crates. Open decision for the
compute plan: keep "agent opens no port". Hub commands would then travel over a connection
the agent starts. That change goes on the compute branch, not here.
