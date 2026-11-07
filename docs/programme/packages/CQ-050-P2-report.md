# CQ-050-P2 report — Phase 2 code-quality SPEC items

Status: **done.** All three SPEC items that remained open under R-076 are accepted and sent to the programme
orchestrator, which merges them under R-067. The plan and the review log of every round are in
[CQ-050-P2.md](CQ-050-P2.md).

| Task | Item | Spec | Accepted SHA | Base | Gates | Review rounds | Merged |
| --- | --- | --- | --- | --- | --- | --- | --- |
| T1 `q2-02-random` | Q2-02 | CQ-023, CQ-021 | 97d4c17d | 063ac826 | 12/12 (709 DB integration tests) | 2 (reviewer-security, reviewer) | c21d03ac |
| T2 `q2-03-modal-shell` | Q2-03 rest | MAT-003, R-049, K5, MV-003, MV-004 | aec3eca6 | bc373c2c | 4–12 (2946 client tests) | 3 + R-078 fix (reviewer-interface, reviewer) | sent |
| T3 `q2-04-detail-route` | Q2-04 | CQ-044 | 2ff2b7c8 | 6dbae664 | 4–12 (2935 client tests) | 2 (reviewer) | yes, coulds at 063ac826 |

`quality/baseline.json` stays `{}`. No task needed a line in `schema.rs` or `main.rs` (PKG-001).

## What changed

- **Q2-02 (CQ-023, CQ-021).** Secret-making no longer panics when the OS random source fails.
  - `Vault::seal` returns `Result`, and `credentials::Error` has a `Random` case.
  - The callers for SSH, RDP and VNC passwords, HTTP and SNMP check secrets, and TOTP were updated. TOTP maps a
    seal failure to `Random` or `TotpSeal`, both INTERNAL.
  - The push token is made by `checks::fresh_push_token` with `getrandom` and `checks::Error::Random`. The check
    form draws it only after validation, so a failed draw never hides a VALIDATION error.
  - The SNMP probe takes `Draws`, built by the pure `Draws::from_bytes` and the edge `Draws::fresh`. The scheduler
    draws them. If the draw fails it logs and skips the attempt, so the device is never reported down.
  - Wire codes are unchanged. The `# Panics` docs are removed.
- **Q2-03 (MAT-003, R-049, K5).** The three session modals (certificate, host key, credentials) render inside
  `forms/modal/Modal.svelte`, and `sessions/view/hostkey/focus.ts` is deleted, which leaves one focus trap.
  - **New Modal props:** `warning`, `describedBy`, `closeOnScrim`, `returnTo`, `width` and `lowered`.
  - **Behaviour:** global transitions, so the fades play for callers mounted already open. A dialog stack
    (`stack.ts`) means only the topmost dialog answers Escape and Tab. A focusin pull-back keeps focus in the
    topmost dialog. A scrim press does not take focus.
  - **Size:** each prompt keeps its previous size and offset (R-077).
  - **Opacity rule:** stylelint refuses a literal `opacity`, `fill-opacity` or `stroke-opacity` other than 0
    and 1. Its two real hits use the new tokens `opacity-disabled` and `opacity-band` (CR-4, K5 row in
    `contracts/tokens.md`).
  - **Measurement:** MV-003 and MV-004 were re-measured in WebKit and Chromium.
- **Q2-04 (CQ-044).** `routes/devices/[id]/+page.svelte` is down from 275 to 24 lines. It shows the not-found state
  or `detail/DetailBody.svelte`, which takes a non-null `DeviceId`, so every `id ?? ''` is gone.
  - The two effects are now `Detail.showTab` and `Detail.showMetrics`.
  - The mutation wrappers are `Detail` methods.
  - `detail/calls.ts` builds the settings, test and delete calls.

## Decisions and rulings

- **CR-4:** the K5 Opacity row (`opacity-disabled` 0.5, `opacity-band` 0.2).
- **R-077:** a third round for T2. Restore the focusin pull-back. Keep the previous prompt sizes, because the
  spec sets no modal width.
- **R-078:** a scoped fix for a round-3 regression in T2. One Escape in the inner of two stacked dialogs closed
  both, because Svelte delegates `onkeydown` to `document`. The fix is a `defaultPrevented` guard.
- **Final package review (Section 3.2 step 4).** Each task was reviewed against `phase-2` at its rebased base, and
  T1 and T3 are already merged and gated by the programme orchestrator. The three tasks touch disjoint files, so
  there is no separate combined diff to review. The full 12 gates on `phase-2` after the T2 merge are the
  combined check.

## Contract change requests

- CR-4 (approved). None open.

## Handoff notes for shared files

- `forms/modal/Modal.svelte` is the shell for every modal, and its public surface grew by six props. A new modal
  should use `warning` and `width` rather than its own scrim or trap.
- A caller that mounts a Modal already open gets its fades through the `|global` transitions. A caller that wraps
  a Modal in its own `{#if}` must not add its own transition.
- `forms/modal/` is at the 12-file limit.
- `crates/hub/src/checks.rs` is about 282 code lines, near the 300 limit.

## Should and could findings for issue #1

**T1 Q2-02**
- **Could, CQ-021, Predictable:** `Vault::seal` draws its own nonce, so the nonce cannot be passed in. The
  `seal_short` mutant (a 2-byte random nonce) survives. Fix: a private seal that takes the nonce, with
  `seal` as the edge.
- **Could, CQ-023:** `checks/scheduler.rs:48` `StdRng::from_rng(&mut rand::rng())` panics if the OS source fails. It
  only seeds startup jitter.
- **Could, security:** the `token_half` mutant survives (`checks.rs:253-257`). Fix: a test that the token text
  decodes to all 32 bytes from `fresh_push_token`.
- **Should, tests:** the `getrandom` failure branches in `Vault::seal`, `fresh_push_token`, `Draws::fresh` and the
  scheduler skip cannot be forced in a test. Only the error-to-code mappings are tested.
- **Could:** `checks/graphql/form.rs` keeps the single-use `Draft` enum with
  `#[expect(clippy::large_enum_variant)]`. Its reason is in commit 97d4c17d.
- **Could:** no test pins that an update with only `config: Push(true)` draws exactly once.

**T2 Q2-03**
- **Should, MV-004, MOT-007:** the host key surface grows 96 px and the credential surface 32 px in one frame after
  open. This is the same as base.
- **Should, Composable:** in WebKit, focus return to the opener misses conditionally mounted callers, because
  `watchClicks` runs inside Modal after the click. This is the same as base.
- **Should, UI contrast:** on `--surface-raised`, fields and secondary buttons share the surface fill with only a
  1.2:1 border. This is shell-wide; AddDeviceModal has it too.
- **Could, TOK-003:** the opacity rule skips `stop-opacity`, `flood-opacity` and `style:` bindings. There are no
  uses of these today.
- **Could, UI-013:** the "Save password" checkbox is 12 px.
- **Could:** prompt heights differ from base by -22 to +10 px, because spacing moved to the shell gap.
- **Could, tests:** the stack test hangs rather than fails for the `isTopmost` mutant. Add a bound.
- **Could, Unix:** the unused `.content { display: contents }` wrapper in CredentialPrompt.

**T3 Q2-04**
- **Could, Composable:** `Detail.applyCheck` could be private.
- **Could, tests:** `pauseCheck` and `saveCheckValues` are tested only through `page.test.ts`.
- **Could, Idiomatic:** `DeviceMissing.svelte` needs an empty `<script lang="ts">` for svelte-check.
- **Could, Predictable:** the malformed-id branch passes no-op callbacks to `Header`.
- **Could, Unix:** `Detail` is about 240 lines and owns load, live updates, tab data and mutations. Split it if it
  grows.

## Open risks

- The T2 merge still has to run the full 12 gates on `phase-2` (R-068). T1 full gates on c21d03ac were still
  running when this report was written.
- The SNMP draw-failure skip in the scheduler is checked by reading only. `getrandom` cannot be faked.
