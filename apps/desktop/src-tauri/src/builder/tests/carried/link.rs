//! `anima-link` in the release (S12 2.2, D-2, review S-5): built by `/bin/sh link/build.sh` from
//! the clone with a fixed Go environment, after `go mod verify`, and staged for each platform.

use std::path::Path;

use super::{STAGE, args};
use crate::builder::{carried::link, job::Target, plan::Program};

#[test]
fn s12_d_2_anima_link_is_built_by_bin_sh_from_the_clone_and_staged_per_platform() {
    let clone = Path::new("/c");
    let build = link::build(clone);
    assert_eq!(build.program, Program::Sh);
    assert_eq!(Program::Sh.fixed_path(), Some(Path::new("/bin/sh")));
    assert_eq!(
        (args(&build), build.cwd.as_path()),
        (vec!["link/build.sh"], clone)
    );
    let go: Vec<(&str, &str)> = build
        .env
        .iter()
        .map(|(name, value)| (*name, value.as_str()))
        .collect();
    assert_eq!(
        go,
        [
            ("GOENV", "off"),
            ("GOTOOLCHAIN", "local"),
            ("GOWORK", "off"),
            ("GOFLAGS", "-mod=readonly"),
            ("GOPROXY", "https://proxy.golang.org"),
            ("GOSUMDB", "sum.golang.org"),
            ("GONOSUMDB", ""),
            ("GONOSUMCHECK", ""),
            ("GONOPROXY", ""),
            ("GOINSECURE", ""),
            ("GOPRIVATE", ""),
        ]
    );
    let verify = link::verify(clone);
    assert_eq!(verify.program, Program::Go);
    assert_eq!(verify.cwd, clone.join("link"));
    assert_eq!(args(&verify), ["mod", "verify"]);
    assert_eq!(verify.env, build.env);
    assert_eq!(
        link::COMMITTED,
        ["link/build.sh", "link/go.mod", "link/go.sum"]
    );
    assert_eq!(link::ABSENT, ["link/go.work", "link/vendor"]);
    assert_eq!(
        link::copies(clone, Target::LinuxX86_64),
        [(
            clone.join("link/bin/anima-link-linux-amd64"),
            Path::new(STAGE).join("anima-link-x86_64-unknown-linux-gnu")
        )]
    );
    assert_eq!(
        link::copies(clone, Target::DarwinUniversal),
        [
            (
                clone.join("link/bin/anima-link-darwin-arm64"),
                Path::new(STAGE).join("anima-link-aarch64-apple-darwin")
            ),
            (
                clone.join("link/bin/anima-link-darwin-amd64"),
                Path::new(STAGE).join("anima-link-x86_64-apple-darwin")
            )
        ]
    );
}
