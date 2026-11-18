//! A hosted hub is a device of its own device list (hosted-mode.md HMD-045, R-137): the
//! serving hub adds the device `Hub` with the host `localhost` at its start, with one PUSH
//! check that it reports as Up, and a hub that is not hosted adds nothing.

use std::{
    fs::Permissions,
    os::unix::fs::PermissionsExt as _,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

use anima_hub::{fleet::checks, ids::DeviceId, status::Status};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::start::url_for;

/// How long the hub may take to start and report its check.
const DEADLINE: Duration = Duration::from_secs(30);
const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";
const LINK_SUBNET: &str = "fd41:4e49:4d41::/48";

/// Kills the hub when the test ends, also when it fails.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// The files of one hub and its database role, which lives as long as the test database.
struct Home {
    folder: tempfile::TempDir,
}

impl Home {
    async fn new(pool: &PgPool) -> Self {
        let login = format!("anima_test_{}_hub", Uuid::now_v7().simple());
        sqlx::query(&format!("create role {login} login in role anima_runtime"))
            .execute(pool)
            .await
            .unwrap();
        let database = pool.connect_options().get_database().unwrap().to_owned();
        let url = url_for(&database);
        let (scheme, rest) = url.split_once("://").unwrap();
        let host = rest.split_once('@').map_or(rest, |(_, host)| host);
        let folder = tempfile::tempdir().unwrap();
        // The folder of the link socket is the hub user's alone (NET-003).
        std::fs::set_permissions(folder.path(), Permissions::from_mode(0o700)).unwrap();
        let file = |name: &str| folder.path().join(name);
        std::fs::write(file("url"), format!("{scheme}://{login}@{host}")).unwrap();
        std::fs::write(file("key"), STANDARD.encode([3_u8; 32])).unwrap();
        std::fs::write(
            file("cloud_key"),
            "cloud-key-of-the-test-hub-0123456789abcdef\n",
        )
        .unwrap();
        Self { folder }
    }

    /// Starts `anima-hub` on a free loopback port, in the hosted mode or not.
    fn start(&self, hosted: bool) -> Running {
        let file = |name: &str| self.folder.path().join(name);
        let mut hub = Command::new(env!("CARGO_BIN_EXE_anima-hub"));
        hub.env_clear()
            .env("ANIMA_KEY_FILE", file("key"))
            .env("ANIMA_DATABASE_URL_FILE", file("url"))
            .env("ANIMA_LOG", "error")
            .env("ANIMA_LISTEN", "127.0.0.1:0")
            .stdout(Stdio::null())
            .stderr(Stdio::inherit());
        if hosted {
            hub.env("ANIMA_LINK_SOCKET", file("link.sock"))
                .env("ANIMA_LINK_CONTROL", file("control.sock"))
                .env("ANIMA_HOSTED_CLOUD_KEY_FILE", file("cloud_key"));
        }
        Running(hub.spawn().unwrap())
    }

    /// Runs the hub until `done` is true, then stops it.
    async fn run_until<F: Future<Output = bool>>(&self, hosted: bool, done: impl Fn() -> F) {
        let mut hub = self.start(hosted);
        let started = Instant::now();
        while !done().await {
            assert!(hub.0.try_wait().unwrap().is_none(), "the hub stopped");
            assert!(started.elapsed() < DEADLINE, "not before the deadline");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// One start of a hosted hub, to the report of its own check.
    async fn hosted_start(&self, pool: &PgPool) {
        let before = reports(pool).await;
        self.run_until(true, || async { reports(pool).await > before })
            .await;
    }
}

/// How many results the checks of the `localhost` devices have.
async fn reports(pool: &PgPool) -> i64 {
    sqlx::query_scalar(
        "select count(*) from checks.heartbeats h join checks.checks c on c.id = h.check_id \
         join devices.devices d on d.id = c.device_id where d.host = 'localhost'",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

/// The id, name, kind and operating system of each device with the host `localhost`.
async fn hub_devices(pool: &PgPool) -> Vec<(DeviceId, String, String, String)> {
    sqlx::query_as("select id, name, kind, os from devices.devices where host = 'localhost'")
        .fetch_all(pool)
        .await
        .unwrap()
}

/// The type and name of each check of the device.
async fn checks_of(pool: &PgPool, device: DeviceId) -> Vec<(String, String)> {
    sqlx::query_as("select type, name from checks.checks where device_id = $1")
        .bind(device)
        .fetch_all(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn hmd_045_a_hosted_hub_is_one_device_of_its_own_list_and_it_is_up(pool: PgPool) {
    let home = Home::new(&pool).await;
    home.hosted_start(&pool).await;

    let devices = hub_devices(&pool).await;
    let [(id, name, kind, os)] = devices.as_slice() else {
        panic!("not one device: {devices:?}");
    };
    assert_eq!(
        (name.as_str(), kind.as_str(), os.as_str()),
        ("Hub", "hub", "linux")
    );
    let site: (String, String) = sqlx::query_as(
        "select s.name, s.subnet::text from devices.sites s join devices.devices d \
         on d.site_id = s.id where d.id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(site, ("Link".to_owned(), LINK_SUBNET.to_owned()));
    let remote: (bool, bool, bool) = sqlx::query_as(
        "select s.enabled, r.enabled, v.enabled from devices.ssh_settings s \
         join devices.rdp_settings r using (device_id) \
         join devices.vnc_settings v using (device_id) where s.device_id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(remote, (false, false, false));
    let only = vec![("push".to_owned(), "Hub running".to_owned())];
    assert_eq!(checks_of(&pool, *id).await, only);
    let status = checks::device_status(&pool, *id, Utc::now()).await.unwrap();
    assert_eq!(status, Status::Up);
    // The hub did it, for no operator (LOG-014).
    let audit: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "select action, operator_name, actor_kind from events.audit_log \
         where target_type = 'device' and target_id = $1::text",
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    .unwrap();
    let by_hub = (
        "create".to_owned(),
        "Anima".to_owned(),
        Some("hub".to_owned()),
    );
    assert_eq!(audit, [by_hub]);

    // A second start finds the device and its check, and writes no second entry.
    home.hosted_start(&pool).await;
    assert_eq!(hub_devices(&pool).await, devices);
    assert_eq!(checks_of(&pool, *id).await, only);
    let entries: i64 = sqlx::query_scalar("select count(*) from events.audit_log")
        .fetch_one(&pool)
        .await
        .unwrap();
    // The site and the device, each one time.
    assert_eq!(entries, 2);
    let status = checks::device_status(&pool, *id, Utc::now()).await.unwrap();
    assert_eq!(status, Status::Up);
}

#[sqlx::test(migrations = "./migrations")]
async fn hmd_045_a_device_that_a_person_renamed_stays(pool: PgPool) {
    let home = Home::new(&pool).await;
    home.hosted_start(&pool).await;
    sqlx::query("update devices.devices set name = 'Rack hub' where host = 'localhost'")
        .execute(&pool)
        .await
        .unwrap();

    home.hosted_start(&pool).await;

    let devices = hub_devices(&pool).await;
    let names: Vec<&str> = devices.iter().map(|device| device.1.as_str()).collect();
    assert_eq!(names, ["Rack hub"]);
}

#[sqlx::test(migrations = "./migrations")]
async fn hmd_045_a_device_that_a_person_deleted_is_added_again_at_the_next_start(pool: PgPool) {
    let home = Home::new(&pool).await;
    home.hosted_start(&pool).await;
    let first = hub_devices(&pool).await[0].0;
    sqlx::query("delete from devices.devices where id = $1")
        .bind(first)
        .execute(&pool)
        .await
        .unwrap();

    home.hosted_start(&pool).await;

    let devices = hub_devices(&pool).await;
    let [(id, name, ..)] = devices.as_slice() else {
        panic!("not one device: {devices:?}");
    };
    assert_ne!(*id, first);
    assert_eq!(name, "Hub");
    let status = checks::device_status(&pool, *id, Utc::now()).await.unwrap();
    assert_eq!(status, Status::Up);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn hmd_045_a_taken_name_and_a_site_of_the_link_do_not_stop_the_start(pool: PgPool) {
    sqlx::query("update devices.devices set name = 'Hub' where id = $1::uuid")
        .bind(PVE_01)
        .execute(&pool)
        .await
        .unwrap();
    // The client makes the site of the link under this subnet; a person can rename it.
    let tunnel = Uuid::now_v7();
    sqlx::query("insert into devices.sites (id, name, subnet) values ($1, 'Tunnel', $2::cidr)")
        .bind(tunnel)
        .bind(LINK_SUBNET)
        .execute(&pool)
        .await
        .unwrap();
    let home = Home::new(&pool).await;

    home.hosted_start(&pool).await;

    let placed: Vec<(String, Uuid)> =
        sqlx::query_as("select name, site_id from devices.devices where host = 'localhost'")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(placed, [("Hub (hosted)".to_owned(), tunnel)]);
    let other: String = sqlx::query_scalar("select host from devices.devices where name = 'Hub'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_ne!(other, "localhost");
}

#[sqlx::test(migrations = "./migrations")]
async fn hmd_045_a_hub_that_is_not_hosted_adds_no_device(pool: PgPool) {
    let home = Home::new(&pool).await;
    // The first upkeep makes this partition, and the upkeep starts after the hub's own device.
    let upkeep_ran = || async {
        sqlx::query_scalar(
            "select to_regclass('events.event_log_' || to_char((now() at time zone 'UTC') \
             + interval '7 days', 'YYYYMMDD')) is not null",
        )
        .fetch_one(&pool)
        .await
        .unwrap()
    };

    assert!(!upkeep_ran().await);

    home.run_until(false, upkeep_ran).await;

    let devices: i64 = sqlx::query_scalar("select count(*) from devices.devices")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(devices, 0);
}
