-- NOT-004, D-16 (K2 names this change 0503): the notification dispatcher reports its own
-- events (a delivery given up), so `events.event_log.source` accepts 'notification'. As 0701
-- did for 'metrics': 0500 created the check unnamed, Postgres named it
-- `event_log_source_check`, and every partition inherits it.
--
-- It is numbered 0852, not 0503, on purpose: 0701 replaces this same check, and a migration
-- numbered 0503 would run before 0701 on a new database, so 0701 would drop 'notification' again.
-- This one runs after it and lists every source 0701 listed.
--
-- A plain drop and add. DROP CONSTRAINT takes ACCESS EXCLUSIVE on the table until the
-- migration transaction ends, and ADD CONSTRAINT scans every partition under that lock; the log
-- holds at most 90 days, so the scan is short (DB-010). NOT VALID would not help: the new
-- check admits every old value, and the drop already holds the strong lock.

alter table events.event_log drop constraint event_log_source_check;

alter table events.event_log
    add constraint event_log_source_check
    check (source in ('check', 'session', 'device', 'access', 'hub', 'metrics', 'notification'));
