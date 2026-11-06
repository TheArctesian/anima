-- RDP and VNC sessions and the accepted RDP server certificate (K2 range 0610-0649; RA-020,
-- RA-030, SEC-007 for RDP, WP-22 D-5).

-- A session's protocol may now be RDP or VNC as well as SSH (K1 `Protocol`).
alter table sessions.sessions
    drop constraint sessions_protocol_check,
    add constraint sessions_protocol_check check (protocol in ('ssh', 'rdp', 'vnc'));

-- K1 SessionEndReason in lower_snake case, with the RDP certificate mismatch (K4 Section 5).
alter table sessions.sessions
    drop constraint sessions_end_reason_check,
    add constraint sessions_end_reason_check check (end_reason in (
        'operator', 'remote_closed', 'gateway_error', 'auth_failed', 'host_key_mismatch',
        'protocol_error', 'timeout', 'hub_shutdown', 'server_certificate_mismatch'
    ));

-- The one accepted RDP server certificate per device (D-5). The hub gives guacd its
-- fingerprint as `cert-fingerprints`, so a changed certificate stops the session (SEC-007).
-- `fingerprint_sha256` is the SHA-256 of the leaf certificate's DER as FreeRDP writes it:
-- 32 lower-case hex pairs joined by colons. `subject` and `not_after` are what the device's
-- certificate says, for the operator to compare; neither is trusted.
create table sessions.server_certificates (
    device_id uuid primary key,
    fingerprint_sha256 text not null,
    subject text null,
    not_after timestamptz null,
    first_seen_at timestamptz not null,
    accepted_by uuid not null,
    constraint server_certificates_device_id_fkey foreign key (device_id)
        references devices.devices (id) on delete cascade,
    constraint server_certificates_accepted_by_fkey foreign key (accepted_by)
        references access.operators (id),
    constraint server_certificates_fingerprint_check check (
        fingerprint_sha256 ~ '^([0-9a-f]{2}:){31}[0-9a-f]{2}$'
    ),
    -- The device chooses its subject: the hub keeps at most 1024 characters of it and no
    -- control character, so it cannot break a line wherever it is shown.
    constraint server_certificates_subject_check check (
        subject is null
        or (char_length(subject) between 1 and 1024 and subject !~ '[[:cntrl:]]')
    ),
    -- Real instants only, as `sessions_times_finite` for session rows.
    constraint server_certificates_times_finite check (
        isfinite(first_seen_at) and (not_after is null or isfinite(not_after))
    )
);

-- As for host keys (0601): no hub code deletes a certificate; deleting a device removes its
-- certificate through `server_certificates_device_id_fkey`'s cascade, which runs with the
-- table owner's rights. Accepting a certificate upserts it and never moves it to another
-- device, so the runtime role may update every column but `device_id`
-- (`sessions::store::put_server_certificate`).
revoke delete, update on sessions.server_certificates from anima_runtime;
grant update (fingerprint_sha256, subject, not_after, first_seen_at, accepted_by)
    on sessions.server_certificates to anima_runtime;
