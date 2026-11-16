//! Builds the release version into `anima-agent` (R-118, ROL-051): the builder sets
//! `ANIMA_VERSION` to the release version, as it does for `anima` (INS-003, OD-17). The script
//! sets `CARGO_PKG_VERSION` for this package's compiler runs, so each batch carries it. Unset,
//! the crate version stays. The rule is the one `anima` uses, from the same file.

#[path = "../anima-cli/release_version.rs"]
mod release_version;

fn main() {
    println!("cargo::rerun-if-env-changed={}", release_version::VARIABLE);
    let release = std::env::var(release_version::VARIABLE).ok();
    let crate_version = env!("CARGO_PKG_VERSION");
    let Ok(version) = release_version::chosen(release.as_deref(), crate_version) else {
        panic!(
            "{} must be MAJOR.MINOR.PATCH (BLD-002)",
            release_version::VARIABLE
        );
    };
    if version != crate_version {
        println!("cargo::rustc-env=CARGO_PKG_VERSION={version}");
    }
}
