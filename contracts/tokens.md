# K5 — Design token names

Status: **frozen for Phase 1** (change only through a programme ruling, CON-001).

WP-02 owns the token source (`design/tokens/`, W3C DTCG) and the build. Every other package uses **only the names below**. Values come from spec Sections 5.1, 5.2, 5.5, 5.6 and 5.7; WP-02 may tune a value, but a name change is a contract change.

## Outputs

| Output | Path | Form |
| --- | --- | --- |
| CSS | `apps/client/src/styles/tokens.css` | `--<name>` custom properties on `:root`, plus `@media (prefers-reduced-motion: reduce)` and `(prefers-reduced-transparency: reduce)` overrides |
| TypeScript | `apps/client/src/lib/motion/tokens.ts` | `export const spring = { default: { dampingRatio, responseS, stiffness, damping }, … }`, `duration`, `ease`, `stagger`, `threshold` objects |
| Rust | `crates/theme` | `pub const NORD: [Rgb; 16]`, `pub fn guac_color_scheme() -> String` (RA-010) |

The TypeScript output may also hold `reducedMotion` and `breakpoint` objects: they carry values of the names below and add no new names (R-013).

All three are generated files (listed in `quality/generated.txt`). Do not edit them by hand.

## Names

Components use semantic or component names. Primitive names (`nord-*`) appear only inside the token source.

| Group | Names |
| --- | --- |
| Colour, primitive | `nord-0` … `nord-15` |
| Colour, semantic | `bg`, `surface`, `surface-raised`, `border`, `text`, `text-muted`, `accent`, `accent-alt`, `accent-deep`, `info`, `status-up`, `status-down`, `status-degraded`, `status-warning`, `status-maintenance`, `status-unknown` |
| Colour, tint | `accent-tint`, `status-up-tint`, `status-down-tint`, `status-degraded-tint`, `status-warning-tint`, `status-maintenance-tint`, `status-unknown-tint` |
| Font family | `font-ui`, `font-mono`, `font-icon` |
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
| Gesture | `projection-deceleration`, `rubberband-constant` (TypeScript only, in a `gesture` object; spec 5.7.2, MOT-032, MOT-033; R-013) |

Space numbers follow the 4-pixel grid index (`space-4` = 1 rem = 16 px), as in spec 5.5.3.

## Rules for consumers

- No raw colour, size, duration or easing literal in `apps/client` outside the generated files (the token-literal gate: a Stylelint rule and an ESLint rule, owned by WP-02).
- Motion code reads springs only through `$lib/motion/` helpers (MOT-001). It never converts damping ratio and response itself.
- Reduced motion changes `duration-*`, `spring-*` and `stagger-*`; it never changes `threshold-*` (MOT-021).
