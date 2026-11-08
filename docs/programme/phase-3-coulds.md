# Phase 3 should and could items (for issue #1)

Items found during Phase 3 reviews that are not fixed in Phase 3. Tagged by package and task.

## WP-42 (client groundwork)

- T1 (could): the error text in `quality/src/hub-modules.ts` for `{"domains":[]}` changed wording (same class); the `octet()` helpers in `deploy/localapi` and `deploy/dns` duplicate each other on purpose (separate workspaces); no test for an invalid `graphql` value.
- T2 (could): import order in `MetricsSkeleton`; `DEGRADED_ABOVE` could move to `thresholds.ts`; three import styles for units; `phase-2-acceptance.md:25` cites the old `charts.test.ts` path.
- T3 (could): `Capture` reads stdout after exit (document it); the `try_wait` error reap path is untested (cannot be provoked); `name.rs` poll 10 → 20 ms. Behaviour: `sharing_settings_open` now has the 3 s opener wait limit.
- T3 (should): hub test `api_005_interfaces_and_suggestions` (`web_interfaces.rs:133`) is load-sensitive (14 > 12 statements under load).
- T4 (could): `devices/form/form` is an awkward carried-over name; `RemoteLoginButton` uses a relative import and its tests still live in `desktop/system-settings.svelte.test.ts`.
- T5 (could): `web-interfaces/tab/api.ts` (AcceptWebCertificate) and `web-interfaces/api.ts` are two GraphQL clients in one domain; fold into one. `open/` ↔ `sessions` type-only edges are worth watching; `shell.ts` re-exports make `events` and `fingerprint` reachable two ways.

## WP-40 (skeleton and contract gate)

- T5 (could): NET-004 (`design-spec.md` line 289) still says the nav bar shows the hub address; it should be desktop-only. UI-005 holds several rules in one requirement.
