-- H-1, R-073: the metrics collector reports its own events, so `events.event_log.source`
-- accepts 'metrics'. 0500 created the check unnamed; Postgres named it
-- `event_log_source_check`. The check is inherited by every 90-day partition.
--
-- NOT VALID is possible on a partitioned table for a check constraint: the parent and the
-- partitions take the new check without scanning, and new rows are checked at once. The
-- VALIDATE step then scans each partition under a lock that lets writes continue, so the
-- event log is never blocked for the scan (DB-010). The new check admits every old value,
-- so validation cannot fail.

alter table events.event_log drop constraint event_log_source_check;

alter table events.event_log
    add constraint event_log_source_check
    check (source in ('check', 'session', 'device', 'access', 'hub', 'metrics')) not valid;

alter table events.event_log validate constraint event_log_source_check;
