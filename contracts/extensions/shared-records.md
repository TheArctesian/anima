# K-E4 — Record links and the shared subject keys (engine)

Status: new (operator, 2026-11-16; ruling number to be assigned). Proposed for engine task E0-4; it is not part of the Phase 3 freeze. Change it only through a programme ruling (CON-001). Requirement IDs: LNK-001, LNK-002, LNK-004 of `docs/extensions-plan.md`. Owner: engine (`crates/hub/src/engine/links*`, `crates/hub/src/engine/records.rs`).

**Gap:** the places part and the files part of K-E4 (Sections 2.6 and 2.7 of the plan, PLC-* and FIL-*) are not written. Engine task E0-5 (#8) adds them to this file. LNK-003, the one shared client component, is not built: the operator has not ruled on it (Section 7).

## 1. Record reference (`crates/hub/src/engine/records.rs`, leaf)

A record reference names one record of one domain. It is the text `kind:id`.

- `kind` is lower snake case, 1 to 40 characters, and starts with a letter (`^[a-z][a-z0-9_]{0,39}$`). It says which sort of record it is, for example `event`, `book`, `person`. The domain that owns the record chooses it.
- `id` is the text after the first colon: 1 to 200 characters, no control character, no space at its start or end. It is opaque. Only the owner reads a meaning into it.
- The plan writes `table:id` (TML-002, LNK-001). The reference is not the address of a database row: `kind` is not a table name, and no code other than the owner uses `id` to read a table (operator, 2026-11-16).
- A reference does not prove that the record exists.

```rust
pub struct Reference(/* String */);
impl Reference {
    pub fn parse(text: &str) -> Result<Self, BadReference>;
    pub fn new(kind: &str, id: &str) -> Result<Self, BadReference>;
    pub fn as_str(&self) -> &str;
    pub fn kind(&self) -> &str;
    pub fn id(&self) -> &str;
}
```

Reserved kinds: `event` (an entry of the event log, K-E3 Section 7) and the four subject kinds of Section 3.

## 2. Record links (LNK-001, LNK-002)

One table holds every link (schema `links`, migration 2030):

`links.record_link (id uuid pk, source text not null, target text not null, kind text not null, label text, created_at timestamptz not null, created_by text not null, unique (source, target, kind), check (source <> target))`, with an index on `target`.

- `source` and `target` are record references. No foreign key: the record of each side belongs to a different domain and does not have to exist (LNK-002).
- `kind` says what the link means, for example `lent_to`. It has the form of a reference kind. The extension that makes the link names it. This contract has no list of link kinds.
- `label` is the text to show for the target while the target has no record, for example the name of a person. It is one line of at most 200 characters. The link keeps its label after the target has a record.
- A link has a direction. The same source, target and kind is one link. To make it again writes nothing; with a different label it replaces the label. With no label it keeps the stored label.
- A record cannot link to itself.
- To remove a link does not change the two records. To delete a record does not delete its links: Section 7.

```rust
// engine::links
pub async fn connect(conn: &mut PgConnection, new: NewLink, (at, by): (DateTime<Utc>, &str)) -> Result<(Link, Outcome), Error>;
pub async fn disconnect(conn: &mut PgConnection, id: RecordLinkId) -> Result<Option<Link>, Error>;
pub async fn touching(db: &PgPool, record: &Reference) -> Result<Vec<Link>, Error>;   // newest first, at most 200
pub fn own_title(record: &Reference) -> Option<String>;
```

An extension makes a link through `links::connect` in its own transaction, or through `Mutation.linkRecords`. It does not import the extension that owns the other record.

## 3. Shared subject keys (LNK-004, leaf)

A subject key is a reference that two extensions can both make without an import, so that their records meet at it.

| Subject | Reference | Rule |
| --- | --- | --- |
| A GitHub repository | `repository:<owner>/<name>` | Lower case. Each part has letters, digits, `-`, `_` and `.`; the owner at most 39 characters, the name at most 100. |
| A place | `place:<PlaceId>` | A UUID in lower case with hyphens. |
| A person | `person:<PersonId>` | A UUID in lower case with hyphens. |
| A date | `date:<YYYY-MM-DD>` | A calendar date. It has no time and no time zone. It is not an instant. |

```rust
pub enum Subject { Repository(String), Place(PlaceId), Person(PersonId), Date(NaiveDate) }
impl Subject {
    pub fn repository(name: &str) -> Option<Self>;        // any letter case in, lower case kept
    pub fn reference(&self) -> Reference;                 // the one form
    pub fn of(reference: &Reference) -> Option<Self>;     // None for any other form
}
```

- Each subject has one form. A link whose source or target has a subject kind and is not in that form is refused (`VALIDATION`). Thus `repository:Okita/Anima` and `date:2026-11-16T00:00:00Z` cannot be stored.
- `PlaceId` and `PersonId` are leaf IDs (`crates/hub/src/ids.rs`). E0-5 and the People extension give them records. Until then a `place` or a `person` reference names a record that does not exist.
- A repository key and a date key name themselves: `own_title` gives `<owner>/<name>` or the date. A link to one of them needs no label.

## 4. Titles of the other records (not built)

For a record that is not a repository or a date, the domain that owns it must give its title. This slice has no such domain, so `sourceTitle` and `targetTitle` are null for these records and a reader shows `label`. The interface comes with the first domain that has records to link (ENG-010, the sample extension):

```rust
// planned, not built
#[async_trait]
pub trait Titles: Send + Sync {
    fn kind(&self) -> &'static str;
    /// The title of each id that exists. An id with no record is not in the answer.
    async fn titles(&self, db: &PgPool, ids: &[String]) -> Result<HashMap<String, String>, Unavailable>;
}
```

## 5. API (K1) and commands (CMD-030)

| K1 | Behaviour |
| --- | --- |
| `Query.recordLinks(record: String!): [RecordLink!]!` | The links that start or end at `record`, newest first, at most 200. `VALIDATION` when `record` is not `kind:id`. |
| `Mutation.linkRecords(input: RecordLinkInput!): RecordLink!` | Section 2. `VALIDATION` with `extensions.fields` for a bad `source`, `target`, `kind` or `label`. |
| `Mutation.unlinkRecords(id: ID!): ID!` | Removes one link. `NOT_FOUND` when no link has this ID. |

- Each change writes the link, one audit entry (`target_type` `record_link`; action `create`, `update` or `delete`; the change holds the source, the target, the kind and the label) and one event (source `hub`) in one transaction. The event names the two references and the kind. It does not hold the label. A call that changes nothing writes no entry.
- A read key can read links. A change needs a write key or a login, as other mutations.
- Commands: `anima link list <record>`, `anima link add <source> <target> --kind <kind> [--label <text>]`, `anima link remove <id>`. MCP tools: `anima_record_links_list`, `anima_record_link_add`, `anima_record_link_remove`.

## 6. K2 and K3

- K2: range 2030–2039, domain engine/links, schema `links`, table `record_link`. No foreign key leaves the schema.
- K3: `links -> events` (the trail); `links -> records` and `timeline -> records` (leaf). `records` is a leaf module in the group `engine/`: it imports only `ids`. No domain imports `links` now.
- `fleet/events/model.rs` gains `AuditTarget::RecordLink`. `ids.rs` gains `RecordLinkId`, `PlaceId` and `PersonId`.

## 7. Open questions

| Question | State in this contract |
| --- | --- |
| The portable form of a reference across machines (issue #7) | `kind:id` with an opaque `id`. How a second hub or a replica names the same record is not decided. |
| A target that is deleted | Not decided. Now the link stays, with its label, and its target reference names a record that does not exist. No domain deletes links. |
| The data class of a label (it can be the name of a person) | Not decided; it waits for K-E5. Now the label is in `links.record_link` and in the audit entry, and not in the event log. |
| A closed list of link kinds | None. An extension names its kinds. |
| LNK-003, the shared client component | Not built. Issue #7 says it is not a foundation deliverable; the plan requires it. The operator decides. |
| More than 200 links for one record | `recordLinks` gives the newest 200 and has no pages. |
