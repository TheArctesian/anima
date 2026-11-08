# K5 — Design token names

Status: **frozen for Phase 3** (change only through a programme ruling, CON-001). Phase 2 additions are marked (Phase 2). Phase 3 additions are marked (Phase 3).

WP-02 owns the token source (`design/tokens/`, W3C DTCG) and the build. Every other package uses **only the names below**. Values come from spec Sections 5.1, 5.2, 5.5, 5.6 and 5.7; WP-02 may tune a value, but a name change is a contract change.

## Outputs

| Output | Path | Form |
| --- | --- | --- |
| CSS | `apps/client/src/styles/tokens.css` | `--<name>` custom properties on `:root`, plus `@media (prefers-reduced-motion: reduce)` and `(prefers-reduced-transparency: reduce)` overrides |
| TypeScript | `apps/client/src/lib/motion/tokens.ts` | `export const spring = { default: { dampingRatio, responseS, stiffness, damping }, … }`, `duration`, `ease`, `stagger`, `threshold` objects |
| Rust | `crates/theme` | `pub const NORD: [Rgb; 16]`, `pub fn guac_color_scheme() -> String` (RA-010; used by the Guacamole tunnel, which SSH no longer uses since R-062; the SSH terminal takes its Nord theme from the CSS tokens) |

The TypeScript output may also hold `reducedMotion` and `breakpoint` objects: they carry values of the names below and add no new names (R-013).

All three are generated files (listed in `quality/generated.txt`). Do not edit them by hand.

## Names

Components use semantic or component names. Primitive names (`nord-*`) appear only inside the token source.

| Group | Names |
| --- | --- |
| Colour, primitive | `nord-0` … `nord-15` |
| Colour, semantic | `bg`, `surface`, `surface-raised`, `border`, `text`, `text-muted`, `accent`, `accent-alt`, `accent-deep`, `info`, `status-up`, `status-down`, `status-degraded`, `status-warning`, `status-maintenance`, `status-unknown`, `scrim` (R-049), `surface-hover` (CR-10: the fill under `text` for a hovered row or control) |
| Colour, chart (Phase 2) | `chart-grid`, `chart-axis`, `chart-threshold` (the 85 % line, MET-004), `chart-series-1` … `chart-series-6` (Nord frost then aurora, so up to six GPUs or mounts differ; K5) |
| Colour, tint | `accent-tint`, `status-up-tint`, `status-down-tint`, `status-degraded-tint`, `status-warning-tint`, `status-maintenance-tint`, `status-unknown-tint` |
| Font family | `font-ui`, `font-mono`, `font-icon`, `font-ui-opendyslexic`, `font-mono-opendyslexic` (added by operator decision, 2026-11-05: the Settings font choice, UI-020) |
| Font size | `text-xs`, `text-sm`, `text-md`, `text-lg`, `text-xl`, `text-2xl` |
| Line height | `leading-tight`, `leading-normal`, `leading-relaxed` |
| Tracking | `tracking-display`, `tracking-body`, `tracking-small` |
| Weight | `weight-regular`, `weight-bold` |
| Space | `space-0`, `space-1`, `space-2`, `space-3`, `space-4`, `space-5`, `space-6`, `space-8`, `space-10`, `space-12`, `space-16` |
| Radius | `radius-sm`, `radius-md`, `radius-lg`, `radius-pill` |
| Border width | `border-thin`, `border-focus` |
| Icon size | `icon-sm`, `icon-md`, `icon-lg` |
| Target size | `target-min-desktop`, `target-min-touch` |
| Elevation | `elevation-0`, `elevation-1`, `elevation-2`, `elevation-3` |
| Material | `material-chrome`, `material-panel`, `material-overlay` (each expands to `-bg`, `-blur`, `-saturate` custom properties) |
| Layer | `layer-base`, `layer-sticky`, `layer-nav`, `layer-popover`, `layer-modal`, `layer-toast` |
| Breakpoint | `bp-compact`, `bp-medium`, `bp-wide` (TypeScript only; CSS custom properties cannot hold media queries) |
| Duration | `duration-instant`, `duration-press`, `duration-fast`, `duration-base`, `duration-highlight` |
| Easing | `ease-enter`, `ease-exit` |
| Spring | `spring-default`, `spring-snappy`, `spring-move`, `spring-momentum` |
| Stagger | `stagger-list` |
| Threshold | `threshold-drag`, `threshold-loading`, `threshold-loading-min` |
| Opacity | `opacity-disabled`, `opacity-band` (Phase 2; a literal opacity other than 0 and 1 in a component is refused, TOK-003) |
| Gesture | `projection-deceleration`, `rubberband-constant` (TypeScript only, in a `gesture` object; spec 5.7.2, MOT-032, MOT-033; R-013) |

Space numbers follow the 4-pixel grid index (`space-4` = 1 rem = 16 px), as in spec 5.5.3.

## Light theme (Phase 3)

- No new semantic or component names. The light theme changes only semantic values (TOK-011).
- The token source gains a `light` mode. `tokens.css` gains `:root[data-theme="light"] { … }`, and
  `@media (prefers-color-scheme: light) { :root[data-theme="system"] { … } }`. `data-theme="dark"`, or no
  attribute, keeps today's values. `prefers-contrast: more` applies on top of either mode (MAT-006).
- D-27 takes option B, derived against `nord-4` (R-086): the primitive row adds `nord-11-deep`,
  `nord-12-deep`, `nord-13-deep`, `nord-14-deep` and `nord-15-deep`. Each is the Nord colour mixed toward
  `nord-0` until it reaches 3:1 against `nord-4`, the darkest light surface, so it holds 3:1 on `nord-4`,
  `nord-5` and `nord-6`. Only the light mode's `status-*` and `chart-*` values use them. Components never
  name them.
- Light surfaces (R-086): `bg` is `nord-5`, `surface` is `nord-6`, `surface-raised` is `nord-4`. The light
  `accent` is `nord-3`; no derived accent primitive is added.
- `surface-hover` (CR-10): dark is `nord-3`, today's hover look. Light is a grey that gives `text` at least 4.5:1 and differs visibly from `surface` and `surface-raised`. A component never uses `border` as a fill.
- Reduce motion in Settings (D-28, MOT-043, R-085): `tokens.css` gains `:root[data-motion="reduce"] { … }`, holding the
  reduced values of the motion tokens, and `@media (prefers-reduced-motion: reduce) { :root[data-motion="full"] { … } }`,
  putting their full values back. No attribute (System) leaves the media query to decide. No token name is added, and
  the blocks hold motion tokens only, so they never meet the theme blocks.
- The Rust output (`crates/theme`) does not change.

## Rules for consumers

- No raw colour, size, duration or easing literal in `apps/client` outside the generated files (the token-literal gate: a Stylelint rule and an ESLint rule, owned by WP-02).
- Motion code reads springs only through `$lib/motion/` helpers (MOT-001). It never converts damping ratio and response itself.
- Reduced motion changes `duration-*`, `spring-*` and `stagger-*`; it never changes `threshold-*` (MOT-021).
