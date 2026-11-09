-- A domain expiry check does not retry (K2 range 0320-0349; spec 6.4, CHK-009, D-29, R-102).
--
-- It runs at most once an hour (0322) and RDAP servers are asked at most that often,
-- retries included. So its failures settle at once: `retries` is 0 for this type, and the
-- scheduler never schedules a retry for it. A row made before this rule is brought to 0.
update checks.checks set retries = 0 where type = 'domain_expiry';

alter table checks.checks
    add constraint checks_domain_expiry_retries_check
        check (type <> 'domain_expiry' or retries = 0);
