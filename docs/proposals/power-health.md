---
kind: Proposal
state: Draft
evidence: Version 0.1 (2026-11-14). It was `power-health-plan.md` in `plans/` until 2026-11-17: nothing in it is approved, so it is a proposal (PLN-018). Its header says: for operator review, nothing in this plan is accepted. `PWR`, `HWH`, `HWI` are proposed. `HWH-001` to `HWH-008` are its requirements; `HWH-001` to `HWH-007` had the prefix `HLT` until 2026-11-17; the Health extension of the extensions plan owns `HLT` (PLN-016). No tier in Section 4 yet.
---

# Anima Power and Hardware Health Plan

| Item | Value |
| --- | --- |
| Document | Power use, hardware health and hardware inventory for each device |
| Version | 0.1 (draft from an ideation session; no ruling yet) |
| Date | 2026-11-14 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | The header of this document holds the state (PLN-020). History: Draft. For operator review. Nothing in this plan is accepted. |
| Base | Branch `main` at `baa7eeeb`. All paths and requirement IDs in this plan are on that branch. |
| Related | [design-spec.md](../spec/design-spec.md) Section 6 (MET-*), [compute-plan.md](../plans/compute-plan.md) Section 3.1 (CAP-*), [extensions-plan.md](../plans/extensions-plan.md), [one-click-plan.md](../plans/one-click/one-click-plan.md), `contracts/daemon-push.md` (K8) |

This document obeys the writing rules in Section 0 of the design specification.

## 1. Purpose and scope

### 1.1 Purpose

Anima shows the load of each device: CPU, memory, disks and GPUs. It does not show how much
power a device uses, the health of its parts, or which parts it has. This plan adds the three.

The operator gets these answers:

- **Power.** How much power does each computer use? What does it cost in a month? What on the
  computer uses the most power? Which computer is on all day and does no work?
- **Health.** Is a drive or a battery near the end of its life? Does a computer get too hot?
- **Inventory.** Which parts does a computer have? Can it take more memory or one more drive?

### 1.2 What the plan cannot do

These limits are facts of the hardware. The interface must say them plainly.

| Wish | Limit |
| --- | --- |
| The wall power of a desktop computer | Software cannot read the power supply. Only a power meter at the plug or a server management controller gives this value. |
| Watts for each program | Operating systems give a share or a score, not watts. Anima shows an order and an estimate. |
| A forecast of a memory fault | Memory does not wear slowly in a way that software can read. Only memory with ECC counts corrected errors. Most Macs and home computers do not have it. |
| The size of the power supply | No software interface gives it. The operator must type it. |

### 1.3 Ideas that this plan does not take

| Idea | Reason |
| --- | --- |
| A memory test from Anima | A real test needs a restart into a test program. It is out of scope. |
| Automatic power actions (sleep a computer that is idle) | It changes a device (MET-002). It can be a later plan with operator rules. |
| A carbon value from a live grid service | A new outside dependency. A fixed value that the operator sets is enough at the start. |

## 2. Rulings and constraints

| ID | Source | Effect on this plan |
| --- | --- | --- |
| MET-002 | spec Section 6 | The daemon and the SSH collector must only read. All readers in this plan only read. |
| MET-016 | spec Section 6 | A parser is a pure function in `daemon-core`. The daemon and the SSH collector both call it. |
| MET-013, K8 | `contracts/daemon-push.md` | The daemon token permits only push and renew. New readings go in the push body. K8 is a frozen contract; a change needs a programme ruling. |
| MET-014, MET-015 | spec Section 6 | Points are at 10 seconds. Health and inventory change slowly and must not use this rate. |
| MET-026 to MET-029 | spec Section 6 | Storage is shown drive by drive. Drive health joins these rows. |
| CAP-004 | compute-plan.md | The daemon already reports mains or battery and the thermal state for compute nodes. This plan uses the same readers; it does not make a second set. |
| CHK-012 | spec Section 6 | The hub and the daemon must not start a command with operator text. Each command here is fixed text. |
| DEV-026 | spec Section 6 | "Needs attention" is the place for a health warning. |
| INT-003, CLI | spec Section 3, `docs/plans/cli-mcp-plan.md` | AI agents and scripts read through the GraphQL API. New values in K1 are then available to the CLI and to MCP with no extra path. |
| K1, K2 | `contracts/schema.graphql`, `contracts/database.md` | Frozen contracts. New fields, tables and migration ranges need a programme ruling. |
| R-076 | `docs/rulings.md` | No Windows work. |
| one-click-plan.md Section 1.2 | this branch | The one-click install needs no administrator rights. Section 4 of this plan keeps that true. |
| Limits | AGENTS.md | 300 code lines for each file, 12 files for each folder. |

## 3. Facts

Items 1 to 4 are from `main`. Items 5 to 12 are general platform facts; each is marked "check"
and must be proved in task P0 before a builder relies on it.

1. The readers are in `crates/daemon/src/readers/` (fetch) and `crates/daemon-core/src/readings/`
   (pure parsers: `linux.rs`, `macos.rs`, `drives.rs`, `df.rs`, `gpu/`). A command is fixed
   argv with no shell and a time limit (`readers/command.rs`).
2. The agent runs as its own user `anima-daemon` (system unit), as the signed-in user (user unit),
   or as a LaunchAgent on macOS. None of these is root.
3. `nvidia-smi` and `amdgpu` sysfs output is already parsed for GPU load. Both also give GPU power.
4. The client has `$lib/metrics/` with `storage/`, `bars/` and overview charts.
5. Laptop (check): the battery controller gives voltage and current, thus the power of the whole
   computer. Linux: `/sys/class/power_supply`. macOS: `ioreg` (`AppleSmartBattery`). No root.
6. Linux CPU power (check): `/sys/class/powercap` (RAPL) gives energy counters for Intel and AMD.
   Since 2020 the counter file is readable by root only.
7. macOS power (check): `powermetrics` gives CPU and GPU power and the energy of each program. It
   needs root. `top -stats power` gives a power score for each program with no root.
8. Drive health (check): `smartctl -j` gives SMART data for SATA and NVMe drives. It needs root and
   the `smartmontools` package. macOS `diskutil info` gives only "Verified" or "Failing", with no root.
9. Battery health (check): cycle count and full capacity against design capacity, with no root, on
   Linux and macOS.
10. Temperature and fans (check): Linux `hwmon` sysfs, no root. macOS gives only a thermal pressure
    level with no root.
11. Memory errors (check): Linux EDAC sysfs gives corrected and uncorrected counts, only with ECC memory.
12. Inventory (check): macOS `system_profiler -json` gives memory, storage and model with no root.
    Linux `dmidecode` (memory slots, maximum memory) needs root; `lsblk` and `/proc` do not.

## 4. Access levels

Many readers need root (facts 6, 7, 8 and 12). The daemon must not need root to start.

| Level | Needs | Gives |
| --- | --- | --- |
| Basic | Nothing more than today | Laptop power, GPU power, battery health, temperatures on Linux, the macOS power score for each program, macOS drive verdict, most inventory |
| Full hardware readings | One set-up step with administrator rights, started by the operator | CPU power, the energy of each program on macOS, SMART detail, memory slots on Linux |

Proposed design for the full level: a small root helper that runs only a fixed list of read
commands and gives the output to the daemon. The daemon stays without root. The helper takes no
text from the hub or the operator (CHK-012). This design has a security tag.

## 5. Work items

### 5.1 P0. Prove the readers

Capture real output of each command in Section 3 on the operator's Macs and Linux computers.
Put the captures in the `daemon-core` test fixtures with invented names and serial numbers. Mark
each fact proved or wrong. No product code.

### 5.2 P1. Power

Proposed requirements (prefix `PWR`, free on `main`):

- **PWR-001.** For each device with a source, the hub must store the power in watts with the metrics.
- **PWR-002.** Each power value must have a source: "measured" (battery controller, power meter),
  "partial" (CPU and GPU only) or "estimated". The client must show the source with the value.
- **PWR-003.** The device detail view must show a power chart for 1 hour, 24 hours and 7 days (as MET-003).
- **PWR-004.** The hub must show the energy in kWh for a day, a week and a month for each device and for all devices.
- **PWR-005.** The operator can set a price for each kWh and a carbon value for each kWh in Settings.
  The client then shows the cost and the carbon with the energy.
- **PWR-006.** The device detail view must show the programs that use the most power, in order.
  The client must name this an estimate. Do not show watts for a program unless the source gives watts.
- **PWR-007.** The hub must show idle cost: the energy that a device used while its CPU and GPU
  load was low. The limit for "low" is a constant at the start.
- **PWR-008.** A device with no power source must show "No power reading", with the reason and
  what gives one (the full level, or a power meter). Do not show 0 W.
- **PWR-009.** A program name comes from the device. Limit its length and permit only printable
  characters before the hub stores it.

### 5.3 P2. Health

Proposed requirements (prefix `HWH`, free on `main`):

- **HWH-001.** For each drive (MET-027), the hub must store the health verdict and, at the full
  level: reallocated and pending sectors, media errors, the used life of an SSD, power-on hours
  and temperature.
- **HWH-002.** The daemon must read health values once each hour, not with each 10-second point.
- **HWH-003.** The Storage section (MET-029) must show the health of each drive in its row.
- **HWH-004.** The hub must open a "Needs attention" problem (DEV-026) when: a drive verdict is
  failing; pending or reallocated sectors increase; the used life of an SSD is more than 90
  percent; uncorrected memory errors are more than zero.
- **HWH-005.** For a battery, the hub must store the cycle count and the full capacity as a
  percentage of the design capacity. The client must show both.
- **HWH-006.** The hub must store temperatures and fan speeds where the device gives them, and
  must show when a device limits its speed because of heat.
- **HWH-007.** For memory with ECC, the hub must store the corrected and uncorrected error
  counts. For memory without ECC, the client must show "This memory cannot report errors". Do
  not show "Healthy".
- **HWH-008.** A health event must go to notifications as a check event does.

### 5.4 P3. Inventory

Proposed requirements (prefix `HWI`, free on `main`):

- **HWI-001.** The hub must store for each device: the model, the CPU, the memory modules (size,
  type, speed), the memory slots used and free, the maximum memory, the drives and their
  interface, and the GPUs.
- **HWI-002.** The daemon must read the inventory at start and once each 24 hours (as MET-006).
- **HWI-003.** The device detail view must have a Hardware section with the inventory.
- **HWI-004.** The Hardware section must show upgrade facts: free memory slots and the maximum
  memory, free drive interfaces where the device reports them, and "memory is fixed" for a
  computer whose memory cannot change.
- **HWI-005.** The operator can type values that no reader gives, for example the size of the
  power supply. The client must show that the operator typed the value.
- **HWI-006.** A serial number is not secret, but the hub must not write it to a log (LOG-007).
- **HWI-007.** The hub must write an event when the inventory of a device changes, for example
  when a drive or a memory module is gone.

### 5.5 P4. Power meters (extension)

A smart plug that measures power gives the wall power of any device, also a device with no
daemon. This is an extension source ([extensions-plan.md](../plans/extensions-plan.md)), not daemon work.
The operator links one plug to one device. Its value is "measured" (PWR-002) and replaces a
partial value. The choice of plug types needs a short research task.

### 5.6 P5. API, CLI and AI agents

The new values go in K1. The CLI and the MCP tools then read them as they read metrics. Add one
query that gives, for one device, the power, the health problems and the upgrade facts in one
answer. An AI agent can then answer "why is this computer slow?" or "what uses the power?".

## 6. Contracts and storage

| Contract | Change |
| --- | --- |
| K8 (`daemon-push.md`) | Power joins the 10-second points. Health and inventory are new slow bodies in the same push call, so the token rule of MET-013 does not change. |
| K1 (`schema.graphql`) | Power fields on metrics; `health` and `hardware` on a device; the Settings values of PWR-005. |
| K2 (`database.md`) | Power columns in the metric tables and aggregates. New tables for health readings, inventory and its history. A migration range from the programme. |
| Hub domains | Health and inventory are new concepts beside `metrics`. Whether they are one domain or two is a builder decision after P0 (`quality/hub-modules.json` is a contract). |

## 7. Order and dependencies

```text
P0 prove readers ─┬─> P1 power (basic) ──┬─> P5 API and CLI
                  ├─> P2 health (basic) ─┤
                  └─> P3 inventory ──────┘
Root helper (Section 4) ─> the full level of P1, P2, P3
Extensions engine ─> P4 power meters
```

P0 to P3 at the basic level do not need the one-click plan, the root helper or the extensions
engine. They can start after a ruling.

## 8. Risks

| Risk | Effect | Answer |
| --- | --- | --- |
| A value looks exact but is an estimate | The operator trusts a wrong number | PWR-002, PWR-006. The source is always shown. |
| The root helper | A fault gives root to an attacker | Fixed command list, no input text, security review. The basic level works without it. |
| Readers differ between computers and versions | Missing values | P0 captures. A missing value shows a reason (PWR-008), not zero. |
| Too many health warnings | The operator stops reading them | HWH-004 uses changes and firm limits only. |
| The SSH collector cannot use the root helper | Devices with no daemon get the basic level only | Accepted. Say it in the client. |

## 9. Acceptance criteria

1. A Mac laptop and a Linux laptop show a measured power chart with no administrator step.
2. A desktop computer with no power source shows "No power reading" and the reason, not 0 W.
3. The month view shows kWh, cost and carbon for each device, with the operator's values.
4. A drive with a failing verdict in a test capture opens a "Needs attention" problem and sends a notification.
5. A computer with fixed memory shows "memory is fixed". A computer with free slots shows the number.
6. The CLI gives the power, health and hardware of a device with one command.
7. With the full level off, the agent runs no command as root.

## 10. Open questions for the operator

| ID | Question | Recommendation |
| --- | --- | --- |
| Q1 | Is the root helper (Section 4) accepted as a design? | Yes, after a security review. Ship the basic level first. |
| Q2 | Does the SSH collector also get these readers, or the daemon only? | Daemon first. Add the basic readers to the SSH collector later (MET-016 makes this cheap). |
| Q3 | Power meters: which plug types first? | Decide after the research task in P4. |
| Q4 | Is this a part of Phase 3 metrics work or a new phase? | A new package set after the one-click spikes report, so the two do not compete for the same builders. |
| Q5 | Does the hub keep the power use of each program over time, or only the latest list? | The latest list only at the start. History is large and its value is not proved. |
