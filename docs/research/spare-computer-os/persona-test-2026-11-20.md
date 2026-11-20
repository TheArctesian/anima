# Adapted ISO and console persona test

## Limitations

This is an owner-approved adaptation of `persona-test` for native Linux QEMU interfaces, not the stock web/iOS workflow. Two synthetic personas used the inherited Codex model and real GUI screens; observations are hypotheses, not evidence about real-user frequency or impact. A bounded QMP/xdotool proxy enforced 25 input actions and keyboard-only input for the second persona. Source blindness and interpretation of wrong turns were instruction-based, not filesystem isolation. There was no network adapter, production sign-in, screen-reader, touch or physical-radio test.

Owner: OS3 in [the spare-computer proposal](../../proposals/spare-computer-os.md). The tested ISO SHA-256 is `e43c2a9521370232076618beefe3b2d286e483eafcf4d6be3a9d323ed75749f0`. Functional results and delivery details are in [the hardware follow-up](hardware-followup-2026-11-20.md).

## Method and evidence

The first persona installed onto a disposable 64 GiB disk while leaving a separate 32 GiB disk unselected. The second persona used the resulting installation after reboot, with a separate action ledger and keyboard-only policy. A fresh disposable VM supported installer reproduction. All credentials were synthetic. The two sessions therefore represent distinct tasks and input constraints, not independently provisioned installed machines.

Local raw evidence is under `/home/okita/.local/share/anima-os-prototype/2026-11-20/`: `persona-installer/`, `persona-console/` and `persona-verify-install/` contain screenshots and action ledgers. Reports alongside them retain original observations and verifier disagreements. The merged verdict below takes precedence over refuted statements in those raw reports.

## Session outcomes

| Persona / task | End | End state evidence | Inputs | Wrong turns / backtracks |
| --- | --- | --- | --- | --- |
| First-time owner: install on 64 GiB, preserve 32 GiB | Done | Completion screen; review marks 32 GiB untouched. Separate functional acceptance tests check preservation after boot. | 13/25 | One coordinate miss, refuted as a UI defect; no navigation backtracks |
| Keyboard-only owner: inspect connectivity/storage, lock without shutdown | Done, connectivity explanation partial | Storage capacity visible; final password screen says local session locked and continued server operation. | 21/25 | No wrong turns; seven reverse Tabs were intentional focus navigation, not repeated failed journeys |

The persona screenshots establish the local lock result, not independently measured service continuity. Separate functional checks cover the installed runtime. The first task ended at completion; a harness reboot prepared the installed-console task. No persona performed a production account enrollment.

## Findings

No observed installation or local-lock blocker. Severity uses Nielsen's 0–4 scale and is provisional; it does not estimate real-user impact. Reach below names enforced input conditions, not population coverage.

| Finding | Severity / tier | Evidence and verification | Interface recommendation |
| --- | --- | --- | --- |
| Network page does not explain the no-adapter state or scan outcome | 2 / trusted feedback | Keyboard persona: overview `disconnected:unknown`; Network stayed at generic scan instructions after activation and sampled waits. Screenshot reviewer confirms absent outcome in this deliberately no-NIC fixture. Independent keyboard reproduction also found unchanged feedback after five seconds and successfully relocked in 18 actions. Reach: keyboard-only. | Distinguish no adapter, disabled radio, scan in progress, no networks and scan failure; replace raw unknown state with a useful explanation. |
| Installation progress stays at 5% across repeated samples | 2 / trusted feedback | Installer views 015–019 show unchanged progress, then completion. Fresh reproduction samples at about +24, +49, +71 and +128 seconds also show 5%, with completion at the next sample (within roughly +160 seconds). Screenshot reviewer confirms sampled feedback, not a continuous hang. Toggle log exists and was usable; do not claim all diagnostics are absent. Reach: mouse/keyboard. | Show the current installation phase and ongoing activity without implying an exact percentage that the job cannot report. |
| Review and completion disagree about account setup scope | 2 / medium copy comprehension | Review says hub-driven setup is not included; completion directs sign-in to Anima. Both reproduction and screenshot review confirm the wording. Neither proves sign-in is impossible or required for local operation. Reach: mouse/keyboard. | Explain what works locally and what sign-in adds, consistently across review and completion. Validate interpretation with real users. |
| Ready footer Continue looks actionable but does nothing | 1 / trusted visual affordance | Screenshot reviewer sees cyan Continue beside muted Back. Fresh verifier clicked Continue with no visible result; keyboard traversal skipped it and cycled between Shut down and settings. This supports an inactive control with misleading styling, not a second working completion path. Reach: mouse/keyboard. | Hide irrelevant navigation on completion or give inactive navigation an unmistakably disabled treatment. |

These are report-only recommendations. The delivered ISO includes the hardware fixes, not an untested follow-up redesign based on these findings.

## Refuted observation

The first installer persona claimed the erase-confirmation checkbox moved from y560 to y688. The screenshot reviewer found it at y688 throughout; the first click missed it by 128 pixels. A reproduction agent also claimed movement, but its before/after frames `persona-verify-install/view-010.png` and `view-011.png` have identical decoded pixels (`magick compare -metric AE`: zero). This is an agent coordinate-reading error, not a demonstrated layout defect. No change is recommended for it.

## Coverage gaps

The fixture intentionally lacks network hardware. It cannot qualify physical Wi-Fi, internet recovery, saved wireless credentials, WorkOS sign-in or daemon enrollment with a real hub. Installer/console checks at 1280×800 do not establish physical readability, touch mapping or portrait behavior. Encryption, hardware-specific boot paths, backups and production workload management were outside these two journeys. The local-console journey does not cover the separate general Anima desktop client. The different goals and enforced input methods prevent treating these as duplicate paths, but provide no population coverage.

The standalone VM reboot helper treated a successful keymap oneshot's inactive state as an error. The system had booted; a separate probe confirmed `Result=success`, with display manager and NetworkManager active. This fixture issue is not a product finding.

## Validate with real users

Check whether the review/completion account wording causes uncertainty; whether phase-based progress is understandable during a real installation; and whether network diagnostics lead to a useful next action on real hardware. The brief unlock text overlap reported by the keyboard persona settled in the next screenshot and was not independently reproduced; retain it as unverified, not a persistent defect.

## Teardown

Both disposable QEMU machines were stopped after verification. Screenshots, ledgers and reports remain in the local evidence directory. No physical disks or USB media were written by this persona run.
