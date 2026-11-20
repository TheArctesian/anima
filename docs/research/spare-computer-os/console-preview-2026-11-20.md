# Installed-console ISO preview — 2026-11-20

Owner: OS3, proposal Section 14; R-NEW-aosconsole. Experimental x86-64 UEFI image, not a stable hardware qualification.

## Implemented

The installed machine opens a fullscreen Anima console using the approved web design inside an unprivileged Qt WebEngine shell. The header and power footer remain visible; content scrolls independently. Local admin authentication is required before management. Passwords reach sudo through stdin; each unlock authenticates again. The bridge accepts only packaged local navigation. The separate HTTPS sign-in browser has no native bridge and uses an off-record profile; its cookies are cleared when the local session locks or sign-in finishes. Core dumps are disabled for the console and its authenticated helper.

Real operations include NetworkManager device status, adapter-specific Wi-Fi scan/connect, disk capacity, display selection/rotation with backend-enforced rollback, diagnostics and confirmed restart/shutdown. There is no demo device, connection or progress state in the installed UI. OS rollback remains in the boot menu and advanced recovery remains on an authenticated text console. Enterprise network editing and report export are not yet implemented in the new console.

Motion uses the approved press, fast exit and base crossfade roles. Outgoing content is inert and retained through its exit. Modal focus is contained and restored; background controls are inert. Reduced motion follows the system preference or the Display setting, and changing it finishes active animations. Lock conceals content immediately. No decorative background loop runs. The shell keeps local display preferences in a separate persistent browser profile.

## Automatic enrollment

The browser opens WorkOS’s prefilled confirmation URL, with the public confirmation code and current origin visible in its header. No code is typed or copied. This follows the [WorkOS device authorization flow](https://workos.com/docs/authkit/cli-auth). After WorkOS sign-in, **Add this computer** uses the existing cloud `/v1/hub` and `/v1/hub/access` contracts, passes the private invitation to `anima-link` on stdin, obtains a hub Login token through `claimDeviceToken`, identifies or creates the machine's device record, obtains `enrolDaemon` credentials internally and passes the code directly to the packaged daemon on stdin. The temporary hub Login token is revoked after the attempt. Account tokens stay in helper memory; they are not exported to the renderer or persisted.

The machine-ID-derived name avoids collisions between computers with the default hostname. Existing device matching follows all device pages and normalizes IP addresses. Existing daemon ownership is never overwritten. A root-owned cross-process flock serializes enrollment across local sessions, including when an authorized enrollment outlives screen locking. An independently supervised private-link service runs as the daemon user and provides its private Unix socket after reboot; locking the UI does not stop it.

This reuses existing contracts and permissions. It does not add managed job or agent execution, and an active daemon process is not proof of a successful remote heartbeat.

## Verification

- Six synthetic backend tests pass: complete automatic handoff with a paginated existing device; existing-owner refusal; enrollment failure without starting monitoring; escaped NetworkManager fields; concurrent enrollment rejection; public browser confirmation without exposing the private polling credential. The complete-handoff case also covers a making/waking hub before it becomes ready.
- Existing installer/display/harness Python checks pass: 13 installer, one orientation and three harness tests.
- Repository quality gates pass: limits, names, imports, documents and baseline. The missing local `phase-2` reference was fetched from the existing remote branch; the baseline was not changed.
- Independent code review found and resolved the Qt locale path, inaccessible error dismissal, stale account state after failure, duplicate machine names and concurrent enrollment. A second read-only review found no further blocker in the existing GraphQL/Unix-socket handoff.
- The first installed-VM run passed graphical multidrive installation, erase-confirmation reset, wrong/right GUI password handling, local Network/Display navigation, relocking, read-only root, saved network/keymap/data state through reboot, and unselected-disk protection. Its screenshot review found a footer overflow at 1280×800, which was corrected before the final image.
- The final delivered image passed all ten [CLI-install and installed-GUI checks](console-preview-results.json): live UEFI boot, supplicant D-Bus activation, mounted-child exclusion, cancellation before erase, offline installation, read-only installed runtime, local rootless app/job execution, GUI authentication/lock/network/display with the fixed footer, reboot persistence and unselected-disk protection. Final screenshot inspection at 1280×800 confirmed visible power controls and no horizontal scrollbar.
- The full [Calamares multidrive test](console-calamares-results.json) passed on the preceding image (`efa1cb1192049d0b37a69d8081f78715ccc40a229449d74c70677dc3d1879461`). Subsequent changes added the sign-in browser confirmation header, waiting for a waking hub, and the console horizontal-overflow correction; the Calamares installer code was unchanged. The final exact image was then independently installed through the CLI and its installed GUI exercised as above.

Headless and VM checks are local evidence, not physical frame-pacing measurements. No production account was signed in and no real hub enrollment or remote heartbeat was exercised. Hardware Wi-Fi, orientation/touch mapping, live authentication and motion under load remain in HT-NEW-aospreview. The sign-in/backend failures must be reported honestly on hardware; do not bypass existing trust rules to force enrollment.

## Artifact

Delivery: `/home/okita/.local/share/anima-os-prototype/2026-11-20/delivery-console/anima-os-preview-x86_64.iso`.

SHA-256: `a9dfc0badc4a65cc56c3d69f268df8d68acd503ee841b004335822f53126a364`.

No USB or physical installation disk was written during this build task.
