-- RDP and VNC passwords join the SSH secrets in credentials.secrets (SEC-003, DEV-011).
--
-- Each password is one sealed row of its own kind, bound to its device and kind by the AEAD
-- context `credentials.secrets:<device_id>:<kind>` (R-010). The unique (device_id, kind)
-- constraint keeps one row per kind; the device foreign key still cascades on delete.
--
-- Privileges: migration 0001's default privileges already cover credentials.secrets; no grant
-- changes here.

alter table credentials.secrets
    drop constraint secrets_kind_check,
    add constraint secrets_kind_check check (
        kind in (
            'ssh_password', 'ssh_private_key', 'ssh_passphrase', 'rdp_password', 'vnc_password'
        )
    );
