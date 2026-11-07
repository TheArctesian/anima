-- H-1, R-073: the metrics collector reports its own events, so `events.event_log.source`
-- accepts 'metrics'. 0500 created the check unnamed; Postgres named it
-- `event_log_source_check`. Every 90-day partition inherits it.
--
-- A plain drop and add. DROP CONSTRAINT takes ACCESS EXCLUSIVE on the table until the
-- migration transaction ends, and ADD CONSTRAINT scans every partition under that lock, so
-- the event log blocks for the length of the migration. The log holds at most 90 days, so
-- the scan is short (DB-010). NOT VALID would not help: the new check admits every old
-- value, and the drop already holds the strong lock.

alter table events.event_log drop constraint event_log_source_check;

alter table events.event_log
    add constraint event_log_source_check
    check (source in ('check', 'session', 'device', 'access', 'hub', 'metrics'));
