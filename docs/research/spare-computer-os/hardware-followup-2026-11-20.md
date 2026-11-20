# Display sizing, installed Wi-Fi and rotation follow-up

Owner: OS3, spare-computer-os proposal; follow-up to R-272 and R-271. The operator reported tiny content on a high-resolution panel, requested the change on every page, reported Wi-Fi working in the installer but unavailable after installation, and asked for confirmed rotation to survive installation.

## Changes

Installer and installed-console launchers share resolution-based quarter-step Qt scaling. The minimum target logical canvas is 1280×800 where the panel permits it. Small displays remain at 1×. This is a resolution-based default, not measured physical DPI. Installer fields expand up to 1100 logical pixels; welcome/completion artwork follows the page. The installed console no longer centers a capped 1000-pixel content column far away from navigation. All pages, dialogs and footer controls share scaling. Scaling is selected at launch; runtime display-mode changes require reopening the application to recalculate it.

The installed supplicant uses a declarative `update_config=0` configuration. NetworkManager still owns connections through D-Bus and retains them in its designated writable directory. This removes the upstream imperative configuration file writes during supplicant startup without making the host root writable or replacing the Wi-Fi stack. The console reports disabled Wi-Fi or a failed backend explicitly and offers a deliberate radio-enable action.

Only confirmed rotations are saved. The live profile is copied after the long installation operation, before sync/unmount. Calamares disables rotation changes once installation starts. The installed console saves confirmations to `/persist/display.json`; display-session startup reapplies matching connector settings before the GUI opens. Unknown connectors are not rotated from another connector's saved setting. Unconfirmed previews retain the existing timed rollback. Save failures in the interactive fallback report failure and restore the prior rotation.

## Evidence

The [old delivered image regression](installed-wifi-before-results.json) passed live supplicant D-Bus activation but failed installed startup after root locking. The installed journal explicitly reported `touch: cannot touch '/etc/wpa_supplicant/imperative.conf': Read-only file system`. This reproduces an installed-only failure; it does not establish physical radio reliability.

Independent review identified the rotation-transfer timing and interactive-save failure handling; both were corrected. A provisional directory-permissions concern was refuted by the installer's explicit umask 022. Installed-user read access also passed in the VM.

The final image passed all twelve [CLI acceptance checks](hardware-followup-cli-results.json) and all three [graphical acceptance cases](hardware-followup-gui-results.json). These include installed supplicant restart/D-Bus access after root locking, confirmed rotation transfer/application and application again after reboot, owner GUI authentication/locking, separate data-drive persistence and unselected-disk protection. Local app/job checks use Podman, not Anima-managed agents. Three orientation unit tests, six console backend tests and thirteen installer safety tests pass. All five repository quality gates pass.

A preceding CLI attempt reached its final assertions but terminated before writing its report; it is not counted as a passed run. The repeated run above completed normally and records the delivered image's hash. A 1920×1200 visual pass inspected Welcome, Network, Keyboard and Account at 1.5× scaling, with footer controls visible; it used the preceding image with identical sizing code. The final image also passed the complete 1280×800 graphical flow. Physical readability and runtime rotation/scale interaction still require hardware checking.

Delivery: `/home/okita/.local/share/anima-os-prototype/2026-11-20/delivery-hardware-fixes/anima-os-preview-x86_64.iso` (3,934,945,280 bytes).

SHA-256: `e43c2a9521370232076618beefe3b2d286e483eafcf4d6be3a9d323ed75749f0`.

The USB was not rewritten during this follow-up. The [adapted, owner-approved persona test](persona-test-2026-11-20.md) completed installation and keyboard-only console/locking journeys, followed by independent reproduction and screenshot review. It found network-feedback, progress-feedback and copy/disabled-control issues, with no observed install or lock blocker. These report-only findings are not additional changes in the delivered ISO. The report states the native-driver limitations rather than claiming the stock web/iOS workflow ran.

Physical Wi-Fi association, internet access, touch mapping and readability on the operator's computer remain human checks. WorkOS/hub integration retains the previous preview's live-account verification limitation.
