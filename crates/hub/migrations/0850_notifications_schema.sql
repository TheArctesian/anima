-- Notifications: channels, per-check choices, the decision state, the outbox and the delivery
-- log (K2, NOT-001 to NOT-007, D-16, D-17, D-21, D-22, D-25).
--
-- Every default lives here, not only in the code (DB-011). Secrets are sealed by the hub
-- (context `notifications.channels:<channel_id>`) before they reach `channels.secrets`; the
-- database only sees bytes. `outbox.payload` holds rendered placeholder values and never a
-- secret. `deliveries` is partitioned by UTC day (DB-004) with no foreign key, as the logs:
-- a delivery keeps the channel's name after the channel is deleted. Its partition functions
-- are in 0851. Outbox rows are deleted by the hub 7 days after they were sent or given up.
--
-- `outbox.check_id` has no foreign key: a row for a deleted check is still sent (it was
-- decided while the check existed), and the table is short-lived.
--
-- Privileges (K2 "Roles"): migration 0001's default privileges give `anima_runtime` SELECT,
-- INSERT, UPDATE and DELETE on every table created here. The runtime role owns nothing.

create schema notifications;

create table notifications.channels (
    id uuid not null,
    name text not null,
    kind text not null,
    on_by_default boolean not null default false,
    -- The non-secret fields of the kind, typed in Rust.
    settings jsonb not null default '{}',
    -- Sealed JSON of the kind's secret fields; null when the kind has none stored.
    secrets bytea null,
    title_template text null,
    body_template text null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint channels_pkey primary key (id),
    constraint channels_name_key unique (name),
    constraint channels_name_check check (char_length(name) between 1 and 100),
    constraint channels_kind_check check (kind in ('webhook', 'ntfy', 'email', 'apprise')),
    constraint channels_settings_check check (jsonb_typeof(settings) = 'object'),
    constraint channels_title_template_check check (char_length(title_template) <= 500),
    constraint channels_body_template_check check (char_length(body_template) <= 4000)
);

-- The channels a check notifies (NOT-003).
create table notifications.check_channels (
    check_id uuid not null,
    channel_id uuid not null,
    constraint check_channels_pkey primary key (check_id, channel_id),
    constraint check_channels_check_id_fkey foreign key (check_id)
        references checks.checks (id) on delete cascade,
    constraint check_channels_channel_id_fkey foreign key (channel_id)
        references notifications.channels (id) on delete cascade
);

create index check_channels_channel_id on notifications.check_channels (channel_id);

-- How often a check that stays down is announced again, in Down beats; 0 means never (NOT-005).
create table notifications.check_settings (
    check_id uuid not null,
    resend_every_beats integer not null default 0,
    constraint check_settings_pkey primary key (check_id),
    constraint check_settings_check_id_fkey foreign key (check_id)
        references checks.checks (id) on delete cascade,
    constraint check_settings_resend_check check (resend_every_beats between 0 and 1000)
);

-- What the decision core remembers about a check between beats (D-16, D-22).
create table notifications.check_state (
    check_id uuid not null,
    -- The status last announced; null before the first announcement.
    notified_status text null,
    -- Down beats since the last Down or Still-down notification.
    down_beats integer not null default 0,
    -- The expiry thresholds already sent, each with the `notAfter` it was sent for.
    expiry_marks jsonb not null default '[]',
    constraint check_state_pkey primary key (check_id),
    constraint check_state_check_id_fkey foreign key (check_id)
        references checks.checks (id) on delete cascade,
    constraint check_state_notified_status_check check (
        notified_status in ('up', 'down', 'degraded', 'maintenance', 'unknown')
    ),
    constraint check_state_down_beats_check check (down_beats >= 0),
    constraint check_state_expiry_marks_check check (jsonb_typeof(expiry_marks) = 'array')
);

-- Notifications decided and not yet finished, one row per channel (D-16). A row is finished
-- when it is sent or given up, never both.
create table notifications.outbox (
    id uuid not null,
    channel_id uuid not null,
    check_id uuid null,
    kind text not null,
    payload jsonb not null,
    created_at timestamptz not null default now(),
    attempts integer not null default 0,
    next_attempt_at timestamptz not null default now(),
    sent_at timestamptz null,
    given_up_at timestamptz null,
    constraint outbox_pkey primary key (id),
    constraint outbox_channel_id_fkey foreign key (channel_id)
        references notifications.channels (id) on delete cascade,
    constraint outbox_kind_check check (kind in (
        'down', 'up', 'still_down', 'certificate_expiry', 'domain_expiry', 'test'
    )),
    constraint outbox_payload_check check (jsonb_typeof(payload) = 'object'),
    constraint outbox_attempts_check check (attempts >= 0),
    constraint outbox_finished_once_check check (sent_at is null or given_up_at is null)
);

create index outbox_channel_id on notifications.outbox (channel_id);

-- The dispatcher's claim: due rows that are not finished.
create index outbox_due on notifications.outbox (next_attempt_at)
    where sent_at is null and given_up_at is null;

-- The clean-up: rows finished before a cutoff.
create index outbox_finished on notifications.outbox ((coalesce(sent_at, given_up_at)))
    where sent_at is not null or given_up_at is not null;

-- What was sent, or failed to send, to whom. No foreign key (DB-004): the name is a copy.
-- `reason` is a fixed word and `code` an HTTP status or SMTP reply code: never the target's
-- reply text (CR-14, LOG-007).
create table notifications.deliveries (
    id uuid not null,
    channel_id uuid null,
    channel_name text not null,
    kind text not null,
    at timestamptz not null,
    ok boolean not null,
    reason text null,
    code int null,
    constraint deliveries_pkey primary key (at, id),
    constraint deliveries_kind_check check (kind in (
        'down', 'up', 'still_down', 'certificate_expiry', 'domain_expiry', 'test'
    )),
    constraint deliveries_reason_check check (reason in (
        'target_not_permitted', 'bad_address', 'secret_unreadable', 'setup_failed',
        'connection_failed', 'tls_failed', 'timed_out', 'http_status', 'no_starttls',
        'smtp_refused', 'not_understood'
    )),
    constraint deliveries_outcome_check check (ok = (reason is null))
) partition by range (at);

-- The last delivery of each channel.
create index deliveries_channel_at on notifications.deliveries (channel_id, at desc, id desc)
    where channel_id is not null;

-- The installation's settings: one row (D-21, D-22). `id` can only be true, so a second row
-- cannot exist. The thresholds are days before expiry, at most 5, each 1 to 365; the hub
-- keeps them in descending order.
create table notifications.settings (
    id boolean not null default true,
    certificate_days integer[] not null default '{21,14,7}',
    title_template text not null default '{device}: {check} is {status}',
    body_template text not null default
        '{check} on {device} was {previous} and is now {status} at {at}. {message}',
    constraint settings_pkey primary key (id),
    constraint settings_single_row_check check (id),
    constraint settings_certificate_days_check check (
        array_ndims(certificate_days) = 1
        and cardinality(certificate_days) between 1 and 5
        and array_position(certificate_days, null) is null
        and 1 <= all (certificate_days)
        and 365 >= all (certificate_days)
    ),
    constraint settings_title_template_check check (char_length(title_template) <= 500),
    constraint settings_body_template_check check (char_length(body_template) <= 4000)
);

insert into notifications.settings (id) values (true);
