-- What an API key is made for (LOG-015; operator, 2026-11-13, LOG-012 to LOG-020; ruling number
-- to be assigned): `user` for a script that the operator runs, `agent` for an AI agent. The hub
-- records each request with an agent key as Agent (LOG-014, LOG-017). Every API key has it and no
-- other kind of token does. A key made before this migration is `user`. The value never changes
-- after the key is made: a trigger refuses an update of it, also from the owner's own code.

alter table access.api_tokens
    add column made_for text null check (made_for in ('user', 'agent'));

update access.api_tokens set made_for = 'user' where kind = 'api_key';

alter table access.api_tokens
    add constraint api_tokens_made_for_keys check ((kind = 'api_key') = (made_for is not null));

-- Refuses a change of `made_for` (LOG-015). Not SECURITY DEFINER: it reads only the row. The
-- search path is fixed so a caller's temporary schema cannot change what it calls.
create function access.keep_made_for()
returns trigger
language plpgsql
set search_path = pg_catalog, pg_temp
as $$
begin
    if new.made_for is distinct from old.made_for then
        raise exception 'access.api_tokens: made_for cannot change'
            using errcode = 'check_violation';
    end if;
    return new;
end;
$$;

create trigger api_tokens_keep_made_for
    before update of made_for on access.api_tokens
    for each row execute function access.keep_made_for();

-- No grants here: anima_runtime gets its table privileges from the default privileges that
-- migration 0001 sets for the owner role (R-016, K2 "Roles").
