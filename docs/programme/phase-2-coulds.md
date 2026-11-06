# Phase 2 coulds for issue #1

These items are not blocking. The programme orchestrator collects them here and posts them to GitHub issue #1 in batches.

## WP-23 (gateway, compose and VPN policy)

- [ ] The Dockerfile is checked only as text.
- [ ] Images are pinned by tag, not by digest.
- [ ] The Windows machine needs its own RDP grant once it has a name and an address.
- [ ] `deploy/README.md` needs the two new variables.
- [ ] The SSH session target check does not read `ANIMA_PROBE_DENY` yet.

## WP-24 (client RDP and VNC tabs)

- [ ] The `if (!closed)` guard in `read()` in `view/certificate/CertificateModal.svelte` has no test.
- [ ] The `target === client` and `rules.paste` guards are redundant. They survive mutation.

## WP-25 (client device form)

- [ ] Phase 1 patterns to fix everywhere in one change:
  - 13 px checkboxes (UI-013)
  - hover contrast of 1.17:1 under more contrast (MAT-006)
  - primary button contrast (UI-004)
  - the press role animates background colour (MOT-006)
- [ ] Comments and test titles cite decision IDs (COM-002).
- [ ] The port and URL checks are duplicated.
- [ ] `sameJson` depends on key order.
- [ ] Changing an SNMP check's destination gives no warning.
- [ ] Saving an unchanged web interface row gives no feedback.
- [ ] `hub-pending.ts` still holds the device and web interface read types. This waits on `lib/devices/api.ts`.

## WP-20 (skeleton and contract gate)

- [ ] `host_port` accepts `[]:4822` and port 0.
- [ ] The fixture test does not check field names.
- [ ] The contract test has duplicated code.
- [ ] A blank line splits a table in `hub-domains.md`.
- [ ] The MV-008 allow-list does not exist.

## WP-30 (hub web interfaces)

- [ ] The T1 security mutation run hit its 20-minute limit and returned no results. It covered the URL rule, fingerprint parse, origin and pin drop, and the suggestion filter.
- [ ] Web interface events have device `None`. Filling it needs an import from `devices`.
- [ ] Add an `AuditTarget::WebInterface` kind. Today the entries target the device (H-3).
- [ ] Accepting or dismissing a suggestion reads the check outside the caller's transaction. Swap to `checks::http_urls` (H-1, WP-29).

## Programme

- [ ] Three tests fail under load:
  - `k4_8_5` (WP-22 is fixing it);
  - the deploy test DNS-003, for the `make-secrets` interrupt timing;
  - `checks::api::batching::api_005_status_last_heartbeat…`. With the 10 ms loader window under load, the run took 24 statements against a limit of 20, and it passed 3/3 alone. WP-29, which owns checks, should give the test schema a longer batch window.
