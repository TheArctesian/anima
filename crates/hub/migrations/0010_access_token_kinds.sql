-- Token kinds (K2 0010-0029, WP-43, D-3, D-23). Every K2 column and check of the range is here,
-- so the API key task adds no migration (A-5).
--
-- A `login` token signs the desktop app in (30 days); a `browser` token sits only in the
-- `__Host-anima` cookie (7 days); an `api_key` is named, has its own expiry, and is presented
-- as `anima_` plus the token text. All three keep only the SHA-256 of their 32 random bytes.
-- Existing rows are login tokens.

alter table access.api_tokens
    add column kind text not null default 'login'
        check (kind in ('login', 'browser', 'api_key')),
    -- The operator's name for an API key, unique among keys; none for the other kinds.
    add column name text null,
    -- The first 12 characters of an API key's secret (`anima_` and six more), so the operator
    -- can tell keys apart; never enough to use it. None for the other kinds.
    add column display_prefix text null;

alter table access.api_tokens
    add constraint api_tokens_name_for_keys check (
        case
            when kind = 'api_key' then name is not null and char_length(name) between 1 and 64
            else name is null
        end
    ),
    add constraint api_tokens_display_prefix_for_keys check (
        case
            when kind = 'api_key' then display_prefix is not null
                and char_length(display_prefix) = 12
            else display_prefix is null
        end
    ),
    -- Only an API key may be read-only: a signed-in operator may do everything.
    add constraint api_tokens_signed_in_write check (kind = 'api_key' or scope = 'write');

create unique index api_tokens_key_name on access.api_tokens (name) where kind = 'api_key';
create index api_tokens_kind_operator on access.api_tokens (kind, operator_id);

-- No grants here: anima_runtime gets its table privileges from the default privileges that
-- migration 0001 sets for the owner role (R-016, K2 "Roles").
