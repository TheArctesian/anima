-- The domain expiry check (K2 range 0320-0349; spec 6.4, CHK-009, D-29).
--
-- Adds the fixed identifier `domain_expiry` (CHK-009: the eleven earlier ids keep their
-- meaning) and `domain_not_after`, the registration expiry the last RDAP reply gave, beside
-- `cert_not_after`. Only a domain expiry check holds one. A domain expiry check runs at most
-- once an hour (D-29), so RDAP servers are not asked more often whatever the client sends.
-- Privileges are unchanged: the new column is covered by the table grants of 0001.
alter table checks.checks drop constraint checks_type_check;
alter table checks.checks
    add constraint checks_type_check check (type in (
        'icmp', 'tcp', 'tls', 'http', 'dns', 'push', 'group',
        'docker', 'systemd', 'snmp', 'tailscale_ping',
        'domain_expiry'
    ));

alter table checks.checks add column domain_not_after timestamptz null;

alter table checks.checks
    add constraint checks_domain_not_after_check
        check (type = 'domain_expiry' or domain_not_after is null);

alter table checks.checks
    add constraint checks_domain_expiry_interval_check
        check (type <> 'domain_expiry' or interval_seconds >= 3600);
