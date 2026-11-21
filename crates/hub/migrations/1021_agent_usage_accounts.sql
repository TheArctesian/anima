-- The accounts of the AI agent programs on each device, as keys only (K2 1020-1039;
-- R-NEW-accountgroup, AIS-022; K8 Section 4 `agentUsage.accounts` and Section 8.2 `accountSalt`).
--
-- `salt` holds one row: the account salt of this hub, 32 random bytes made once. Each daemon of
-- this hub makes the key of an account with it, so one account on two devices of this hub has
-- one key, and a different hub has different keys.
--
-- `accounts` holds the newest account that each device reported for each source: its key (the
-- HMAC-SHA-256 of the vendor's identifier, never the identifier), the name of the plan, and the
-- time of the report. `account_limits` holds the limit windows of that report; a new report of
-- the device and source replaces them.
--
-- No column holds the identifier of an account, its name, an address or a token (AIS-022).
--
-- Deleting a device deletes its accounts and their limits (DEV-015).
--
-- Privileges (K2 "Roles"): migration 0001's default privileges give `anima_runtime` SELECT,
-- INSERT, UPDATE and DELETE on the new tables. The migration only creates empty tables, so it is
-- cheap on a large database (DB-010).

create table agent_usage.salt (
    one boolean not null default true,
    salt bytea not null,
    constraint salt_pkey primary key (one),
    constraint salt_one_check check (one),
    constraint salt_salt_check check (octet_length(salt) = 32)
);

create table agent_usage.accounts (
    device_id uuid not null,
    source text not null,
    key text not null,
    plan text,
    reported_at timestamptz not null,
    constraint accounts_pkey primary key (device_id, source),
    constraint accounts_device_id_fkey foreign key (device_id)
        references devices.devices (id) on delete cascade,
    -- Fixed identifiers: never change the meaning of one; add a new one instead.
    constraint accounts_source_check check (source in ('claude_code', 'codex')),
    constraint accounts_key_check check (key ~ '^[0-9a-f]{64}$'),
    constraint accounts_plan_check check (plan ~ '^[A-Za-z0-9 ._-]{1,32}$')
);

-- The page reads the devices of one key.
create index accounts_key_idx on agent_usage.accounts (key);

create table agent_usage.account_limits (
    device_id uuid not null,
    source text not null,
    window_name text not null,
    used_percent smallint,
    resets_at timestamptz,
    reached boolean not null,
    constraint account_limits_pkey primary key (device_id, source, window_name),
    constraint account_limits_account_fkey foreign key (device_id, source)
        references agent_usage.accounts (device_id, source) on delete cascade,
    constraint account_limits_window_check check (window_name ~ '^[a-z0-9_ ]{1,32}$'),
    constraint account_limits_used_check check (used_percent between 0 and 100)
);
