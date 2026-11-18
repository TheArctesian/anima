# KEYS: Accounts, keys and setup checks

Progress: the index [README.md](../README.md) holds the state of this item (PLN-006). The plan register in [work-plan.md](../../work-plan.md) Section 9 lists the index.

| Item | Value |
| --- | --- |
| Identifier | KEYS |
| Kind | Checklist |
| Owner | [extensions-plan.md](../../plans/extensions-plan.md) Section 9 |
| Serves | Work plan Section 4, tier 1 (decisions and approvals) |
| Version | 1.0 |
| Date | 2026-11-16 |
| Origin | GitHub issue #31: made 2026-11-06, last changed 2026-11-08, open at import. Labels: extensions, infrastructure, priority: foundation. |
| Text | Copied from the issue on 2026-11-16 (R-119). Only the links were changed. Not written in STE yet. "This issue" in the text means this document. |

**Ownership / priority clarification (2026-11-07):**

Provision accounts/keys per consuming extension, not all up front. Shared private vision infrastructure is C1 [C1](../compute/C1.md); the hosted-model key below is optional and must not be created unless [OD-E4](../decisions/OD-E4.md) permits that fallback. Never store values in issues.

---

Create each account or key from plan Section 9 when its extension starts, and check the items that are not verified yet. Never paste a value in this issue.

**Developer apps**
- [ ] GitHub OAuth app (XP-02, XP-05)
- [ ] Strava API app (XP-08)
- [ ] WHOOP developer app (XP-08)
- [ ] Oura API app (XP-08)
- [ ] Etherscan API key (XP-06)
- [ ] Hosted vision model key, only if OD-E4 is yes

**Personal tokens**
- [ ] Wise read-only token (XP-06)
- [ ] Gemini exchange key, Auditor role only (XP-06)
- [ ] Coinbase key, View permission only (XP-06)
- [ ] IBKR Flex Queries and Flex Web Service token (XP-06)
- [ ] SimpleFIN Bridge setup token (XP-06, after OD-E3)
- [ ] Vercel token, narrowest scope, with expiry (XP-02)
- [ ] Neon API key (XP-02)
- [ ] Hostinger API token with expiry (XP-02)
- [ ] PostHog personal API key, read scopes (XP-02)

**Checks not yet verified**
- [ ] Strava: a new app allows one athlete; callback domain `localhost` works
- [ ] Oura: OAuth app works without a personal access token
- [ ] WHOOP: accepts a loopback redirect
- [ ] HSBC HK: which export formats online banking gives (PDF, CSV)
- [ ] Vercel: which Web Analytics data the REST API gives
- [ ] Neon: whether a project-scoped API key exists
- [ ] Garmin: `python-garminconnect` can load and save tokens without a file (GAR-002)

**Plan:** [Section 9](../../plans/extensions-plan.md)
**Requirements:** KEY-001–003
