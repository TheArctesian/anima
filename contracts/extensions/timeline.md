# K-E3 — Timeline adapter interface and the category list (engine)

Status: new (operator, 2026-11-16; ruling number to be assigned). Proposed for engine task E0-4; it is not part of the Phase 3 freeze. Change it only through a programme ruling (CON-001). Requirement IDs: TML-001 to TML-007 of `docs/extensions-plan.md`. Owner: engine (`crates/hub/src/engine/timeline*`). Section 8 lists the questions that are open; a consumer must not rely on an answer to them.

## 1. What the timeline is

The timeline shows the records of every domain as Evidence, newest first. It has no table, no schema and no migration (TML-001). Each domain gives an adapter. When the timeline is read, it asks each adapter and joins the answers. A record stays in the domain that owns it.

## 2. Evidence (`crates/hub/src/engine/timeline/model.rs`)

```rust
pub struct Evidence {
    pub reference: Reference,              // the record, kind:id (K-E4 Section 1)
    pub source: String,                    // the key of the source, e.g. event_log or a K-E1 source key
    pub rank: u16,                         // the rank of that source; the higher rank wins (Section 5)
    pub category: Category,                // Section 3
    pub title: String,                     // one line for a list; no secret
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
    pub basis: Basis,                      // Section 4
    pub uncertainty: Vec<String>,          // one sentence for each value the row does not give (TML-006)
    pub occurrence: Option<String>,        // Section 5
    pub superseded_by: Option<Reference>,  // Section 5; the timeline fills it
}
```

- TML-002 names the reference, the source, the category, the start time, the optional end time, the basis and the uncertainty notes. `title`, `rank` and `occurrence` are additions of this contract: a list needs one line of text, and TML-005 needs a rank and a rule for "the same event".
- `start` and `end` are instants in UTC with microsecond precision, as PostgreSQL stores them (K2). In K1 they are `DateTime` (RFC 3339), as all other instants of the API.
- A value that is a calendar date and not an instant (K-E4 Section 3, the `date` subject) must not become an Evidence instant at midnight. An adapter that has only a date writes the uncertainty note "Time of day not given by the source." and uses the start of that date in UTC.
- An uncertainty note is a full sentence in plain English. The fixed notes are: "End time not given by the source." and "Time of day not given by the source." An adapter can add others.

## 3. Categories (TML-004)

The list is closed: `sleep`, `workout`, `visit`, `travel`, `code`, `message`, `post`, `media`, `reading`, `spending`, `document`, `device`, `session`, `note`. K1 `EvidenceCategory` has the same 14 values in upper case. To add a category, change Section 2.3 of `docs/extensions-plan.md` first, then this contract and K1.

## 4. Basis (TML-003)

| Value | K1 | Meaning |
| --- | --- | --- |
| `Observed` | `OBSERVED` | A device or a service recorded it when it happened. |
| `Planned` | `PLANNED` | It is in a plan or a calendar. It did not necessarily happen. |
| `OperatorReported` | `OPERATOR_REPORTED` | The operator said so. |
| `Inferred` | `INFERRED` | The hub or a source derived it from other records. |

## 5. Two sources of one event (TML-005)

- Two entries record the same event when they have the same `category` and the same `occurrence`, and `occurrence` is not empty. The adapter gives `occurrence`. An adapter that knows no second record of an event gives `None`.
- The timeline keeps both entries. It sets `superseded_by` of the lower-ranked entry to the reference of the higher-ranked entry. It removes no entry.
- The leader of an event is the entry with the highest `rank`. For equal ranks it is the entry whose source key, then whose reference, sorts first. Thus the result does not depend on the order of the adapters.
- An entry of the same source as the leader is not superseded. A replay of one source record is not a second source: the domain that owns the record makes the replay idempotent (K-E1 Section 3, SRC-012).
- If an adapter sets `superseded_by` itself, the timeline keeps that value.
- Limit: the timeline finds a leader only among the entries that it read for one page. An entry can thus show `superseded_by` on one page and not on a different page when its leader is not in that page's read. Section 8 lists this.
- The rank of the event log is 100. No other rank is assigned.

## 6. Adapter (`crates/hub/src/engine/timeline/adapter.rs`)

```rust
pub struct Mark { pub start: DateTime<Utc>, pub reference: Reference }

pub struct Ask {
    pub from: DateTime<Utc>,                 // only entries with from <= start
    pub to: DateTime<Utc>,                   // only entries with start < to
    pub after: Option<Mark>,                 // only entries after this place in the order
    pub categories: Option<Vec<Category>>,   // None: all; empty: none
    pub limit: usize,
}

#[async_trait]
pub trait Adapter: Send + Sync {
    fn name(&self) -> &'static str;
    async fn evidence(&self, db: &PgPool, ask: &Ask) -> Result<Vec<Evidence>, Unavailable>;
}

pub struct Adapters(/* built once in wiring */);
pub async fn read(adapters: &Adapters, db: &PgPool, ask: &Ask) -> Result<Page, Error>;
```

- The order is newest first: by `start`, then by the text of `reference`, both descending. An entry is after a mark when `(start, reference)` is less than the mark.
- An adapter must: copy nothing (TML-001); return only entries that `ask` wants, at most `limit`, in the order; not drop an entry because a second source recorded the same event (TML-005); write an uncertainty note for each value that the row does not give (TML-006); put no secret in `title` (LOG-007).
- The timeline asks each adapter for `limit + 1` entries, sorts the answers, keeps each reference once, applies Section 5 and cuts the page at `limit`. If one adapter fails, the read fails and gives no partial page.
- The engine imports no extension (EXT-005). `schema.rs` builds `Adapters` and puts it into the schema. An extension adds its adapter there.
- K3 edge: `timeline -> events` (`before`, `EventSummary`, `Source` only), `timeline -> records` (leaf).

## 7. The event log adapter (TML-007) and the API

The first adapter reads the event log of the device manager through `events::before`. It reads `events.event_log` through the events domain and writes nothing.

| Evidence field | Value for an event |
| --- | --- |
| `reference` | `event:<event ID>` |
| `source` | `event_log` |
| `rank` | 100 |
| `category` | `session` for an event of source `session`; `device` for every other event source |
| `title` | the event message, with the device name and a colon before it when the event has a device |
| `start` | the instant of the event |
| `end` | none, with the note "End time not given by the source." |
| `basis` | `Observed` |
| `occurrence` | none |

- An event leaves the timeline when the event log drops its partition (DB-005). The timeline keeps no copy.
- K1: `Query.timeline(from, to, categories, first = 100, after): EvidenceConnection!`. `first` is 1 to 200. `VALIDATION` when `to` is not after `from`, for a bad `first` and for a cursor that the timeline did not make. The cursor is opaque: `t1.`, the start in microseconds since the epoch, a dot and the reference.
- Commands (CMD-030): `anima timeline list`; MCP tool `anima_timeline_list`.

## 8. Open questions

Issue #7 lists the first four as not decided. This contract gives no answer to them.

| Question | State in this contract |
| --- | --- |
| The wire unit for an event instant (Unix seconds, milliseconds or a different precision) | Not decided. The hub stores `timestamptz` (microseconds) and K1 uses RFC 3339 `DateTime`, as before. |
| Clock synchronization between machines: how skew is found and reported, and what an unreliable clock does | Not decided and not built. The timeline claims no exact order between entries of different machines. |
| Event time and ingest time as two values | Not in Evidence. `start` is the event time that the source gives. An adapter must not put the ingest time there; it writes an uncertainty note when the event time is not known. |
| How `occurrence` is made when two domains record one event | Not decided. No inference from overlapping times is built. |
| Who assigns the rank of each source | Not decided. Only the rank of the event log (100) exists. |
| `superseded_by` across pages (Section 5, the limit) | Open. |
| `title` in Evidence | An addition to TML-002 for the operator to accept or remove. |
