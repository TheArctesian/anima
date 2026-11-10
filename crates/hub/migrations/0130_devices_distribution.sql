-- The Linux distribution of a device: the `ID` of its /etc/os-release, read over SSH with the
-- metrics collection (DEV-016). Null until read. Only a short lower-case token is stored, so
-- text from the device can never be more than a name of the distribution.

ALTER TABLE devices.devices
    ADD COLUMN distribution text,
    ADD CONSTRAINT devices_distribution_check
        CHECK (distribution ~ '^[a-z0-9._-]{1,32}$');
