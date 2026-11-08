-- Builder tokens (WP-36 D-1, BLD-022): `access.api_tokens.scope` accepts 'builder'. 0002
-- created the check inline; Postgres named it `api_tokens_scope_check`. A builder token is an
-- access token like any other: only its SHA-256 is stored (BLD-023).

alter table access.api_tokens drop constraint api_tokens_scope_check;

alter table access.api_tokens
    add constraint api_tokens_scope_check check (scope in ('read', 'write', 'builder'));
