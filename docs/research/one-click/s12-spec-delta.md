# S12 spec delta: proposal

Progress: the plan register in [work-plan.md](../../work-plan.md) Section 9 holds the state of this plan (PLN-005).

Version 1.0. Date: 2026-11-16. Status: **Draft: a proposal for open item OI-2 of `docs/one-click-plan.md`. Nothing in
`docs/design-spec.md` or `contracts/` is changed by this file.** The operator approves the text
before it goes into the spec.

Context: plan item S12. The release builder puts the hub's programs into the application
([s12-packaging-tasks.md](s12-packaging-tasks.md)). The Docker installation does not change.

## 1. Changed requirements

| ID | Text today | Proposed text |
| --- | --- | --- |
| UPD-011 | The macOS builder must verify the bundle with `codesign --verify --deep --strict` and must verify that `lipo -archs` gives `x86_64` and `arm64`. After notarization it must verify the app and the DMG with `xcrun stapler validate`. | Add: "The builder must verify each Mach-O file of the application, also below `Contents/Resources`: `lipo -archs` gives `x86_64` and `arm64`; the file has no other architecture; the signature is a Developer ID Application signature of the team of the identity that the builder application was built with; the hardened runtime is on; the signature has a secure timestamp; the file has no entitlement." |
| INS-002 | Each artefact must contain the `anima` program beside the desktop application. | Add: "Each artefact must also contain `anima-local-hub`, `anima-hub`, `anima-database`, `anima-gateway` and `anima-link` beside the desktop application, and the PostgreSQL tree as a resource. All come from the commit of the job." |
| INS-007 | An update of the desktop application must replace the application and the `anima` program together. | Add: "After the update, the application must replace the programs of a hub that is installed on this computer and start the hub on them at its next start. It must not change the database, the hub key or the link's state. A hub that the operator stopped must stay stopped. The application must not put hub programs of a lower version in place." |
| BLD-083 | On macOS the builder must sign with the Developer ID Application identity that `bundle.macOS.signingIdentity` in `tauri.conf.json` names. | Add: "The identity is the one in the `tauri.conf.json` that the builder application was built from. A job whose commit names another identity must fail." |
| BLD-040 | The clean tree of the checkout. | Add: "The builder must rebuild git's index from the commit before it asks git for changes, and the build scripts that a step runs must be the files of the commit directly before the step runs them." |
| BLD-028 | The tools a builder needs before it claims a job. | Add: on Linux, the tools that build the PostgreSQL tree (`patchelf`, `bison`, `flex`, `readelf`, `make`, the zlib headers). |
| BLD-043 | The builder must compile with `tauri build --no-bundle` … then it must bundle. | Add: "Before it compiles, the builder must build the carried programs and the PostgreSQL tree from the job's commit. It must build the tree from the pinned source at each job and must not use a tree that an earlier job left." |
| The table of Section 3.10.4 ("Contains") | The desktop application and the `anima` program. | Add the five programs and the tree, with their paths: `Contents/MacOS/` and `Contents/Resources/database/` on macOS; `usr/bin/` and `usr/lib/Anima.ing/database/` in the AppImage. |

## 2. Requirements that S12 touches and does not change

| ID | State after S12 |
| --- | --- |
| BLD-044 | Kept. The two new build scripts run with `/bin/sh` from the clone at the job's commit, in the scrubbed environment. The tree recipe gets one more variable, `SIGN_IDENTITY`, which is the public name of the identity in `tauri.conf.json`, not a secret. |
| BLD-084, BLD-085 | Kept. The notary service gets the same DMG; it now holds more signed code. |
| UPD-003 | Kept. The hub's programs change at the start after "Restart to update", never before it. |
| LHB-007 (S4 delta) | Built in full: the shell runs the update at each start. |
| CMD-030, CMD-031 | Do not apply: S12 adds no root field to `contracts/schema.graphql`. `anima-local-hub update` is the command line of the new function. It gets no MCP tool, because it changes what runs at login. |

## 3. Not proposed

- No backup before an update and no step back to the programs before it: S9.
- No Linux package other than the AppImage.
