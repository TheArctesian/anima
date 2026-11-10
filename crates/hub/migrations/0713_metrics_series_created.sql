-- When each series was created (K2 0710-0729; R-098 e, CR-20).
--
-- A device may create at most 1024 series in any hour. The count goes by this column, which the
-- hub sets from its own clock when it inserts the series, never by sample time, which a batch
-- chooses. A series made before this migration has '-infinity': it was not created in the last
-- hour. The default is a constant, so adding the column rewrites no table (DB-010).

alter table metrics.series add column created_at timestamptz not null default '-infinity';

create index series_device_id_created_at on metrics.series (device_id, created_at);
