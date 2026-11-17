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

- A mockup and its screens must agree. A commit that changes the layout of a screen in the column "Screens" changes the mockup in the same commit and writes the commit of the comparison in "Compared".
- If the commit cannot change the mockup, it writes the difference in the column "Differences". A difference is a defect: correct the mockup or the screen.
- Before a screen is built, the approved mockup is correct. After the screen is built, the two must agree, and the plan in the column "Owner" says which one a change starts in.
- A mockup takes its colours, sizes and motion from the design tokens (`design/tokens/`). It does not hold a second set of values.
- The states are: Proposed (the operator did not approve it), Approved (approved, and the screens are not all built), Built (the screens are built and were compared), Superseded (a newer mockup replaces it; delete the file in the next commit that touches it).
- A mockup of an idea is in the folder of the idea (`docs/ideas/`), not here (PLN-018).
- To compare: open the mockup and the screen of the application at the same size, in the light and the dark appearance, and look at the layout, the order of the parts, the text and the sizes. Then write the short hash of the commit that you compared, in backticks, with the day (for example `` `1a2b3c4` (2026-11-17) ``), in "Compared" and each difference in "Differences" ("None" if there is none).
- The gate reads this table. A mockup in the state Built must name a commit in "Compared" and at least one screen. When a row names a commit, the gate fails if a Svelte or CSS file below one of its screen folders changed after that commit. Then compare again and write the new commit. Name a commit of `main`: a commit of a branch that was squashed does not exist in a different clone, and the gate fails there. The gate reads commits, so it fails in the first gate run after the commit that changed the screen, which is the run before the merge. The gate does not see a layout that is in `apps/client/src/lib/`: put the folder of the component in "Screens" too.
- The first comparison of each row is open. Until it is made the row says "Not compared", and the gate cannot find a difference for that row.

## Index

| Mockup | Owner | Screens | State | Compared | Differences |
| --- | --- | --- | --- | --- | --- |
| [index.html](index.html) | [design-spec.md](../spec/design-spec.md) Section 5; [projects-and-builds-plan.md](../plans/projects-and-builds-plan.md) for the Projects screens | [devices](../../apps/client/src/routes/devices), [projects](../../apps/client/src/routes/projects), [logs](../../apps/client/src/routes/logs), [settings](../../apps/client/src/routes/settings), [sign-in](../../apps/client/src/routes/sign-in) | Approved | Not compared | The Devices table has shorter rows and separate columns for each metric. It holds its own colours and has no link to the design tokens. |
| [redesign/index.html](redesign/index.html), [redesign/specimen.html](redesign/specimen.html) | [ui-redesign-direction.md](../plans/ui-redesign-direction.md), [ui-redesign-plan.md](../plans/ui-redesign-plan.md) | [specimen](../../apps/client/src/routes/specimen) | Proposed | Not compared | The proposed table has a separate Host column. The register says that the two owner plans are not checked. |
| [site-language/index.html](site-language/index.html) | [ui-site-language-plan.md](../plans/ui-site-language-plan.md) | [specimen](../../apps/client/src/routes/specimen), [devices](../../apps/client/src/routes/devices), [settings](../../apps/client/src/routes/settings), [sign-in](../../apps/client/src/routes/sign-in) | Approved | `118002cd` (2026-11-17), wide Devices list at 1280 px in light and dark | The Devices list uses different sample devices and status groups. The plan lists D11 and D15 as values of the mockup that the application does not use. |
| [mobile/index.html](mobile/index.html) | [mobile-plan.md](../plans/mobile-plan.md) Section 4 | The phone layout of [devices](../../apps/client/src/routes/devices), [logs](../../apps/client/src/routes/logs), [settings](../../apps/client/src/routes/settings), [sign-in](../../apps/client/src/routes/sign-in) | Approved (operator, 2026-11-16) | Not compared | Not known. Tasks M2-2 to M2-4 of the mobile plan are in progress; compare each screen when its task is done. |
