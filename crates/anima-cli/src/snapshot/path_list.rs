//! Paths as a message names them. Pure.

use std::fmt::Write as _;

/// How many paths a message names before it counts the rest.
const SHOWN: usize = 20;

/// The first 20 paths, then "and N more".
#[must_use]
pub fn shown(paths: &[String]) -> String {
    let mut text = paths
        .iter()
        .take(SHOWN)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    if paths.len() > SHOWN {
        let _ = write!(text, " and {} more", paths.len() - SHOWN);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_017_a_message_names_twenty_paths_then_counts_the_rest() {
        let few = vec!["a.pem".to_owned(), ".env".to_owned()];
        assert_eq!(shown(&few), "a.pem, .env");

        let many: Vec<String> = (0..23).map(|n| format!("k{n}.key")).collect();
        let text = shown(&many);
        assert!(text.starts_with("k0.key, k1.key"), "{text}");
        assert!(text.contains("k19.key"), "{text}");
        assert!(!text.contains("k20.key"), "{text}");
        assert!(text.ends_with(" and 3 more"), "{text}");
    }
}
