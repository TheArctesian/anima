# K-E6 — Reminders: the trigger interface, the rule evaluation and the delivery (engine)

Status: approved for the engine build (R-148 of 2026-11-18). Change it only through a programme ruling (CON-001). Requirement IDs: REM-001 to REM-010 and ENG-001 of `docs/plans/extensions-plan.md` (REM-008 as changed by R-148). Owner: engine (`crates/hub/src/engine/reminders/`, schema `reminders`, migrations 2090–2099). The Reminders page and the rule editor are XP-10, not this contract.

## 1. What an extension gives (the trigger interface)

An extension never changes the reminder code (ENG-001). It gives occurrences. There are two ways.

```rust
// engine::reminders
pub enum Occurrence {
    /// A record that a source or a domain stored: "a Strava workout arrives", "device Y goes Down".
    Record { source: String, kind: String, record: Reference, title: String, class: DataClass, at: DateTime<Utc> },
    /// A number at a time: "sleep of last night, in hours", "spend of this week".
    Measure { key: String, value: f64, class: DataClass, at: DateTime<Utc> },
    /// A thing that ends at a time: a document, a certificate, a token, a key rotation.
    Expiry { subject: String, record: Reference, title: String, class: DataClass, expires_at: DateTime<Utc> },
}

/// Push: in the transaction that stores new records (REM-002).
pub async fn notice(conn: &mut PgConnection, new: &[Occurrence], now: DateTime<Utc>) -> Result<(), Error>;

/// Pull: read on the sweep, for what has no moment of storage.
#[async_trait]
pub trait Triggers: Send + Sync {
    fn name(&self) -> &'static str;                                  // the key of the extension (K-E7)
    async fn expiries(&self, db: &PgPool, now: DateTime<Utc>) -> Result<Vec<Occurrence>, Unavailable>;
    async fn measures(&self, db: &PgPool, now: DateTime<Utc>) -> Result<Vec<Occurrence>, Unavailable>;
}
```

- `source` and `kind` have the form of a reference kind. `key` and `subject` are `<extension>.<name>` in lower snake case, for example `health.sleep_hours`, `documents.passport`. An extension lists its sources, kinds, keys and subjects in its launch brief; the engine has no closed list.
- `title` is one line of at most 200 characters. `class` is the data class of the thing that the occurrence is about (K-E5). The engine uses it only for REM-009.
- A `Triggers` is given to the engine at wiring, as a timeline adapter is. When an extension is off (K-E7), its `Triggers` is not read and its `notice` calls store nothing.
- The engine itself gives three sources with no extension: the event log (each new event is a `Record` with the source and the kind of the event, class C), the source framework (Section 3, source state) and the operational keys of K-E5 (`Expiry` with the subject `keys.rotation` or `keys.expiry`).

## 2. Rules

`reminders.rule (id uuid pk, name text not null, message text not null, trigger jsonb not null, unless jsonb null, cooldown_minutes integer not null default 60, daily_limit integer not null default 3, channels uuid[] not null default '{}', enabled boolean not null default true, revision integer not null default 1, created_at timestamptz not null, updated_at timestamptz not null)`

A rule has one trigger:

| Trigger | Fields | It is due when |
| --- | --- | --- |
| `time` | `at` (`HH:MM`), `days` (a set of weekdays, or empty for each day), `zone` (an IANA name); or `once` (an instant) | The local time of the zone passes `at` on a day of `days`; or the instant passes. A time that a change of the clocks skips is due at the next minute that exists; a time that happens two times is due one time. |
| `place` | `place` (`PlaceId`), `on` (`arrive` or `leave`) | The place state changes to inside, or to outside (Section 4). |
| `source_event` | `source`, `kind`, `contains` (optional text) | A `Record` with this source and kind arrives, and its title contains `contains` (no case). |
| `source_state` | `account` (a source account, or any), `state` (`connected`, `stalled`, `needs_reconnect`) | The health of the account changes to `state` (K-E1 Section 9). |
| `threshold` | `key`, `is` (`below` or `above`), `value` | A `Measure` with this key is below, or above, `value`, and the measure before it was not. |
| `expiry` | `subject`, `days_before` (a list, for example 21, 14, 7) | An `Expiry` with this subject is not more than N days away, one time for each N and each record. |

- `unless` (REM-003) is one condition that stops the reminder: `{ "record_today": { "source", "kind" } }` (a `Record` with this source and kind is known for the local day of the rule's zone, or UTC) or `{ "measure": { "key", "is", "value" } }` (the newest measure of the key meets it).
- `cooldown_minutes` is 1 to 10 080 and `daily_limit` is 1 to 100; the defaults are 60 and 3 (REM-004). The day of the limit is the local day of the zone of a time rule, else UTC.
- `message` is the text of the reminder, one line of at most 200 characters. It can hold `{title}`, which is the title of the occurrence.
- `channels` are notification channels (`notifications.channels`). Empty means: each channel that is "on by default". Each change of a rule adds 1 to `revision`.

## 3. The evaluation (REM-001 to REM-006)

```rust
pub struct Moment<'a> { pub now: DateTime<Utc>, pub new: &'a [Occurrence], pub places: &'a PlaceStates, pub known: &'a Known }
pub struct Due { pub rule: RuleId, pub revision: i32, pub subject: String, pub text: String, pub class: DataClass }
pub fn evaluate(rule: &Rule, history: &History, moment: &Moment<'_>) -> Vec<Due>;
```

- `evaluate` is a pure function: no clock, no database, no random value (REM-001, CQ-021). `History` is what the rule sent before (the times of the last firings of the day and the subjects that fired). `Known` holds the records of the day and the newest measure of each key, for `unless` and for the edge of a threshold.
- `subject` says what the firing is about (the minute of a time rule, the record of an event, `<record>:<N>` of an expiry). One rule fires at most one time for one subject.
- The order of the checks: the trigger is due; `unless` is not met; the cooldown passed since the last firing of the rule; the daily limit is not reached. A firing that the cooldown or the limit stops is stored as `suppressed` with its reason, so that the operator sees it; it is not sent later.
- **When it runs (REM-002).** `notice` evaluates the event rules in the transaction of the caller. A sweep each minute evaluates the time rules, reads the `Triggers`, the new events of the event log, the health of the sources and the new location samples. The sweep takes `now` as a parameter and keeps a cursor for each thing that it reads (`reminders.cursor`), so a hub that was stopped fires a time rule that it missed one time and not one time for each minute.

`reminders.firing (id uuid pk, rule_id uuid not null references reminders.rule on delete cascade, revision integer not null, subject text not null, text text not null, class text not null, state text not null, reason text null, due_at timestamptz not null, sent_at timestamptz null, snoozed_until timestamptz null, decided_at timestamptz null, unique (rule_id, subject))`

- `state` is `pending`, `sent`, `suppressed`, `snoozed` or `acknowledged`.
- **The check at send time (REM-005).** The sender reads the rule again in the transaction that sends. It sends only when the rule exists, is enabled and has the revision of the firing; else the firing becomes `suppressed` with the reason `rule_changed`. A snoozed firing is sent again at `snoozed_until` with the same check.

## 4. Place state (REM-006)

`reminders.place_state (place_id uuid pk, inside boolean not null, changed_at timestamptz not null, sample_id uuid not null)`

- The sweep reads the new location samples in the order of their `observed_at` (K-E4 Section 11). A sample whose point is inside the radius of a top-level place sets that place to inside; a sample outside sets it to outside. `arrive` is the change from outside to inside; a new fix inside a place that is inside is not an arrival. A place with no state is outside.
- A sample with no time, with no accuracy or with an accuracy worse than 200 m changes no state (R-145). A sample older than the sample of the stored state changes no state: a late import never makes an arrival in the past fire now. A place rule fires only when the sample is not more than 10 minutes old at the sweep.

## 5. Delivery (REM-007 to REM-010; R-148)

One delivery code for reminders and for device status (REM-010): a reminder goes through the outbox, the dispatcher, the channels and the egress rules of `notifications` (K3). The engine builds no second transport.

```rust
// fleet::notifications (new)
pub struct Reminder { pub firing: Uuid, pub title: String, pub body: String }
pub async fn queue_reminder(conn: &mut PgConnection, channels: &[ChannelId], reminder: &Reminder, now: DateTime<Utc>) -> Result<u32, Error>;   // rows queued
```

- `notifications.outbox.kind` gains `reminder` (a migration in the range of `notifications`); the row has no check. A delivery of it is in `notifications.deliveries` as each other delivery.
- **To an open application.** `Subscription.reminderFirings: ReminderFiring!` gives each firing when it is sent. The desktop application and the iOS application subscribe and show a notification of the device. The hub counts the open subscriptions.
- **To ntfy and the other channels.** When no application has the subscription open at the send, the hub queues the reminder to the channels of the rule. When one is open, it queues nothing: the application shows it. On a hosted hub Anima runs no ntfy server; the channel is the ntfy channel that the operator set, on any ntfy server (REM-008).
- **Random topic (REM-008).** A reminder is not queued to an ntfy channel whose topic is shorter than 16 characters; the firing then has the reason `topic_not_random` and the channel list of the API says so. `anima notification` commands that make an ntfy channel offer a random topic of 22 characters.
- **The text on a lock screen (REM-009).** For a firing of class C the title is the name of the rule and the body is its text. For a firing of class A or B the title is `Anima` and the body is `You have a reminder.`; the name of the rule, the title of the record and each value stay in the hub, and the application shows them after it is unlocked. The same rule applies to the subscription payload that an application uses for the notification of the device.
- **Snooze and acknowledge (REM-007).** `Mutation.snoozeReminder(firing, minutes)` (1 to 10 080) and `Mutation.acknowledgeReminder(firing)`. The application calls them from the buttons of its notification. A notification of ntfy has no buttons now: the hub has no public address setting, so the operator answers in the application or with the command.
- More channels (the applications as subscribers of a channel, message channels as plug-ins) are OD-E19 and are not in this contract.

## 6. API (K1) and commands (CMD-030)

| K1 | Behaviour |
| --- | --- |
| `Query.reminderRules: [ReminderRule!]!`, `Query.reminderRule(id: ID!): ReminderRule` | The rules with their trigger, their last firing and their count of today. |
| `Mutation.createReminderRule(input)`, `updateReminderRule(id, input)`, `removeReminderRule(id)`, `setReminderRuleEnabled(id, enabled)` | `VALIDATION` with `extensions.fields` for a bad trigger, zone, place or channel. |
| `Mutation.testReminderRule(id): ReminderFiring!` | Sends the rule one time now, with the subject `test:<time>`; no cooldown and no limit apply, and none is used up. |
| `Query.reminderFirings(rule: ID, state: FiringState, first: Int): [ReminderFiring!]!` | Newest first, at most 200. |
| `Mutation.snoozeReminder(firing, minutes)`, `Mutation.acknowledgeReminder(firing)` | Section 5. |
| `Subscription.reminderFirings` | Section 5. |

- Each change of a rule writes one audit entry and one event in one transaction. A firing writes one event with the name of the rule for class C, and with no name for class A or B.
- Commands: `anima reminder list|show|add|set|remove|enable|disable|test`, `anima reminder firings`, `anima reminder snooze <firing> <minutes>`, `anima reminder ack <firing>`, `anima reminder watch` (the subscription, one line for each firing). MCP tools: one for each command, apart from `watch`.
- K2: range 2090–2099, schema `reminders`, tables `rule`, `firing`, `place_state`, `cursor`, `known`. K3: `reminders -> notifications (queue_reminder, the channel list), places, locations (the new samples), sources (health), credentials::operational (the due state), events, records`. No domain other than the wiring imports `reminders`, apart from a call of `notice`.

## 7. Open questions

| Question | State in this contract |
| --- | --- |
| Buttons in an ntfy notification | Not built: the hub has no public address setting. |
| Push from Apple and Google with no open application (Phase 5) | Not built; ntfy is the path now. |
| More channels and the code of other projects (OD-E19) | Not decided. |
| A rule that needs two triggers at one time | Not built. One trigger and one `unless`. |
