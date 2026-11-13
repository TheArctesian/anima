//! Creating, changing, deleting and pausing checks, in the caller's transaction, with each
//! refused constraint named as the field to fix (CHK-005, DB-011).

use anima_hub::{
    fleet::checks::{
        self, Check, CheckChange, CheckSettings, Definition, Error, Missing, NewCheck, PushToken,
        config::{CheckConfig, IpVersion},
    },
    ids::CheckId,
    status::Status,
};
use sqlx::{Connection as _, PgPool};

use super::{
    DOCKER_HOST, PVE_01, beat, create, database_now, device, icmp, new_check, record, sealer, vault,
};

macro_rules! devices_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../fixtures", scripts("devices"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

fn tcp(port: u16) -> Definition {
    Definition::Config(CheckConfig::Tcp {
        port,
        timeout_ms: 5000,
    })
}

fn group() -> Definition {
    Definition::Config(CheckConfig::Group)
}

/// The fields of a validation error, or the error itself.
fn fields(error: Error) -> Vec<&'static str> {
    match error {
        Error::Validation(errors) => errors.into_iter().map(|e| e.field).collect(),
        other => panic!("not a validation error: {other:?}"),
    }
}

async fn try_create(pool: &PgPool, new: NewCheck) -> Result<Check, Error> {
    let mut conn = pool.acquire().await.unwrap();
    checks::create_check(&mut conn, sealer(&vault()), new).await
}

async fn update(pool: &PgPool, id: CheckId, change: CheckChange) -> Result<Check, Error> {
    let mut conn = pool.acquire().await.unwrap();
    checks::update_check(&mut conn, sealer(&vault()), id, change).await
}

fn with_settings(settings: CheckSettings) -> NewCheck {
    NewCheck {
        settings,
        ..new_check(PVE_01, "Ranged", icmp())
    }
}

devices_test! {
    async fn spec_6_2_the_default_check_is_an_icmp_ping_with_the_contract_defaults(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();
        let id = checks::add_default_check(&mut conn, device(PVE_01)).await.unwrap();
        let added = checks::get(&pool, id).await.unwrap();
        assert_eq!(added.name, "Ping");
        assert_eq!(added.config, CheckConfig::Icmp { timeout_ms: 2000, ip_version: IpVersion::default() });
        assert_eq!(added.settings, CheckSettings::default());
        assert_eq!((added.paused, added.parent, added.state.status), (false, None, Status::Unknown));
        let stored: (i32, i32, i32, i32, bool) = sqlx::query_as(
            "SELECT interval_seconds, retries, retry_interval_seconds, degraded_threshold_ms, invert \
             FROM checks.checks WHERE id = $1",
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(stored, (60, 3, 20, 1000, false));
    }

    async fn spec_6_2_the_default_check_in_a_rolled_back_transaction_leaves_nothing(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();
        let mut tx = conn.begin().await.unwrap();
        checks::add_default_check(&mut tx, device(PVE_01)).await.unwrap();
        tx.rollback().await.unwrap();
        assert_eq!(checks::for_device(&pool, device(PVE_01)).await.unwrap().len(), 0);
    }

    async fn k3_for_device_lists_the_checks_in_byte_order_of_the_name(pool: PgPool) {
        for name in ["b", "SSH", "a", "Ping"] {
            create(&pool, new_check(PVE_01, name, icmp())).await;
        }
        create(&pool, new_check(DOCKER_HOST, "other", icmp())).await;
        let names: Vec<String> = checks::for_device(&pool, device(PVE_01))
            .await
            .unwrap()
            .into_iter()
            .map(|check| check.name)
            .collect();
        assert_eq!(names, ["Ping", "SSH", "a", "b"]);
    }

    async fn k2_names_are_trimmed_plain_and_unique_per_device(pool: PgPool) {
        let created = try_create(&pool, new_check(PVE_01, "  Web  ", icmp())).await.unwrap();
        assert_eq!(created.name, "Web");
        let refused = [
            new_check(PVE_01, "Web", icmp()),
            new_check(PVE_01, "   ", icmp()),
            new_check(PVE_01, "a\u{202E}b", icmp()),
            new_check(PVE_01, &"x".repeat(201), icmp()),
        ];
        for new in refused {
            assert_eq!(fields(try_create(&pool, new).await.unwrap_err()), ["name"]);
        }
        try_create(&pool, new_check(DOCKER_HOST, "Web", icmp())).await.unwrap();
    }

    async fn k2_each_out_of_range_setting_names_its_field(pool: PgPool) {
        let base = CheckSettings::default();
        let cases = [
            (CheckSettings { interval_seconds: 9, ..base }, "intervalSeconds"),
            (CheckSettings { interval_seconds: 86_401, ..base }, "intervalSeconds"),
            (CheckSettings { retries: 11, ..base }, "retries"),
            (CheckSettings { retry_interval_seconds: 4, ..base }, "retryIntervalSeconds"),
            (CheckSettings { degraded_threshold_ms: 0, ..base }, "degradedThresholdMs"),
        ];
        for (settings, field) in cases {
            let error = try_create(&pool, with_settings(settings)).await.unwrap_err();
            assert_eq!(fields(error), [field], "{settings:?}");
        }
        let edges = CheckSettings { interval_seconds: 10, retries: 0, retry_interval_seconds: 3600, degraded_threshold_ms: 600_000, invert: true };
        let created = try_create(&pool, with_settings(edges)).await.unwrap();
        assert_eq!(created.settings, edges);
    }

    async fn k2_the_configuration_is_validated_and_names_its_field(pool: PgPool) {
        let error = try_create(&pool, new_check(PVE_01, "Port", tcp(0))).await.unwrap_err();
        assert_eq!(fields(error), ["port"]);
        let http = Definition::Config(CheckConfig::Http(checks::config::HttpConfig::new("http://x/")));
        let error = try_create(&pool, new_check(PVE_01, "Bare", http)).await.unwrap_err();
        assert_eq!(fields(error), ["config"]);
        let error = try_create(&pool, new_check(PVE_01, " ", tcp(0))).await.unwrap_err();
        assert_eq!(fields(error), ["name", "port"]);
    }

    async fn k2_an_unknown_device_names_the_device_field(pool: PgPool) {
        let mut new = new_check(PVE_01, "Lost", icmp());
        new.device = uuid::Uuid::now_v7().into();
        assert_eq!(fields(try_create(&pool, new).await.unwrap_err()), ["deviceId"]);
    }

    async fn chk_004_a_parent_must_be_a_group_on_the_same_device(pool: PgPool) {
        let parent = create(&pool, new_check(PVE_01, "Group", group())).await;
        let plain = create(&pool, new_check(PVE_01, "Plain", icmp())).await;
        let elsewhere = create(&pool, new_check(DOCKER_HOST, "Group", group())).await;
        let child = |name: &str, parent: CheckId, definition: Definition| NewCheck {
            parent: Some(parent),
            ..new_check(PVE_01, name, definition)
        };
        let created = try_create(&pool, child("Child", parent, icmp())).await.unwrap();
        assert_eq!(created.parent, Some(parent));
        let parent_type: Option<String> =
            sqlx::query_scalar("SELECT parent_type FROM checks.checks WHERE id = $1")
                .bind(created.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(parent_type.as_deref(), Some("group"));
        for refused in [
            child("Under a plain check", plain, icmp()),
            child("Under another device", elsewhere, icmp()),
            child("Nested group", parent, group()),
        ] {
            assert_eq!(fields(try_create(&pool, refused).await.unwrap_err()), ["parentId"]);
        }
    }

    async fn api_003_update_changes_only_the_fields_given(pool: PgPool) {
        let parent = create(&pool, new_check(PVE_01, "Group", group())).await;
        let id = create(&pool, new_check(PVE_01, "Port", tcp(22))).await;
        let before = checks::get(&pool, id).await.unwrap();
        let renamed = update(&pool, id, CheckChange { name: Some(" SSH ".into()), ..CheckChange::default() }).await.unwrap();
        assert_eq!(renamed.name, "SSH");
        assert_eq!((renamed.config.clone(), renamed.settings), (before.config.clone(), before.settings));

        let changed = update(&pool, id, CheckChange {
            parent: Some(Some(parent)),
            definition: Some(tcp(2222)),
            retries: Some(0),
            invert: Some(true),
            ..CheckChange::default()
        }).await.unwrap();
        assert_eq!(changed.parent, Some(parent));
        assert_eq!(changed.config, CheckConfig::Tcp { port: 2222, timeout_ms: 5000 });
        assert_eq!((changed.settings.retries, changed.settings.invert, changed.settings.interval_seconds), (0, true, 60));
        assert_eq!(changed.name, "SSH");

        let kept = update(&pool, id, CheckChange { interval_seconds: Some(30), ..CheckChange::default() }).await.unwrap();
        assert_eq!(kept.parent, Some(parent));
        let left = update(&pool, id, CheckChange { parent: Some(None), ..CheckChange::default() }).await.unwrap();
        assert_eq!(left.parent, None);
        assert_eq!(left.settings.interval_seconds, 30);
    }

    async fn api_003_update_refuses_bad_fields_and_a_new_type(pool: PgPool) {
        let id = create(&pool, new_check(PVE_01, "Port", tcp(22))).await;
        create(&pool, new_check(PVE_01, "Taken", icmp())).await;
        let cases = [
            (CheckChange { definition: Some(icmp()), ..CheckChange::default() }, "config"),
            (CheckChange { name: Some("Taken".into()), ..CheckChange::default() }, "name"),
            (CheckChange { retries: Some(11), ..CheckChange::default() }, "retries"),
            (CheckChange { parent: Some(Some(id)), ..CheckChange::default() }, "parentId"),
        ];
        for (change, field) in cases {
            assert_eq!(fields(update(&pool, id, change).await.unwrap_err()), [field]);
        }
        assert_eq!(checks::get(&pool, id).await.unwrap().name, "Port");
        let gone: CheckId = uuid::Uuid::now_v7().into();
        let error = update(&pool, gone, CheckChange::default()).await.unwrap_err();
        assert!(matches!(error, Error::NotFound(Missing::Check(missing)) if missing == gone));
    }

    async fn chk_004_deleting_a_group_frees_its_children(pool: PgPool) {
        let parent = create(&pool, new_check(PVE_01, "Group", group())).await;
        let child = create(&pool, NewCheck { parent: Some(parent), ..new_check(PVE_01, "Child", icmp()) }).await;
        let mut conn = pool.acquire().await.unwrap();
        let deleted = checks::delete_check(&mut conn, parent).await.unwrap();
        assert_eq!((deleted.id, deleted.name.as_str()), (parent, "Group"));
        assert_eq!(checks::get(&pool, child).await.unwrap().parent, None);
        let error = checks::delete_check(&mut conn, parent).await.unwrap_err();
        assert!(matches!(error, Error::NotFound(Missing::Check(id)) if id == parent));
    }

    async fn chk_005_pausing_starts_the_state_over(pool: PgPool) {
        let id = create(&pool, new_check(PVE_01, "Paused", icmp())).await;
        let mut down = beat(database_now(&pool).await, Status::Down, None);
        down.state.failures = 4;
        record(&pool, id, &down).await;
        let mut conn = pool.acquire().await.unwrap();
        let paused = checks::set_paused(&mut conn, id, true).await.unwrap();
        assert!(paused.paused);
        assert_eq!((paused.state.status, paused.state.failures, paused.state.next_due_at), (Status::Unknown, 0, None));
        let resumed = checks::set_paused(&mut conn, id, false).await.unwrap();
        assert!(!resumed.paused);
        assert_eq!(resumed.state.next_due_at, None);
        let gone: CheckId = uuid::Uuid::now_v7().into();
        assert!(matches!(checks::set_paused(&mut conn, gone, true).await, Err(Error::NotFound(_))));
    }

    async fn sec_005_a_push_check_keeps_its_token_and_hides_it_from_debug(pool: PgPool) {
        let token = PushToken::from_random(&[9; 32]);
        let created = try_create(&pool, new_check(PVE_01, "Push", Definition::Push(token.clone()))).await.unwrap();
        assert_eq!(created.config, CheckConfig::Push);
        assert_eq!(created.push_token.as_ref().map(PushToken::expose), Some(token.expose()));
        assert_eq!(token.expose().len(), 43);
        assert!(!format!("{created:?}").contains(token.expose()));
        let bare = Definition::Config(CheckConfig::Push);
        let error = try_create(&pool, new_check(PVE_01, "Tokenless", bare)).await.unwrap_err();
        assert_eq!(fields(error), ["config"]);
    }

    async fn log_007_a_taken_push_token_is_refused_without_the_token(pool: PgPool) {
        let token = PushToken::from_random(&[7; 32]);
        try_create(&pool, new_check(PVE_01, "First", Definition::Push(token.clone()))).await.unwrap();
        let error = try_create(&pool, new_check(DOCKER_HOST, "Second", Definition::Push(token.clone()))).await.unwrap_err();
        assert!(matches!(error, Error::PushTokenTaken), "{error:?}");
        assert!(!format!("{error:?} {error}").contains(token.expose()));

        let other = PushToken::from_random(&[8; 32]);
        let id = try_create(&pool, new_check(DOCKER_HOST, "Other", Definition::Push(other))).await.unwrap().id;
        let change = CheckChange { definition: Some(Definition::Push(token.clone())), ..CheckChange::default() };
        let error = update(&pool, id, change).await.unwrap_err();
        assert!(matches!(error, Error::PushTokenTaken), "{error:?}");
        assert!(!format!("{error:?} {error}").contains(token.expose()));
    }
}
