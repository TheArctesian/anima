-- RA-036, RA-040 (K2 0120, WP-47 S-14): how many sessions a device allows at once, and whether
-- its sessions are recorded. A null limit is no limit; WP-47 T2 enforces it and WP-48 acts on
-- the flag. Existing devices keep no limit and no recording. Both are plain columns of the
-- table the devices role already owns, so no grant changes.

ALTER TABLE devices.devices
    ADD COLUMN session_limit integer,
    ADD COLUMN record_sessions boolean NOT NULL DEFAULT false,
    ADD CONSTRAINT devices_session_limit_check CHECK (session_limit BETWEEN 1 AND 100);
