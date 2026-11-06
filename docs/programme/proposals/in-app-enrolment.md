# Proposal: in-app VPN enrolment (no install, no terminal)

From: the explainer session. The operator asked for this in chat on 2026-11-05 ("I don't want the
user to have to install anything else or use the terminal") and then said "Yes, send it".
Status: a proposal for a ruling and for issue #1 (R-007). Not Phase 1 scope (contracts frozen, CON-001).

## Problem

Adding a device at a site that already has a router is already fully in-app (spec 6.2, DEV-013,
SEC-006). The friction is joining the VPN:

- Client computer: install the Tailscale app, run `tailscale login --login-server …`, and approve it
  with `hsctl auth register` on the VPS (vpn.md §6).
- New site: install Tailscale on a Linux PC, paste a pre-auth key, and approve the route with
  `hsctl nodes approve-routes` (vpn.md §5).

## Proposal

1. **Embedded VPN client.** The Tauri Rust layer embeds Tailscale's tsnet (through libtailscale), so
   the app joins the VPN in userspace. No Tailscale app, no OS routes, no admin rights. Only Anima's
   own traffic uses it, which is enough because all device traffic goes through the hub.
   Costs: a bundled Go library and a bigger app. The Tailscale menu-bar app and account switching
   are not used for Anima. iOS (Phase 5) still needs checking.
2. **Invite codes.** Settings gets "Add a computer". The hub has a single-use, 1-hour, untagged
   pre-auth key created for user `operator` and shows it once as a code or QR. The new machine's
   setup screen takes the code, joins the VPN, then signs in at `hub.anima.ing`. Before the 180-day
   expiry (VPN-003) the app re-enrols itself through the hub.
3. **Sites from the app.** Settings gets "Add a site", which creates a router key with
   `tag:router,tag:site-<x>`. When the router comes online, the app shows "Approve route
   <subnet>?" in place of the `approve-routes` command.
4. **Something must still run at each new site.** In order of preference: a router or NAS with
   Tailscale in its own UI (OPNsense, pfSense, Synology, some GL.iNet; it must support a custom login
   server); an Anima site-agent installer (spec Phase 6, VPN-006) that takes the invite code; or the
   app itself acting as a subnet router on a machine that is always on.

## The trade-off that needs a ruling

vpn.md §11 deliberately has **no Headscale API keys**, so that full control of the hub cannot add VPN
nodes. Headscale API keys give full power. Instead of one, run a small root-owned broker on the VPS,
reached over a Unix socket or loopback, with exactly three verbs:

- create one single-use 1-hour personal key for `operator`;
- create one router key for a named site's tags;
- approve a route that a router has already advertised, only for its own site subnet in the policy.

The hub calls the broker, never Headscale directly. Each call needs a signed-in operator with TOTP
and writes an audit entry (LOG-005). This still widens the OD-9/OD-10 risk (the hub and VPN control
share a VPS), so it is the operator's decision.

## Needs

- A spec change: a setup screen, invite and enrolment requirements, and Settings items beyond SET-002.
- Rulings: (a) embedded VPN in the desktop shell; (b) the hub-to-broker permission and the broker's
  exact verbs.
- Issue #1 checklist entries for both (R-007).
