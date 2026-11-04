-- A session ends once, never before it started; the runtime role holds the least it needs
-- (RA-008, D-9, MV-003; WP-10 final review).

-- Every session time is a real instant: `infinity` would sort after every end and never let
-- the row read as ended at a time (C3).
alter table sessions.sessions
    add constraint sessions_times_finite check (
        isfinite(started_at) and (ended_at is null or isfinite(ended_at))
    );

-- An end is never before the start, on insert or update. The hub writes
-- `greatest(<now>, started_at)`, so a row started ahead of a clock that stepped back still
-- ends; this check is the backstop.
alter table sessions.sessions
    add constraint sessions_end_after_start check (ended_at is null or ended_at >= started_at);

-- `ended_at` and `end_reason` set together is `sessions_end_pair_check` from 0600.

-- The update rule a check cannot state, because it compares the row with its old self: an
-- ended row is never updated again, by any role. Every hub end path updates with
-- `where ended_at is null`, so it never reaches an ended row: ending an ended session stays
-- a no-op that affects 0 rows, not an error.
--
-- The body names no table, type or function outside pg_catalog, and search_path is pinned
-- like 0500 and 0502, so a caller's search_path or pg_temp objects cannot change what it
-- runs. It is not SECURITY DEFINER: it runs as the role that updates.
create function sessions.refuse_end_rewrite() returns trigger
language plpgsql
set search_path = pg_catalog, pg_temp
as $$
begin
    if old.ended_at is not null then
        raise exception 'session % has already ended', old.id
            using errcode = 'check_violation', constraint = 'sessions_end_once';
    end if;
    return new;
end
$$;

create trigger sessions_end_once
    before update on sessions.sessions
    for each row execute function sessions.refuse_end_rewrite();

-- The runtime role starts a session live: it may insert the start columns only, the columns
-- `sessions::store::insert` names, so it cannot insert a row that has already ended (C2).
revoke insert on sessions.sessions from anima_runtime;
grant insert (id, device_id, device_name, protocol, operator_id, operator_name, client_addr,
              started_at)
    on sessions.sessions to anima_runtime;

-- No hub code deletes a host key: deleting a device removes its key through
-- `host_keys_device_id_fkey`'s cascade, which runs with the table owner's rights, not the
-- caller's. Accepting a key upserts it and never moves it to another device, so the runtime
-- role may update every column but `device_id` (`sessions::store::put_host_key`).
revoke delete, update on sessions.host_keys from anima_runtime;
grant update (algorithm, fingerprint_sha256, public_key, first_seen_at, accepted_at,
              accepted_by)
    on sessions.host_keys to anima_runtime;
