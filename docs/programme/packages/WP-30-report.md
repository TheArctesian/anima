# WP-30 report — Hub web interfaces

Status: done. Branch `claude/wp-30-web-interfaces`. Accepted tasks at b65732e (12/12, BASE_REF=d20df2a).

## What changed

- Migration `0800_web_interfaces_schema.sql`: schema `web_interfaces`, tables `interfaces`, `pins`,
  `dismissed` (K2), constraints matching the code rules.
- Domain `crates/hub/src/web_interfaces/`: K3 faces `for_devices`, `suggestions`; writes `create`,
  `update` (drops the pin on an origin change, D-6), `delete`, `accept_suggestion` (advisory lock per
  device), `dismiss_suggestion`, `accept_certificate` (row lock; https only; 64 lower-hex pin).
- GraphQL: the six K1 mutations, each in one transaction with an audit entry on `AuditTarget::Device`
  and an event (LOG-005, WEB-009). Guarded by `access::Authenticated`.

## Edits outside owned paths (PKG-001, for the programme orchestrator)

- `crates/hub/src/lib.rs`: `pub mod web_interfaces;`
- `crates/hub/src/schema.rs`: `web_interfaces::graphql::Mutation` in `Mutation`;
  `.register_output_type::<web_interfaces::Suggestion>()` until `Device.webInterfaceSuggestions` exists.
- `quality/hub-modules.json`: domain `web_interfaces: [checks, events]`, graphql `web_interfaces: [access]` (K3).
- `contracts/schema.graphql`: `@pending` removed from `OpenIn`, `WebInterface`, `WebInterfaceSuggestion`,
  `WebInterfaceInput` and the six mutations. `Device.webInterfaces`, `Device.webInterfaceSuggestions` stay.
- `crates/hub/tests/it/main.rs`: `mod web_interfaces;` (tests in `tests/it/web_interfaces/`).
- `main.rs`: no change.

## Handoffs

- **H-1 WP-29 (checks).** Add `checks::http_urls` (K3). Then replace the private `http_urls` in
  `web_interfaces.rs` with one call, and `http_check` (uses `checks::get`) with a lookup by check id.
- **H-2 WP-21 (devices/graphql.rs, R-011).** Add `Device.webInterfaces` and `Device.webInterfaceSuggestions`
  with loaders over `web_interfaces::for_devices` / `suggestions` (registered in `schema.rs`); return the
  domain types directly (they are the GraphQL objects); add `web_interfaces` to `graphql.devices` in
  `hub-modules.json`; remove `register_output_type` and the two `@pending`.
- **H-3 events (could).** An `AuditTarget::WebInterface` kind; today entries target the device.
- **H-4 WP-31 (web view).** `acceptWebCertificate(id, fingerprintSha256)`: SHA-256 of the leaf
  certificate, 64 hex digits, colons between pairs and any case allowed; returned as 64 lower-case hex.
  Only `https` interfaces take a pin (else VALIDATION `fingerprintSha256`). Accept again replaces the pin.
  A URL change of scheme, host or port drops the pin, so the tab must ask again; a path change keeps it.
  The hub never sees the certificate: the client compares and stops on a mismatch (WEB-008).
  URLs never hold userinfo, `#`, whitespace or Unicode format characters.

## Client shapes (WP-25)

No mismatch with `hub-pending.ts` / generated types. Field errors are named `url`, `label`,
`fingerprintSha256`. The hub stores the URL as typed (trimmed).

## Open coulds (issue #1)

- No second review of the T1 lock fix (closed by its own race test).
- Event `device` is None (needs the `devices` import).
- Suggestion accept/dismiss read the check through the pool, outside the caller's transaction (until H-1).
- Mutation testing of the T1 rules hit the 20-minute limit without results.
