-- Web interfaces of devices (K2 range 0800-0849; WEB-001, WEB-002, WEB-008).
--
-- Privileges (R-016, K2 "Roles"): anima_runtime gets SELECT, INSERT, UPDATE and DELETE on
-- these tables from the default privileges that migration 0001 sets for the owner role.

create schema web_interfaces;

-- One row per web address the operator keeps for a device. The url is stored as typed
-- (trimmed); the hub's rules refuse user info and fragments, and the checks below keep the
-- scheme and the size even if a writer skips them (WEB-001).
create table web_interfaces.interfaces (
    id uuid primary key,
    device_id uuid not null,
    label text not null,
    url text not null,
    open_in text not null,
    created_at timestamptz not null,
    constraint interfaces_device_id_fkey foreign key (device_id)
        references devices.devices (id) on delete cascade,
    constraint interfaces_label_check check (char_length(label) between 1 and 100),
    constraint interfaces_url_check check (
        octet_length(url) <= 2048 and url ~* '^https?://[^[:space:][:cntrl:]#]+$'
    ),
    constraint interfaces_open_in_check check (open_in in ('anima_tab', 'system_browser'))
);

create index interfaces_device_id_idx on web_interfaces.interfaces (device_id, created_at, id);

-- The certificate the operator accepted for an https interface (WEB-008): the SHA-256 of the
-- leaf certificate as 64 lower-case hex digits. A change of origin deletes the row.
create table web_interfaces.pins (
    interface_id uuid primary key,
    fingerprint_sha256 text not null,
    accepted_by uuid not null,
    accepted_at timestamptz not null,
    constraint pins_interface_id_fkey foreign key (interface_id)
        references web_interfaces.interfaces (id) on delete cascade,
    constraint pins_accepted_by_fkey foreign key (accepted_by)
        references access.operators (id),
    constraint pins_fingerprint_check check (fingerprint_sha256 ~ '^[0-9a-f]{64}$')
);

-- HTTP checks whose suggestion the operator dismissed (WEB-002). Deleting the check
-- forgets the dismissal.
create table web_interfaces.dismissed (
    check_id uuid primary key,
    dismissed_at timestamptz not null,
    constraint dismissed_check_id_fkey foreign key (check_id)
        references checks.checks (id) on delete cascade
);
