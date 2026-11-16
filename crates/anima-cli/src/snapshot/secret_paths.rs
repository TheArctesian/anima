//! The secret patterns of JOB-017: which paths of a tree look like secrets, judged on the file
//! name alone. The files are never opened. A fixed list until a project has its own.

use std::fmt::Write as _;

/// How many paths a refusal names before it counts the rest.
const SHOWN: usize = 20;

/// Endings that make a `.env` name a template rather than a secret.
const TEMPLATE_ENDINGS: [&str; 3] = [".example", ".sample", ".template"];

/// Endings of key and certificate files.
const KEY_ENDINGS: [&str; 6] = [".pem", ".key", ".p12", ".pfx", ".keystore", ".env"];

/// Starts of SSH private key files (`id_rsa`, `id_rsa.old`, ...).
const SSH_KEY_STARTS: [&str; 3] = ["id_rsa", "id_ed25519", "id_ecdsa"];

/// Whether the last part of `path` matches a secret pattern, in any case.
#[must_use]
pub fn is_secret(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase();
    let dot_env = name == ".env" || name.starts_with(".env.");
    let template = TEMPLATE_ENDINGS.iter().any(|end| name.ends_with(end));
    (dot_env && !template)
        || KEY_ENDINGS.iter().any(|end| name.ends_with(end))
        || SSH_KEY_STARTS.iter().any(|start| name.starts_with(start))
}

/// The paths of `paths` that match, in their order.
#[must_use]
pub fn secret_paths<'a>(paths: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    paths
        .into_iter()
        .filter(|path| is_secret(path))
        .map(str::to_owned)
        .collect()
}

/// The paths for a refusal: the first 20, then "and N more".
#[must_use]
pub fn listed(paths: &[String]) -> String {
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
    fn job_017_env_files_and_key_files_match_at_any_depth() {
        for path in [
            ".env",
            ".env.local",
            ".env.production",
            "apps/web/.env",
            "deploy/.ENV.prod",
            "certs/server.pem",
            "a/b/c/tls.key",
            "client.p12",
            "client.pfx",
            "release.keystore",
            "prod.env",
            "id_rsa",
            "home/.ssh/id_ed25519",
            "id_ecdsa.old",
            "id_rsa.pub",
            "SERVER.PEM",
        ] {
            assert!(is_secret(path), "{path}");
        }
    }

    #[test]
    fn job_017_templates_and_ordinary_files_pass() {
        for path in [
            ".env.example",
            "apps/.env.sample",
            ".env.template",
            ".envrc",
            "environment.rs",
            "keys/README.md",
            "src/key.rs",
            "pem/notes.txt",
            "docs/id_rsa.md.txt.d/x",
            "env",
            "my.environment",
            ".env/config",
        ] {
            assert!(!is_secret(path), "{path}");
        }
    }

    #[test]
    fn job_017_the_matching_paths_keep_their_order() {
        let found = secret_paths(["a.rs", "b/.env", "c.txt", "d.pem"]);
        assert_eq!(found, ["b/.env", "d.pem"]);
    }

    #[test]
    fn job_017_a_refusal_names_twenty_paths_then_counts_the_rest() {
        let few = vec!["a.pem".to_owned(), ".env".to_owned()];
        assert_eq!(listed(&few), "a.pem, .env");

        let many: Vec<String> = (0..23).map(|n| format!("k{n}.key")).collect();
        let text = listed(&many);
        assert!(text.starts_with("k0.key, k1.key"), "{text}");
        assert!(text.contains("k19.key"), "{text}");
        assert!(!text.contains("k20.key"), "{text}");
        assert!(text.ends_with(" and 3 more"), "{text}");
    }
}
