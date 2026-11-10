-- Publishing (WP-36 T3 round 2, R-108; BLD-070, UPD-008, D-25). Columns only.
--
-- `feed_sent_at` is set just before the publisher's first `latest.json` write for a job. From
-- then on the feed may name the job's files, so the hub never deletes them, and a later try
-- confirms the feed instead of writing it again.

alter table builds.jobs add column feed_sent_at timestamptz null;
