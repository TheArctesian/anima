//! The recipes that ship inside each binary that uses this crate (task H38): `build.rs` builds
//! the texts of `entries/` in, so that the hub reads no file at run time.

use crate::Recipe;

/// One recipe folder as `build.rs` built it in: the entry file, the Compose file and the
/// example environment file, `None` for a file that is not there.
struct Folder {
    name: &'static str,
    texts: [Option<&'static str>; 3],
}

/// Each recipe of `entries/`, in the order of the names.
const SHIPPED: &[Folder] = include!(concat!(env!("OUT_DIR"), "/shipped.rs"));

/// Each recipe of `entries/`, as it was when the binary was built. The caller runs
/// [`crate::check`] on each one: a recipe with a finding is not listed.
#[must_use]
pub fn shipped() -> Vec<Recipe> {
    SHIPPED
        .iter()
        .map(|folder| {
            let [entry, compose, environment] = folder.texts.map(|text| text.map(str::to_owned));
            Recipe {
                folder: folder.name.to_owned(),
                entry,
                compose,
                environment,
            }
        })
        .collect()
}
