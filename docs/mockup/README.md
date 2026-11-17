# Mockups

| Item | Value |
| --- | --- |
| Document | The index of the mockups, with the state of each and the screens of the application that each shows (PLN-019) |
| Version | 1.0 |
| Date | 2026-11-17 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | Approved (operator, 2026-11-17: a mockup stays in sync with the application) |

A mockup is a static HTML file that shows a layout before or beside the application. This table is the only list of the mockups. `bun run quality:documents` fails when an HTML file in this folder has no link in this table, or when a link in the column "Screens" does not resolve (PLN-017).

## Rules (PLN-019)

- A mockup and its screens must agree. A commit that changes the layout of a screen in the column "Screens" changes the mockup in the same commit and sets "Compared" to the date.
- If the commit cannot change the mockup, it writes the difference in the column "Differences". A difference is a defect: correct the mockup or the screen.
- Before a screen is built, the approved mockup is correct. After the screen is built, the two must agree, and the plan in the column "Owner" says which one a change starts in.
- A mockup takes its colours, sizes and motion from the design tokens (`design/tokens/`). It does not hold a second set of values.
- The states are: Proposed (the operator did not approve it), Approved (approved, and the screens are not all built), Built (the screens are built and were compared), Superseded (a newer mockup replaces it; delete the file in the next commit that touches it).
- A mockup of an idea is in the folder of the idea (`docs/ideas/`), not here (PLN-018).

## Index

| Mockup | Owner | Screens | State | Compared | Differences |
| --- | --- | --- | --- | --- | --- |
| [index.html](index.html) | [design-spec.md](../spec/design-spec.md) Section 5; [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md) for the Projects screens | [devices](../../apps/client/src/routes/devices), [projects](../../apps/client/src/routes/projects), [logs](../../apps/client/src/routes/logs), [settings](../../apps/client/src/routes/settings), [sign-in](../../apps/client/src/routes/sign-in) | Approved | Not compared | Not known. It holds its own colours and has no link to the design tokens. |
| [redesign/index.html](redesign/index.html), [redesign/specimen.html](redesign/specimen.html) | [ui-redesign-direction.md](../plans/ui-redesign-direction.md), [ui-redesign-plan.md](../plans/ui-redesign-plan.md) | [specimen](../../apps/client/src/routes/specimen) | Proposed | Not compared | Not known. The register says that the two owner plans are not checked. |
| [site-language/index.html](site-language/index.html) | [ui-site-language-plan.md](../plans/ui-site-language-plan.md) | [specimen](../../apps/client/src/routes/specimen), [devices](../../apps/client/src/routes/devices), [settings](../../apps/client/src/routes/settings), [sign-in](../../apps/client/src/routes/sign-in) | Approved | Not compared | Not known. The plan lists D11 and D15 as values of the mockup that the application does not use. |
| `mobile/index.html` | [mobile-plan.md](../plans/mobile-plan.md) Section 4 | The phone layout of [devices](../../apps/client/src/routes/devices), [logs](../../apps/client/src/routes/logs), [settings](../../apps/client/src/routes/settings), [sign-in](../../apps/client/src/routes/sign-in) | Approved (operator, 2026-11-16) | Not compared | The file is not in the repository (2026-11-17). It is only in one checkout on the operator's computer. Commit it; then this row gets its link. |
