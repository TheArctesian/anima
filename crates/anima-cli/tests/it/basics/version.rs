//! `anima --version` (INS-003, OD-17): the release version the builder passes in
//! `ANIMA_VERSION`, else the crate version.

use crate::kit::Anima;

#[path = "../../../release_version.rs"]
#[expect(dead_code, reason = "only the build script reads the variable's name")]
mod release_version;

use release_version::{Refused, chosen};

#[tokio::test]
async fn ins_003_version_prints_the_version_the_package_was_built_with() {
    // The build script sets `CARGO_PKG_VERSION` for every target of the package, this test too.
    let run = Anima::new().run(&["--version"]).await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(
        run.stdout.trim_end(),
        format!("anima {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn ins_003_a_release_version_replaces_the_crate_version() {
    assert_eq!(chosen(Some("1.4.0"), "0.1.0"), Ok("1.4.0"));
    assert_eq!(chosen(Some("0.0.0"), "0.1.0"), Ok("0.0.0"));
    assert_eq!(
        chosen(Some("123456789.0.10"), "0.1.0"),
        Ok("123456789.0.10")
    );
}

#[test]
fn ins_003_no_release_version_keeps_the_crate_version() {
    assert_eq!(chosen(None, "0.1.0"), Ok("0.1.0"));
    assert_eq!(chosen(Some(""), "0.1.0"), Ok("0.1.0"));
}

#[test]
fn ins_003_a_version_the_builder_would_refuse_fails_the_build() {
    for text in [
        "1.4",
        "1.4.0.1",
        "v1.4.0",
        "01.4.0",
        "1.4.0-rc1",
        "1.4.0 ",
        "1..0",
        "1234567890.0.0",
    ] {
        assert_eq!(chosen(Some(text), "0.1.0"), Err(Refused), "{text}");
    }
}
