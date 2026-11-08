-- The build queue (K2 range 0900-0949; WP-36; spec Section 3.10, BLD-001 to BLD-082).
--
-- Privileges (R-016, K2 "Roles"): anima_runtime gets SELECT, INSERT, UPDATE and DELETE on
-- these tables from the default privileges that migration 0001 sets for the owner role.

create schema builds;

-- One row per release the operator starts (BLD-001). The version is MAJOR.MINOR.PATCH with no
-- leading zeros; the commit is 40 lower-case hex digits. The operator's name is copied, as the
-- logs do, so the row keeps it.
create table builds.releases (
    id uuid primary key,
    version text not null,
    commit_sha text not null,
    created_by uuid not null,
    created_by_name text not null,
    created_at timestamptz not null,
    constraint releases_created_by_fkey foreign key (created_by)
        references access.operators (id),
    constraint releases_version_check check (
        version ~ '^(0|[1-9][0-9]{0,8})\.(0|[1-9][0-9]{0,8})\.(0|[1-9][0-9]{0,8})$'
    ),
    constraint releases_commit_check check (commit_sha ~ '^[0-9a-f]{40}$')
);

create index releases_newest on builds.releases (created_at desc, id desc);

-- A computer the operator enrolled to build (BLD-021). The token is an access token with the
-- `builder` scope (D-1); deleting it (revocation, `operator set`, expiry pruning) deletes the
-- builder. `client_addr` is the VPN address at enrolment: builder requests from another
-- address are refused (BLD-024).
create table builds.builders (
    id uuid primary key,
    name text not null,
    os text not null,
    arch text not null,
    client_addr inet not null,
    token_id uuid not null,
    enrolled_at timestamptz not null,
    last_seen_at timestamptz null,
    constraint builders_token_id_key unique (token_id),
    constraint builders_token_id_fkey foreign key (token_id)
        references access.api_tokens (id) on delete cascade,
    constraint builders_name_check check (char_length(name) between 1 and 64),
    constraint builders_os_check check (os in ('linux', 'macos')),
    constraint builders_arch_check check (arch in ('x86_64', 'aarch64'))
);

-- One row per target of a release (BLD-001). A lease is held exactly while the job is
-- claimed, building or uploading (BLD-005); a failed job has its reason. Each `<state>_at`
-- is the last time the job entered that state; `state_changed_at` the last change of all.
create table builds.jobs (
    id uuid primary key,
    release_id uuid not null,
    target text not null,
    version text not null,
    state text not null,
    failure_reason text null,
    attempts integer not null default 0,
    builder_id uuid null,
    builder_name text null,
    lease_ends_at timestamptz null,
    log_sequence bigint not null default 0,
    created_at timestamptz not null,
    queued_at timestamptz not null,
    claimed_at timestamptz null,
    building_at timestamptz null,
    uploading_at timestamptz null,
    published_at timestamptz null,
    failed_at timestamptz null,
    cancelled_at timestamptz null,
    state_changed_at timestamptz not null,
    constraint jobs_release_id_fkey foreign key (release_id)
        references builds.releases (id) on delete cascade,
    constraint jobs_builder_id_fkey foreign key (builder_id)
        references builds.builders (id) on delete set null,
    constraint jobs_release_target_key unique (release_id, target),
    constraint jobs_target_check check (target in ('linux-x86_64', 'darwin-universal')),
    constraint jobs_state_check check (
        state in ('queued', 'claimed', 'building', 'uploading', 'published', 'failed', 'cancelled')
    ),
    constraint jobs_attempts_check check (attempts between 0 and 3),
    constraint jobs_lease_check check (
        (state in ('claimed', 'building', 'uploading')) = (lease_ends_at is not null)
    ),
    constraint jobs_failure_check check (
        (state = 'failed') = (failure_reason is not null)
        and char_length(failure_reason) <= 500
    ),
    constraint jobs_log_sequence_check check (log_sequence >= 0)
);

-- A version is built at most once for a target (BLD-002): a failed or cancelled job frees it.
create unique index jobs_one_build_per_version on builds.jobs (target, version)
    where state not in ('failed', 'cancelled');
-- The oldest queued job of a target first (D-3).
create index jobs_queue on builds.jobs (target, created_at, id) where state = 'queued';
create index jobs_leases on builds.jobs (lease_ends_at) where lease_ends_at is not null;
create index jobs_release_id on builds.jobs (release_id);

-- The files a builder reported uploaded, with the proposed feed on the job (D-5). Publishing
-- (task 3) verifies them.
create table builds.job_files (
    job_id uuid not null,
    name text not null,
    sha256 text not null,
    size_bytes bigint not null,
    primary key (job_id, name),
    constraint job_files_job_id_fkey foreign key (job_id)
        references builds.jobs (id) on delete cascade,
    constraint job_files_name_check check (name ~ '^[A-Za-z0-9_][A-Za-z0-9._-]{0,127}$'),
    constraint job_files_sha256_check check (sha256 ~ '^[0-9a-f]{64}$'),
    constraint job_files_size_check check (size_bytes > 0)
);

create table builds.feeds (
    job_id uuid primary key,
    feed text not null,
    reported_at timestamptz not null,
    constraint feeds_job_id_fkey foreign key (job_id)
        references builds.jobs (id) on delete cascade,
    constraint feeds_size_check check (octet_length(feed) <= 65536)
);

-- Each job's log, redacted before it is stored (BLD-049) and trimmed to its last 2 MB
-- (BLD-048, D-4).
create table builds.log_lines (
    job_id uuid not null,
    sequence bigint not null,
    text text not null,
    primary key (job_id, sequence),
    constraint log_lines_job_id_fkey foreign key (job_id)
        references builds.jobs (id) on delete cascade,
    constraint log_lines_text_check check (octet_length(text) <= 4096)
);
