# Anima — agent and contributor rules

Anima is a self-hosted device manager: a Tauri client (macOS, Linux; later web and iOS),
a Rust hub (axum, async-graphql, PostgreSQL) and a Guacamole gateway (`guacd`).
The design spec is [docs/design-spec.md](docs/design-spec.md). It is the source of truth;
cite requirement IDs (for example `CHK-011`) in commits and pull requests.

## Before you change code

- Read the spec sections for the area you touch. Section 12 (code quality) and
  Section 13 (repository structure) apply to every change.
- Match the neighbouring code. If the local pattern is wrong, change it everywhere in a
  separate change.
- Never read secret files (`.env*`, Docker secrets, key files). Never put credentials,
  tokens or session content in logs (`LOG-007`).
- Do not change the Hostinger VPS, Headscale or any cloud account without the operator's
  confirmation in chat (`VPS-005`).

## Code quality: CUPID

Every change moves code toward the five CUPID properties. Every review finding names the
property it serves. Limits point at places to look; never change code only to satisfy a number.

- **Composable** — small public surface; pure core, I/O at the edge; no single-use
  interfaces, traits or factories.
- **Unix philosophy** — each unit has one purpose you can state without "and". Do not
  split code that always changes together.
- **Predictable** — names tell the truth; core logic takes time, randomness and config
  as parameters; typed errors (`thiserror` per Rust domain, no `anyhow` in domains);
  logs and metrics only at I/O boundaries; never swallow an error.
- **Idiomatic** — rustfmt + Clippy `pedantic` (deny warnings); typescript-eslint `strict`
  + `stylistic`, eslint-plugin-svelte, Svelte 5 runes; one GraphQL client per domain.
- **Domain-based** — folders and types are operator nouns (`devices/`, `checks/`,
  `sessions/`, `DeviceId`, `Fingerprint`). Banned in names: `Manager`, `Service`,
  `Helper`, `Handler`, `Processor`, `Factory`, `Util`, `data`, `info`, `item`, `misc`,
  `common`, and the folders `utils/ helpers/ common/ components/ services/ models/`.

### Limits (code lines, blanks and comments excluded)

| File | Function | Params | Complexity | Nesting | Files per folder |
|---|---|---|---|---|---|
| 300 | 60 | 4 | 15 | 4 | 12 |

`quality/baseline.json` is shrink-only: an entry may decrease, never increase, and is never
added by hand. Generated files are listed in `quality/generated.txt`. Suppress a lint only
with a reason: `#[expect(clippy::rule, reason = "...")]` or
`// eslint-disable-next-line rule -- reason`.

### Where code goes

- Hub domain folder: `<domain>.rs` (public face), `model.rs`, a pure-logic file named for
  the concept, `store.rs` (the only SQL for that domain's tables), `graphql.rs`, `tests.rs`.
- A domain never imports another domain's `store`. No import cycles.
- `main.rs` and `schema.rs` wire; they hold no business rules.
- Client: routes compose components; logic lives in `.ts` beside the component;
  shared code lives in `$lib/<domain>/`.
- Tauri shell (`apps/desktop/src-tauri`) holds only OS work: windows, key store, web views.
- Design values come from tokens (`design/tokens/`); motion uses the helpers in
  `$lib/motion/` (spec Sections 5.5 and 5.7).

### Fixing a failed limit

Measure, read the whole unit and its callers, find the seam (domain concept, pure vs I/O,
UI section, pipeline stage), move code without rewriting it, name the new unit for the
domain, run the gates, then lower the baseline.

### Review checklist

- **C** Is every export used outside its module? Usable from the signature alone? I/O at the edge?
- **U** One sentence, no "and", for each new or changed unit?
- **P** Names truthful? Core deterministic? Errors typed, logs at boundaries?
- **I** Reads like the language's standard style and its neighbours?
- **D** Names from the domain? Would a newcomer find it?
- Did a baselined file grow, or a suppression appear without a reason?

## Tests

Follow the `test-audit` rules: test behaviour, not structure. If moving code breaks a test,
fix the test. Rust integration tests live in one crate per binary (`tests/it/main.rs`).

## Gates

Run `bun run gates` before you ask for review and before every merge. There is no CI; gates run on the machine that makes the change.

```sh
bun run gates
BASE_REF=my-branch bun run gates   # baseline gate compares against this ref (default: phase-1)
```

The gates run in this order and stop at the first failure: `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo test`, `bun install --frozen-lockfile`,
`bun run lint`, `bun run check`, `bun run test`, `bun run --cwd apps/client build`,
`bun run quality:limits`, `bun run quality:names`, `bun run quality:imports`,
`bun run quality:baseline -- --base $BASE_REF`. See [quality/README.md](quality/README.md).
