-- The Phase 2 check types (K2 range 0310-0399; CHK-009, WP-29 D-1).
--
-- One migration adds all four identifiers, so the tasks that build each type need no
-- migration of their own and cannot conflict on this constraint. A type the hub does not
-- read yet is refused by the code (the stored configuration is unreadable), not by the
-- database.
--
-- Identifiers are fixed (CHK-009): the seven of Phase 1 keep their meaning, and the new ones
-- are `docker`, `systemd`, `snmp` and `tailscale_ping`. Privileges are unchanged.
alter table checks.checks drop constraint checks_type_check;
alter table checks.checks
    add constraint checks_type_check check (type in (
        'icmp', 'tcp', 'tls', 'http', 'dns', 'push', 'group',
        'docker', 'systemd', 'snmp', 'tailscale_ping'
    ));
