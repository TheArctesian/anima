//! Keeping the release bucket small (BLD-086, R-118): after a job publishes, the files of the
//! versions of its target older than the newest few are deleted. The choice of versions is pure;
//! the deletes are the edge. A fault is logged and never fails the publish.

use super::{run::Reach, tries};
use crate::{
    builds::{BuildJob, Error, store, version::Version},
    ids::BuildJobId,
};

/// How many published versions of one target stay in the bucket (R-118).
pub(super) const KEEP: usize = 3;

/// The entries of `held`, versions with their owners, that are not among the `keep` newest and
/// are not `latest`, which `latest.json` points at. Newest first.
fn stale<T>(mut held: Vec<(Version, T)>, keep: usize, latest: Version) -> Vec<T> {
    held.sort_by_key(|(version, _)| std::cmp::Reverse(*version));
    held.into_iter()
        .skip(keep)
        .filter(|(version, _)| *version != latest)
        .map(|(_, owner)| owner)
        .collect()
}

/// Deletes the files of the versions of `published`'s target that are older than the [`KEEP`]
/// newest published ones, and records each as removed. A fault is logged and the files stay,
/// to be tried again after the next publish of the target.
pub(super) async fn prune(db: &sqlx::PgPool, reach: Reach<'_>, published: &BuildJob) {
    if let Err(error) = remove_stale(db, reach, published).await {
        tracing::warn!(
            domain = "builds",
            fault = error.to_string(),
            "publishing could not list the old versions to remove"
        );
    }
}

async fn remove_stale(
    db: &sqlx::PgPool,
    reach: Reach<'_>,
    published: &BuildJob,
) -> Result<(), Error> {
    let held = store::unremoved_published(db, published.target)
        .await?
        .into_iter()
        .filter_map(|(job, version)| Some((version.parse::<Version>().ok()?, (job, version))))
        .collect();
    for (job, version) in stale(held, KEEP, published.version) {
        if remove(db, reach, published, (job, &version)).await? {
            store::mark_removed(db, job, reach.clock.now()).await?;
        }
    }
    Ok(())
}

/// Deletes the files of job `old` of `published`'s target; whether every delete went through.
async fn remove(
    db: &sqlx::PgPool,
    reach: Reach<'_>,
    published: &BuildJob,
    (old, version): (BuildJobId, &str),
) -> Result<bool, Error> {
    let files = store::files_of(db, old).await?;
    Ok(tries::delete_objects(reach, (published.target, version), &files, "an old version").await)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(versions: &[(u32, u32, u32)]) -> Vec<(Version, u32)> {
        versions
            .iter()
            .enumerate()
            .map(|(at, (a, b, c))| (Version::new(*a, *b, *c), u32::try_from(at).unwrap()))
            .collect()
    }

    #[test]
    fn bld_086_four_published_versions_leave_the_three_newest() {
        let held = held(&[(0, 1, 2), (0, 1, 10), (0, 1, 9), (0, 1, 11)]);
        // Owner 0 is 0.1.2, the oldest; 0.1.10 sorts above 0.1.9.
        assert_eq!(stale(held, 3, Version::new(0, 1, 11)), vec![0]);
    }

    #[test]
    fn bld_086_three_or_fewer_versions_are_all_kept() {
        let held = held(&[(0, 1, 1), (0, 1, 2), (0, 1, 3)]);
        assert!(stale(held, 3, Version::new(0, 1, 3)).is_empty());
    }

    #[test]
    fn bld_086_the_version_the_feed_names_is_never_removed() {
        let held = held(&[(0, 1, 1), (0, 1, 2), (0, 1, 3), (0, 1, 4), (0, 1, 5)]);
        // A feed rolled back to the oldest version stays in the bucket.
        assert_eq!(stale(held, 3, Version::new(0, 1, 1)), vec![1]);
    }

    #[test]
    fn bld_086_the_count_is_a_parameter() {
        let held = held(&[(1, 0, 0), (2, 0, 0), (3, 0, 0)]);
        assert_eq!(stale(held, 1, Version::new(3, 0, 0)), vec![1, 0]);
    }
}
