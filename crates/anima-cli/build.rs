//! Builds the release version into `anima` (INS-003, OD-17): the builder sets `ANIMA_VERSION` to
//! the version it gives the Tauri configuration (BLD-043). The script sets `CARGO_PKG_VERSION`
//! for this package's compiler runs, so clap's `--version` and the MCP server's version both
//! print it. Unset, the crate version stays.

#[path = "release_version.rs"]
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
