-- Publishing (WP-36 T3, R-108; BLD-060 to BLD-070, D-25, D-26). K2 lets a package add columns
-- to its own tables; there is no new table.
--
-- `builds.job_files` now holds the files the hub gave upload URLs for (D-26); the `uploaded`
-- report must name exactly these. Once that report is in, the builder's lease ends and the hub
-- owns the job: `publishing_since` is set while it stays `uploading`. The publisher takes the
-- job for a while (`publish_held_until`) and counts its tries (`publish_attempts`), so a hub
-- that stops mid-publish takes the job again later (D-25). `feeds.written` lists the feed
-- folders the hub wrote for a published job: empty for a macOS job (BLD-068).

alter table builds.jobs
    add column publishing_since timestamptz null,
    add column publish_held_until timestamptz null,
    add column publish_attempts integer not null default 0,
    drop constraint jobs_lease_check,
    add constraint jobs_lease_check check (
        (state in ('claimed', 'building') or (state = 'uploading' and publishing_since is null))
            = (lease_ends_at is not null)
    ),
    add constraint jobs_publishing_check check (
        publishing_since is null or state = 'uploading'
    ),
    add constraint jobs_publish_held_check check (
        publish_held_until is null or publishing_since is not null
    ),
    add constraint jobs_publish_attempts_check check (publish_attempts between 0 and 3);

create index jobs_publishing on builds.jobs (publishing_since, id)
    where publishing_since is not null;

alter table builds.feeds
    add column written text[] null,
    add constraint feeds_written_check check (
        written is null or written <@ array['linux-x86_64', 'darwin-aarch64', 'darwin-x86_64']
    );
