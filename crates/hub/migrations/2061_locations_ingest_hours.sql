-- The count of the fixes that each ingest token stored in each hour (R-208; K-E4 Section 13).
-- A stored fix writes no event and no audit entry of its own: when an hour has ended, the hub
-- writes one audit entry for the token and the hour with this count, and deletes the row.
-- `client` is the address of the last post of the hour, for that entry.
create table locations.ingest_hour (
    token_id uuid        not null references locations.ingest_token (id) on delete cascade,
    hour     timestamptz not null,
    fixes    integer     not null check (fixes > 0),
    client   inet        not null,
    primary key (token_id, hour)
);
