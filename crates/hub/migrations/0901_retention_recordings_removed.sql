-- R-113 (RA-040 deferred): there are no recordings to keep, so `recordings` is no longer a
-- retention subject. Deletes its row from 0900 and restates both checks of 0900 without it:
-- `retention::periods` reads every row and refuses a subject it does not know (DB-011, D-25).

delete from retention.periods where subject = 'recordings';

alter table retention.periods
    drop constraint periods_subject_check,
    drop constraint periods_days_check,
    add constraint periods_subject_check check (
        subject in ('heartbeats', 'metrics', 'events', 'deliveries')
    ),
    add constraint periods_days_check check (
        case subject
            when 'heartbeats' then days between 1 and 400
            when 'metrics' then days between 1 and 400
            when 'events' then days between 30 and 365
            when 'deliveries' then days between 7 and 365
        end
    );
