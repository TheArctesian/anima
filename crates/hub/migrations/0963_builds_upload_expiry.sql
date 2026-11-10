-- The end of a job's upload URLs (WP-36 T3 round 1, R-108 point 8, D-28). The hub reads a job's
-- files back only once every presigned URL it gave for them has expired, so no builder can write
-- an object after it was checked. `buildUploadUrls` keeps the latest end here; the publisher
-- takes the job only after it, with a margin for clock skew.

alter table builds.jobs
    add column upload_urls_expire_at timestamptz null;
