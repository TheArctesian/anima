-- The vpn domain's tables (K2 range 1000-1019; VPN-010, VPN-011, BD-4).
--
-- One row for each warning event the hub has written, so each is written once, also after a
-- restart. `node_key` is the node's public WireGuard key (not a secret). A renewed key has a new
-- expiry, so its warnings start again. Privileges: migration 0001's default privileges give
-- `anima_runtime` what it needs on the new schema (K2 "Roles").
create schema vpn;

create table vpn.warnings (
    node_key   text        not null check (node_key like 'nodekey:%' and length(node_key) <= 80),
    kind       text        not null check (kind in ('expiry_30', 'expiry_14', 'expiry_7', 'expiry_1',
                                                     'tagged_expiry', 'not_in_anima')),
    key_expiry timestamptz,
    written_at timestamptz not null,
    constraint warnings_once unique nulls not distinct (node_key, kind, key_expiry)
);
