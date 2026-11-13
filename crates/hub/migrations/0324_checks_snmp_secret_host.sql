-- SNMP secrets are bound to the device host they were entered for (K2 range 0320-0349;
-- SEC-010, security review 2026-11-13).
--
-- A secret sealed from now on holds its host inside the sealed binding, so no column is needed
-- for it. A secret sealed before has no host in its binding, and SQL cannot open it to add
-- one. For such a row this column keeps the host its device had when this migration ran: the
-- assumption is that the secret was entered for that host. The hub reads the column only when
-- the binding names no host. A new configuration or the removal of the secrets clears it.
--
-- Privileges are unchanged: the new column is covered by the table grants of 0001.

alter table checks.checks add column snmp_legacy_host text null;

update checks.checks c set snmp_legacy_host = d.host
from devices.devices d
where d.id = c.device_id and c.type = 'snmp' and c.secrets_sealed is not null;

alter table checks.checks
    add constraint checks_snmp_legacy_host_check
        check (snmp_legacy_host is null or (type = 'snmp' and secrets_sealed is not null));
