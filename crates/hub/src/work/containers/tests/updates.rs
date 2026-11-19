//! A newer image for a container (HOST-021): the read of a registry against a local
//! stand-in, and the comparison for each container of a device.

use sqlx::PgPool;

use super::{
    SECRET, device, execs, exiting, policy, reach,
    registry::{NEW, OLD, answer, known, serve},
};
use crate::{
    fleet::ssh::CommandOutput,
    work::containers::{Error, ImageUpdate, NotKnown, UpdateState, image_updates_through, updates},
};

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
    let registry = known().await;
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
    let registry = known().await;
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
    let registry = known().await;
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
