//! `Query.maintenanceWindows` and the fields of `MaintenanceWindow` (MW-001 to MW-004, API-003).

use std::collections::BTreeSet;

use serde_json::{Value, json};
use sqlx::PgPool;

use super::harness::{FIELDS, create, data, instant, manual_input, schema_at, schema_for};

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../fixtures", scripts("maintenance"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

fn list_query() -> String {
    format!("{{ maintenanceWindows {{ {FIELDS} }} }}")
}

fn non_null_fields(window: &Value) -> BTreeSet<&str> {
    window
        .as_object()
        .unwrap()
        .iter()
        .filter(|(_, value)| !value.is_null())
        .map(|(key, _)| key.as_str())
        .collect()
}

fn set(extra: &[&'static str]) -> BTreeSet<&'static str> {
    ["id", "name", "kind", "timezone", "targets", "activeNow"]
        .into_iter()
        .chain(extra.iter().copied())
        .collect()
}

fixture_test! {
    async fn mw_004_the_k6_windows_are_listed_by_name_with_their_rule_fields(pool: PgPool) {
        let listed = data(&schema_for(&pool), &list_query()).await;
        let windows = listed["maintenanceWindows"].as_array().unwrap();
        assert_eq!(windows.len(), 2);

        assert_eq!(windows[0], json!({
            "id": "0190a000-0000-7000-8004-000000000002", "name": "Overnight backup",
            "kind": "RECURRING_WEEKDAY", "timezone": "Pacific/Auckland", "activeManual": null,
            "startsAt": null, "endsAt": null, "firstStartAt": null, "everyDays": null,
            "weekdays": ["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"], "daysOfMonth": null,
            "startTime": "23:30", "durationMinutes": 90,
            "targets": [{ "type": "TAG", "target": "storage" }], "activeNow": false,
        }));
        assert_eq!(windows[1]["name"], "Sunday updates");
        assert_eq!(windows[1]["weekdays"], json!(["SUN"]));
        assert_eq!(windows[1]["startTime"], "03:00");
        assert_eq!(windows[1]["durationMinutes"], 60);
        assert_eq!(windows[1]["targets"], json!([{ "type": "TAG", "target": "infra" }]));
    }

    async fn mw_002_active_now_follows_the_schema_clock_across_midnight(pool: PgPool) {
        // 4 November 2026 in Auckland is NZDT (+13).
        let cases = [
            ("2026-11-04T09:00:00Z", [false, false]), // 22:00
            ("2026-11-04T10:29:59Z", [false, false]), // 23:29:59
            ("2026-11-04T10:30:00Z", [true, false]),  // 23:30
            ("2026-11-04T11:59:59Z", [true, false]),  // 00:59:59 on the 5th
            ("2026-11-04T12:00:00Z", [false, false]), // 01:00
            ("2026-11-07T14:00:00Z", [false, true]),  // Sunday 03:00
            ("2026-11-07T14:59:59Z", [false, true]),
            ("2026-11-07T15:00:00Z", [false, false]),
        ];
        for (now, expected) in cases {
            let schema = schema_at(&pool, instant(now)).0;
            let listed = data(&schema, "{ maintenanceWindows { name activeNow } }").await;
            let states: Vec<bool> = listed["maintenanceWindows"].as_array().unwrap().iter()
                .map(|w| w["activeNow"].as_bool().unwrap()).collect();
            assert_eq!(states, expected, "at {now}");
        }
    }

    async fn mw_004_a_window_shows_only_the_fields_of_its_kind(pool: PgPool) {
        let schema = schema_for(&pool);
        let inputs = [
            manual_input("a manual"),
            json!({ "name": "b single", "kind": "SINGLE", "timezone": "UTC",
                    "startsAt": "2026-11-04T09:00:00Z", "endsAt": "2026-11-04T10:00:00Z",
                    "targets": [{ "type": "TAG", "target": "infra" }] }),
            json!({ "name": "c interval", "kind": "RECURRING_INTERVAL", "timezone": "UTC",
                    "firstStartAt": "2026-11-01T00:00:00Z", "everyDays": 3, "durationMinutes": 45,
                    "targets": [{ "type": "TAG", "target": "infra" }] }),
            json!({ "name": "d weekday", "kind": "RECURRING_WEEKDAY", "timezone": "UTC",
                    "weekdays": ["MON"], "startTime": "01:02", "durationMinutes": 5,
                    "targets": [{ "type": "TAG", "target": "infra" }] }),
            json!({ "name": "e day of month", "kind": "RECURRING_DAY_OF_MONTH", "timezone": "UTC",
                    "daysOfMonth": [1, -1], "startTime": "23:59", "durationMinutes": 10080,
                    "targets": [{ "type": "TAG", "target": "infra" }] }),
        ];
        for input in inputs {
            create(&schema, input).await;
        }
        let listed = data(&schema, &list_query()).await;
        let windows = listed["maintenanceWindows"].as_array().unwrap();
        let expected = [
            set(&["activeManual"]),
            set(&["startsAt", "endsAt"]),
            set(&["firstStartAt", "everyDays", "durationMinutes"]),
            set(&["weekdays", "startTime", "durationMinutes"]),
            set(&["daysOfMonth", "startTime", "durationMinutes"]),
        ];
        // The two K6 windows come first in byte order ("Overnight", "Sunday"), after "a" to "e".
        let ours: Vec<&Value> = windows.iter().filter(|w| w["name"].as_str().unwrap().len() > 1
            && w["name"].as_str().unwrap().as_bytes()[1] == b' ').collect();
        assert_eq!(ours.len(), 5);
        for (window, expected) in ours.iter().zip(expected) {
            assert_eq!(non_null_fields(window), expected, "{}", window["name"]);
        }
        assert_eq!(ours[0]["activeManual"], false);
        assert_eq!(ours[2]["everyDays"], 3);
        assert_eq!(ours[4]["daysOfMonth"], json!([1, -1]));
        assert_eq!(ours[4]["durationMinutes"], 10_080);
        assert_eq!(
            instant(ours[1]["startsAt"].as_str().unwrap()),
            instant("2026-11-04T09:00:00Z")
        );
    }

    async fn mw_003_a_window_lists_its_targets_by_type_and_value(pool: PgPool) {
        let schema = schema_for(&pool);
        let mut input = manual_input("targets");
        input["targets"] = json!([
            { "type": "TAG", "target": "zeta" },
            { "type": "DEVICE", "target": "0190a000-0000-7000-8002-000000000001" },
            { "type": "CHECK", "target": "0190a000-0000-7000-8003-000000000001" },
            { "type": "TAG", "target": "alpha" },
        ]);
        create(&schema, input).await;
        let listed = data(&schema, "{ maintenanceWindows { name targets { type target } } }").await;
        let window = listed["maintenanceWindows"].as_array().unwrap().iter()
            .find(|w| w["name"] == "targets").unwrap();
        assert_eq!(window["targets"], json!([
            { "type": "CHECK", "target": "0190a000-0000-7000-8003-000000000001" },
            { "type": "DEVICE", "target": "0190a000-0000-7000-8002-000000000001" },
            { "type": "TAG", "target": "alpha" },
            { "type": "TAG", "target": "zeta" },
        ]));
    }
}
