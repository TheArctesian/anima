# anima-cli

The `anima` command line (docs/cli-mcp-plan.md). `hub/` is the only network code (CMD-003);
`devices/` is the worked example of a noun. Copy it.

## Add a noun (for example `checks/`)

1. `src/checks.rs`: `pub mod documents; pub mod list; ...` and `pub enum CheckCommand`
   (`#[derive(Debug, clap::Subcommand)]`) with `pub async fn run(self, invocation: &Invocation)
   -> Result<(), Failure>` that matches each verb to its `Args::run`.
2. `src/checks/<verb>.graphql`: one operation per file, named `Check<Verb>` (`query CheckGet(...)`).
   Select only what the command prints; never a secret field.
3. `src/checks/documents.rs`: one struct per file, as in `devices/documents.rs`:
   `#[derive(GraphQLQuery)] #[graphql(schema_path = "../../contracts/schema.graphql",
   query_path = "src/checks/get.graphql", response_derives = "Debug, Serialize",
   variables_derives = "Debug", skip_serializing_none)] pub struct CheckGet;`
   The build fails when the document does not agree with K1 (CMD-004). Import the custom
   scalars the document uses from `crate::hub::scalars`. A document whose variables or answer
   hold a secret derives neither `Debug` nor `Serialize` (see `auth/documents.rs`).
4. `src/checks/<verb>.rs`: `#[derive(Debug, clap::Args)] pub struct GetArgs` and its `run`:
   `let hub = invocation.hub()?;` then `hub.run::<CheckGet>(check_get::Variables { .. }).await?`.
5. `src/lib.rs`: one `pub mod checks;` line. `src/cli/`: one variant and its arm in the group the
   noun belongs to: `fleet.rs` (what the operator works on), `hub_records.rs` (what the hub keeps
   and decides) or `program.rs` (the program itself). `cli.rs` joins the groups and does not change.
6. `tests/it/coverage/k1_exceptions.rs`: delete the entries your documents now use. The
   coverage test (CMD-031) fails until you do, and for any K1 root field neither used nor
   excepted.

## Print

`invocation.output.data(&value, table)?` prints `value` as JSON with `--json` (K1 field names,
from the generated `Serialize`) or else the `Table` that `table(&value)` builds: `Table::new`
with column names and `row`s for a list, `Table::record` with `pair`s for one thing.
`Cell::status` colours a K1 `Status`; `output::text::enum_name` gives an enum's K1 name. Cells
are cleaned of control characters. Messages for people go through `invocation.output.note`
(standard error, silent with `--json`). Lists take `--limit`, `--after` and `--all`
(`devices/list.rs`, CMD-026).

## Fail

Return `Failure`; never print an error yourself. A hub error becomes `Failure::Hub` through `?`
and keeps its K7 code; `failure.rs` maps codes to exit codes (CMD-021: NOT_FOUND 3,
UNAUTHENTICATED 4, FORBIDDEN 5, CONFLICT 6, UNAVAILABLE 7, VALIDATION and usage 2) and hints.
For your own checks use `Failure::NotFound { message, hint }` or `Failure::Usage`. Ask through
`invocation.prompter` (`ask`, `ask_secret`, `confirm`): it fails with exit 2 and names the flag
under `--no-input` (CMD-023). A delete calls `confirm` (`--yes`, CMD-024). A secret comes from
a prompt, standard input or `--from-file`, never an argument (CMD-025).

## Watch

A `watch` command is a subscription document and a loop (`events/watch.rs` is the example):
`src/<noun>/watch.graphql` (`subscription DeviceStatusChanged(..)`), its `GraphQLQuery` struct, then
`Subscription::open::<DeviceStatusChanged>(&hub, variables).await?` (`hub/subscription.rs`) and
`while let Some(answer) = stream.next().await { let value = answer?.device_status_changed; .. }`.
The socket speaks `graphql-transport-ws`, sends the token in `connection_init` (never in the URL),
answers pings, and maps close code 4403 to `UNAUTHENTICATED`. Print each answer with
`output::stream::emit` (one line for a person, one JSON object with `--json`) and stop when it
returns `Reader::Gone`. Test it with `SubscriptionHub::start(Script::Next(..))`. `output::stream::emit` prints one change as a line or as
JSON; `release log --follow` and `device metrics --watch` are `watch` flags on a noun's own command.

## Test

Integration tests run the real program: `tests/it/<group>/<noun>.rs` (a folder when it grows), one
`mod` line in the group file (`tests/it/fleet.rs`); `fleet/devices.rs` is the example.
`FakeHub::start()` serves `POST /graphql` and answers by operation name
(`hub.answer("CheckGet", data(json!(..)))` or `refusal("NOT_FOUND", "..")`); `hub.seen()`
returns what it was sent. `Anima::new().with("ANIMA_TOKEN", ..).run(&[.., "--hub",
&hub.origin])` returns stdout, stderr and the exit code. Assert on table and `--json` output,
on exit codes, and that no output holds a token (`assert_never_shows`). Use the K6 fixtures.
Tests keep tokens in the 0600 file (`ANIMA_TOKEN_STORE=file`), never the key store.

## Add an MCP tool

`anima mcp` lives in `src/mcp/` (docs/cli-mcp-plan.md Section 7). `tools/<set>.rs` holds one tool
set: an enum with one variant per tool, its `ALL` list, `definition` and `call`.

1. Add the variant, and its arm in `definition` and `call`. A new set also needs a `ToolSet`
   variant (`catalogue.rs`) and a `Tool` variant (`tools.rs`).
2. `definition`: `Definition::new::<Input>(name, access, effect, description).returns(&[..])`.
   The name is `anima_<noun>_<verb>` (MCP-011). The description tells an agent that has never
   seen Anima what it returns, when to use it and what to call next. `Access::Login` for what
   INT-003 denies an API key.
3. `Input`: `#[derive(Deserialize, JsonSchema)]` with `deny_unknown_fields` and a doc comment on
   each field (it becomes the schema). Lists flatten `arguments::Page`; `detail` adds fields.
4. `call`: `parse` the input, resolve names with the noun's resolver, then
   `hub.run::<Document>(..)` with the noun's existing document. Return a JSON object of named
   fields; the server cleans and caps every string (MCP-008). A delete calls `confirmed`.
5. Never a tool that takes or returns a secret, makes a trust decision or opens a session
   (MCP-004..006). Test it over JSON-RPC in `tests/it/mcp/`.
