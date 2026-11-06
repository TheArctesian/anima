-- The last TOTP time step that signed the operator in (WP-05 T6, SEC-002, RFC 6238 5.2).
-- A code is accepted only for a later step, so each code works once. Enrolling or disabling
-- TOTP clears it. K2 lets a package add columns to its own tables.
alter table access.operators add column totp_last_step bigint null;

alter table access.operators add constraint operators_totp_step_needs_secret
    check (totp_last_step is null or totp_secret_enc is not null);
