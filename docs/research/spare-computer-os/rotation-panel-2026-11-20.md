# Installer settings after rotation

Owner: OS3 in [the spare-computer proposal](../../proposals/spare-computer-os.md). The operator reported that changing orientation left the settings panel off-center and Done unreachable.

## Change

The settings dialog observes screen geometry and available-area changes and screen transitions. It queues repositioning until Qt processes the display change, constrains its size to the available screen, and centers there. Settings scroll when vertical space is insufficient; Done stays outside that scroll area. The new scrollbar follows the existing Anima colors. Existing orientation confirmation, persistence and timed rollback behavior are unchanged.

## Regression evidence

The prior delivered ISO (`e43c2a9521370232076618beefe3b2d286e483eafcf4d6be3a9d323ed75749f0`) reproduced the defect: opening settings in portrait then returning to 1280×800 landscape left the 440×561 panel at x282,y320, extending to y881. The settings regression failed its visible-bounds assertion.

The regression uses real Qt/X11 in a disposable offline VM, changes display geometry while the dialog remains open, checks complete on-screen bounds and centering, then clicks Done at 640×480 and checks that the dialog closes. It does not inspect source text or add application test hooks. Initial test margins were relaxed from an exact 16-pixel inset to the user-visible contract (fully on-screen and centered), because integer center rounding produced a harmless 15-pixel inset. The old panel still violates that contract by 81 pixels.

An intermediate patched ISO passed rotation, 800×600, 640×480 and Done activation. Final themed-image results are recorded below. VM screenshots demonstrate settings-panel behavior, not full installer support at 640×480; the underlying installer still has larger minimum dimensions. Physical panel/touch behavior remains an open check in [human testing](../../need-human-testing.md).

## Final image

The [final regression](rotation-panel-results.json) passed on the themed ISO. Screenshots at 800×600 and 640×480 were visually inspected; the compact panel has a themed scrollbar and a separate visible Done button. The test then clicked Done and verified dismissal. All five repository quality gates, Python compilation and mockup JavaScript syntax checks passed. The C++ change compiled as part of the ISO build. This is a focused follow-up, not a rerun of the complete install/backup/workload suite.

Delivery: `/home/okita/.local/share/anima-os-prototype/2026-11-20/delivery-rotation-panel/anima-os-preview-x86_64.iso`.

SHA-256: `6aa46abdedd6dc0a857a799959fce965edbd5d79f7efa8a820dcfbd65c0c12f4`.

Local screenshots and serial evidence are under `/home/okita/.local/share/anima-os-prototype/2026-11-20/rotation-panel-final/`; pre-fix evidence is under `rotation-regression-before/`. All disposable test VMs were stopped. The USB has not been rewritten with this follow-up image.
