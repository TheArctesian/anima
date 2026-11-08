# Prompt: set up the Pop!_OS machine for Anima

Paste everything below the line into Claude Code on the Pop!_OS machine.

---

You are setting up this Pop!_OS computer as the second operator computer and a Linux test device for Anima, a self-hosted device manager. The repository is `TheArctesian/anima`. Work branch: `phase-2`. Read `AGENTS.md` and `docs/design-spec.md` Sections 4.2, 12 and 13 before you change anything.

Rules:
- Never read `.env*` files or key files, and never print secrets.
- Run every `sudo` step yourself only if I type the password into the prompt. Never pipe a password into `sudo -S`.
- The VPS (`anima-admin@100.64.0.1`) and Headscale are changed only after I say yes in chat (VPS-005). For each change, show me the command first, then wait.
- Use `tailscale`, not a second VPN. If another full-tunnel VPN is on, tell me (VPN-007).

## 1. Join the Anima VPN (Headscale, personal login like the Mac)

1. Install Tailscale 1.80 or later: `curl -fsSL https://tailscale.com/install.sh | sh`. Show me the script before you run it.
2. Run `sudo tailscale login --login-server https://hs.anima.ing --accept-routes --hostname pop-os`.
3. It prints an auth URL with an auth ID. Tell me the auth ID. On the Mac I will register it on the VPS:
   `hsctl auth register --user operator --auth-id <AUTH_ID>`, then set the 180-day expiry (VPN-003) as in WP-03 step 7.
4. Check:
   - `tailscale ip -4` gives a 100.64.0.x address;
   - `tailscale ping hub` works;
   - `curl -sS https://hub.anima.ing/health` prints `ok`.

## 2. Allow the hub to reach this computer (VPN rule, needs my yes)

Draft, but do not apply, the change to `deploy/headscale/config/policy.hujson`:
- under `hosts`, add `"device-pop-os": "<this computer's 100.64 address>/32"`;
- a grant like the Mac's: `{"src": ["tag:hub"], "dst": ["device-pop-os"], "ip": ["tcp:22", "tcp:5900"]}`.
Show me the diff. Once I say yes, I apply it on the VPS the same way as the WP-23 policy step, and you log the change in `docs/programme/operator-changes.md`.

## 3. Turn on SSH and screen sharing

- SSH: `sudo apt install openssh-server`, then `sudo systemctl enable --now ssh`.
- Screen sharing:
  - Prefer GNOME Remote Desktop with VNC (Settings → Sharing), if this Pop!_OS version offers VNC.
  - Otherwise install `wayvnc`, or `x11vnc` on an X11 session.
  - Set a password, and confirm it listens on port 5900: `ss -ltnp | grep 5900`.
  - Tell me which server and display system (Wayland or X11) you used.

## 4. Build the desktop app

1. Install the build tools:
   - `sudo apt install build-essential curl wget file libssl-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev pkg-config`
   - Rust via rustup (stable)
   - bun
2. Get the code: clone `TheArctesian/anima` and check out `phase-2`. If the branch is not on GitHub yet, stop and tell me, and I will push it or copy a bundle over.
3. Build:
   - `bun install --frozen-lockfile`
   - `bun run --cwd apps/client build`
   - `bun run --cwd apps/desktop tauri build --bundles deb,appimage`

   A "no private key" error about the updater after the bundles are made is expected (the signing key is not set up yet).
4. Run the built app, not `tauri dev`: the hub only accepts the app's own origin.

## 5. First checks (report pass or fail with what you saw)

- Sign in with my Anima user name, password and authenticator code. I type them myself.
- The device list loads, and the "Not on the network" banner is gone.
- Add this computer: the app should offer "This computer" (R-060).
- Phase 2 acceptance O-3 (VNC to a Linux desktop): open VNC to this computer from the Mac, and from this computer to the Mac. See `docs/programme/phase-2-acceptance.md`.
- Note every Linux-only failure (web view, key store, system-settings button, fonts, scaling) as a finding with the spec ID. Do not fix it in this session; list it for me.

At the end, give me:
- the 100.64 address and node name;
- the VNC server you chose;
- the app build result;
- the list of findings.
