-- A reminder goes through the same outbox, dispatcher and channels as a device status
-- notification (REM-010; K-E6 Section 5). The kind `reminder` joins the two kind checks; a
-- reminder row has no check.
alter table notifications.outbox
    drop constraint outbox_kind_check,
    add constraint outbox_kind_check check (kind in (
        'down', 'up', 'still_down', 'certificate_expiry', 'domain_expiry', 'test', 'reminder'
    ));

alter table notifications.deliveries
    drop constraint deliveries_kind_check,
    add constraint deliveries_kind_check check (kind in (
        'down', 'up', 'still_down', 'certificate_expiry', 'domain_expiry', 'test', 'reminder'
    ));
