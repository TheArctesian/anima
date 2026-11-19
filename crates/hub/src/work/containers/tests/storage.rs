//! The images, the volumes and the networks of a device (HOST-020): the command lines the
//! device hears and the pure reading of what Docker 29 writes for them.

use sqlx::PgPool;

use super::{SECRET, device, execs, exiting, policy, reach};
use crate::{
    fleet::ssh::CommandOutput,
    work::containers::{DiskUsage, Error, Image, Network, Volume, networks, storage},
};

/// What each container uses, as the last command of both command lines writes it.
const USERS: &str = concat!(
    r#"{"kind":"container","names":"dawarich_app","image":"freikin/dawarich:latest","#,
    r#""mounts":"app-data,/srv/config","networks":"dawarich_default,bridge"}"#,
    "\n",
    r#"{"kind":"container","names":"web/db,old-db","image":"f59abe656b69","#,
    r#""mounts":"","networks":"bridge"}"#,
    "\n",
    r#"{"kind":"container","names":"cache","image":"redis","mounts":"app-data-old","#,
    r#""networks":"none"}"#,
    "\n",
);

const SIZES: &str = concat!(
    r#"{"kind":"image","id":"sha256:5a703ad0d72cd5e4fb4beca8024d6eddef6d22a46ed1d9812bf3d24ff43ccbce","#,
    r#""repository":"freikin/dawarich","tag":"latest","size":"470MB","shared":"420.8MB","#,
    r#""unique":"49.49MB","count":"1","created":"2026-11-16 12:24:00 -0800 PST"}"#,
    "\n",
    r#"{"kind":"image","id":"sha256:f59abe656b692d901e184be3445e084786602108599d2a51d229c0f5a48c71bf","#,
    r#""repository":"<none>","tag":"<none>","size":"13.6MB","shared":"13.5MB","#,
    r#""unique":"87.52kB","count":"1","created":"2026-11-05 10:00:31 -0800 PST"}"#,
    "\n",
    r#"{"kind":"image","id":"sha256:4bbc7223f81db790097c6dea42fb93e05b8fead4f0431fbe6ccaba6e8473201c","#,
    r#""repository":"redis","tag":"latest","size":"421MB","shared":"0B","unique":"421MB","#,
    r#""count":"1","created":"2026-11-16 12:08:25 -0800 PST"}"#,
    "\n\n",
    r#"{"kind":"volume","name":"app-data","driver":"local","size":"232.9MB","count":"1"}"#,
    "\n",
    r#"{"kind":"volume","name":"unused","driver":"local","size":"0B","count":"0"}"#,
    "\n\n",
    r#"{"kind":"usage","type":"Images","total":"3","active":"3","size":"470.4MB","#,
    r#""reclaimable":"49.63MB (10%)"}"#,
    "\n",
    r#"{"kind":"usage","type":"Local Volumes","total":"2","active":"1","size":"232.9MB","#,
    r#""reclaimable":"0B (0%)"}"#,
    "\n",
);

const NETWORKS: &str = concat!(
    r#"{"kind":"network","id":"479212f7b5c312875879801adcc1f8e4d881272a6fe7cc84643b8236039c2e95","#,
    r#""name":"bridge","driver":"bridge","scope":"local","internal":"false"}"#,
    "\n",
    r#"{"kind":"network","id":"b2b5094da00f1cd8f7cc88273dedf1d086e483428cc6b2b8bc7ab8ef099c5d0c","#,
    r#""name":"backend","driver":"overlay","scope":"swarm","internal":"true"}"#,
    "\n",
);

fn output(stdout: &str, exit_status: u32) -> CommandOutput {
    CommandOutput {
        exit_status: Some(exit_status),
        stdout: stdout.as_bytes().to_vec(),
        cut: false,
    }
}

fn names(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn host_020_each_image_has_its_sizes_and_the_containers_that_name_it() {
    let read = storage::storage(&output(&format!("{SIZES}{USERS}"), 0)).unwrap();
    let dawarich = Image {
        id: "sha256:5a703ad0d72cd5e4fb4beca8024d6eddef6d22a46ed1d9812bf3d24ff43ccbce".to_owned(),
        repository: Some("freikin/dawarich".to_owned()),
        tag: Some("latest".to_owned()),
        size: "470MB".to_owned(),
        shared_size: "420.8MB".to_owned(),
        unique_size: "49.49MB".to_owned(),
        created: "2026-11-16 12:24:00 -0800 PST".to_owned(),
        container_count: Some(1),
        containers: names(&["dawarich_app"]),
    };
    assert_eq!(read.images[0], dawarich);
    // An image with no name is found by the digits of its identifier, and the name a link
    // of another container gives (`web/db`) is not the container's own.
    let unnamed = &read.images[1];
    assert_eq!((&unnamed.repository, &unnamed.tag), (&None, &None));
    assert_eq!(unnamed.containers, ["old-db"]);
    // A container that names a repository with no tag uses its tag `latest`.
    assert_eq!(read.images[2].containers, ["cache"]);
    assert_eq!(read.images.len(), 3);
}

#[test]
fn host_020_a_count_that_docker_does_not_give_and_a_short_identifier_hide_no_row() {
    let text = SIZES.replace(r#""count":"1""#, r#""count":"N/A""#).replace(
        "sha256:f59abe656b692d901e184be3445e084786602108599d2a51d229c0f5a48c71bf",
        "f59abe656b69",
    );
    let users = USERS.replace(
        r#""image":"f59abe656b69""#,
        r#""image":"sha256:f59abe656b692d901e184be3445e084786602108599d2a51d229c0f5a48c71bf""#,
    );
    let read = storage::storage(&output(&format!("{text}{users}"), 0)).unwrap();
    assert_eq!(read.images.len(), 3);
    assert_eq!(read.images[0].container_count, None);
    assert_eq!(read.volumes[0].container_count, None);
    // An older Docker writes 12 digits of the identifier; the container names all 64.
    assert_eq!(read.images[1].containers, ["old-db"]);
}

#[test]
fn host_020_a_container_that_names_a_digest_is_found_by_the_identifier_only() {
    // `redis@sha256:...` was made from an older image; the name `redis` is now of a new one.
    let users = USERS.replace(
        r#""image":"redis""#,
        r#""image":"redis@sha256:0000000000000000000000000000000000000000000000000000000000000000""#,
    );
    let read = storage::storage(&output(&format!("{SIZES}{users}"), 0)).unwrap();
    assert_eq!(read.images[2].containers, [] as [String; 0]);
}

#[test]
fn host_020_each_volume_has_its_size_and_the_containers_that_mount_it() {
    let read = storage::storage(&output(&format!("{SIZES}{USERS}"), 0)).unwrap();
    let used = Volume {
        name: "app-data".to_owned(),
        driver: "local".to_owned(),
        size: "232.9MB".to_owned(),
        container_count: Some(1),
        // `app-data-old` of the container `cache` is a different volume.
        containers: names(&["dawarich_app"]),
    };
    assert_eq!(read.volumes[0], used);
    assert_eq!(
        (
            read.volumes[1].container_count,
            read.volumes[1].containers.len()
        ),
        (Some(0), 0)
    );
}

#[test]
fn host_020_the_space_a_clean_up_can_free_is_what_docker_says() {
    let read = storage::storage(&output(&format!("{SIZES}{USERS}"), 0)).unwrap();
    let images = DiskUsage {
        kind: "Images".to_owned(),
        total_count: 3,
        active_count: 3,
        size: "470.4MB".to_owned(),
        reclaimable: "49.63MB (10%)".to_owned(),
    };
    assert_eq!(read.usage[0], images);
    assert_eq!(read.usage[1].kind, "Local Volumes");
}

#[test]
fn host_020_each_network_has_the_containers_on_it() {
    let read = storage::networks(&output(&format!("{NETWORKS}{USERS}"), 0)).unwrap();
    let bridge = Network {
        id: "479212f7b5c312875879801adcc1f8e4d881272a6fe7cc84643b8236039c2e95".to_owned(),
        name: "bridge".to_owned(),
        driver: "bridge".to_owned(),
        scope: "local".to_owned(),
        internal: false,
        containers: names(&["dawarich_app", "old-db"]),
    };
    assert_eq!(read[0], bridge);
    assert_eq!((read[1].internal, read[1].containers.len()), (true, 0));
}

#[test]
fn host_020_a_device_with_nothing_has_empty_lists_and_a_failed_answer_is_an_error() {
    let empty = storage::storage(&output("", 0)).unwrap();
    assert_eq!(
        (empty.images, empty.volumes, empty.usage),
        (vec![], vec![], vec![])
    );
    assert_eq!(storage::networks(&output("", 0)).unwrap(), []);
    for read in [
        storage::storage(&output(SECRET, 0)),
        storage::storage(&output(NETWORKS, 0)),
    ] {
        assert!(matches!(read, Err(Error::Unreadable)), "{read:?}");
    }
    assert!(matches!(
        storage::networks(&output(SIZES, 0)),
        Err(Error::Unreadable)
    ));
    assert!(matches!(
        storage::networks(&output(SECRET, 127)),
        Err(Error::NoDocker)
    ));
    assert!(matches!(
        storage::storage(&output(SECRET, 1)),
        Err(Error::DockerFailed)
    ));
    let cut = CommandOutput {
        cut: true,
        ..output(SIZES, 0)
    };
    assert!(matches!(storage::storage(&cut), Err(Error::Unreadable)));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn host_020_both_reads_run_fixed_read_only_docker_commands_and_ask_for_no_label(
    pool: PgPool,
) {
    let bench = device(&pool, exiting(USERS, 0)).await;
    let policy = policy();
    let now = crate::fleet::ssh::tests::fixture::now();
    let read = storage(reach(&bench, &policy), bench.device.id, now)
        .await
        .unwrap();
    assert_eq!(read.images, []);
    let read = networks(reach(&bench, &policy), bench.device.id, now)
        .await
        .unwrap();
    assert_eq!(read, []);
    let heard = execs(&bench);
    assert_eq!(heard.len(), 2);
    let users = concat!(
        r#" && docker ps -a --no-trunc --format '{"kind":"container","names":{{json .Names}},"#,
        r#""image":{{json .Image}},"mounts":{{json .Mounts}},"networks":{{json .Networks}}}'"#,
    );
    assert!(
        heard[0].starts_with("docker system df -v --format '{{range .Images}}"),
        "{heard:?}"
    );
    assert!(
        heard[0].contains("' && docker system df --format '{\"kind\":\"usage\","),
        "{heard:?}"
    );
    assert_eq!(
        heard[1],
        format!(
            "{}{users}",
            concat!(
                r#"docker network ls --no-trunc --format '{"kind":"network","id":{{json .ID}},"#,
                r#""name":{{json .Name}},"driver":{{json .Driver}},"scope":{{json .Scope}},"#,
                r#""internal":{{json .Internal}}}'"#,
            )
        )
    );
    for line in &heard {
        assert!(line.ends_with(users), "{line}");
        assert!(!line.contains("Label"), "{line}");
        // Every docker word of the line is one of three that only read.
        let read_only = line.matches("docker ").count()
            == line.matches("docker system df ").count()
                + line.matches("docker network ls ").count()
                + line.matches("docker ps ").count();
        assert!(read_only, "{line}");
    }
}
