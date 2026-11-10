-- Builder tokens (WP-36 D-1, BLD-022, R-107). A builder token is its own kind of access token
-- with its own scope, `builder`: it travels as a bearer like a login token, stands for no
-- operator and may only claim and report build jobs. Only its SHA-256 is stored (BLD-023).
-- Numbered after 0010, which added `kind`; 0002 and 0010 created the scope and kind checks
-- inline, so Postgres named them `api_tokens_scope_check` and `api_tokens_kind_check`.

alter table access.api_tokens
    drop constraint api_tokens_scope_check,
    drop constraint api_tokens_kind_check,
    drop constraint api_tokens_signed_in_write;

alter table access.api_tokens
    add constraint api_tokens_scope_check check (scope in ('read', 'write', 'builder')),
    add constraint api_tokens_kind_check
        check (kind in ('login', 'browser', 'api_key', 'builder')),
    -- Only an API key may be read-only, and only a builder token has the builder scope.
    add constraint api_tokens_signed_in_write
        check (kind in ('api_key', 'builder') or scope = 'write'),
    add constraint api_tokens_builder_scope check ((kind = 'builder') = (scope = 'builder'));
