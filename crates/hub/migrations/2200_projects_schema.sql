-- The projects domain's tables (K-E8 range 2200-2299; XP-02 first slice; PRO-001 to PRO-004,
-- PRJ-001, PRJ-016).
--
-- A project has one or more repositories (PRO-001). A repository's identity is its forge and
-- the forge's numeric id, so a rename upstream is the same row; `full_name` follows the
-- rename and is the subject key (PRJ-016). `left_out_at` set: the operator left it out and the
-- hub does not import or read it again (PRO-002); the row itself is the leave-out list.
-- `gone_at` set: the last complete listing did not hold it; the row is kept. Text from the
-- forge (names, descriptions, paths, `original`) is stored as given and never interpreted.
-- `original` is the forge's record (SRC-011). `account_id` is set null when the source
-- account is removed with its records kept (ITG-004).
--
-- A resource is a proposal while `state` is 'found' (PRO-004): the hub adds and removes it as
-- the signs come and go. A confirmed resource is never removed by a sync. Only hosting kinds
-- exist (PRO-006). Class C data: nothing is sealed.
--
-- Privileges: migration 0001's default privileges give `anima_runtime` what it needs on the
-- new schema (K2 "Roles").
create schema projects;

create table projects.projects (
    id         uuid        primary key,
    slug       text        not null unique
                           check (slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$' and length(slug) <= 120),
    name       text        not null check (length(name) between 1 and 300),
    created_at timestamptz not null
);

create table projects.repositories (
    id             uuid        primary key,
    project_id     uuid        not null references projects.projects (id) on delete cascade,
    account_id     uuid        null references sources.accounts (id) on delete set null,
    forge          text        not null check (forge in ('github')),
    external_id    text        not null check (external_id ~ '^[0-9]{1,20}$'),
    full_name      text        not null check (length(full_name) between 3 and 300),
    html_url       text        not null,
    default_branch text        null,
    private        boolean     not null,
    archived       boolean     not null,
    fork           boolean     not null,
    description    text        null,
    pushed_at      timestamptz null,
    original       jsonb       not null,
    left_out_at    timestamptz null,
    gone_at        timestamptz null,
    first_seen_at  timestamptz not null,
    updated_at     timestamptz not null,
    listed_at      timestamptz null,
    constraint repositories_identity unique (forge, external_id)
);

create index repositories_project on projects.repositories (project_id, first_seen_at, id);
create index repositories_account on projects.repositories (account_id);

create table projects.signs (
    repository_id uuid not null references projects.repositories (id) on delete cascade,
    origin        text not null check (origin in ('tree', 'deployments')),
    sign          text not null check (sign in ('wrangler_config', 'vercel_json',
                      'vercel_deployment', 'dockerfile', 'compose_file', 'tauri_conf')),
    path          text null
);

create unique index signs_identity
    on projects.signs (repository_id, origin, sign, coalesce(path, ''));

create table projects.resources (
    id           uuid        primary key,
    project_id   uuid        not null references projects.projects (id) on delete cascade,
    kind         text        not null
                             check (kind in ('cloudflare', 'vercel', 'server', 'desktop_application')),
    state        text        not null check (state in ('found', 'confirmed')),
    found_at     timestamptz not null,
    confirmed_at timestamptz null,
    confirmed_by text        null check (confirmed_by in ('operator', 'provider')),
    constraint resources_one_per_kind unique (project_id, kind),
    constraint resources_confirmed
        check ((state = 'confirmed') = (confirmed_at is not null and confirmed_by is not null))
);
