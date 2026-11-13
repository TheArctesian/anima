-- Keeping 3 versions per target (BLD-086, R-118). Columns only.
--
-- `objects_removed_at` is set when the hub deleted a published job's files from the release
-- bucket. The job stays `published`, so its version is still the floor of BLD-069 and is never
-- built again (D-27); the column only stops the hub deleting the same files again.

alter table builds.jobs add column objects_removed_at timestamptz null;
