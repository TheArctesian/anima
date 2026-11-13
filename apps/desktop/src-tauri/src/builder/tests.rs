//! Argument checks, plans, the environment allow-list and the tool check are pure; the
//! process tests in `tests/child_process.rs` start real (harmless) programs.

mod checkout;
mod child_process;
mod files;
mod output;
mod plans;
// The reaper is Linux only: macOS has no subreaper.
#[cfg(target_os = "linux")]
mod reaping;
mod signing;
mod upload;

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use super::{
    environment::Environment,
    error::BuilderError,
    job::{Commit, Target, Version},
    tools::{self, Tool},
};

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

fn os(pairs: &[(&str, &str)]) -> Vec<(OsString, OsString)> {
    pairs
        .iter()
        .map(|(k, v)| (OsString::from(k), OsString::from(v)))
        .collect()
}

/// The app's environment on a builder with every kind of secret in it.
fn parent_with_secrets() -> Vec<(OsString, OsString)> {
    os(&[
        ("PATH", "/usr/bin:.::relative/bin:/home/op/bin"),
        ("HOME", "/home/op"),
        ("LANG", "en_GB.UTF-8"),
        (
            "TAURI_SIGNING_PRIVATE_KEY",
            "dW50cnVzdGVkIGNvbW1lbnQ6IHNlY3JldA==",
        ),
        ("TAURI_SIGNING_PRIVATE_KEY_PASSWORD", "hunter2"),
        ("ANIMA_BUILDER_TOKEN", "anb_secret"),
        ("ANIMA_ACCESS_TOKEN", "ana_secret"),
        ("DATABASE_URL", "postgres://u:p@db/anima"),
        ("AWS_SECRET_ACCESS_KEY", "aws-secret"),
        ("GITHUB_TOKEN", "ghp_secret"),
        ("HTTPS_PROXY", "http://user:pw@proxy:3128"),
        ("SSH_AUTH_SOCK", "/tmp/agent"),
        ("LD_PRELOAD", "/tmp/evil.so"),
        ("NODE_OPTIONS", "--require /tmp/evil.js"),
        ("RUSTC_WRAPPER", "/tmp/evil"),
    ])
}

#[test]
fn bld_027_a_commit_is_exactly_40_lower_case_hex_digits() {
    assert_eq!(Commit::parse(COMMIT).unwrap().as_str(), COMMIT);
    for bad in [
        "",
        &COMMIT[..39],
        &format!("{COMMIT}0"),
        &COMMIT.to_uppercase(),
        "--upload-pack=touch${IFS}x--------------------",
        &format!("{}\n", &COMMIT[..39]),
        "g123456789abcdef0123456789abcdef01234567",
        "HEAD",
        "origin/main",
    ] {
        assert_eq!(Commit::parse(bad), Err(BuilderError::Commit), "{bad}");
    }
}

#[test]
fn bld_027_a_version_is_major_minor_patch_as_the_hub_has_it() {
    for good in ["0.1.12", "1.0.0", "999999999.0.1"] {
        assert_eq!(Version::parse(good).unwrap().as_str(), good);
    }
    for bad in [
        "",
        "v0.1.12",
        "0.1",
        "0.1.12.1",
        "01.1.1",
        "1.2.3-beta",
        "1.2.3+b",
        "1.2.3 ",
        "1.2.3\n",
        "../1.2",
        "1..2",
        "1000000000.0.0",
        "1.2.-3",
        "1.2.3\"}",
    ] {
        assert_eq!(Version::parse(bad), Err(BuilderError::Version), "{bad:?}");
    }
}

#[test]
fn bld_003_targets_parse_from_k1_and_match_their_platform() {
    assert_eq!(Target::parse("LINUX_X86_64"), Ok(Target::LinuxX86_64));
    assert_eq!(
        Target::parse("DARWIN_UNIVERSAL"),
        Ok(Target::DarwinUniversal)
    );
    for bad in ["linux_x86_64", "WINDOWS", "", "LINUX_X86_64 "] {
        assert_eq!(Target::parse(bad), Err(BuilderError::Target), "{bad}");
    }
    assert!(Target::LinuxX86_64.buildable_on("linux", "x86_64"));
    assert!(!Target::LinuxX86_64.buildable_on("linux", "aarch64"));
    assert!(Target::DarwinUniversal.buildable_on("macos", "aarch64"));
    assert!(!Target::DarwinUniversal.buildable_on("macos", "x86_64"));
}

#[test]
fn bld_044_a_step_gets_only_the_allow_list_and_never_a_secret() {
    let env = Environment::from_parent(parent_with_secrets());
    let vars = env.for_step(&[("BASE_REF", COMMIT.to_owned())]);
    let names: Vec<&str> = vars.iter().map(|(k, _)| k.to_str().unwrap()).collect();
    assert_eq!(
        names,
        [
            "HOME",
            "LANG",
            "PATH",
            "CI",
            "NO_COLOR",
            "CARGO_TERM_COLOR",
            "GIT_TERMINAL_PROMPT",
            "GIT_CONFIG_NOSYSTEM",
            "GIT_CONFIG_GLOBAL",
            "GIT_NO_REPLACE_OBJECTS",
            "GIT_GRAFT_FILE",
            "BASE_REF"
        ]
    );
    let all = format!("{vars:?}");
    for secret in [
        "dW50cnVz", "hunter2", "anb_", "ana_", "u:p@", "aws-", "ghp_", "pw@",
    ] {
        assert!(!all.contains(secret), "{secret} reached a step");
    }
}

#[test]
fn bld_044_relative_path_entries_are_dropped_and_tool_folders_are_added() {
    let env = Environment::from_parent(parent_with_secrets());
    let dirs: Vec<&Path> = env.dirs().iter().map(PathBuf::as_path).collect();
    assert_eq!(
        &dirs[..4],
        [
            Path::new("/usr/bin"),
            Path::new("/home/op/bin"),
            Path::new("/home/op/.cargo/bin"),
            Path::new("/home/op/.bun/bin")
        ]
    );
    assert!(dirs.iter().all(|dir| dir.is_absolute()));
    let with_cargo = Environment::from_parent(os(&[("CARGO_HOME", "/opt/cargo")]));
    assert_eq!(with_cargo.dirs()[0], Path::new("/opt/cargo/bin"));
    let found = env.find("git", |path| path == Path::new("/home/op/bin/git"));
    assert_eq!(found.as_deref(), Some(Path::new("/home/op/bin/git")));
}

#[test]
fn bld_028_each_missing_tool_is_named() {
    let env = Environment::from_parent(os(&[("PATH", "/usr/bin"), ("HOME", "/home/op")]));
    let everything = tools::missing(Target::LinuxX86_64, &env, |_| false);
    assert_eq!(
        everything,
        [
            Tool::Git,
            Tool::Bun,
            Tool::Rust,
            Tool::CargoNextest,
            Tool::Postgres,
            Tool::TauriPackages,
        ]
    );
    let present = |path: &Path| {
        let text = path.to_str().unwrap();
        [
            "/usr/bin/git",
            "/home/op/.bun/bin/bun",
            "/home/op/.cargo/bin/cargo",
            "/home/op/.cargo/bin/cargo-nextest",
            "/usr/bin/rustc",
        ]
        .contains(&text)
            || text.starts_with("/usr/lib/postgresql/17/bin/")
            || text == "/usr/bin/pkg-config"
            || text == "/usr/bin/cc"
            || text.starts_with("/usr/lib/x86_64-linux-gnu/pkgconfig/")
    };
    assert_eq!(tools::missing(Target::LinuxX86_64, &env, present), []);
    let no_rustc = |path: &Path| present(path) && path != Path::new("/usr/bin/rustc");
    assert_eq!(
        tools::missing(Target::LinuxX86_64, &env, no_rustc),
        [Tool::Rust]
    );
    let pg16 = |path: &Path| present(path) && !path.starts_with("/usr/lib/postgresql/17");
    assert_eq!(
        tools::missing(Target::LinuxX86_64, &env, pg16),
        [Tool::Postgres]
    );
    let no_nextest = |path: &Path| present(path) && !path.ends_with("cargo-nextest");
    assert_eq!(
        tools::missing(Target::LinuxX86_64, &env, no_nextest),
        [Tool::CargoNextest]
    );
    // The gates never run `psql`, so the check does not ask for it.
    let no_psql = |path: &Path| present(path) && !path.ends_with("psql");
    assert_eq!(tools::missing(Target::LinuxX86_64, &env, no_psql), []);
    let json = serde_json::to_string(&[Tool::TauriPackages, Tool::CargoNextest]).unwrap();
    assert_eq!(json, r#"["tauri-packages","cargo-nextest"]"#);
}

#[test]
fn log_007_builder_errors_serialise_as_a_code_and_fixed_text() {
    let json = serde_json::to_value(BuilderError::NotOnMain).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "code": "not-on-main",
            "message": "the commit is not on main of the Anima repository"
        })
    );
}

/// N-4, R-108: an app built with the placeholder `plugins.updater.pubkey` names the updater
/// public key as missing, so it claims no job; the shipped key is a real key line, not missing.
#[test]
fn r_108_an_app_without_its_updater_public_key_claims_no_job() {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let placeholder = tools::key_line("REPLACE_WITH_OPERATOR_PUBLIC_KEY");
    assert_eq!(
        tools::app_public_key(placeholder.as_deref()),
        Some(Tool::UpdaterPublicKey)
    );
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
    let pubkey = config["plugins"]["updater"]["pubkey"].as_str().unwrap();
    let line = tools::key_line(pubkey);
    assert_eq!(
        tools::app_public_key(line.as_deref()),
        None,
        "the shipped plugins.updater.pubkey must be a real key"
    );
    assert_eq!(tools::app_public_key(None), Some(Tool::UpdaterPublicKey));
    let mut key = b"Ed".to_vec();
    key.extend([7_u8; 40]);
    let real = STANDARD.encode(&key);
    assert_eq!(tools::app_public_key(Some(&real)), None);
    let mut other_kind = key.clone();
    other_kind[0] = b'X';
    for wrong in [&key[..41], &key[1..], &other_kind[..]] {
        let wrong = STANDARD.encode(wrong);
        assert_eq!(
            tools::app_public_key(Some(&wrong)),
            Some(Tool::UpdaterPublicKey)
        );
    }
    let json = serde_json::to_string(&[Tool::UpdaterPublicKey]).unwrap();
    assert_eq!(json, r#"["updater-public-key"]"#);
}
