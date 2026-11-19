//! The name of an image as a place at a public registry (HOST-021).

use crate::work::containers::updates::registry::{Named, Reference, Registry, named};

const OLD: &str = "sha256:f59abe656b692d901e184be3445e084786602108599d2a51d229c0f5a48c71bf";

fn tag(registry: Registry, repository: &str, tag: &str) -> Named {
    Named::Tag(Reference {
        registry,
        repository: repository.to_owned(),
        tag: tag.to_owned(),
    })
}

#[test]
fn host_021_an_image_name_is_read_as_docker_reads_it() {
    assert_eq!(
        named("alpine"),
        tag(Registry::DockerHub, "library/alpine", "latest")
    );
    assert_eq!(
        named("alpine:3"),
        tag(Registry::DockerHub, "library/alpine", "3")
    );
    assert_eq!(
        named("freikin/dawarich:0.30"),
        tag(Registry::DockerHub, "freikin/dawarich", "0.30")
    );
    assert_eq!(
        named("docker.io/library/postgres:17"),
        tag(Registry::DockerHub, "library/postgres", "17")
    );
    assert_eq!(
        named("ghcr.io/home-assistant/home-assistant:stable"),
        tag(Registry::GitHub, "home-assistant/home-assistant", "stable")
    );
    assert_eq!(
        named("quay.io/prometheus/prometheus"),
        tag(Registry::Quay, "prometheus/prometheus", "latest")
    );
    assert_eq!(
        named("mcr.microsoft.com/dotnet/runtime:8.0"),
        tag(Registry::Microsoft, "dotnet/runtime", "8.0")
    );
}

#[test]
fn host_021_a_name_the_hub_does_not_read_reaches_no_registry() {
    assert_eq!(named(&format!("alpine@{OLD}")), Named::Pinned);
    assert_eq!(named(&format!("ghcr.io/a/b:1@{OLD}")), Named::Pinned);
    for other in [
        "registry.example.com/team/app:1",
        "localhost:5000/app",
        "lscr.io/linuxserver/sonarr",
        "10.0.0.5:5000/app:1",
        // A host that only looks like one the hub reads.
        "ghcr.io.example.com/a/b",
    ] {
        assert_eq!(named(other), Named::OtherRegistry, "{other}");
    }
    for no_name in [
        "",
        "Alpine",
        "alpine:",
        "alpine:-1",
        "library/../alpine",
        "a//b",
        "alpine:3?x=1",
        "ghcr.io/a/b:1#frag",
        "alpine:3 --help",
        "ghcr.io/a b/c",
        "ghcr.io/%2e%2e/c",
    ] {
        assert_eq!(named(no_name), Named::NoName, "{no_name:?}");
    }
    assert_eq!(named(&format!("a:{}", "t".repeat(129))), Named::NoName);
}
