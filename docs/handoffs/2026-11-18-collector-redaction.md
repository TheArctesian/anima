# Collector and redaction session handoff

Date: 2026-11-18 (America/Los_Angeles). State: checkpoint; integration is unfinished.
Branch: `handoff/2026-11-18-collector-redaction`.
Base: `cef2e9173c6e50ce8cd7ef12e3fbec1469c0be7e`, the old local main.

## Saved work

The original uncommitted changes are saved here without implementing further features.
The `collector` crate supplies pure folder rules and read-position logic (COL-004, COL-006, COL-008).
The `redaction` crate extracts shared masking logic (LOG-007, RED-001 to RED-004).
The hub event redaction imports that crate. The workspace manifests and lockfile include both crates.
The existing extensions-plan edit records that collection belongs in `anima-agent`, with contract review before file access.
The old mobile HTML mockup is also retained at `docs/mockup/mobile/index.html`; it is a design artifact.

## Checks performed for this checkpoint

- `cargo test --offline --locked -p collector -p redaction`: passed; 8 collector and 8 redaction tests.
- The fixture values were inspected as synthetic examples, following the repository's test-audit rules.
- `cargo fmt --check`: passed across this workspace.
- `cargo clippy --offline --locked -p collector -p redaction --all-targets -- -D warnings`: passed.
- `git diff --check` is required before the checkpoint commit.
- Full workspace gates, hub integration tests, browser checks and live collection are not established by these focused tests.

## Resume

1. Read `AGENTS.md` and the current `origin/main` planning register before changing code.
2. Fetch origin. Review this checkpoint against current main in a separate worktree.
3. Port only the intended changes; main has advanced substantially since this base.
4. Current plans have moved to `docs/plans/`; do not restore an obsolete plan layout from this snapshot.
5. Complete the K8 and K-E2 contract review before adding filesystem readers or network ingestion.
6. Resolve the agent account, source folder permissions and macOS access permissions in the owning plan.
7. Check current issue #6 and compute-plan OD-C2 before repeating the old session's proposed documentation updates.
8. Run the current repository's full gates before review or merge. No merge or deployment occurred during this checkpoint.

## Session evidence

Collector conversation: `620c0127-1b80-48f8-83bc-5845ec998f81` (collector/redaction side conversation).
Mockup conversation: `c8c99fd3-2604-4a20-b113-a8b11329aeb1` (Anima mobile app mockups).
The earlier session reported passing focused tests but had not committed these files.
Session transcripts remain local; this handoff contains task state only.
