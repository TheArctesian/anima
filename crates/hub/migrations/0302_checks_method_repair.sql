-- Checks that repair their method: the operator-edited marker and the repair attempt time
-- (CHK-013, R-064, DB-011).
--
-- `operator_edited` is true once an operator has updated the check through `updateCheck`. A
-- check the operator never edited is still as created, so the hub may repair its method; an
-- edited one is never touched again. `repair_tried_at` is when the hub last tried a repair, so
-- it tries each check once. Both defaults live here, not only in the code (DB-011).
--
-- Privileges: migration 0001's default privileges give `anima_runtime` SELECT, INSERT, UPDATE
-- and DELETE on checks.checks as a whole table, which covers columns added later; no grant
-- changes here.

alter table checks.checks
    add column operator_edited boolean not null default false,
    add column repair_tried_at timestamptz null;

-- Existing rows default to not edited, which is wrong for a check an operator already changed.
-- The audit log is the real record of operator edits, so every check with an `update` entry
-- there is marked edited. On a fresh database this migration runs before 0500 creates the
-- audit log, and there are no checks yet, so the update runs only when the table exists.
do $$
begin
    if pg_catalog.to_regclass('events.audit_log') is not null then
        update checks.checks as c
        set operator_edited = true
        where exists (
            select 1
            from events.audit_log as a
            where a.target_type = 'check'
              and a.action = 'update'
              and a.target_id = c.id::pg_catalog.text
        );
    end if;
end
$$;
