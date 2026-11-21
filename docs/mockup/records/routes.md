# The record of the mockup routes/index.html

One row of the index in [README.md](../README.md) (PLN-019). `bun run quality:register` writes the row from this file.

- Mockup: [routes/index.html](../routes/index.html)
- Owner: [hosted-applications-plan.md](../../plans/hosted-applications-plan.md), tasks H21 and H40 (HOST-046, HOST-050; OD-HOST-43, OD-HOST-46 to OD-HOST-49, OD-HOST-54); Sections 12 and 16 of [hosted-applications-stage-2.md](../../proposals/hosted-applications-stage-2.md)
- Screens: The section Routes of the Settings page, in the group Hub (not built; it goes in `apps/client/src/routes/settings`), and the entry point from the page of an application (Frame 6)
- State: Approved overnight by the orchestrator, human check pending (R-NEW-hostovernight). A separate design reviewer read it on 2026-11-20; the findings of severity High and Medium are in this version. The check by a person is HT-NEW-routesmockup
- Compared: Not compared
- Differences: None known. The mockup is drawn before the screen. The parts marked Planned (redirects, streams, the access of a route, the step that makes a route public) need task H40 and fields that the API of task H21 does not have. Routes is a section of Settings, group Hub, and not an item of the navigation bar (NAV-001 does not change); SET-004 does not list Routes yet, and its builder adds it there. Open for the builder: how a private name gets its certificate (DNS-01 is not used, Section 12.5). Compare at 1280 px in the light and dark appearance when the screen exists.
