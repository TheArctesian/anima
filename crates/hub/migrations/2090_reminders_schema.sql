-- The reminders domain's tables (K-E8 range 2090-2099; K-E6 Sections 2 to 4; REM-001 to
-- REM-006). The delivery of a firing is the notifications domain's (REM-010).
--
-- `channels` holds IDs of `notifications.channels` as values: no foreign key leaves this
-- schema. A firing is unique for its rule and its subject, so one rule fires at most one time
-- for one subject. `known` holds what the hub knew: the newest time of each sort of record and
-- the newest measure of each key, for `unless` and for the edge of a threshold, and the last
-- health of each source account, for a change of it.
-- Privileges: migration 0001's default privileges give `anima_runtime` what it needs on the
-- new schema (K2 "Roles").
create schema reminders;

create table reminders.rule (
    id               uuid        primary key,
    name             text        not null check (length(name) between 1 and 80),
    message          text        not null check (length(message) between 1 and 200),
    trigger          jsonb       not null,
    unless           jsonb,
    cooldown_minutes integer     not null default 60 check (cooldown_minutes between 1 and 10080),
    daily_limit      integer     not null default 3 check (daily_limit between 1 and 100),
    channels         uuid[]      not null default '{}',
    enabled          boolean     not null default true,
    revision         integer     not null default 1 check (revision >= 1),
    created_at       timestamptz not null,
    updated_at       timestamptz not null
);

create table reminders.firing (
    id            uuid        primary key,
    rule_id       uuid        not null references reminders.rule (id) on delete cascade,
    revision      integer     not null,
    subject       text        not null,
    text          text        not null,
    class         text        not null check (class in ('A', 'B', 'C')),
    state         text        not null check (state in
                      ('pending', 'sent', 'suppressed', 'snoozed', 'acknowledged')),
    reason        text,
    due_at        timestamptz not null,
    sent_at       timestamptz,
    snoozed_until timestamptz,
    decided_at    timestamptz,
    unique (rule_id, subject)
);

create index firing_rule_due on reminders.firing (rule_id, due_at desc);
create index firing_state on reminders.firing (state, due_at);

create table reminders.place_state (
    place_id   uuid        primary key,
    inside     boolean     not null,
    changed_at timestamptz not null,
    sample_id  uuid        not null
);

create table reminders.cursor (
    name     text        primary key,
    position timestamptz not null
);

create table reminders.known (
    sort  text             not null check (sort in ('record', 'measure', 'state')),
    name  text             not null,
    value double precision,
    state text,
    at    timestamptz      not null,
    primary key (sort, name)
);
