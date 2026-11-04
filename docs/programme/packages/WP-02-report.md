# WP-02 report (package orchestrator)

Status: **done**. Branch `claude/wp-02-tokens-327209`, based on `phase-1` (`dee79b3` merged in). Not pushed, not merged.
`bun run gates` on the tip: **12/12 pass**, exit 0. Plan, decisions D-1..D-11 and the review log: [WP-02.md](WP-02.md).

## What is delivered

| Output | Path | Spec |
| --- | --- | --- |
| Token source (W3C DTCG) | `design/tokens/*.tokens.json` (colour, typography, space, layer, material, motion, terminal) | TOK-001, 5.1, 5.5.3, 5.6, 5.7.2 |
| Token build (Style Dictionary 5.7.1, custom formats) | `design/src/`, entry `design/bin/tokens.ts`; `bun run --cwd design tokens` | TOK-002, D-1..D-4 |
| CSS | `apps/client/src/styles/tokens.css` — every K5 name except `bp-*`; reduced-motion, reduced-transparency and `prefers-contrast: more` blocks; springs as `<settle ms> linear(…)` | UI-001–005, TOK-010/011/020/021, MAT-006, MOT-021/040 |
| TypeScript | `apps/client/src/lib/motion/tokens.ts` — `spring` (dampingRatio, responseS, stiffness, damping), `duration`, `ease`, `stagger`, `threshold`, `breakpoint`, `reducedMotion` | K5, MOT-041 |
| Rust | `crates/theme`: `Rgb`, `NORD: [Rgb; 16]`, terminal palette, `guac_color_scheme()` (syntax checked against guacamole-server `color-scheme.c`) | RA-010 |
| Token page | `design/tokens/reference.md` (generated) | 5.5.1 |
| Fonts | `apps/client/src/styles/fonts/` (Propo Regular/Bold/Italic/BoldItalic, Mono, Symbols Nerd Font, 3 licence files, `fonts.lock.json` with SHA-256), `fonts.css`, `global.css` | UI-010, 011, 014, 017, 018, 022 |
| TOK-002 gate | `design` workspace `lint` = ESLint + `tokens:check` (runs in gate 5, `bun run lint`); fails naming each stale or missing output | TOK-002 |
| TOK-003/010 gate | `.stylelintrc.json` (closes the WP-01 gaps; refuses `var(--nord-*)` outside tokens.css) and an ESLint `no-restricted-syntax` token-literal rule for `apps/client` | TOK-003, TOK-010 |

Tests: design 115 (golden spring values incl. RK4, 79 spec values table-driven, K5 names parsed from `contracts/tokens.md`, staleness in a temp dir, source validation), client 111 (font faces, lock, licences, lint rules), Rust `anima-theme` 5.
Interface review (WebKit 27.2, Chromium 156): fonts bundled and served from the app origin only, glyphs render, every K5 property resolves, `linear()` springs accepted, reduced motion and contrast verified in both engines.

## Handoffs (PKG-001)

- **H-1 (programme orchestrator, root `package.json`):** added `"design"` to `workspaces` (plus `bun.lock`). Needed for the token check, tests and type check to run in gates 5–7.
- **ESLint/Stylelint (WP-01 files):** `.stylelintrc.json` and `eslint.config.js` were changed by WP-02 as owner of the TOK-003 rule (brief); `eslint.config.js` also lints `design/**/*.ts`.
- **H-3 WP-11:** import `apps/client/src/styles/global.css` once in the root layout (tokens + fonts). Do not set `no-restricted-syntax` for `apps/client` files in another ESLint config object — it replaces the token-literal list; extend `tokenLiterals` in `eslint.config.js`. Under reduced motion, springs become instant (`reducedMotion.springs = 'instant'`); `duration`/`stagger` reduced values are in `reducedMotion`.
- **H-4 WP-15:** show `fonts/LICENSE-OpenDyslexic.txt`, `LICENSE-SymbolsNerdFont.txt`, `LICENSE-NerdFonts.txt` in About (UI-018). Note: the file the spec calls OFL is the OpenDyslexic (Bitstream-style) licence upstream.
- **WP-16:** the `--spring-*` CSS values use `linear()`: needs Safari/WKWebView 17.2+ and WebKitGTK 2.44+; on older webviews the whole `transition` declaration is dropped. Set the minimum webview accordingly.
- **WP-10/hub:** `anima_theme::guac_color_scheme()` gives the guacd `color-scheme` parameter (RA-010).
- `quality/generated.txt` needed no change (`design/tokens/reference.md` is not measured).

## Contract change requests (escalation, CON-001)

- **K5 missing two motion tokens.** Spec 5.7.2 lists `projection-deceleration` (0.998, MOT-032) and `rubberband-constant` (0.55, MOT-033); K5 does not name them. WP-02 did not add them. WP-11 needs them for gestures. Request: add both to K5 (TypeScript only, e.g. under `threshold` or a new `gesture` object).
- **Clarification:** the TS output adds `reducedMotion` and `breakpoint` objects (D-5, D-11) — values of K5 names, no new token names. Confirm this reading of K5.

## Deferred (for issue #1, R-007)

- Semantic shadow-colour token so `elevation-2/3` do not reference `nord-0` directly (needed for the light theme, UI-005/TOK-011; a K5 change).
- `size-adjust` / ascent-override on `@font-face` to make the font swap shift exactly 0 in Chromium (measured CLS 0.002; WebKit 0) (MOT-007).
- Route the remaining renderer guards in `design/src` through `TokenSourceError` with token paths (CQ: typed errors).
- `prefers-reduced-transparency` block verified in Chromium only; WebKit cannot emulate it (MAT-006, MV-001).

## Open questions

- None blocking. Decisions D-1..D-11 in the plan were made by the package orchestrator (notably: CSS springs as `linear()` curves; `target-min-touch` emitted as 44px; `elevation-1` shadow `none`, border drawn by the component; tints with `color-mix` on `var(--surface)`).

## Notes

- Task branches `wp/02-tokens/{build,fonts,lint,theme,cleanup}` are merged into the package branch and can be deleted.
- `.gitignore` ignores every `build/` folder; the first T1 tip lost its build code that way (caught, moved to `design/src/`). Worth knowing for any package that names a source folder `build`.
