-- Who did a recorded action (LOG-012 to LOG-018; operator, 2026-11-13, LOG-012 to LOG-020;
-- ruling number to be assigned): `user`, `agent` or `hub`; for a request, the kind of token it
-- signed in with and, for an API key, the key's name. Never the token or a part of it (LOG-007).
--
-- Each column is null for a row written before this migration. Adding a nullable column with no
-- default changes no stored row (LOG-006): the hub does not give an old entry a kind. The
-- columns and checks reach every existing and future day partition through the parents.
-- Privileges do not change: the runtime role still only reads and appends (0500, R-016).

alter table events.audit_log
    add column actor_kind text null check (actor_kind in ('user', 'agent', 'hub')),
    add column token_kind text null check (token_kind in ('login', 'browser', 'api_key')),
    add column api_key_name text null check (char_length(api_key_name) between 1 and 64),
    -- A Hub entry has no token kind and no key name; only an API key entry names its key.
    add constraint audit_log_hub_has_no_token
        check (actor_kind is distinct from 'hub' or token_kind is null),
    add constraint audit_log_key_name_for_keys
        check (api_key_name is null or token_kind = 'api_key');

alter table events.event_log
    add column actor_kind text null check (actor_kind in ('user', 'agent', 'hub'));
