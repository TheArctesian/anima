# Installed-operation preview — 2026-11-20

Scope: OS3 and the experimental OS5 slice authorized by R-NEW-aosinstalledfollowup. This is an x86-64 UEFI preview, not stable hardware qualification.

## Changes

- The separate sign-in browser has a fixed-height header and gives the remaining viewport to the provider page. Provider-neutral Anima copy and a visible self-hosted hub form preserve the existing enrollment contracts. Remote content receives no native command bridge.
- Installer completion exposes Restart and Shut down. Battery icons occupy the top-right of both interfaces, including the locked console; CPU, memory and uptime readings appear locally. Missing batteries are not simulated in production.
- The daemon now finds NixOS's system tools after clearing its inherited environment. Actual packaged-daemon messages in the VM contain drive inventory and disk usage.
- Installed machines generate `anima-node-xxxx` once and persist the hostname. Newly created hosted device records use that name; the longer private-link identity remains separate.
- The updater validates a signed manifest and the full archive, stages a boot-counted candidate, retains the previous system, and requires explicit restart. Failed local health checks cause bounded retries and fallback without restoring old workload data. Preview updates use a separate hosted OS namespace.

## Review and regression evidence

Predictability: the old sign-in layout failed the native geometry regression. The compact header passed landscape, portrait and small-window cases. A real failed-service update test found that selecting only candidate entries also forced an exhausted candidate; selecting the candidate and previous-system entries fixed bounded fallback. The corrected VM returned to the previous system after two failed candidate boots.

Domain boundaries: shared battery readings are separate from installer/console rendering; privileged enrollment and update actions remain behind local-owner authentication. The hosted-name test failed against the old machine-ID display name and passes with the actual hostname. No production test-only flags were added.

Repository validation before the final hostname change: all 21 required gates passed across the full run and the ShellCheck-enabled Bun/quality rerun. Rust ran 6,594 tests with one skipped; the final Python/Nix/UI follow-up also passes focused hosted-enrollment (6), self-hosted-enrollment (2), telemetry (1), manifest verification (1), syntax and five quality gates. The final ISO lifecycle evidence is recorded separately below.

## Limits and physical acceptance

Battery screenshots use an explicitly simulated Linux battery in disposable VMs; laptop charge, unplug/replug, thermal behavior and touch remain hardware checks. The operator has reported that Wi-Fi works on the latest physical install. Real hosted sign-in and self-hosted enrollment still require account/network acceptance. The remote sign-in page retains its provider-managed theme.

Updates currently transfer a full closure, do not automatically drain workloads, and do not retire old EFI assets or garbage-collect old store paths. Low-space updates are refused while retaining recovery. Existing images without the updater require the requested bootstrap installation. The hardware entry HT-069 remains Open.

## Final ISO lifecycle

The [final ISO results](installed-operation-results.json) pass actual graphical multi-drive installation, completion Restart, installed-console authentication, Wi-Fi backend activation on read-only root, battery rendering, real daemon storage messages, signed HTTPS staging, candidate boot and owner rollback. Unselected-disk bytes are unchanged. The generated `anima-node-176b` name survived reboot, update and rollback alongside device identity and workload-data sentinels.

ISO SHA-256: `139011f6fa415a055a7364bdac5de622322cdb27aef58e94e6530ea2ad145dcb`.
Appliance closure: `/nix/store/n6z8g1bnsh00vvgjxjrpvvd546cxj8yd-nixos-system-anima-Preview`.
The signed public package matches this exact appliance closure; its signature, size and archive digest were verified locally.

Local evidence: `/home/okita/.local/share/anima-os-prototype/2026-11-20/node-delivery-002/`. The installer and console screenshots in that folder show the battery fixture; no synthetic battery data ships in the image.

The [automatic-recovery test](installed-operation-recovery.json) passed: an actually masked NetworkManager caused two failed candidate boots, followed by the previous system with its identity and persistent data intact.

## Hosted delivery check

Public delivery rejected urllib's default client identity with Cloudflare HTTP 403 / code 1010. The feed was not published. An explicit `AnimaOS-Updater/1` User-Agent returned HTTP 200; the production fetch function then downloaded and verified the complete public archive. The image was rebuilt with this transport correction before delivery. Signature, redirect and size restrictions remain unchanged.

The transport-corrected final ISO (source `31257587`) repeated all five lifecycle checks successfully. Automatic fallback was verified on the preceding image with identical boot/recovery code; the transport correction only adds the public HTTP client identity.
