-- The program `anima-agent` is `anima-daemon` (K2 0710-0729; R-126). "Agent" now names an AI
-- agent (the actor kind of 0505, the `made_for` of 0011); those two keep their values.
--
-- This migration renames the table of the program's tokens, its constraints, the version
-- column and the metrics source. It keeps each row: a token row is of no use after R-126,
-- because a token with the old prefix is not a credential, and the next enrolment of the
-- device replaces it. The device stays on the source, shown as stale, until then.

alter table metrics.agents rename to daemons;
alter table metrics.daemons rename column agent_version to daemon_version;

alter table metrics.daemons rename constraint agents_pkey to daemons_pkey;
alter table metrics.daemons rename constraint agents_device_id_fkey to daemons_device_id_fkey;
alter table metrics.daemons rename constraint agents_token_sha256_key to daemons_token_sha256_key;
alter table metrics.daemons rename constraint agents_next_token_sha256_key to daemons_next_token_sha256_key;
alter table metrics.daemons rename constraint agents_token_sha256_check to daemons_token_sha256_check;
alter table metrics.daemons rename constraint agents_next_token_sha256_check to daemons_next_token_sha256_check;
alter table metrics.daemons rename constraint agents_next_check to daemons_next_check;

-- The drop and add hold the devices table's lock for the length of the migration, as in 0710.
alter table devices.devices drop constraint devices_metrics_source_check;

update devices.devices set metrics_source = 'daemon' where metrics_source = 'agent';

alter table devices.devices
    add constraint devices_metrics_source_check
    check (metrics_source in ('none', 'ssh', 'daemon'));
