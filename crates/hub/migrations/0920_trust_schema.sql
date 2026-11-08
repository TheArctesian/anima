-- The trust domain's tables (K2 range 0920-0939; D-31, Q2-05; SEC-006, SEC-007).
--
-- The accepted SSH host keys and RDP server certificates move from schema `sessions` to schema
-- `trust`, which owns them now. Metadata only (DB-010): `set schema` moves each table with its
-- rows, columns, indexes, constraints (their names unchanged) and privileges, so the runtime
-- role keeps exactly what migrations 0601 and 0610 left it: SELECT, INSERT, and UPDATE on every
-- column but `device_id`; no DELETE and no table-wide UPDATE (R-040).
--
-- Privileges (K2 "Roles"): migration 0001's default privileges give `anima_runtime` USAGE on
-- the new schema.

create schema trust;

alter table sessions.host_keys set schema trust;
alter table sessions.server_certificates set schema trust;
