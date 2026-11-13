# K-E7 — Extension registry and switch (engine)

Status: new (operator, 2026-11-13; ruling number to be assigned). Proposed for the path to Projects; it is not part of the Phase 3 freeze. Change it only through a programme ruling (CON-001). Requirement IDs: EXT-000 to EXT-012, NAV-* of `docs/extensions-plan.md`. Owner: engine (`crates/hub/src/engine/extensions*`, `$lib/extensions/**`).

## 1. Entry (`crates/hub/src/engine/extensions/registry.rs`, data only)

```rust
pub struct Entry {
    pub key: Key,                         // newtype over &'static str, snake_case
    pub name: &'static str,
    pub data_class: DataClass,            // A | B | C (EXT-004)
    pub needs: &'static [Key],            // EXT-010
    pub works_with: &'static [Key],       // EXT-006
    pub navigation: Option<Navigation>,   // { group, label, route }
    pub migrations: Option<(u16, u16)>,   // the K-E8 range
    pub built: bool,
    pub fixed: bool,                      // part of the engine, always on, no switch
}
```

## 2. The fifteen entries

The list is the fifteen extensions of Section 1.1 of `docs/extensions-plan.md`, in that order. `integrations` is engine (ENG-002): it is listed with `fixed` true and cannot be switched. `project_design` is not an entry yet; it is added (needs `projects`, migrations 3400–3499) when XP-14 starts.

| key | name | class | needs | works with | nav (group, route) | migrations | built |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `map` | Map | B | — | health, history, people | Life, /map | 2100–2199 | no |
| `projects` | Projects | C | — | secrets, model_gateway, assistant_sessions, writing | Infrastructure, /projects | 2200–2299 | **yes** |
| `writing` | Writing | C | — | projects | Work, /writing | 2300–2399 | no |
| `secrets` | Secrets | B | — | projects | Vault, /secrets | 2400–2499 | no |
| `integrations` | Integrations | C | — | — | none | 2000–2019 | **yes, fixed** |
| `people` | People | C | — | map, library | Life, /people | 2500–2599 | no |
| `money` | Money | B | — | wardrobe | Life, /money | 2600–2699 | no |
| `history` | History | B | — | map | Life, /history | none | no |
| `health` | Health | B | — | map | Life, /health | 2800–2899 | no |
| `documents` | Documents | A | — | — | Vault, /documents | 2900–2999 | no |
| `reminders` | Reminders | C | — | — | Bottom, /reminders | none | no |
| `assistant_sessions` | AI coding sessions | B | — | projects | none (the plan gives no nav group) | 3100–3199 | no |
| `wardrobe` | Wardrobe | C | — | money | Belongings, /wardrobe | 3200–3299 | no |
| `library` | Library | C | — | people | Belongings, /library | 3300–3399 | no |
| `model_gateway` | Model gateway | C | secrets | projects | Infrastructure, /model-gateway | 3500–3599 | no |

History's data class is B: the registry needs one value, and the plan says "the highest class on the screen".

## 3. Switch table (schema `extensions`, migration 2020)

`extensions.switches (key text pk, is_on boolean not null, changed_at timestamptz not null, changed_by text not null)`. No row means off (EXT-001); nothing seeds it. Rows whose key is not in the registry are ignored.

## 4. Hub behaviour

The pure rule is `switching.rs`: `plan(registry, on_now: &HashSet<Key>, key, want: bool) -> Result<Vec<Key>, Refusal>`.

- **EXT-001:** every extension is off by default; `setExtension` is the only writer.
- **EXT-011 (on):** the result is `key` followed by each entry of `needs` that is not on yet. All are written in one transaction, with one audit entry each (`AuditTarget::Setting("extensions.<key>")`, action Activate). `Refusal::NotBuilt(k)` when `key` or a need has `built == false`; `Refusal::Fixed` for `integrations`. Both are CONFLICT.
- **EXT-012 (off):** when any entry that is on has `key` in its `needs`, the result is `Refusal::Needed { by }`. That is CONFLICT with the message "<Name of by> is on and needs <Name>. Set <Name of by> off first." and `extensions.neededBy: [keys]` (K7). The hub never sets a second extension off.
- **EXT-003:** off writes only the switch row. No delete mutation exists in this slice.
- **EXT-002:** the scheduler and each run in flight read `extensions::on_keys(db)` (K-E1 Sections 8 and 10). With Projects off, no request leaves for GitHub and a running sync stops before its next request with outcome `stopped`. State is kept, so turning it on again resumes without a second import. `Extension.navigation` is returned for the client to show only while `on`.
- **EXT-005:** the K3 edges are `extensions -> events` and `sources -> extensions, credentials, events` (and `access` in `graphql.rs`). No foundation or engine module imports `work::projects`; wiring passes the intake in.
- **EXT-006, EXT-007, EXT-010:** `needs` and `works_with` are data. A unit test proves that no needed entry has needs. `worksWith` is display-only in this slice.
- **When an extension is off:** every Projects root field calls `extensions::require_on(db, PROJECTS)` and fails with CONFLICT "Projects is off. Set it on in Settings."
- **EXT-008, EXT-009:** not built.

## 5. K3 face of `engine::extensions`

`Key`, `PROJECTS`, `on_keys(db) -> HashSet<Key>`, `require_on(db, Key) -> Result<(), Error>`, `registry() -> &'static [Entry]`. The GraphQL surface is in K1 (`Extension`, `Query.extensions`, `Mutation.setExtension`).
