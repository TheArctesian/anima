//! What turning an extension on turns on with it (EXT-011): its needs, and theirs, that are off.

use std::collections::BTreeMap;

/// The keys that are off and that `key` needs, directly or through other needs, sorted.
/// `extensions` maps each key to `(on, needs)`.
#[must_use]
pub fn also_turned_on(extensions: &BTreeMap<&str, (bool, &[String])>, key: &str) -> Vec<String> {
    let mut pending = vec![key];
    let mut found: Vec<&str> = Vec::new();
    while let Some(next) = pending.pop() {
        let Some((_, needs)) = extensions.get(next) else {
            continue;
        };
        for need in *needs {
            let off = extensions.get(need.as_str()).is_some_and(|(on, _)| !on);
            if off && !found.contains(&need.as_str()) {
                found.push(need);
                pending.push(need);
            }
        }
    }
    found.sort_unstable();
    found.into_iter().map(str::to_owned).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ext_011_the_needs_of_the_needs_that_are_off_are_found_once() {
        let projects = vec!["github".to_owned(), "links".to_owned()];
        let links = vec!["github".to_owned()];
        let none: Vec<String> = Vec::new();
        let extensions = BTreeMap::from([
            ("projects", (false, projects.as_slice())),
            ("links", (false, links.as_slice())),
            ("github", (false, none.as_slice())),
            ("vault", (true, none.as_slice())),
        ]);
        assert_eq!(also_turned_on(&extensions, "projects"), ["github", "links"]);
        assert_eq!(also_turned_on(&extensions, "vault"), Vec::<String>::new());
    }
}
