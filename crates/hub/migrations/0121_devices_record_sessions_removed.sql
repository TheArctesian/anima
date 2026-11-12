-- R-113 (RA-040 deferred, RA-008): sessions are not recorded, so a device has no recording
-- flag. Drops the column 0120 added; `session_limit` (RA-036) stays. Migrations are never
-- edited once applied, so the drop is a new migration (K2).

ALTER TABLE devices.devices DROP COLUMN record_sessions;
