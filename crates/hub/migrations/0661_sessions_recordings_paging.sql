-- Paging of recordings, newest first, of every device or of one (K1 `recordings`, API-006,
-- RA-041). The same keyset indexes `sessions.sessions` has (migration 0600): `Query.recordings`
-- orders by `(started_at desc, id desc)` and filters on `device_id`.

create index recordings_started_at_id_idx on sessions.recordings (started_at desc, id desc);
create index recordings_device_started_at_idx
    on sessions.recordings (device_id, started_at desc, id desc);
