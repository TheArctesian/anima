# K-E3 — Timeline adapter interface and the category list (engine)

Status: new (operator, 2026-11-16; ruling number to be assigned). Proposed for engine task E0-4; it is not part of the Phase 3 freeze. Change it only through a programme ruling (CON-001). Requirement IDs: TML-001 to TML-012 of `docs/extensions-plan.md` (TML-008 to TML-012 from issue #7, operator, 2026-11-16). Owner: engine (`crates/hub/src/engine/timeline*`). Section 8 lists the questions that are open; a consumer must not rely on an answer to them.

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
    pub start: DateTime<Utc>,              // the event time the source gives (TML-009)
    pub end: Option<DateTime<Utc>>,
    pub received_at: Option<DateTime<Utc>>, // when the hub received a record later than the event (TML-009)
    pub basis: Basis,                      // Section 4
    pub uncertainty: Vec<String>,          // one sentence for each value the row does not give (TML-006)
    pub occurrence: Option<String>,        // Section 5
    pub superseded_by: Option<Reference>,  // Section 5; the timeline fills it
}
```

- TML-002 names the reference, the source, the category, the start time, the optional end time, the basis and the uncertainty notes. `title`, `rank` and `occurrence` are additions of this contract: a list needs one line of text, and TML-005 needs a rank and a rule for "the same event".
- TML-002 names the reference, the source, the category, the start time, the optional end time, the basis and the uncertainty notes. `received_at` comes from TML-009.

### 2.1 Instants (TML-008)

- An event instant that Anima machines exchange or store is one count: microseconds since 1970-01-01T00:00:00Z (UTC), as a signed 64-bit integer. `crates/hub/src/engine/instants.rs` (leaf) holds the conversion: `micros(at) -> i64` and `from_micros(i64) -> Option<DateTime<Utc>>`. Precision below a microsecond is cut, as PostgreSQL stores `timestamptz`.
- The hub stores an instant as `timestamptz`, which holds that count. The timeline cursor writes it as that count.
- In K1 an instant is `DateTime`: RFC 3339 text, which must give the same count when it is read again. Input text must have an offset (`Z` or `+08:00`); the hub converts it to UTC. The hub writes UTC with the offset `+00:00`.
- A time zone changes only how a client shows an instant. To change the time zone of a machine changes no stored instant and no old source time.

### 2.2 Event time and received time (TML-009)

- `start` and `end` are the times that the source gives for the event. An adapter must not put the time that the hub received the record in their place.
- `received_at` is the time that the hub received the record, when that is later than the event, for example an import of last year's workouts. A replay of the import keeps the first event time.
- When the source gives no event time, or an approximate one, the adapter writes an uncertainty note. It does not use the received time as the event time.
- For the event log, the hub records the event when it happens. `received_at` is null.

### 2.3 Dates and schedules (TML-010)

- A calendar date, a recurring schedule and its time zone are not instants. A record that has only a date names it with the subject key `date:<YYYY-MM-DD>` (K-E4 Section 3) and keeps it as a date.
- An adapter that must give a timeline entry for a record with only a date writes the uncertainty note "Time of day not given by the source." and uses the start of that date in UTC for `start`. The record keeps its date.

### 2.4 Clocks (TML-011)

- The timeline claims no exact order between entries that come from the clocks of different machines. Two entries in the newest-first order are in that order only by their recorded instants.
- An adapter must not change an old source time to hide clock skew.
- The clock policy (how skew is found and reported, the permitted skew, what an unreliable clock does) is not decided (OD-E16). E0-4 builds no clock check.

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
- An entry of the same source as the leader is not superseded. A replay of one source record is not a second source (TML-012): the domain that owns the record makes the replay idempotent (K-E1 Section 3, SRC-012), and the timeline keeps each reference once on a page.
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
| `received_at` | none: the hub recorded the event when it happened |
| `basis` | `Observed` |
| `occurrence` | none |

- An event leaves the timeline when the event log drops its partition (DB-005). The timeline keeps no copy.
- K1: `Query.timeline(from, to, categories, first = 100, after): EvidenceConnection!`. `first` is 1 to 200. `VALIDATION` when `to` is not after `from`, for a bad `first` and for a cursor that the timeline did not make. The cursor is opaque: `t1.`, the start in microseconds since the epoch, a dot and the reference.
- Commands (CMD-030): `anima timeline list`; MCP tool `anima_timeline_list`.

## 8. Open questions

| Question | State in this contract |
| --- | --- |
| Clock synchronization between machines (OD-E16) | Not decided and not built (Section 2.4). |
| How `occurrence` is made when two domains record one event | Not decided. No inference from overlapping times is built. |
| Who assigns the rank of each source | Not decided. Only the rank of the event log (100) exists. |
| `superseded_by` across pages (Section 5, the limit) | Open. |
| `title` in Evidence | An addition to TML-002 for the operator to accept or remove. |
| An instant in K1 as an integer count | Not added. K1 keeps RFC 3339 `DateTime` as the text form of the count (Section 2.1). |
