# WP-07 report (package orchestrator)

Status: **done**. All tasks are accepted, the final package review (Opus, security) is accepted, and `bun run gates` passes 12/12 on the branch tip. Not pushed, not merged into `phase-1` or `main`.

| Item | Value |
| --- | --- |
| Branch | `claude/wp-07-credentials-5b3ac6` |
| Tip | `b03a75f` (includes `phase-1` at `75d1690`) |
| Already on `phase-1` | T1 (`81b6c0c`, merged as `78a6461`), T2 (`26ccd79`, merged as `4d61119`) |
| New since the early merges | final-review fixes `a4c93ea`, `with_ssh` (R-017) `f1501d7`, migration restore `8375e8f`, doc wrap `12a6387`; merged as `08537e4` |
| Plan and review log | [WP-07.md](WP-07.md) |
| Spec | SEC-003, SEC-004, SEC-005, LOG-007, CLI-004 (plus DEV-015 cascade and API-005 batch read) |

## Gates (`bun run gates`, tip `b03a75f`)

| # | Gate | Result |
| --- | --- | --- |
| 1 | cargo fmt --check | pass |
| 2 | cargo clippy --all-targets -- -D warnings | pass |
| 3 | cargo test (throwaway PostgreSQL, R-009) | pass (hub: 54 unit, 33 integration; 24 of the integration tests are credentials tests) |
| 4 | bun install --frozen-lockfile | pass |
| 5 | bun run lint | pass |
| 6 | bun run check | pass |
| 7 | bun run test | pass |
| 8 | client build | pass |
| 9 | quality:limits | pass |
| 10 | quality:names | pass |
| 11 | quality:imports | pass |
| 12 | quality:baseline --base phase-1 | pass |

## What the package delivers

- `credentials::Vault`:
  - XChaCha20-Poly1305, a random 192-bit nonce for each seal.
  - The key comes from `ANIMA_KEY_FILE` as base64 of 32 bytes.
  - `key_id` is a domain-separated SHA-256 prefix of the key.
  - `seal`/`open` bind a context as associated data (R-010). The associated data is length-framed and pinned by a known-answer test.
- `Secret`: zeroized on drop; no Debug, Display, Serialize, Clone or PartialEq (trait-absence tests).
- `Sealed`: accessors, `from_parts`, and a versioned one-column `to_bytes`/`from_bytes` for WP-05's `totp_secret_enc`.
- Migration `0200_credentials_secrets.sql`: the K2 columns, `unique (device_id, kind)`, the kind CHECK, and the foreign key to `devices.devices` with ON DELETE CASCADE.
- `SshSecret`, `store_ssh`, `load_ssh`, `has_ssh`, `clear_ssh` (R-010) and `with_ssh` (R-017).
  - `store_ssh` and `clear_ssh` each run in a savepoint under a per-device `pg_advisory_xact_lock`.
  - The lock serialises concurrent stores and clears; the tests kill the mutant that removes it.
  - Rows are sealed with the context `credentials.secrets:<device>:<kind>`.
  - Moved, relabelled, re-keyed and tampered rows fail with `Decrypt`, `WrongKey` or `Malformed`.
- No logging in the domain. Errors are generic sentences and never contain a path, key or plaintext.

## Handoffs (PKG-001)

1. **Already accepted at the early merges:**
   - `pub mod credentials;` in `crates/hub/src/lib.rs`.
   - Root `Cargo.toml` workspace dependencies: `chacha20poly1305` 0.11 (with `zeroize`), `zeroize` 1.9, `sha2` 0.11 (with `zeroize`), `base64` 0.23; dev: `static_assertions` 1.1, `tempfile` 3.27. Also the matching `crates/hub/Cargo.toml` lines and `Cargo.lock`.
   - `mod credentials;` in `crates/hub/tests/it/main.rs`.
2. **Open: `main.rs` wiring.** Nothing builds the vault yet, so SEC-004 does not hold at runtime until this is added. The owner is WP-01 or the programme orchestrator (R-002). Add:
   - `let vault = credentials::Vault::from_key_file(&config.key_file)?;`, with a `StartError` variant from `credentials::Error`;
   - `.data(vault)` in `build_schema`;
   - and pass the vault to `sessions::tunnel_router`.
3. **WP-06:**
   - Call `store_ssh`/`clear_ssh` before any `FOR UPDATE` or DELETE on the device row in the same transaction. A violation is detected as a deadlock (40P01, returned as `Error::Database`), never a hang.
   - The audit `change` must not contain the secret.
   - Map `UnknownDevice` to NOT_FOUND.
4. **WP-05:** for TOTP, use the context `access.operators.totp:<operator_id>` with `Sealed::to_bytes`/`from_bytes`.
5. **WP-10:** the only caller of `load_ssh`. It must not log the result.

## Deferred items (for issue #1, R-007)

- **COM-002 sweep across the repository.** `phase-1` code comments still cite plan, contract and ruling IDs, for example `crates/hub/src/paging.rs:1-10` (K1, R-014) and `crates/hub/src/scalars.rs:1` (K1, R-012). WP-07 removed them from its own files only (AGENTS.md: change everywhere in a separate change).

## Open questions and risks

- **Stale-ciphertext replay (accepted risk).** Someone with database write access can put back an older sealed value for the same device and kind, and it opens, because the fixed context has no row version. Detecting this needs a version or MAC kept outside the database. Is that wanted in a later phase?
- **Key rotation.** `key_id` is stored with every value, and `open` refuses another key's value with `WrongKey`. Phase 1 has one key and no re-seal path. Does rotation belong on issue #1?
- **No permission check on the key file.** The vault does not check the key file's mode, because Docker secrets are mounted world-readable inside the container (D-1). The VPS file permissions are WP-04's concern.
- **R-017 test.** The test that a device "with another kind only" is left out cannot be built: the kind CHECK admits only SSH kinds. The other `with_ssh` tests are there (empty slice, duplicates, unknown ids, devices without a secret).
- **K3 `Db`.** K3 writes `db: &Db`, but no `Db` alias exists, so the functions take `&PgPool`. Either add the alias or update the contract text.
- **Migration 0200 stays byte-identical.** A comment-only edit was reverted, because sqlx checks the checksum of applied migrations.

## Contract change requests

None open. E-1 was answered by R-009, E-2 and E-3 by R-010, and the `with_ssh` request by R-017.
