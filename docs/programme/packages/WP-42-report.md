# WP-42 report — Client groundwork and platform seam

Status: **done** (all seven tasks accepted). Package branch `claude/wp-42-client-groundwork`; plan and review log
in [WP-42.md](WP-42.md). Ended early under the programme wind-down; no task is unfinished.

## Accepted tasks

| Task | SHA | Base | Gates | Rounds |
| --- | --- | --- | --- | --- |
| T1 casts (Q2-15) | c0a97e9e | 08f87bd7 | 4–12 | 1 (merged into phase-3) |
| T2 charts (Q2-16) | 04e941aa | 08f87bd7 | 4–12 | 2 |
| T3 os-program (Q2-13) | aee3e146 | 08f87bd7 | 12/12 | 2 |
| T4 domain-moves (Q2-12) | 20b390a8 | e09d7062 | 4–12 | 1 |
| T5 web-interfaces (Q2-12) | dc26ca78 | 2b03a3e3 | 4–12 | 1 |
| T6 platform (CLI-001, UPD-001) | e791d121 | 7953dbbc | 4–12 | 2 |
| T7 hub-face (Q2-12) + two handoff commits | 8c5ad545 | 3052b201 | 4–12 | 1 |

## Handoffs (R-082 commits on T7)

- 02009182 `eslint.config.js` + `apps/client/src/lint/import-boundaries.test.ts`: hub face restriction (faces:
  `hub`, `context`, `generated`, `generated/graphql`, `notice/LiveNotice.svelte`; `socket/fake` in tests and
  fixtures; `codegen.ts` exempt) and the UPD-001 updater rule, both as `@typescript-eslint/no-restricted-imports`
  so the core rule's lists stay intact.
- 8c5ad545 `quality/src/names.ts` + `quality/tests/names.test.ts`: `file-casing` (kebab-case scripts,
  PascalCase `.svelte`, `+` route files).

## Notes for WP-44

- `lib/platform/browser.ts` is yours next; `platform.ts` is where a build-time switch belongs, which would also
  drop `@tauri-apps/api` from the browser bundle (still pulled in through `lib/desktop/capabilities.ts` and
  `lib/web-interfaces/shell/client-shell.ts`).
- `popOut` answers no on desktop until a shell window command and a `PopOutPort` exist.
- The Sign-out row is not desktop-only (SET-002); choose `clearToken` by `capabilities().keyStore` in `lib/access/`.
- `lib/access/navigation.ts` and `routes/sign-in/+page.svelte` got one-line swaps to `capabilities().keyStore`.

## Should items for issue #1

- Casing rule: rename `HeartbeatBar.test.ts`, `MetricBars.test.ts`, `ScreenSharingButton.svelte.test.ts`,
  `WakeButton.svelte.test.ts` to kebab-case and drop the PascalCase test exception.
- Hub and updater lint rules do not see `vi.mock(...)` or dynamic imports; add a source scan or note the scope.

## Could items for issue #1

- `web-interfaces/tab/api.ts` and `web-interfaces/api.ts`: two GraphQL clients in one domain; fold into one.
- `lib/devices/form/form/` name; RemoteLoginButton relative import; its tests live in
  `lib/desktop/system-settings.svelte.test.ts`.
- `DEGRADED_ABOVE` could move to `lib/metrics/thresholds.ts`; MetricsSkeleton import order; three import
  styles for `units`; `phase-2-acceptance.md:25` cites the old `charts.test.ts` path.
- `os_program`: document that `Capture` reads after exit; the try_wait-error reap path has no test.
- Adapters in `lib/desktop` keep their own `isTauri` guards; folder-level type import desktop → platform.
- hub-modules error wording for `{"domains":[]}`; no invalid-`graphql` test case.
- `quality/README.md` names row should mention file casing; `export type *` comment wording in
  `import-boundaries.test.ts`.
- Load-flaky tests seen: hub `api_005_interfaces_and_suggestions` (14 > 12 under a full gate run) and deploy
  `localapi-server.test.ts` "endless upstream body gives 502 promptly" (504 once); both pass alone.
