-- Corrects 0717 (K2 0710-0729; R-126). The comment of 0717 says that it keeps each row; 0717
-- is not changed, because its checksum is already in the database of a hub that ran it. This
-- migration deletes each token and each enrolment code, and renames the check that 0717 left.
--
-- R-126 makes the old tokens not valid. A row holds the SHA-256 of the 32 bytes of a token,
-- not of its prefix, so a kept row would accept an old token that is spelled again with the
-- new prefix. After the delete, the state of a device is the state after a revoke: it stays on
-- the source `daemon`, shown as stale, until the operator enrols it again.
--
-- The hub writes no "stale" event for these devices, because the row that would carry the
-- mark is gone. The operator gets no notification.

-- 0716 did not name its check, so PostgreSQL named it for the table and the column of then.
alter table metrics.daemons rename constraint agents_agent_version_check to daemons_daemon_version_check;

delete from metrics.daemons;
delete from metrics.enrolments;
