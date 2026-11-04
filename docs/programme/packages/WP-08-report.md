# WP-08 report (package orchestrator)

Status: **done**. Branch `claude/wp-08-guac-protocol-2cb3bb`, based on `phase-1` (latest, `75d1690` with R-015–R-017, merged in). Not pushed, not merged.
`bun run gates` on `94d48d8` (the tip before this line was filled in; this commit changes only this file): **12/12 pass**, exit 0. Plan, decisions D-1..D-9, handoffs and the review log: [WP-08.md](WP-08.md).

## What is delivered

`crates/guac-protocol/`: the Guacamole protocol library. No I/O, no async runtime, no dependency besides `thiserror` (`proptest` for tests only).

| Output | Path | Spec |
| --- | --- | --- |
| `Instruction` (redacting `Debug`) and `encode` (lengths in code points) | `src/instruction.rs` | GW-011, LOG-007 |
| Incremental `Decoder::feed`, typed `DecodeError`; guacd's limits as constants (`MAX_INSTRUCTION_BYTES` 8192, `MAX_ELEMENTS` 128, `MAX_LENGTH_DIGITS` 5); NUL refused (guacd cuts an element at NUL); sticky failure; memory bounded to 8 KiB per decoder | `src/decoder.rs` | GW-011 |
| Pure `Handshake` state machine: `select` → `args` → `size`, `audio`, `video`, `image`, `timezone`, `name`, `connect` → `ready`; version ≥ 1.5.0 | `src/handshake.rs` | GW-010, GW-014 |
| `Display` (+ `within_bounds`), `ArgValues` (hub values only, no `Debug`), `ConnectParams`, `is_zone_name` | `src/connect.rs` | GW-012, GW-013 |
| Tests: 76 (example, property and fuzz-style), one crate `tests/it/` | `tests/it/*.rs` | GW-010–014, LOG-007 |
| guacd SSH `args` fixtures 1.5.5 and 1.6.0 (official image, plus `--enable-ssh-agent` variants), built from `GUAC_SSH_CLIENT_ARGS` | `tests/it/fixtures/` | K4 §7 |

K3 surface: as frozen (`Instruction`, `encode`, `Decoder::feed`, `Handshake::new`, `on_instruction`), plus the names K3 leaves open (`DecodeError`, `HandshakeStep`, `HandshakeError`, `ConnectParams`) and `ArgValues`, `Display`, `is_zone_name` and the limit constants for WP-10.

K4 §7 handshake obligations: both `args` lists give the expected `connect` values at their exact positions; property tests show no client input (display, time zone) changes, adds or injects any `connect` value or instruction, so a forbidden name (GW-013) can never be emitted from client input.

Fail-closed additions beyond K4 (from the reviews): client time zone must be IANA-shaped (it reaches the SSH server as `TZ`); display bounds checked before anything is sent; a NUL in any emitted value and any emitted instruction over guacd's limits fail the handshake; gateway `error` text cut to 128 characters; the guacd connection ID redacted in `Debug`.

## Review summary

| Task | Rounds | Reviewers | Outcome |
| --- | --- | --- | --- |
| T1 `codec` | 2 | reviewer (Opus), reviewer-security (Opus) | accept. Found: NUL instruction smuggling past the relay's `size` bound (fixed), untested many-element byte limit (fixed). |
| T2 `handshake` | 3 | reviewer (Opus), reviewer-security (Opus) | accept. Found: fixtures counted the `#ifdef` `enable-agent` (fixed), gateway message and connection ID leaks in errors and `Debug` (fixed), client time zone unchecked on its way to `TZ` (fixed). |
| Package | 1 + re-review | reviewer (Opus) | accept. Plan accuracy, exports and handoffs fixed. |

Every round used mutation testing in detached worktrees. All surviving mutants were either killed by new tests or shown to be equivalent.

## Handoffs (PKG-001)

- **H-1 (programme orchestrator, `Cargo.lock`).** Adds `proptest` 1.12 and its dependencies (dev-dependency of `guac-protocol` only). Root `Cargo.toml` is unchanged; the workspace member line existed (WP-01).
- **H-2 (WP-01 / programme, root `Cargo.toml`, could).** Move `proptest` to `[workspace.dependencies]` when a second package needs it.
- **H-3 (WP-10 relay).** Cap the WebSocket maximum message and frame size at a small multiple of `MAX_INSTRUCTION_BYTES`. One `feed` returns all complete instructions, and a 64 MiB message of `0.;` would be about 1 GiB of `Instruction`s.
- **H-4 (WP-10 relay).** `Decoder::feed` is all-or-nothing per call: on a bad instruction it returns only the error; close with 1002.
- **H-5 (WP-10 relay).** After `HandshakeStep::Ready`, pass the rest of the same feed to the relay. guacd can send display instructions in the same read, and `on_instruction` refuses anything after `ready`.
- **H-6 (WP-10 relay).** The client `size` has no DPI (guacamole-common-js `sendSize(width, height)`). Check width and height with `Display::MIN_SIDE_PX..=MAX_SIDE_PX`, and DPI only when present. Parse digits only, because `u32::from_str` accepts a leading `+`.
- **H-7 (WP-10 relay).** The open instruction is `encode(&Instruction::new("", [uuid]))`. The internal ping is an empty opcode with `args[0] == "ping"`. Re-encoding is canonical (`004.ping;` comes back as `4.ping;`).
- **H-8 (WP-10 sessions).** Resolve the time zone before building `ConnectParams`: `DisplayInput.timezone` if `is_zone_name`, else the site zone, else `UTC` (K4 §3). The handshake refuses a bad zone rather than falling back.
- **H-9 (WP-10 sessions).** Never log `HandshakeStep::Send` arguments (`connect` holds secrets; only `Instruction`'s `Debug` is safe). Never send `connection_id` or `HandshakeError::Gateway`'s message to the client (GW-014, LOG-007). Leave guacd's SSH `timezone` argument empty unless the hub sets it; guacd then uses the `timezone` instruction.
- **H-10 (WP-10 tests).** The fixtures are built from source, not captured. Compare one real `args` from the official `guacamole/guacd` image with them in the fake-guacd relay test or at deploy (K4 §7 says "recorded").
- **H-11 (WP-10).** `anima_theme::guac_color_scheme()` (WP-02) gives the `color-scheme` value; K4 §3 font, scrollback, terminal and keep-alive values go into `ArgValues` by name.

## Contract change requests (CON-001) / open questions

- **Q-1 (K4 §3 step 2).** K4: the hub "answers with the same version". The crate answers `VERSION_1_5_0`, the newest version it implements. That is identical for guacd 1.5 and 1.6, which both send `VERSION_1_5_0`. For a later guacd, echoing its version would claim unimplemented features. Request: reword to "the highest version both sides support (`VERSION_1_5_0` in Phase 1)". No code change is needed either way for Phase 1.
- **Q-2 (K4 §4).** K4 forwards client `size` only when "DPI is 48..480", but guacamole-common-js sends `size` without DPI. Request: "DPI, when present, is 48..480".

## Deferred (for issue #1, R-007)

- Capture a real `args` instruction from the deployed `guacamole/guacd` image and check it against the fixtures (GW-001, K4 §7). Owned by WP-10 or WP-04 once a gateway runs (H-10).
- RDP and VNC handshakes reuse `Handshake` (protocol string and `ArgValues` only), with their own `args` fixtures, when those protocols are in scope (RA-*, later phase).

## Notes

- Task branches `wp/08-guac-protocol/{codec,handshake,final}` are merged into the package branch and can be deleted.
- No contract, root `Cargo.toml` or other package path was changed. `contracts/` untouched.
- The guacamole-server sources used for the protocol facts: tags 1.5.5 and 1.6.0 (`parser-constants.h`, `protocol-constants.h`, `protocol.c`, `ssh/settings.c`, `Dockerfile`), and guacamole-client 1.5.5/1.6.0 `Client.js` (`sendSize`).
