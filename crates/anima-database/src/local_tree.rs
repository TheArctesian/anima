//! The PostgreSQL 17 of the computer that runs the tests: the one the gates already need
//! (`quality/bin/with-postgres.ts`). A test never skips for lack of it (CQ-024).

use std::path::PathBuf;

use crate::Tree;

/// # Panics
/// When this computer has no PostgreSQL 17.
pub fn local_tree() -> Tree {
    let named = std::env::var_os("ANIMA_POSTGRES_TREE").map(PathBuf::from);
    let known = [
        "/opt/homebrew/opt/postgresql@17",
        "/usr/local/opt/postgresql@17",
        "/usr/lib/postgresql/17",
    ]
    .map(PathBuf::from);
    named
        .iter()
        .chain(&known)
        .find_map(|folder| Tree::at(folder).ok())
        .expect(
            "no PostgreSQL 17 found: install it (macOS: brew install postgresql@17) \
             or set ANIMA_POSTGRES_TREE to a folder with bin/postgres",
        )
}
