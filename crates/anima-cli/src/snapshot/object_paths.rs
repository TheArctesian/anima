//! The paths in git's NUL-separated object listings, read without trusting them: the entries
//! of `ls-tree -r -z` and the objects of `rev-list -z --objects`. A path that is not UTF-8 is
//! kept with its bad bytes replaced, so it can still be matched and named. Pure.

/// The file mode git gives a submodule or a nested repository (a gitlink).
const GITLINK_MODE: &str = "160000";

/// One entry of a tree.
#[derive(Debug, PartialEq, Eq)]
pub struct TreeEntry {
    pub path: String,
    /// A submodule or nested repository: only its commit is recorded, not its files.
    pub gitlink: bool,
}

/// The entries of `git ls-tree -r -z <tree>`: `<mode> <type> <hash>\t<path>`, each ending in
/// NUL. A record without a tab is left out.
#[must_use]
pub fn tree_entries(stdout: &[u8]) -> Vec<TreeEntry> {
    stdout
        .split(|byte| *byte == 0)
        .filter_map(|record| {
            let tab = record.iter().position(|byte| *byte == b'\t')?;
            let (meta, path) = (&record[..tab], &record[tab + 1..]);
            Some(TreeEntry {
                path: String::from_utf8_lossy(path).into_owned(),
                gitlink: meta.starts_with(GITLINK_MODE.as_bytes()),
            })
        })
        .collect()
}

/// The paths of `git rev-list -z --objects`: each object is its hash, then for a tree or a
/// blob a `path=<path>` field, all ending in NUL. Commits and the root tree have no path.
#[must_use]
pub fn object_paths(stdout: &[u8]) -> Vec<String> {
    stdout
        .split(|byte| *byte == 0)
        .filter_map(|field| field.strip_prefix(b"path="))
        .map(|path| String::from_utf8_lossy(path).into_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_017_tree_entries_keep_odd_paths_and_mark_gitlinks() {
        let stdout = b"100644 blob aa11\ta.rs\0\
                       100644 blob bb22\tdir/with\nnewline.pem\0\
                       160000 commit cc33\tvendor/lib\0\
                       100755 blob dd44\tbad\xffname\0\
                       garbage\0";
        let entries = tree_entries(stdout);
        let paths: Vec<&str> = entries.iter().map(|entry| entry.path.as_str()).collect();
        assert_eq!(
            paths,
            [
                "a.rs",
                "dir/with\nnewline.pem",
                "vendor/lib",
                "bad\u{fffd}name"
            ]
        );
        let gitlinks: Vec<&str> = entries
            .iter()
            .filter(|entry| entry.gitlink)
            .map(|entry| entry.path.as_str())
            .collect();
        assert_eq!(gitlinks, ["vendor/lib"]);
        assert_eq!(tree_entries(b""), []);
    }

    #[test]
    fn job_017_rev_list_objects_give_their_paths() {
        let stdout = b"c0ffee\0tree1\0b10b\0path=d d\0b10c\0path=d d/a\nb.env\0\
                       b10d\0path=path=x\0";
        assert_eq!(object_paths(stdout), ["d d", "d d/a\nb.env", "path=x"]);
        assert_eq!(object_paths(b""), Vec::<String>::new());
    }
}
