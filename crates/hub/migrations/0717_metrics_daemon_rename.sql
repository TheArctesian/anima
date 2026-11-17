-- The program `anima-agent` is `anima-daemon` (K2 0710-0729; R-126). "Agent" now names an AI
-- agent (the actor kind of 0505, the `made_for` of 0011); those two keep their values.
--
-- This migration renames the table of the program's tokens, its constraints, the version
-- column and the metrics source, and it deletes each token and each enrolment code. R-126
-- makes the old tokens not valid. A row holds the SHA-256 of the 32 bytes of a token, not of
-- its prefix, so a kept row would accept an old token that is spelled again with the new
-- prefix. With no row, the state of a device is the state after a revoke: it stays on the
-- source, shown as stale, until the operator enrols it again.

alter table metrics.agents rename to daemons;
alter table metrics.daemons rename column agent_version to daemon_version;

alter table metrics.daemons rename constraint agents_pkey to daemons_pkey;
alter table metrics.daemons rename constraint agents_device_id_fkey to daemons_device_id_fkey;
alter table metrics.daemons rename constraint agents_token_sha256_key to daemons_token_sha256_key;
alter table metrics.daemons rename constraint agents_next_token_sha256_key to daemons_next_token_sha256_key;
alter table metrics.daemons rename constraint agents_token_sha256_check to daemons_token_sha256_check;
alter table metrics.daemons rename constraint agents_next_token_sha256_check to daemons_next_token_sha256_check;
alter table metrics.daemons rename constraint agents_next_check to daemons_next_check;
-- 0716 did not name its check, so PostgreSQL named it for the table and the column of then.
alter table metrics.daemons rename constraint agents_agent_version_check to daemons_daemon_version_check;

delete from metrics.daemons;
delete from metrics.enrolments;

-- The drop and add hold the devices table's lock for the length of the migration, as in 0710.
alter table devices.devices drop constraint devices_metrics_source_check;

update devices.devices set metrics_source = 'daemon' where metrics_source = 'agent';

alter table devices.devices
    add constraint devices_metrics_source_check
    check (metrics_source in ('none', 'ssh', 'daemon'));
