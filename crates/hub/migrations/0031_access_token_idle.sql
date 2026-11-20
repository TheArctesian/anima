-- A browser session of a hosted hub ends after a time with no request (hosted-mode.md HMD-062):
-- the token carries that time, so the rule travels with the token and no other token changes.
alter table access.api_tokens
    add column idle_seconds integer null check (idle_seconds > 0);

comment on column access.api_tokens.idle_seconds is
    'When set, the token is refused once this many seconds have passed since last_used_at (HMD-062).';
