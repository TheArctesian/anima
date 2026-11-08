-- How long each kind of data is kept (K2 range 0900-0919; HUB-005, DB-011, D-25).
--
-- Privileges (R-016, K2 "Roles"): anima_runtime gets SELECT, INSERT, UPDATE and DELETE on this
-- table from the default privileges that migration 0001 sets for the owner role.
--
-- One row per subject. The bounds are the operator's range for each subject; the Rust side
-- (retention::bounds) states the same numbers, and a test holds the two together. Raw
-- heartbeats, raw metrics, recordings and deliveries default to 30 days, the event log to 90.
-- The aggregates and the audit log are not here: they stay fixed (DB-009, LOG-006).

create schema retention;

create table retention.periods (
    subject text primary key,
    days integer not null,
    constraint periods_subject_check check (
        subject in ('heartbeats', 'metrics', 'events', 'recordings', 'deliveries')
    ),
    constraint periods_days_check check (
        case subject
            when 'heartbeats' then days between 1 and 400
            when 'metrics' then days between 1 and 400
            when 'events' then days between 30 and 365
            when 'recordings' then days between 1 and 365
            when 'deliveries' then days between 7 and 365
        end
    )
);

insert into retention.periods (subject, days) values
    ('heartbeats', 30),
    ('metrics', 30),
    ('events', 90),
    ('recordings', 30),
    ('deliveries', 30);
