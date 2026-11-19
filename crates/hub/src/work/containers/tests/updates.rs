//! A newer image for a container (HOST-021): the read of a registry against a local
//! stand-in, and the comparison for each container of a device.

use sqlx::PgPool;

use super::{
    SECRET, device, execs, exiting, policy, reach,
    registry::{TOKEN, answer, manifest, serve, token},
};
use crate::{
    fleet::ssh::CommandOutput,
    work::containers::{
        Error, ImageUpdate, NotKnown, UpdateState, image_updates_through,
        updates::{
            self,
            egress::{Egress, FetchError},
            registry::{Named, Reference, named},
        },
    },
};

const OLD: &str = "sha256:f59abe656b692d901e184be3445e084786602108599d2a51d229c0f5a48c71bf";
const NEW: &str = "sha256:3136cbb572a3282f84719ef92f23b8528ea461e632965f72699e2301343339c0";

fn reference(image: &str) -> Reference {
    match named(image) {
        Named::Tag(reference) => reference,
        other => panic!("{image}: {other:?}"),
    }
}

/// A stand-in that knows two tags and refuses a third.
async fn registry() -> super::registry::Registry {
    serve(|method, path| match (method, path) {
        ("GET", path) if path.starts_with("/token?") => token(),
        ("HEAD", "/v2/library/alpine/manifests/3") => manifest(OLD),
        (
            "HEAD",
            "/v2/freikin/dawarich/manifests/latest" | "/v2/prometheus/prometheus/manifests/latest",
        ) => manifest(NEW),
        ("HEAD", "/v2/team/private/manifests/1") => answer("401 Unauthorized", ""),
        ("HEAD", "/v2/library/moved/manifests/1") => answer(
            "307 Temporary Redirect",
            "Location: /v2/library/alpine/manifests/3
",
        ),
        ("HEAD", "/v2/library/odd/manifests/1") => manifest("sha256:not-a-digest"),
        _ => answer("404 Not Found", ""),
    })
    .await
}

#[tokio::test]
async fn host_021_the_hub_asks_for_a_token_with_no_account_then_for_the_head_of_the_manifest() {
    let registry = registry().await;
    let egress = registry.egress();
    assert_eq!(
        egress.digest(&reference("alpine:3")).await,
        Ok(OLD.to_owned())
    );
    let heads = registry.heads();
    assert_eq!(heads.len(), 2, "{heads:?}");
    let asked = "GET /token?service=registry.docker.io&scope=repository%3Alibrary%2Falpine%3Apull ";
    assert!(heads[0].starts_with(asked), "{heads:?}");
    // The request for a token sends no credential.
    assert!(
        !heads[0].to_lowercase().contains("authorization"),
        "{heads:?}"
    );
    assert!(
        heads[1].starts_with("HEAD /v2/library/alpine/manifests/3 "),
        "{heads:?}"
    );
    let lower = heads[1].to_lowercase();
    assert!(lower.contains(&format!("authorization: bearer {}", TOKEN.to_lowercase())));
    assert!(
        lower.contains("application/vnd.oci.image.index.v1+json"),
        "{heads:?}"
    );
    assert!(!lower.contains("cookie"), "{heads:?}");
}

#[tokio::test]
async fn host_021_a_registry_that_asks_for_no_token_gets_one_request() {
    let registry = registry().await;
    let read = registry
        .egress()
        .digest(&reference("quay.io/prometheus/prometheus"))
        .await;
    assert_eq!(read, Ok(NEW.to_owned()));
    assert_eq!(
        registry.lines(),
        ["HEAD /v2/prometheus/prometheus/manifests/latest HTTP/1.1"]
    );
    assert!(!registry.heads()[0].to_lowercase().contains("authorization"));
}

#[tokio::test]
async fn host_021_a_private_or_unknown_tag_is_not_there_and_no_redirect_is_followed() {
    let registry = registry().await;
    let egress = registry.egress();
    for gone in ["ghcr.io/team/private:1", "alpine:nosuchtag"] {
        assert_eq!(
            egress.digest(&reference(gone)).await,
            Err(FetchError::NotThere),
            "{gone}"
        );
    }
    let before = registry.heads().len();
    assert_eq!(
        egress.digest(&reference("moved:1")).await,
        Err(FetchError::Failed)
    );
    // The token and the one manifest request; the redirect led to no third request.
    assert_eq!(registry.heads().len(), before + 2);
    assert_eq!(
        egress.digest(&reference("odd:1")).await,
        Err(FetchError::Failed)
    );
}

#[tokio::test]
async fn host_021_a_registry_at_an_address_the_policy_refuses_is_not_connected_to() {
    let registry = registry().await;
    // The same server and the same trusted certificate as the tests that reach it: only the
    // policy differs. The default policy refuses loopback, where the stand-in listens.
    let (origin, root) = registry.place();
    let refused =
        Egress::new(crate::fleet::targets::AddressPolicy::default()).standing_in(origin, root);
    assert_eq!(
        refused.digest(&reference("alpine:3")).await,
        Err(FetchError::Failed)
    );
    assert_eq!(registry.heads(), [] as [String; 0]);
}

/// What the device writes: seven containers and the digests of four tags.
fn listed() -> String {
    let container = |id: &str, name: &str, image: &str| {
        format!(
            r#"{{"kind":"container","id":"{}","names":"{name}","image":"{image}"}}"#,
            id.repeat(64)
        ) + "\n"
    };
    let image = |repository: &str, tag: &str, digest: &str| {
        format!(
            r#"{{"kind":"image","repository":"{repository}","tag":"{tag}","digest":"{digest}"}}"#
        ) + "\n"
    };
    [
        container("a", "web/alias,current", "alpine:3"),
        container("b", "behind", "freikin/dawarich"),
        container("c", "pinned", &format!("alpine@{OLD}")),
        container("d", "built", "my-app:dev"),
        container("e", "elsewhere", "registry.example.com/team/app:1"),
        container("f", "by-id", "f59abe656b69"),
        container("0", "secret", "ghcr.io/team/private:1"),
        image("alpine", "3", OLD),
        image("freikin/dawarich", "latest", OLD),
        image("my-app", "dev", "<none>"),
        image("ghcr.io/team/private", "1", OLD),
        image("<none>", "<none>", "<none>"),
    ]
    .concat()
}

fn output(stdout: &str, exit_status: u32) -> CommandOutput {
    CommandOutput {
        exit_status: Some(exit_status),
        stdout: stdout.as_bytes().to_vec(),
        cut: false,
    }
}

fn verdicts(updates: &[ImageUpdate]) -> Vec<(&str, UpdateState, Option<NotKnown>)> {
    updates
        .iter()
        .map(|update| (update.container.as_str(), update.state, update.reason))
        .collect()
}

#[tokio::test]
async fn host_021_each_container_has_the_answer_for_the_tag_it_names() {
    let registry = registry().await;
    let found = updates::found(&output(&listed(), 0)).unwrap();
    let read = updates::compare(found, &registry.egress()).await;
    let expected = [
        ("current", UpdateState::UpToDate, None),
        ("behind", UpdateState::NewerImage, None),
        (
            "pinned",
            UpdateState::NotKnown,
            Some(NotKnown::PinnedDigest),
        ),
        (
            "built",
            UpdateState::NotKnown,
            Some(NotKnown::NoLocalDigest),
        ),
        (
            "elsewhere",
            UpdateState::NotKnown,
            Some(NotKnown::OtherRegistry),
        ),
        ("by-id", UpdateState::NotKnown, Some(NotKnown::NoName)),
        (
            "secret",
            UpdateState::NotKnown,
            Some(NotKnown::NotAtRegistry),
        ),
    ];
    assert_eq!(verdicts(&read), expected);
    assert_eq!(
        (
            read[1].local_digest.as_deref(),
            read[1].registry_digest.as_deref()
        ),
        (Some(OLD), Some(NEW))
    );
    assert_eq!(
        (&read[3].local_digest, &read[3].registry_digest),
        (&None, &None)
    );
    // Only the tags a container names and the device has a digest for are asked for, and
    // no request names the registry the hub does not read.
    let lines = registry.lines().join("\n");
    assert_eq!(lines.matches("HEAD ").count(), 3, "{lines}");
    assert!(
        !lines.contains("my-app") && !lines.contains("team/app"),
        "{lines}"
    );
}

#[tokio::test]
async fn host_021_a_registry_that_is_not_read_is_no_error_and_one_tag_is_asked_for_once() {
    let registry = serve(|_, _| answer("503 Service Unavailable", "")).await;
    let twice = listed().replace("freikin/dawarich\"", "alpine:3\"");
    let found = updates::found(&output(&twice, 0)).unwrap();
    let read = updates::compare(found, &registry.egress()).await;
    assert_eq!(read[0].reason, Some(NotKnown::RegistryNotRead));
    assert_eq!(read[1].reason, Some(NotKnown::RegistryNotRead));
    assert_eq!(read[0].local_digest.as_deref(), Some(OLD));
    // `alpine:3` of two containers: one token request. The private tag: one more.
    assert_eq!(registry.heads().len(), 2);
}

#[tokio::test]
async fn host_021_a_tag_with_two_digests_is_up_to_date_when_the_registry_gives_one_of_them() {
    let registry = registry().await;
    // Docker writes one line for each digest it keeps for a tag. The registry gives OLD for
    // `alpine:3` and NEW for `freikin/dawarich`; each is the second digest of its tag here.
    let more = concat!(
        r#"{"kind":"image","repository":"alpine","tag":"3","digest":"NEW"}"#,
        "\n",
        r#"{"kind":"image","repository":"freikin/dawarich","tag":"latest","digest":"NEW"}"#,
        "\n",
    )
    .replace("NEW", NEW);
    let found = updates::found(&output(&format!("{}{more}", listed()), 0)).unwrap();
    let read = updates::compare(found, &registry.egress()).await;
    assert_eq!(
        (read[0].state, read[0].local_digest.as_deref()),
        (UpdateState::UpToDate, Some(OLD))
    );
    assert_eq!(
        (read[1].state, read[1].local_digest.as_deref()),
        (UpdateState::UpToDate, Some(NEW))
    );
}

#[tokio::test]
async fn host_021_a_token_endpoint_that_refuses_is_a_registry_that_was_not_read() {
    let registry = serve(|_, _| answer("403 Forbidden", "")).await;
    let read = registry.egress().digest(&reference("alpine:3")).await;
    assert_eq!(read, Err(FetchError::Failed));
}

#[test]
fn host_021_each_request_goes_to_the_fixed_host_of_the_registry_of_the_name() {
    let egress = Egress::new(crate::fleet::targets::AddressPolicy::default());
    let places = |image: &str| {
        let reference = reference(image);
        let token = egress.token_url(&reference).unwrap().map(String::from);
        (
            token,
            String::from(egress.manifest_url(&reference).unwrap()),
        )
    };
    let scope = "scope=repository%3Alibrary%2Falpine%3Apull";
    assert_eq!(
        places("alpine:3"),
        (
            Some(format!(
                "https://auth.docker.io/token?service=registry.docker.io&{scope}"
            )),
            "https://registry-1.docker.io/v2/library/alpine/manifests/3".to_owned()
        )
    );
    assert_eq!(
        places("ghcr.io/a/b.c:v1_x"),
        (
            Some(
                "https://ghcr.io/token?service=ghcr.io&scope=repository%3Aa%2Fb.c%3Apull"
                    .to_owned()
            ),
            "https://ghcr.io/v2/a/b.c/manifests/v1_x".to_owned()
        )
    );
    assert_eq!(
        places("quay.io/prometheus/prometheus"),
        (
            None,
            "https://quay.io/v2/prometheus/prometheus/manifests/latest".to_owned()
        )
    );
    assert_eq!(
        places("mcr.microsoft.com/dotnet/runtime:8.0"),
        (
            None,
            "https://mcr.microsoft.com/v2/dotnet/runtime/manifests/8.0".to_owned()
        )
    );
}

#[test]
fn host_021_an_answer_of_the_device_that_is_not_the_format_is_an_error() {
    assert!(matches!(
        updates::found(&output(SECRET, 0)),
        Err(Error::Unreadable)
    ));
    assert!(matches!(
        updates::found(&output(SECRET, 127)),
        Err(Error::NoDocker)
    ));
    assert!(matches!(
        updates::found(&output(&listed(), 1)),
        Err(Error::DockerFailed)
    ));
    let cut = CommandOutput {
        cut: true,
        ..output(&listed(), 0)
    };
    assert!(matches!(updates::found(&cut), Err(Error::Unreadable)));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn host_021_the_device_hears_one_fixed_read_only_command_and_nothing_is_pulled(pool: PgPool) {
    let registry = registry().await;
    let bench = device(&pool, exiting(&listed(), 0)).await;
    let policy = policy();
    let now = crate::fleet::ssh::tests::fixture::now();
    let read = image_updates_through(
        reach(&bench, &policy),
        bench.device.id,
        &registry.egress(),
        now,
    )
    .await
    .unwrap();
    assert_eq!(read.len(), 7);
    assert_eq!(read[1].state, UpdateState::NewerImage);
    assert_eq!(
        execs(&bench),
        [concat!(
            r#"docker ps -a --no-trunc --format '{"kind":"container","id":{{json .ID}},"#,
            r#""names":{{json .Names}},"image":{{json .Image}}}' && docker image ls --no-trunc "#,
            r#"--digests --format '{"kind":"image","repository":{{json .Repository}},"#,
            r#""tag":{{json .Tag}},"digest":{{json .Digest}}}'"#,
        )]
    );
}
