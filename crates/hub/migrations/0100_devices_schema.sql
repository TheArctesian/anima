CREATE SCHEMA devices;

CREATE TABLE devices.sites (
    id uuid PRIMARY KEY,
    name text NOT NULL,
    subnet cidr NOT NULL,
    CONSTRAINT sites_name_key UNIQUE (name),
    CONSTRAINT sites_subnet_key UNIQUE (subnet),
    CONSTRAINT sites_subnet_no_overlap EXCLUDE USING gist (subnet inet_ops WITH &&)
);

CREATE TABLE devices.devices (
    id uuid PRIMARY KEY,
    site_id uuid NOT NULL,
    name text NOT NULL,
    host text NOT NULL,
    ip inet,
    kind text NOT NULL,
    os text NOT NULL,
    maintenance boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL,
    CONSTRAINT devices_site_id_fkey FOREIGN KEY (site_id) REFERENCES devices.sites (id),
    CONSTRAINT devices_name_key UNIQUE (name),
    CONSTRAINT devices_os_check CHECK (os IN ('linux', 'macos', 'windows', 'other'))
);

CREATE INDEX devices_site_id_idx ON devices.devices (site_id);

CREATE TABLE devices.device_tags (
    device_id uuid NOT NULL,
    tag text NOT NULL,
    CONSTRAINT device_tags_device_id_fkey FOREIGN KEY (device_id)
        REFERENCES devices.devices (id) ON DELETE CASCADE,
    PRIMARY KEY (device_id, tag)
);

CREATE INDEX device_tags_tag_idx ON devices.device_tags (tag);

CREATE TABLE devices.ssh_settings (
    device_id uuid PRIMARY KEY,
    enabled boolean NOT NULL,
    port integer NOT NULL DEFAULT 22,
    username text NOT NULL,
    auth text NOT NULL,
    CONSTRAINT ssh_settings_device_id_fkey FOREIGN KEY (device_id)
        REFERENCES devices.devices (id) ON DELETE CASCADE,
    CONSTRAINT ssh_settings_port_check CHECK (port BETWEEN 1 AND 65535),
    CONSTRAINT ssh_settings_auth_check CHECK (auth IN ('password', 'private_key'))
);
