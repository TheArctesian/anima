//! The version `anima --version` prints (INS-003, OD-17). A release build sets `ANIMA_VERSION`
//! to the release version the builder also gives the Tauri configuration (BLD-043); any other
//! build prints the crate version. Pure: the build script passes the variable in.

/// The variable that carries the release version into the build.
pub const VARIABLE: &str = "ANIMA_VERSION";

/// Why a release version was refused.
#[derive(Debug, PartialEq, Eq)]
pub struct Refused;

/// The version to build in: `release` when it is set, the crate version when it is unset or
/// empty. A release version must be `MAJOR.MINOR.PATCH`, each part 1 to 9 digits with no leading
/// zero, the rule of the builder's `Version` (BLD-002), so a mistyped variable fails the build.
pub fn chosen<'a>(release: Option<&'a str>, crate_version: &'a str) -> Result<&'a str, Refused> {
    match release {
        None | Some("") => Ok(crate_version),
        Some(text) if is_release(text) => Ok(text),
        Some(_) => Err(Refused),
    }
}

fn is_release(text: &str) -> bool {
    let part = |p: &str| {
        !p.is_empty()
            && p.len() <= 9
            && p.bytes().all(|b| b.is_ascii_digit())
            && (p == "0" || !p.starts_with('0'))
    };
    let parts: Vec<&str> = text.split('.').collect();
    parts.len() == 3 && parts.iter().all(|p| part(p))
}
