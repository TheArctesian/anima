# Anima — agent and contributor rules

Anima is a self-hosted device manager: a Tauri client (macOS, Linux; later web, iOS and Android),
a Rust hub (axum, async-graphql, PostgreSQL) and a Guacamole gateway (`guacd`).
The design spec is [docs/spec/design-spec.md](docs/spec/design-spec.md). It is the source of truth;
cite requirement IDs (for example `CHK-011`) in commits and pull requests.

## Start of a session

Do these steps in this order in each session, before you change code. The planning documents are
the one source of truth; [docs/work-plan.md](docs/work-plan.md) is the entry point.

1. **Get `main`, in a worktree of your own.** Run `git fetch origin`. Do not work in the
   operator's main checkout and do not change its branch: make a worktree,
   `git worktree add ../<name> origin/main`, and read the documents there. A plan that is only on
   a branch is not a plan (PLN-011).
2. **Find the work.** If the operator named the work, find the plan that owns it in
   [docs/plan/ownership.md](docs/plan/ownership.md) and [docs/plan/register.md](docs/plan/register.md).
   If the operator named no work, open [docs/plan/priorities.md](docs/plan/priorities.md) and take the
   first row, from the top, whose column "Waits for" names an agent. A row that waits for the
   operator is not yours: tell the operator what it waits for, then go to the next row.
3. **Find the task.** A row has work for you only if its "Waits for" says "Agent:" with a link. Open
   that plan or work item. Take the named task if its state is "Not started" and no open decision
   blocks it. Do not take a task that is "In progress" with no claim: its session may be alive; find
   its branch or ask the operator. Do not build from a proposal, or from a plan whose state is Draft,
   Direction decided or Not checked, unless a ruling names the task (PLN-007). If the plan has no
   task row for the work, add the row first (PLN-013). If the operator gave an idea: first look in
   [docs/plan/ownership.md](docs/plan/ownership.md); if a plan owns the subject, say which
   requirement has it or change that plan; if none does, record the idea and stop
   ([docs/ideas/README.md](docs/ideas/README.md)). "Build" means that the operator asked for code.
4. **Read the claims.** Look at [docs/claims/](docs/claims/README.md) on `origin/main`. If a claim
   holds the task, a different session builds it. Take the next task or ask the operator.
5. **Claim the task.** Make the claim in a small worktree of its own, so that it never carries
   your other changes: `git worktree add --detach ../claim-<task> origin/main` (for example
   `../claim-e0-5`). If that folder already exists, a session of this computer has or had this
   claim in progress. Do not use the folder. Look at it and ask the operator. In the worktree,
   write `docs/claims/<task>.md` with the task, the plan, the name of this machine in Anima, your
   coding agent, the branch and the day. If the task is new, add its row to the plan in the same
   commit and run `bun run quality:register`. Run `bun run quality:documents`, commit, and run
   `git push origin HEAD:main`. If the push is refused, run `git fetch origin` and
   `git reset --hard origin/main` in the same worktree, and read the claims again. The reset
   removes your claim commit. If the task is free, write the claim file again, commit, and push
   again. When the push has succeeded, or you give the task up, run
   `git worktree remove ../claim-<task>`. Then start or update your branch from `origin/main`
   (`git switch -c <branch> origin/main`, or `git merge origin/main` on a branch that exists), so
   that it holds the claim (PLN-022).
6. **Build** on a branch, in its own worktree, from the task and the requirements that it names.
   Build in the order hub API, `anima` command, MCP tool, client (CMD-030). If you change the
   layout of a screen that has a mockup, change the mockup in the same commit (PLN-019). If the
   gates cannot prove the function, add its entry to
   [docs/need-human-testing.md](docs/need-human-testing.md) (PLN-014).
7. **Finish.** Run `bun run gates`. In the commit that merges the work: set the state of the task
   with its commit (PLN-008), delete your claim, and run `bun run quality:register` if the state of
   the plan changed (PLN-020).
8. **If you stop before the end,** delete your claim on `main` with the same steps as step 5 (a
   small worktree `../claim-<task>`, a commit that deletes the file, a push), and write in the task row what is
   built, unless you will continue on the same machine. Push your branch, so that a different
   session can find the work.

A question that only the operator can answer does not stop the session. Write it as a row in the
open decisions of the plan, run `bun run quality:register` so that it is in
[docs/operator-queue.md](docs/operator-queue.md) (PLN-023), tell the operator, and take a different task.

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
  + `stylistic`, eslint-plugin-svelte, Svelte 5 runes; one GraphQL client per domain. Go (`link/` only):
  gofmt, `go vet`, staticcheck and revive; one error type or sentinel for each package, checked with
  `errors.Is` or `errors.As`.
- **Domain-based** — folders and types are operator nouns (`devices/`, `checks/`,
  `sessions/`, `DeviceId`, `Fingerprint`). Banned in names: `Manager`, `Service`,
  `Helper`, `Handler`, `Processor`, `Factory`, `Util`, `data`, `info`, `item`, `misc`,
  `common`, and the folders `utils/ helpers/ common/ components/ services/ models/`.

### Limits (code lines, blanks and comments excluded)

These limits apply to code. They do not apply to the planning documents in `docs/` (PLN-015).

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
- Command line (`crates/anima-cli`): one folder for each operator noun, with its GraphQL
  documents, its commands and its MCP tools. Only `hub/` talks to the network.
- Link (`link/`, Go): one package for each operator noun. Only `tunnel/` imports the Tailcat library.
  `cmd/anima-link/main.go` wires; it holds no rules.
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
- Does a new or changed hub function have its `anima` command and its MCP tool (CMD-030)?

## Command line first

The plan is [docs/plans/cli-mcp-plan.md](docs/plans/cli-mcp-plan.md). Build each new function in this order
(CMD-030, operator, 2026-11-12):

1. The hub API (and its contract).
2. The `anima` command (`crates/anima-cli/`).
3. The MCP tool (`anima mcp`, same crate), unless Section 7 of the plan excludes it: a tool must
   not take or return a secret, make a trust decision or open a session.
4. The graphical client.

A function is not complete until steps 2 and 3 are done or the plan gives the reason that they do
not apply. Do not merge a change that adds a function only to the graphical client. The coverage
test (CMD-031) fails when a root field of `contracts/schema.graphql` has no command.

## Plans and progress

[docs/work-plan.md](docs/work-plan.md) is the entry point to all planning documents. Start there. Its
parts are in `docs/plan/`: the priority order (`priorities.md`), the register of all plans with their
state (`register.md`) and the rules (`rules.md`, PLN-001 to PLN-023, operator, 2026-11-15 to 2026-11-17).

- Plan first (PLN-013). Before you change code, find the plan that owns the work, read it, and build
  from it. If the plan does not include the work, change the plan first. If no plan owns the work,
  write a short proposal with its register row first. If the build must be different from the plan,
  change the plan in the same commit. A correction of a defect that adds no function needs no plan.
- The path is idea, proposal, plan, contract, tasks, check (PLN-018). An idea is a folder in
  [docs/ideas/](docs/ideas/README.md); do not build from it. A proposal is in `docs/proposals/`; an approved one moves to `docs/plans/<name>-plan.md`.
- A session claims its task (PLN-022). One file in [docs/claims/](docs/claims/README.md) for each task that
  a session builds now. Read them before you take a task; delete yours in the merge.
- A mockup agrees with the application (PLN-019). If you change the layout of a screen that
  [docs/mockup/README.md](docs/mockup/README.md) lists, change its mockup in the same commit.
- Do not build from a document that is not in Section 9 of the work plan.
- Do not make a new plan, proposal or task breakdown unless the same commit adds its row to
  Section 9 and names the row of Section 4 that it serves. If no row applies, ask the operator.
- If a plan already owns the concern, change that plan. Do not write a second one.
- Write each plan in STE (design spec Section 0), with a title, a version, a date and a status.
- Change the state of a task in the commit that does the work. "Done" must name a commit, an
  acceptance record or a ruling.
- The state of a plan is in the header of the plan (`kind:`, `state:`, `evidence:`). Change it there, then
  run `bun run quality:register`: it writes the register. Do not change the register table by hand (PLN-020).
- A report, a log or a procedure must not add a requirement.
- Do not use GitHub issues, labels or project boards as a store (PLN-012, R-119). A work item or a defect
  is a document in [docs/work-items/](docs/work-items/README.md). Deferred work is a line in
  [docs/deferred.md](docs/deferred.md). An open decision is a row in its plan.
- One entry, one job, one owner (PLN-016). Each planning document is reachable from the work plan, has
  one job, and points to the owner of a fact; it does not copy the fact. One document defines each
  requirement identifier. `bun run quality:documents` checks this (PLN-017).

## Tests

Follow the `test-audit` rules: test behaviour, not structure. If moving code breaks a test,
fix the test. Rust integration tests live in one crate per binary (`tests/it/main.rs`).

## Human testing

[docs/need-human-testing.md](docs/need-human-testing.md) lists each check that only a person can do. It is the
one list of such checks (PLN-014): a task that waits for a check is not done until its entry says Passed.

- When you build a function that the gates cannot prove, add an entry in the same commit. This
  applies to work that needs a real account (GitHub, Apple), a real device, a real network, a
  window of the operating system, or a time longer than a test can run.
- An entry gives: what was built and where, what the tests did not cover, the steps, and the
  expected result of each step. Write steps that the operator can do with no help.
- Do not write "done" or "verified" for such a function in a plan, a report or a message. Say
  that it is built and give the entry (`HT-nnn`). PLN-009 applies.
- Only the operator sets an entry to Passed or Failed. If the operator tells you the result,
  write it in the entry with the date.
- When you correct a Failed entry, set it to Open again. Do not delete an entry.

## Gates

Run `bun run gates` before you ask for review and before every merge. A commit that changes only
`docs/claims/` or `docs/ideas/`, or a claim with the row of its new task, needs only `bun run quality:documents`. There is no CI; gates run on the machine that makes the change.
`rust-toolchain.toml` pins the Rust version for every machine; rustup installs it on the first cargo run. `deploy/hub/Dockerfile` must name the same version (a test compares them). Change both in one commit.

```sh
bun run gates
BASE_REF=my-branch bun run gates   # baseline gate compares against this ref (default: phase-2)
```

The gates run in this order and stop at the first failure: `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo nextest run --workspace` (with a throwaway PostgreSQL 17, R-009),
`cargo test --workspace --doc`, `cargo test --workspace --test pinning`, the six Go gates of `link/`
(`gofmt`, `go vet`, `staticcheck`, `revive`, `go test -race`, `go build`), `bun install --frozen-lockfile`,
`bun run lint`, `bun run check`, `bun run test`, `bun run --cwd apps/client build`,
`bun run quality:limits`, `bun run quality:names`, `bun run quality:imports`, `bun run quality:documents`,
`bun run quality:baseline -- --base $BASE_REF`. See [quality/README.md](quality/README.md).
The gates share compiled Rust between worktrees through sccache (`brew install sccache`, or
`cargo install sccache --locked`). When `sccache` is on `PATH`, the runner sets `RUSTC_WRAPPER` to it
and `CARGO_INCREMENTAL=0`, and prints `rust cache: sccache` first; without it, it prints
`rust cache: off` and builds from scratch. Each worktree still keeps its own `target/`: only compiler
outputs are shared, so a new worktree's first run reuses what any other worktree already built. Never
share one `CARGO_TARGET_DIR` between worktrees: cargo locks it, so parallel runs wait on each other.
Set `RUSTC_WRAPPER=` (empty) to run without the cache. `sccache --show-stats` shows the hit rate.
cargo-nextest must be on `PATH` (`cargo install cargo-nextest --locked`; put `~/.cargo/bin` on `PATH`).
Go must be on `PATH` (`brew install go`); `link/go.mod` sets the lowest Go version and pins the two lint tools, so
nothing else is installed for the Go gates.
Leave `DATABASE_URL` unset so each run gets its own PostgreSQL server: sqlx names test databases
by test path, so two worktrees that share one server destroy each other's databases (R-019).
