# Proposal: "Add this machine" after sign-in

From: the orchestrator. The operator asked for this in chat on 2026-11-06 ("on sign in it should
ask in a popup modal add this machine if the machine is not already added … identified across
networks") and chose the machine ID over the MAC address.
Status: a proposal for a ruling and for issue #1 (R-007). Not Phase 1 scope (contracts frozen,
CON-001). It needs WP-13's Add device form, which it reuses.

## Problem

The computer the operator signs in from is usually a device they also want to reach from
elsewhere. Today they must add it by hand, look up its VPN address and know which services are
on. Nothing tells the app that "this machine" is already a device, so it cannot offer to add it
once and then stay quiet.

## Why not the MAC address

- macOS, iOS, Windows and Android give each Wi-Fi network a different private MAC address on
  purpose, so the same Mac looks like a new machine on each network.
- A computer has several MAC addresses (Wi-Fi, Ethernet, Thunderbolt bridge, dock adapters), and
  the hub, behind the VPN, never sees any of them.

## Proposal

1. **Machine ID.** The Tauri shell (OS work, Section 13) reads the operating system's stable ID:
   `IOPlatformUUID` on macOS, `/etc/machine-id` on Linux, `HKLM\SOFTWARE\Microsoft\Cryptography\
   MachineGuid` on Windows. It sends only `HMAC-SHA256(key = "anima machine id v1", message = raw
   ID)` in hex, so the raw hardware ID never leaves the machine and is not reused by other apps.
   The value is not a secret but is not logged either. The web client has no machine ID and never
   shows the prompt.
2. **Device field.** A device gets an optional, unique `machineId`. The hub stores it as given and
   never derives anything from it. A query `deviceByMachineId(machineId)` returns the device or
   null.
3. **The prompt.** After sign-in, and on start when already signed in, the desktop app asks the
   hub for its machine ID. If no device has it and the operator has not chosen "Don't ask on this
   machine" (kept in the app's local settings, per machine), a modal asks **Add this Mac to
   Anima?** with the Add device form pre-filled:
   - name: the computer name (for example "Stephen's MacBook Pro");
   - address: the machine's own VPN address (100.64.x.x), read from the VPN interface, so it is
     the same on every network;
   - connections: SSH if port 22 answers on that address, VNC if 5900 answers, RDP if 3389
     answers, each with the defaults the form already uses;
   - the machine ID, hidden.
   Actions: **Add**, **Not now** (ask again next sign-in), **Don't ask on this machine**.
4. **Already added.** If a device has the machine ID, nothing is shown, whatever network the
   machine is on. If its VPN address changed (a re-enrolled node), the app offers to update the
   device's address instead.
5. **Audit.** Adding writes the usual device-created event (LOG-005); the event says it came from
   the prompt.

## Needs

- A spec change: a desktop requirement for the prompt, the `machineId` field and query, and the
  hashing rule.
- A K-contract change (devices model and API) after Phase 1.
- WP-13 merged (Add device form).
- An issue #1 checklist entry (R-007).
