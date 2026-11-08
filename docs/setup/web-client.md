# Web client

The hub serves the Anima web client at its own address. You open it in a browser on a computer
that is on the VPN. Spec: Section 1.3, CLI-001, SEC-001, SEC-002. Decisions: D-2, D-3, D-8.

## Open it

1. Make sure the computer is on the VPN as a machine of the Headscale user `operator@`.
2. Open `https://hub.anima.ing/` in a browser.

Supported browsers:

| System | Browsers |
| --- | --- |
| macOS | Safari, Chrome |
| Linux | Firefox, Chromium |

A computer that is not on the VPN cannot reach `hub.anima.ing`. There is no public DNS record for it
(DNS-005).

## Sign in

Sign in with your password and your TOTP code, as in the desktop app.

The browser keeps only one cookie, `__Host-anima`. It is `HttpOnly`, `Secure` and `SameSite=Strict`,
and it lasts 7 days with no refresh. No token is kept in storage that a script can read. Signing out
clears the cookie and deletes the token on the hub (D-3).

The hub uses the cookie only for requests from `https://hub.anima.ing`. A request from another origin
is treated as signed out.

## Network and origin

- No public DNS record and no Cloudflare change are needed.
- No CORS change is needed. The web client is served by the hub itself, so it is same-origin.
  `CLIENT_ORIGINS` still lists only the desktop app's two origins.

## Share links

In Phase 3, a share link works only for a machine that is already on the VPN with access to the hub,
for example your second Mac (D-8). A guest user outside the VPN is not supported yet.

## Turn the web client off

1. On the VPS, in `/opt/anima/deploy`, edit `compose.yaml`. In the `hub` service, set
   `ANIMA_WEB_ROOT: ""`.
2. Run the command below.

   ```sh
   # Run on the VPS, in /opt/anima/deploy.
   sudo docker compose up -d hub
   ```

The hub then answers 404 for any path that is not an API route. The desktop app and the API keep
working. To turn the web client on again, put back `ANIMA_WEB_ROOT: /usr/share/anima/web` and run
the same command.
