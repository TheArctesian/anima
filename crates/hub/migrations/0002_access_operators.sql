-- Operators, their API tokens and every sign-in attempt (K2, SEC-002, D-7, D-8).
-- 0001 is the runtime role (R-016); access.permitted_networks comes in 0003 (WP-05 T4).

create schema access;

create table access.operators (
    id uuid primary key,
    username text not null unique check (username ~ '^[A-Za-z0-9._-]{1,64}$'),
    -- argon2id in PHC string form; never the password itself.
    password_hash text not null,
    -- Sealed with credentials::Vault, context `access.operators.totp:<id>` (R-010, WP-05 T6).
    totp_secret_enc bytea null,
    created_at timestamptz not null
);

-- One operator account (OD-2, D-9): a second row fails on this index.
create unique index operators_single on access.operators ((true));

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

-- One row per attempt. The throttle counts failures per client address only (D-8);
-- `username` is kept for the record and for `operator set`, which clears that username's
-- attempts (D-9). `refused` marks an attempt the throttle turned away before any password
-- check; the throttle counts only failures that were checked.
create table access.login_attempts (
    id uuid primary key,
    at timestamptz not null,
    username text not null check (char_length(username) <= 128),
    client_addr inet not null,
    ok boolean not null,
    refused boolean not null default false,
    check (not (ok and refused))
);

create index login_attempts_client_at on access.login_attempts (client_addr, at)
    where not ok and not refused;
-- Retention (90 days, K2) deletes by age; expired tokens are deleted too.
create index api_tokens_expires_at on access.api_tokens (expires_at);
create index login_attempts_at on access.login_attempts (at);
create index login_attempts_username on access.login_attempts (username);

-- No grants here: anima_runtime gets its table privileges from the default privileges that
-- migration 0001 sets for the owner role (R-016, K2 "Roles").
