-- The VPN event source (VPN-010, VPN-011, VP-2). Numbered after 0852, which re-adds this check
-- with seven sources; a lower number would let 0852 drop 'vpn' again (K2, BD-1).
alter table events.event_log drop constraint event_log_source_check;

alter table events.event_log
    add constraint event_log_source_check
    check (source in ('check', 'session', 'device', 'access', 'hub', 'metrics', 'notification', 'vpn'));
