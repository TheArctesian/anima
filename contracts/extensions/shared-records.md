# K-E4 — Record links, the shared subject keys, places, files and location evidence (engine)

Status: approved for the engine build (operator, 2026-11-16 for Sections 1 to 7; R-148 of 2026-11-18 for the whole file). Proposed for engine task E0-4; it is not part of the Phase 3 freeze. Change it only through a programme ruling (CON-001). Requirement IDs: RLK-001, RLK-002, RLK-004, RLK-005 and RLK-006 of `docs/plans/extensions-plan.md` (RLK-005 and RLK-006 from issue #7, operator, 2026-11-16). Owner: engine (`crates/hub/src/engine/links*`, `crates/hub/src/engine/records.rs`).

Sections 8 to 13 are the places, files, extraction and location parts (engine task E0-5; PLC-001 to PLC-006, FIL-001 to FIL-005, VIS-001 to VIS-004; R-145, R-146). RLK-003, the one shared client component, is not built: issue #7 makes it later work for the pages that use links.

## 1. Record reference (`crates/hub/src/engine/records.rs`, leaf)

A record reference names one record of one domain. It is the text `kind:id`.

- `kind` is lower snake case, 1 to 40 characters, and starts with a letter (`^[a-z][a-z0-9_]{0,39}$`). It says which sort of record it is, for example `event`, `book`, `person`. The domain that owns the record chooses it.
- `id` is the text after the first colon: 1 to 200 characters, no control character, no space at its start or end. It is opaque. Only the owner reads a meaning into it.
- The reference is not the address of a database row: `kind` is not a table name, and no code other than the owner uses `id` to read a table (RLK-005, operator, 2026-11-16).
- A reference does not prove that the record exists.
- Identity (RLK-005): the owner gives each record one reference, and it does not change when the record is imported again, so a second import resolves to the same record and makes no second one. A link to the same source, target and kind is one row, also when it is made again. How a second machine or a replica names the same record is not decided (OD-E17).

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

## 2. Record links (RLK-001, RLK-002)

One table holds every link (schema `links`, migration 2030):

`links.record_link (id uuid pk, source text not null, target text not null, kind text not null, label text, created_at timestamptz not null, created_by text not null, unique (source, target, kind), check (source <> target))`, with an index on `target`.

- `source` and `target` are record references. No foreign key: the record of each side belongs to a different domain and does not have to exist (RLK-002).
- `kind` says what the link means, for example `lent_to`. It has the form of a reference kind. The extension that makes the link names it. This contract has no list of link kinds.
- `label` is the text to show for the target while the target has no record, for example the name of a person. It is one line of at most 200 characters. The link keeps its label after the target has a record.
- A link has a direction. The same source, target and kind is one link. To make it again writes nothing; with a different label it replaces the label. With no label it keeps the stored label.
- A record cannot link to itself.
- To remove a link does not change the two records.
- Missing target (RLK-006): a link to a record that does not exist keeps its target reference and its label. It does not block the extension that made it, and no code copies the target's record into the link. To delete a record does not delete its links; what a link does then is not decided (OD-E17).

```rust
// engine::links
pub async fn connect(conn: &mut PgConnection, new: NewLink, (at, by): (DateTime<Utc>, &str)) -> Result<(Link, Outcome), Error>;
pub async fn disconnect(conn: &mut PgConnection, id: RecordLinkId) -> Result<Option<Link>, Error>;
pub async fn touching(db: &PgPool, record: &Reference) -> Result<Vec<Link>, Error>;   // newest first, at most 200
pub fn own_title(record: &Reference) -> Option<String>;
```

An extension makes a link through `links::connect` in its own transaction, or through `Mutation.linkRecords`. It does not import the extension that owns the other record.

## 3. Shared subject keys (RLK-004, leaf)

A subject key is a reference that two extensions can both make without an import, so that their records meet at it.

| Subject | Reference | Rule |
| --- | --- | --- |
| A GitHub repository | `repository:<owner>/<name>` | Lower case. Each part has letters, digits, `-`, `_` and `.`; the owner at most 39 characters, the name at most 100. |
| A place | `place:<PlaceId>` | A UUID in lower case with hyphens. |
| A person | `person:<PersonId>` | A UUID in lower case with hyphens. |
| A date | `date:<YYYY-MM-DD>` | A calendar date. It has no time and no time zone. It is not an instant, and no code changes it to one (TML-010). |

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

## 4. Titles of the other records (built with places, E0-5)

For a record that is not a repository or a date, the domain that owns it must give its title. `places` is the first such domain (Section 8). For a kind that no domain gives titles for, `sourceTitle` and `targetTitle` are null and a reader shows `label`:

```rust
// engine::links; places is the first user (`place`); a domain gives its `Titles` at wiring in `schema.rs`
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
| The portable form of a reference across machines and replicas (OD-E17) | `kind:id` with an opaque `id`. How a second hub or a replica names the same record is not decided. |
| A target that is deleted (OD-E17) | Not decided. Now the link stays, with its label, and its target reference names a record that does not exist. No domain deletes links. |
| The data class of a label (it can be the name of a person) | Not decided; it waits for K-E5. Now the label is in `links.record_link` and in the audit entry, and not in the event log. |
| A closed list of link kinds | None. An extension names its kinds. |
| RLK-003, the shared client component | Not built. Issue #7: later work for the pages that use links (#18, #13, #22, #24, #16). |
| More than 200 links for one record | `recordLinks` gives the newest 200 and has no pages. |

## 8. Places (PLC-001 to PLC-006; `crates/hub/src/engine/places/`, schema `places`, migrations 2040–2049)

`places.place (id uuid pk, parent_id uuid null references places.place, name text not null, latitude double precision null, longitude double precision null, radius_m integer null, category text not null, icon text not null, aliases text[] not null default '{}', geocoder_name text null, merged_into uuid null references places.place, created_at timestamptz not null, updated_at timestamptz not null)`

`places.site_place (site_id uuid pk, place_id uuid not null references places.place)`. No foreign key leaves the schema: `site_id` is a `SiteId` of `devices`, held as a value.

- A top-level place (`parent_id` null) has a centre and a radius: `latitude` −90 to 90, `longitude` −180 to 180, `radius_m` 1 to 100 000. A child place has none of the three (PLC-002); it is inside its top-level place. A place is not its own ancestor. The depth is at most 5.
- `name` is the name that the operator gave: 1 to 120 characters, one line. `geocoder_name` is a name from a geocoder. The title of a place is `name`; a geocoder never changes `name` (PLC-003). `aliases` holds at most 20 other names of the same form.
- `category` is one of `home`, `work`, `site`, `shop`, `leisure`, `travel`, `storage`, `other`. `icon` is a name of the icon table of the client (1 to 40 characters, `^[a-z][a-z0-9-]*$`); the hub does not check that the icon exists.
- The reference of a place is `place:<PlaceId>` (Section 3). `places` implements `Titles` (Section 4) for the kind `place`.
- **References.** A reference to a place is one of: a record link whose source or target is `place:<id>`; a row of `places.site_place`; a child place. A later domain that keeps a `PlaceId` in its own table gives a `Holder` to the places domain at wiring, so that `places` never imports its store:

```rust
#[async_trait]
pub trait Holder: Send + Sync {
    fn name(&self) -> &'static str;                                   // the domain, for the answer of a refused delete
    async fn count(&self, conn: &mut PgConnection, place: PlaceId) -> Result<u64, Unavailable>;
    async fn move_all(&self, conn: &mut PgConnection, from: PlaceId, to: PlaceId) -> Result<u64, Unavailable>;
}
```

- **Merge (PLC-004).** `merge(from, into)` moves every reference of `from` to `into` in one transaction (links through `links::retarget`, site rows, children, each `Holder`), adds the name and the aliases of `from` to the aliases of `into`, and sets `from.merged_into`. A merged place is not in a list and cannot be changed. `place:<from>` still resolves: `places::current(id)` follows `merged_into`. A link that would become a link of a record to itself, or a second copy of a link that exists, is removed.
- **Split (PLC-004).** `split(id, new, moved)` makes one new top-level place from `new` and moves the named children, the named site rows and the named record links from `id` to it, in one transaction. A reference that is not named stays.
- **Delete (PLC-005).** A place with no reference is deleted. A place with references is deleted only with a replacement: the call is then a merge into the replacement. With references and no replacement the answer is `CONFLICT` with the count of each sort of reference.
- **Sites (PLC-006).** One site links to at most one place; one place can have many sites. The place of a device is the place of its site. This is configured evidence: it says where the operator placed the site, not where the device is (Section 12).

`links` gains `pub async fn retarget(conn, from: &Reference, to: &Reference) -> Result<u64, Error>` for the merge, `retarget_named(conn, ids, (from, to))` for the split and `count(conn, record)` for the references of a delete (the count of all links, where `touching` stops at 200). K3: `places -> links` (`retarget`, `retarget_named`, `count`, `Titles` and `Unavailable` only), `places -> events` (the trail), `places -> records`; the GraphQL part of `places` reads `devices::sites` to check that a site exists.

## 9. Files (FIL-001 to FIL-005; `crates/hub/src/engine/files/`, schema `files`, migrations 2050–2059)

`files.piece (hash text pk, key_name text not null, size bigint not null, sealed_size bigint not null, stored_at timestamptz not null)`

`files.archive (id uuid pk, owner text not null, name text not null, created_at timestamptz not null, created_by text not null)`

`files.attachment (id uuid pk, hash text not null references files.piece, owner text not null, name text not null, media_type text not null, archive_id uuid null references files.archive, latitude double precision null, longitude double precision null, created_at timestamptz not null, created_by text not null)`, with an index on `owner`.

- **A piece** is one file after it is sealed. `hash` is the SHA-256 of the plain bytes, in lower case hexadecimal, with the name of its key before it: the same bytes under two keys are two pieces, and the same bytes under one key are stored one time. The store keeps a piece at `<root>/<key_name>/<first two characters>/<hash>` (FIL-001, R-145).
- **The store** is one interface, so that the place of the pieces can change later ([bulk-data-placement.md](../../docs/proposals/bulk-data-placement.md)). Now it has one form, a folder of the hub named by `ANIMA_FILES_DIR`. With no `ANIMA_FILES_DIR` the hub has no file store: an upload answers `UNAVAILABLE` and the other domains work.

```rust
#[async_trait]
pub trait Pieces: Send + Sync {
    async fn put(&self, key_name: &str, hash: &str, sealed: SealedStream) -> Result<u64, StoreError>;   // the sealed size
    async fn get(&self, key_name: &str, hash: &str) -> Result<SealedStream, StoreError>;
    async fn remove(&self, key_name: &str, hash: &str) -> Result<(), StoreError>;
}
```

- **Sealing (FIL-002, R-146).** Each piece is sealed with the sealing key of the extension that owns the record (K-E5, `vault::SealingKeys`; `key_name` is the key of the extension, or `engine`). The form is XChaCha20-Poly1305 in parts of 64 KiB: each part has its own nonce and its tag, the number of the part and a last-part mark are in the additional data, and the hash and the key name are in the additional data of each part. Thus a part cannot move, a piece cannot be cut short, and a piece does not open as a different piece. The hub seals and opens as a stream; it never holds a whole file (FIL-005).
- **Owner.** `owner` is the record reference (`kind:id`) of the record that the file belongs to. An attachment is not a general drive: it has exactly one owner.
- **Upload.** `POST /files/v1/attachments?owner=<reference>&name=<name>&key=<key name>[&archive=<id>][&keep_location=true]`, with the token of K5 in `Authorization: Bearer`, the media type in `Content-Type`, and the file as the body. This path has no 1 MiB limit; its limit is 256 MB (268 435 456 bytes), counted while the body streams, and a larger body gets HTTP 413 and leaves no piece (FIL-005). The answer is the attachment as JSON (`id`, `hash`, `size`, `name`, `mediaType`). A write key or a login can upload; a read key cannot.
- **Download.** `GET /files/v1/attachments/<id>` gives the plain bytes as a stream, with the stored media type and `Content-Disposition: attachment`. For a JPEG the hub removes the GPS part of the EXIF data from the copy that it sends (FIL-003); the stored piece does not change. A media type whose location metadata the hub cannot remove is sent as it is; the list of types that the hub cleans is in the code and in the answer header `Anima-Location-Removed: true|false`.
- **The location field (FIL-003).** With `keep_location=true` the hub reads the GPS position of a photo at upload and keeps it in `latitude` and `longitude`. Without it the two are null. It is metadata of the file. It is not evidence of where the operator is or where the file is stored (Section 12).
- **Archives (FIL-004).** An archive is the unit of deletion of an import. `delete_archive(id)` deletes, in one transaction, each attachment of the archive and each record that came from it, then the pieces that no attachment uses. A domain that makes records from an archive gives a `Derived` to the files domain at wiring:

```rust
#[async_trait]
pub trait Derived: Send + Sync {
    fn name(&self) -> &'static str;
    async fn delete_from_archive(&self, conn: &mut PgConnection, archive: ArchiveId) -> Result<u64, Unavailable>;
}
```

- To delete an attachment removes its piece when no other attachment has the same hash and key.
- Each upload, download and delete writes one audit entry (`target_type` `attachment` or `archive`) with the owner, the name, the size and the hash. A download writes no event. No entry and no log line holds bytes of a file (LOG-007).

## 10. Extraction proposals (VIS-001 to VIS-004; `crates/hub/src/engine/files/proposals.rs`, table `files.proposal`)

`files.proposal (id uuid pk, attachment_id uuid null references files.attachment on delete cascade, owner text not null, form text not null, origin text not null, fields jsonb not null, state text not null, created_at timestamptz not null, decided_at timestamptz null, decided_by text null)`

- A proposal is the result of one extraction: typed fields, each with a confidence (VIS-004). `fields` is a list of `{ name, type, value, confidence }`: `type` is `text`, `number`, `date`, `boolean` or `reference`; `value` is text in the form of its type; `confidence` is 0.0 to 1.0 or null when the extractor gave none. A null confidence is unknown, not certain.
- `form` names what the extension asked for (the form of a reference kind, for example `garment`, `book`). The extension that owns `owner` defines the fields of its forms. The engine does not know them.
- `origin` is `client` (the extraction ran on the client, VIS-002) or `hub`. The engine has no extractor: the model service owns models and inference ([C1](../../docs/work-items/compute/C1.md)). A hosted model is permitted only per extension (VIS-003, OD-E4); the engine does not grant it.
- `state` is `proposed`, `confirmed` or `rejected`. Nothing of a proposal is a record until the operator confirms it (VIS-001). `confirm(id, fields)` takes the fields as the operator left them, stores them as the confirmed fields and sets the state; the owner extension reads confirmed proposals with `files::confirmed(owner)` and makes its record. A proposal that is decided does not change.
- A Class A image never reaches the hub in a form that the hub can read (VLT-001): for a Class A form the client sends only the proposal after the operator confirmed it on the client, sealed as K-E5 says. The engine refuses `origin: hub` for a form that its extension declares Class A.

## 11. Location samples (R-145, R-146; `crates/hub/src/engine/locations/`, schema `locations`, migrations 2060–2069)

`locations.sample (id uuid pk, provider text not null, account text not null, sample_key text not null, observed_at timestamptz null, received_at timestamptz not null, sealed bytea not null, unique (provider, account, sample_key))`, with an index on `observed_at`.

- A sample is one observation of the position of the operator's phone. `provider` is `dawarich`, `owntracks` or `anima`. `account` is the source account or the ingest token that gave it. `sample_key` is the identity that the provider gives the sample (the point id of Dawarich; for OwnTracks `<tid>:<tst>`; for the Anima apps the identifier that the app made). The same sample again is one row: ingestion is safe to repeat.
- `observed_at` is the time of the fix as the provider gave it, to the precision that it gave (K-E3 instants). `received_at` is the time of the hub. A sample with no time has `observed_at` null and never matches an event. A late import keeps its `observed_at`.
- `sealed` holds the position: `{ latitude, longitude, accuracy_m, altitude_m, speed_mps }`, each null when the provider gave none, sealed with the sealing key `locations` (Class B, VLT-003, R-146). The time is not sealed, so that a match needs no opening of other samples. A missing accuracy stays null; no code puts a number in its place.
- The newest sample is not "the position": no code replaces an older sample, and no function gives a current position as evidence for a past event.

Three ways in, one storing path (`locations::store_samples`):

| Provider | Way | Detail |
| --- | --- | --- |
| Dawarich | Pull, through the source framework (K-E1 Section 7b, R-210) | Declaration `dawarich`, with the acquisition `AccountKey`. The account has the base address of the operator's server and its API key (source secret). A run reads `GET /api/v1/points` with `start_at`, `end_at` (RFC 3339; the end is the year 2100), `order=asc`, `page` and `per_page` 1 000, with the key in `Authorization: Bearer`, until a page holds fewer than 1 000 points (a provider reads no header of a reply, K-E1 Section 1). A point gives `id`, `timestamp` (Unix seconds), `latitude` and `longitude` (text or number), `accuracy` and `altitude` (metres), `velocity` (km/h). A point with no `id` is a problem of the run and is not stored. The sync state holds the time of the last point; the next run starts 1 second before it, and a point that the hub has is one row. The first run reads from 1970. The probe of the connection is `GET /api/v1/points?per_page=1`. The intake is of the engine: its consumer is the fixed entry `integrations`, which is always on, and the operator stops the pull with the pause of the account (`setSourceAccountPaused`). `removeSourceAccount` with `deleteRecords` true deletes the samples of the account. |
| OwnTracks | Push | `POST /locations/v1/owntracks` with HTTP Basic: the user is any text and the password is an ingest token (below). The body is the OwnTracks JSON; `_type: location` gives `tst` (Unix seconds), `lat`, `lon`, `acc`, `alt` (metres), `vel` (km/h), `tid`. Every other `_type` is accepted and not stored. The answer is `200` with `[]`. The body limit is 1 MiB. |
| Anima phone apps | Push | `Mutation.recordLocationSamples(samples: [LocationSampleInput!]!): Int!` with the token of the device. A sample gives `key`, `observedAt` (microseconds, K-E3) and the position fields. At most 500 in one call. The phone part is a task of the mobile plan. |

- **Ingest token.** `locations.ingest_token (id uuid pk, name text not null, token_hash bytea not null unique, created_at timestamptz not null, last_used_at timestamptz null, revoked_at timestamptz null)`. `Mutation.createLocationIngestToken(name)` shows the token one time. It permits only the OwnTracks path. The command shows it one time; no MCP tool makes or returns it (CMD-030, Section 7 of the command line plan).
- Code of Dawarich, OwnTracks Recorder, Reitti and GPSLogger is not copied (AGPL-3.0 and GPL). The wire forms are built from their documents.

What comparable programs take (checked 2026-11-18, R-145): OwnTracks HTTP JSON (Dawarich, Reitti, GeoPulse, PhoneTrack); Overland batches (Dawarich, Reitti, GeoPulse); the GPSLogger custom URL (Dawarich, Reitti, GeoPulse); the Traccar Client or OsmAnd protocol (Traccar, GeoPulse, Dawarich); Home Assistant; Google Timeline JSON, GPX and GeoJSON files (Dawarich, Reitti, GeoPulse). This contract builds Dawarich, OwnTracks and the Anima apps. Overland, GPSLogger, the OsmAnd protocol and the file forms are not built; each is one more row of the table above on the same storing path, when the operator asks for it.

## 12. The context of a transfer event (E0-5; `crates/hub/src/engine/locations/context.rs`)

One file transfer over SSH or SFTP is one event of the event log, with one reference `event:<uuid>` (K-E3). The two devices show the same event. `locations.transfer_context` holds what was known at the time of the event; it copies no record of a different domain and no content of the file:

`locations.transfer_context (event_id uuid pk, occurred_at timestamptz not null, direction text not null, source_device uuid null, source_site uuid null, source_place uuid null, destination_device uuid null, destination_site uuid null, destination_place uuid null, actor_kind text not null, actor text null, client_device uuid null)`

- **Roles.** The source and the destination are devices. The actor is who started the transfer: `actor_kind` is `human` (a login of the operator), `agent` (an API key) or `system`; `client_device` is the device of the client that asked, when the hub knows it. The operator is a fourth role, and only a location sample is evidence for it.
- A transfer through the client has a device on one side and the computer of the client on the other. When that computer is not a device of the hub, its side is null: unknown, not the hub. The place of the hub that stores the event is never evidence.
- **Device evidence** is `configured`: the site of the device and the place of that site (Section 8) as they were at the event. A later change of the site or of the place link does not change the row. To reach a host does not show where it is.
- **Operator evidence** is not stored. It is the result of a pure function of the time of the event and the samples near it, so a sample that arrives late gives the same answer as one that arrived in time, and a newer position never replaces it (R-145):

```rust
pub enum Basis { AtTheTime, NearestInferred, Unknown }
pub struct OperatorEvidence { pub basis: Basis, pub sample: Option<SampleId>, pub offset_seconds: Option<i64>, pub uncertain: bool, pub place: Option<PlaceId> }
pub fn operator_evidence(event_at: DateTime<Utc>, near: &[Sample], places: &[Place], rule: &MatchRule) -> OperatorEvidence;
pub struct MatchRule { pub at_the_time: Duration, pub nearest: Duration, pub worst_accuracy_m: f64 }   // 2 minutes, 10 minutes, 200.0
```

- The nearest sample not more than 2 minutes from the event gives `AtTheTime`. If there is none, the nearest sample not more than 10 minutes from the event gives `NearestInferred`. If there is none, the basis is `Unknown` and there is no sample. Two samples at the same distance: the earlier one.
- `uncertain` is true when the sample has no accuracy or an accuracy worse than 200 m. `place` is the top-level place whose radius contains the sample (the nearest centre when more than one does), and it is given only when the basis is `AtTheTime` and the sample is not uncertain. In each other case `place` is null: an inferred or uncertain sample is never a confirmed place.
- The operator role and the device roles are separate in each answer. No code derives the place of the operator from the place of a device, or the place of a device from a sample.
- The clocks of the phone and of the hub are not compared (OD-E16). The rule uses minutes, so a skew of seconds does not change a result; the contract gives no exact order between a sample and an event (TML-011).
- A person and a device: `actor_kind` says who acted, from the token of the call. That a device is of the operator, that it sent an event, that it answers, or that a sample is near its place is not evidence that the operator used it. The engine builds no presence detector.

## 13. API (K1) and commands (CMD-030) of Sections 8 to 12

| K1 | Behaviour |
| --- | --- |
| `Query.places: [Place!]!`, `Query.place(id: ID!): Place` | Places that are not merged. `place` follows a merge and gives the remaining place. |
| `Mutation.createPlace(input)`, `updatePlace(id, input)`, `mergePlaces(from, into)`, `splitPlace(id, input)`, `deletePlace(id, replacement)`, `setSitePlace(site, place)` | Section 8. `VALIDATION` with `extensions.fields`; `CONFLICT` for a refused delete. |
| `Query.attachments(owner: String!): [Attachment!]!`, `Mutation.deleteAttachment(id)`, `Mutation.createArchive(owner, name)`, `Mutation.deleteArchive(id)` | Section 9. The bytes go through `/files/v1`, not through GraphQL. |
| `Query.extractionProposals(owner: String, state: ProposalState): [ExtractionProposal!]!`, `Mutation.proposeExtraction(input)`, `confirmExtraction(id, fields)`, `rejectExtraction(id)` | Section 10. |
| `Mutation.connectDawarich(input: DawarichInput!): SourceAccount!` (R-210) | Section 11 and K-E1 Section 7b. `DawarichInput { label, baseUrl, apiKey }`. Login or browser token only. `VALIDATION` on `baseUrl` for an address that is not `https://host[:port]` or that the hub does not connect to, on `apiKey` when the server refuses the key, on `label`; `UNAVAILABLE` when the server does not answer. The command is `anima source connect dawarich --label <text> --url <address>`, which reads the key from standard input or asks for it with no echo; no MCP tool takes the key. |
| `Query.locationSamples(from: Instant!, to: Instant!, first: Int): [LocationSample!]!`, `Mutation.recordLocationSamples(samples)`, `Mutation.createLocationIngestToken(name)`, `Mutation.revokeLocationIngestToken(id)`, `Query.locationIngestTokens` | Section 11. A span is at most 31 days; at most 1 000 samples. Each read of samples writes one audit entry with the span and the count, not the positions. |
| `Query.transferContext(event: ID!): TransferContext` | Section 12: the stored roles and the operator evidence, computed at the read. Null when the event is not a transfer. |

- Each change writes its row, one audit entry and, apart from downloads, sample reads and stored samples, one event, in one transaction. An audit entry of location samples holds no position.
- **A stored sample (R-209).** A stored fix writes no event, and no audit entry of its own. For the OwnTracks path the hub counts the fixes that it stored for each ingest token in each hour: `locations.ingest_hour (token_id uuid references locations.ingest_token on delete cascade, hour timestamptz, fixes integer, client inet, primary key (token_id, hour))`, where `client` is the address of the last post of the hour. When an hour has ended, the hub writes one audit entry for the token and the hour (`target_type` `location_ingest_token`, action `create`, with the name of the token, the start of the hour and the count) and deletes the row; it does this at the next post of any token and in the daily upkeep. A fix that the hub already had is not counted. A call of `recordLocationSamples` that stored a sample writes one audit entry with its counts and no event. The Dawarich pull writes no entry for a sample: its run has its own record (K-E1 Section 5).
- Commands: `anima place list|show|add|set|merge|split|remove|site`, `anima file list|upload|download|remove`, `anima file archive add|remove`, `anima extraction list|propose|confirm|reject`, `anima location list|import|token add|token list|token revoke`, `anima location context <event>`. `anima location import <file>` reads a file of samples in the Anima form or OwnTracks JSON lines and calls `recordLocationSamples`.
- MCP tools: one for each command, apart from `file upload` and `file download` (bytes), and the three `location token` commands (a token is a secret).
- K2: ranges 2040–2049 `places`, 2050–2059 `files`, 2060–2069 `locations`. K3 (R-210): the wiring gives `sources` the intake `locations::dawarich_intake(seal)`; `locations` imports `sources` (`Intake`, `Page`, `Taken`, `Counts`, `IntakeError`) and `extensions` (`INTEGRATIONS` only) and the crate `provider-dawarich`. K3: `places -> links, events, records` (and `devices` from its GraphQL part); `files -> vault (SealingKeys), events, records`; `locations -> places, sources, vault (SealingKeys), events, records, instants`; `sessions` calls `locations::record_transfer` at the transfer boundary, in the transaction of its trail.
