ALTER TABLE projects.repositories
    ADD COLUMN repository_created_at timestamptz,
    ADD COLUMN stars integer CHECK (stars >= 0),
    ADD COLUMN primary_language text,
    ADD COLUMN size_kib integer CHECK (size_kib >= 0),
    ADD COLUMN last_commit_at timestamptz,
    ADD COLUMN commit_count integer CHECK (commit_count >= 0);
