-- Operators, their API tokens and every sign-in attempt (K2, SEC-002, D-7, D-8).
-- access.permitted_networks comes in 0002 (WP-05 T4).

create schema access;

create table access.operators (
    id uuid primary key,
    username text not null unique,
    -- argon2id in PHC string form; never the password itself.
    password_hash text not null,
    -- Sealed with credentials::Vault, context `access.operators.totp:<id>` (R-010, WP-05 T6).
    totp_secret_enc bytea null,
    created_at timestamptz not null
);

create table access.api_tokens (
    id uuid primary key,
    operator_id uuid not null references access.operators (id) on delete cascade,
    -- SHA-256 of the 32 random token bytes; the token itself is never stored (D-7).
    token_hash bytea not null unique check (octet_length(token_hash) = 32),
    scope text not null check (scope in ('read', 'write')),
    expires_at timestamptz not null,
    created_at timestamptz not null,
    last_used_at timestamptz null
);

create index api_tokens_operator on access.api_tokens (operator_id);

-- One row per attempt. `refused` marks an attempt the throttle turned away before any
-- password check; the throttle counts only failures that were checked (D-8).
create table access.login_attempts (
    id uuid primary key,
    at timestamptz not null,
    username text not null,
    client_addr inet not null,
    ok boolean not null,
    refused boolean not null default false,
    check (not (ok and refused))
);

create index login_attempts_username_at on access.login_attempts (username, at)
    where not ok and not refused;
create index login_attempts_client_at on access.login_attempts (client_addr, at)
    where not ok and not refused;
-- Retention (90 days, K2) deletes by age.
create index login_attempts_at on access.login_attempts (at);

-- The group role the hub login role belongs to, as for events (0500). Roles are
-- cluster-wide, so another database (or a concurrent migration) may have created it.
do $$
begin
    if not exists (select from pg_catalog.pg_roles where rolname = 'anima_access_writer') then
        begin
            create role anima_access_writer nologin;
        exception
            when duplicate_object or unique_violation then
                null;
        end;
    end if;
end;
$$;

grant usage on schema access to anima_access_writer;
grant select, insert, update on access.operators to anima_access_writer;
grant select, insert, update, delete on access.api_tokens to anima_access_writer;
grant select, insert, update, delete on access.login_attempts to anima_access_writer;
