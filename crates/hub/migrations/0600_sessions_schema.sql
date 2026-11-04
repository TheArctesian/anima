-- Remote sessions and accepted SSH host keys (K2 range 0600-0699; RA-008, SEC-006, SEC-007).
--
-- Privileges (R-016, K2 "Roles"): anima_runtime gets SELECT, INSERT, UPDATE and DELETE on
-- these tables from the default privileges that migration 0001 sets for the owner role. The
-- one exception is at the end of this file: session history is append-and-end only.

create schema sessions;

-- One row per session, kept after it ends. History outlives the device: `device_id` has no
-- foreign key and `device_name` and `operator_name` are copies, like the logs (K2).
-- A row is live while `ended_at` is null.
create table sessions.sessions (
    id uuid primary key,
    device_id uuid null,
    device_name text not null,
    protocol text not null,
    operator_id uuid not null,
    operator_name text not null,
    client_addr inet not null,
    started_at timestamptz not null,
    ended_at timestamptz null,
    end_reason text null,
    constraint sessions_protocol_check check (protocol in ('ssh')),
    -- K1 SessionEndReason in lower_snake case.
    constraint sessions_end_reason_check check (end_reason in (
        'operator', 'remote_closed', 'gateway_error', 'auth_failed', 'host_key_mismatch',
        'protocol_error', 'timeout', 'hub_shutdown'
    )),
    constraint sessions_end_pair_check check ((ended_at is null) = (end_reason is null))
);

-- `Query.sessions` pages newest first by (started_at, id) (API-006).
create index sessions_started_at_id_idx on sessions.sessions (started_at desc, id desc);
create index sessions_device_started_at_idx on sessions.sessions (device_id, started_at desc, id desc);
-- Live sessions: the `live` filter and the stale-row close at start.
create index sessions_open_idx on sessions.sessions (started_at desc, id desc)
    where ended_at is null;

-- The one accepted host key per device (SEC-006). The hub pins guacd to it (SEC-007).
-- `public_key` is the OpenSSH key blob in standard base64; `fingerprint_sha256` is OpenSSH's
-- `SHA256:<base64 without padding>` of that blob, and the check below keeps the two equal, so
-- the fingerprint the operator sees is always the key the hub pins.
create table sessions.host_keys (
    device_id uuid primary key,
    algorithm text not null,
    fingerprint_sha256 text not null,
    public_key text not null,
    first_seen_at timestamptz not null,
    accepted_at timestamptz not null,
    accepted_by uuid not null,
    constraint host_keys_device_id_fkey foreign key (device_id)
        references devices.devices (id) on delete cascade,
    constraint host_keys_accepted_by_fkey foreign key (accepted_by)
        references access.operators (id),
    -- An SSH algorithm name: no space or line break can reach a known_hosts line.
    constraint host_keys_algorithm_check check (algorithm ~ '^[A-Za-z0-9@._+-]{1,64}$'),
    -- Whole base64 quanta, so `decode` below never raises. 10920 characters is the 8190-byte
    -- blob cap of `sessions::public_key::MAX_BLOB` (2730 quanta); keep the two equal.
    constraint host_keys_public_key_check check (
        char_length(public_key) between 4 and 10920
        and public_key ~ '^([A-Za-z0-9+/]{4})*([A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$'
    ),
    -- Canonical base64, as the hub's reader requires: `decode` ignores pad bits that are not
    -- zero (`/mAP==`), and re-encoding shows them. Guarded like the check below.
    constraint host_keys_public_key_canonical check (
        case
            when public_key ~ '^([A-Za-z0-9+/]{4})*([A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$'
            then translate(encode(decode(public_key, 'base64'), 'base64'), E'\n', '')
                = public_key
        end
    ),
    constraint host_keys_fingerprint_check check (
        fingerprint_sha256 ~ '^SHA256:[A-Za-z0-9+/]{43}$'
    ),
    -- Checks run in name order, so this one sees a key the shape check above has not yet
    -- refused; it leaves such a key to that check.
    constraint host_keys_fingerprint_matches_key check (
        case
            when public_key ~ '^([A-Za-z0-9+/]{4})*([A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$'
            then fingerprint_sha256
                = 'SHA256:' || rtrim(encode(sha256(decode(public_key, 'base64')), 'base64'), '=')
        end
    )
);

-- Session history is append-and-end only for the runtime role (RA-008): it may insert a row
-- and set its end, but never delete a row or change who, what or when it started.
-- `sessions.host_keys` keeps the default privileges: accepting a key upserts it.
revoke delete, update on sessions.sessions from anima_runtime;
grant update (ended_at, end_reason) on sessions.sessions to anima_runtime;
