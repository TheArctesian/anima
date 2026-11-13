-- The sealed credential of a source account (SRC-008, PRJ-018, SEC-003; K2 range 0221).
--
-- One row per source account: only nonce, ciphertext and key id are stored, never plaintext.
-- The value is sealed under the AEAD context `credentials.source_secrets:<account_id>`, so a
-- row whose account id is changed, or whose sealed columns are copied to another account's
-- row, does not open. `account_id` has no foreign key: `sources.accounts` lives in a higher
-- range and points here through its `credential_id`, never the reverse (K-E8). Storing again
-- for an account replaces the sealed columns and keeps the row's id.
--
-- created_at defaults to now(): the store takes no clock (K3). Privileges come from the
-- default grants of 0001.

create table credentials.source_secrets (
    id uuid primary key,
    account_id uuid not null,
    nonce bytea not null,
    ciphertext bytea not null,
    key_id text not null,
    created_at timestamptz not null default now(),
    constraint source_secrets_account_id_key unique (account_id)
);
