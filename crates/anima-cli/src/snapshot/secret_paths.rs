//! The secret patterns of JOB-017: which paths look like secrets, judged on the path alone.
//! The files are never opened. A fixed list until a project has its own.

/// Endings that make a `.env` name a template rather than a secret.
const TEMPLATE_ENDINGS: [&str; 3] = [".example", ".sample", ".template"];

/// Endings of key, certificate and environment files.
const KEY_ENDINGS: [&str; 10] = [
    ".pem",
    ".key",
    ".p12",
    ".pfx",
    ".keystore",
    ".env",
    ".p8",
    ".jks",
    ".ppk",
    ".der",
];

/// Starts of SSH private key files (`id_rsa`, `id_rsa.old`, ...).
const SSH_KEY_STARTS: [&str; 4] = ["id_rsa", "id_ed25519", "id_ecdsa", "id_dsa"];

/// Whole file names of credential stores.
const CREDENTIAL_NAMES: [&str; 4] = [".netrc", ".pgpass", "credentials", "credentials.json"];

/// A folder with this name holds secrets, whatever its files are called.
const SECRETS_FOLDER: &str = "secrets";

/// Whether `path` matches a secret pattern, in any case: its file name, or a folder on the
/// way to it named `secrets`.
#[must_use]
pub fn is_secret(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let mut parts = lower.rsplit('/');
    let name = parts.next().unwrap_or_default();
    parts.any(|folder| folder == SECRETS_FOLDER) || is_secret_name(name)
}

/// Whether a file name (already lower case) matches.
fn is_secret_name(name: &str) -> bool {
    let template = TEMPLATE_ENDINGS.iter().any(|end| name.ends_with(end));
    (name.starts_with(".env") && !template)
        || KEY_ENDINGS.iter().any(|end| name.ends_with(end))
        || SSH_KEY_STARTS.iter().any(|start| name.starts_with(start))
        || CREDENTIAL_NAMES.contains(&name)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_017_env_key_and_credential_files_match_at_any_depth() {
        for path in [
            ".env",
            ".env.local",
            ".env.production",
            ".envrc",
            ".env-local",
            ".env_prod",
            "apps/web/.env",
            "deploy/.ENV.prod",
            "certs/server.pem",
            "a/b/c/tls.key",
            "client.p12",
            "client.pfx",
            "release.keystore",
            "release.jks",
            "AuthKey_ABC.p8",
            "putty.ppk",
            "cert.der",
            "prod.env",
            "id_rsa",
            "home/.ssh/id_ed25519",
            "id_ecdsa.old",
            "id_dsa",
            "id_rsa.pub",
            "SERVER.PEM",
            "home/.netrc",
            ".pgpass",
            "aws/credentials",
            "gcp/credentials.json",
            "deploy/secrets/hub.toml",
            "Secrets/readme.txt",
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
            ".envrc.example",
            "environment.rs",
            "keys/README.md",
            "src/key.rs",
            "pem/notes.txt",
            "docs/id_rsa.md.txt.d/x",
            "env",
            "my.environment",
            "my.env.d/x",
            "credentials.rs",
            "credentials/README.md",
            "docs/secrets.md",
            "secrets",
            "src/secrets.rs",
            ".npmrc",
        ] {
            assert!(!is_secret(path), "{path}");
        }
    }

    #[test]
    fn job_017_the_matching_paths_keep_their_order() {
        let found = secret_paths(["a.rs", "b/.env", "c.txt", "d.pem"]);
        assert_eq!(found, ["b/.env", "d.pem"]);
    }
}
