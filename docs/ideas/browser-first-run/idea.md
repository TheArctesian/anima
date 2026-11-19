# Browser sign-in and first hub

- From the operator, 2026-11-18
- Source: Anima.ing sign-in and first-run mockups
- Status: Framed
- Serves: Work plan row 4.17
- Owner if it becomes work: [Hosted mode](../../proposals/hosted-mode.md), Section 4.1 and HMD-032 to HMD-037
- Effort: S for the mockup

## The idea

[Open the browser mockup](index.html). A successful first sign-in opens one free personal hub without a payment step. The hub shows Starting and allows setup immediately. Returning users land directly on their hub. The preview keeps its hub and device state in the current browser tab.

On desktop, CLI is the first option. Choose the target machine, copy the sample installer text, and follow the proposed browser sign-in back to the terminal. Then open a terminal preview, add another device, or connect an AI client through the CLI's MCP server. On iOS and Android, the app path starts with “Install or open Anima” for this device. “Set up on another device” lets you choose macOS, Linux, iOS or Android before revealing a sample QR and copy link. The device row appears when app setup starts. The same row changes from Waiting to Online after the device reports in. The [approved first-run board](../../mockup/first-run/index.html) shows the complete desktop sequence. The server path separates an existing machine from a new VPS, which requires a quote and explicit confirmation.

Use the preview controls below the product frame to reset to sign-in, try a returning user, or simulate readiness, CLI sign-in and machine enrollment. The install text is a shell comment. The QR encodes a reserved sample address. Hub provisioning, hosted CLI sign-in, enrollment, opening a terminal and new VPS checkout are presented as local previews.

## Evidence

- The browser layout comes from the website's existing sign-in page and Combined HTML mockup. The particle door uses the website's current renderer and geometry.
- [WorkOS email verification](https://workos.com/docs/reference/authkit/email-verification) documents automatic email verification for Apple, Google and Magic Auth. Hosted AuthKit can ask for an email ownership code after GitHub when the address needs verification.
- [Hosted AuthKit MFA](https://workos.com/docs/authkit/mfa) uses authenticator apps. R-159 requires a second factor before the first device with remote access. The mockup shows the proposed email path with an authenticator challenge. The policy for social providers still needs a check.
- Google is shown in the test route. Its public launch is pending. The preview does not establish provider availability or test live sign-in.

## Open questions

- Can the proposed email and authenticator policy be enforced while meeting R-159 for each social sign-in path?
- How will browser signup make or find the account's one hub without creating duplicates? Can the browser session be reused during CLI and desktop authorization?
- Which installer URLs are verified per target, and what provider quote can be shown for a new VPS? The [CLI and MCP plan](../../plans/cli-mcp-plan.md) describes the existing CLI and MCP session model.
