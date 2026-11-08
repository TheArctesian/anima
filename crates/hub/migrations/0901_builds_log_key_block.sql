-- A private key block in a builder's log may run across reports (BLD-049, LOG-007): the hub
-- drops its lines while they are key material, up to the closing line or a line limit, so it
-- keeps, per job, how many more lines the block left open by the last report may take. Null
-- when none is open. A claim clears it, so an attempt never inherits an open block. K2 lets a
-- package add columns to its own tables.

alter table builds.jobs
    add column log_key_lines_left integer null,
    add constraint jobs_log_key_lines_left_check check (log_key_lines_left between 0 and 200);
