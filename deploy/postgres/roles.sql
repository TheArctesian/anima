-- Roles and database for a new Anima installation (DB-003, R-016, contracts/database.md "Roles").
--
-- Run once, as the PostgreSQL superuser, against the `postgres` database:
--   psql -v ON_ERROR_STOP=1 --dbname postgres -f roles.sql
-- with ANIMA_OWNER_PASSWORD_FILE and ANIMA_HUB_PASSWORD_FILE naming files that hold the passwords.
--
-- The superuser makes `anima_runtime` itself, so the owner role needs no CREATEROLE.
-- Migration 0001 accepts the group because it is a plain NOLOGIN role.

\set ON_ERROR_STOP on
\set VERBOSITY terse

-- Passwords come from files, never from argv. A failing statement must not copy its text (and
-- so a password) into the server log, and no statement is logged either.
set log_min_error_statement = panic;
set log_statement = none;
\set owner_password `cat "$ANIMA_OWNER_PASSWORD_FILE"`
\set hub_password `cat "$ANIMA_HUB_PASSWORD_FILE"`

create role anima_runtime nologin;

create role anima_owner login nosuperuser nocreatedb nocreaterole nobypassrls noreplication
    password :'owner_password';

create role anima_hub login nosuperuser nocreatedb nocreaterole nobypassrls noreplication
    in role anima_runtime
    password :'hub_password';

create database anima owner anima_owner;

-- Nobody connects to a database unless granted: not through PUBLIC, and the hub's group only
-- reaches `anima`. The superuser keeps access to everything.
revoke connect, temporary on database anima from public;
grant connect on database anima to anima_runtime;
revoke connect on database postgres from public;
revoke connect on database template1 from public;

\unset owner_password
\unset hub_password
