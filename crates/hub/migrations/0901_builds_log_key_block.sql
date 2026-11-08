-- A private key block in a builder's log may run across reports (BLD-049, LOG-007): the hub
-- drops its lines up to the closing line, so it keeps, per job, whether the last report ended
-- inside one. A claim clears it, so an attempt never inherits an open block. K2 lets a package
-- add columns to its own tables.

alter table builds.jobs add column log_in_key_block boolean not null default false;
