-- Remote access settings of a device (RA-030..033, DB-011): every K1 default is a column default.

CREATE TABLE devices.rdp_settings (
    device_id uuid PRIMARY KEY,
    enabled boolean NOT NULL DEFAULT false,
    port integer NOT NULL DEFAULT 3389,
    username text NOT NULL DEFAULT '',
    domain text,
    security text NOT NULL DEFAULT 'nla',
    ignore_certificate boolean NOT NULL DEFAULT false,
    resize_method text NOT NULL DEFAULT 'display_update',
    audio_out boolean NOT NULL DEFAULT true,
    microphone boolean NOT NULL DEFAULT false,
    shared_drive boolean NOT NULL DEFAULT false,
    CONSTRAINT rdp_settings_device_id_fkey FOREIGN KEY (device_id)
        REFERENCES devices.devices (id) ON DELETE CASCADE,
    CONSTRAINT rdp_settings_port_check CHECK (port BETWEEN 1 AND 65535),
    CONSTRAINT rdp_settings_security_check
        CHECK (security IN ('any', 'nla', 'nla_ext', 'tls', 'vmconnect', 'rdp')),
    CONSTRAINT rdp_settings_resize_method_check
        CHECK (resize_method IN ('display_update', 'reconnect'))
);

CREATE TABLE devices.vnc_settings (
    device_id uuid PRIMARY KEY,
    enabled boolean NOT NULL DEFAULT false,
    port integer NOT NULL DEFAULT 5900,
    username text,
    CONSTRAINT vnc_settings_device_id_fkey FOREIGN KEY (device_id)
        REFERENCES devices.devices (id) ON DELETE CASCADE,
    CONSTRAINT vnc_settings_port_check CHECK (port BETWEEN 1 AND 65535)
);

INSERT INTO devices.rdp_settings (device_id) SELECT id FROM devices.devices;
INSERT INTO devices.vnc_settings (device_id) SELECT id FROM devices.devices;

-- A device always has one row in each table, so no reader needs an outer join and a
-- device inserted by any path starts with the defaults. A create that carries settings
-- updates these rows.
CREATE FUNCTION devices.add_remote_access_settings() RETURNS trigger
    LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO devices.rdp_settings (device_id) VALUES (NEW.id);
    INSERT INTO devices.vnc_settings (device_id) VALUES (NEW.id);
    RETURN NEW;
END
$$;

CREATE TRIGGER devices_add_remote_access_settings
    AFTER INSERT ON devices.devices
    FOR EACH ROW EXECUTE FUNCTION devices.add_remote_access_settings();

ALTER TABLE devices.devices
    ADD COLUMN mac_address macaddr,
    ADD COLUMN metrics_source text NOT NULL DEFAULT 'none',
    ADD COLUMN transfer_upload boolean NOT NULL DEFAULT true,
    ADD COLUMN transfer_download boolean NOT NULL DEFAULT true,
    ADD COLUMN transfer_copy boolean NOT NULL DEFAULT true,
    ADD COLUMN transfer_paste boolean NOT NULL DEFAULT true,
    ADD CONSTRAINT devices_metrics_source_check CHECK (metrics_source IN ('none', 'ssh'));

ALTER TABLE devices.sites
    ADD COLUMN wake_relay_device_id uuid,
    ADD CONSTRAINT sites_wake_relay_device_id_fkey FOREIGN KEY (wake_relay_device_id)
        REFERENCES devices.devices (id) ON DELETE SET NULL;
