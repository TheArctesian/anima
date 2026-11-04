-- A session ends once (RA-008, D-9; least privilege after the WP-10 final review).
--
-- 0600 lets anima_runtime update only `ended_at` and `end_reason`, but that column grant
-- still lets it reopen an ended row (set both back to null) or rewrite how it ended. This
-- trigger refuses, for every role:
--
-- - any update of a row that has already ended. Every hub end path updates with
--   `where ended_at is null`, so it never reaches an ended row: "already ended" stays a
--   no-op that affects 0 rows, not an error;
-- - an end before the session started.
--
-- `ended_at` and `end_reason` set together is `sessions_end_pair_check` from 0600. A check
-- runs on every row an update writes, after this trigger, so a half end is refused there;
-- this trigger does not repeat it.
--
-- The function is not SECURITY DEFINER and touches no table, so it needs no search_path.

create function sessions.refuse_end_rewrite() returns trigger
language plpgsql
as $$
begin
    if old.ended_at is not null then
        raise exception 'session % has already ended', old.id
            using errcode = 'check_violation', constraint = 'sessions_end_once';
    end if;
    if new.ended_at < old.started_at then
        raise exception 'session % cannot end before it started', old.id
            using errcode = 'check_violation', constraint = 'sessions_end_after_start';
    end if;
    return new;
end
$$;

create trigger sessions_end_once
    before update on sessions.sessions
    for each row execute function sessions.refuse_end_rewrite();

-- No hub code deletes a host key: accepting a key upserts it, and deleting a device removes
-- its key through `host_keys_device_id_fkey`'s cascade, which runs with the table owner's
-- rights, not the caller's.
revoke delete on sessions.host_keys from anima_runtime;
