//! Labels: only the K7 names, no status, time or address, unchanged when values change, and
//! hostile names escaped (INT-002).

use std::collections::BTreeSet;

use chrono::TimeDelta;
use sqlx::PgPool;

use super::{
    HOME_SITE, PVE_01, PVE_01_PING, PVE_01_SSH, body, database_now, record, rig, set_statuses,
};

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../fixtures", scripts("devices", "checks"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

const CHECK_LABELS: [&str; 6] = [
    "check_id",
    "check_name",
    "check_type",
    "device_id",
    "device_name",
    "site",
];

/// The label block of every sample line, without the value.
fn label_sets(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| line.rsplit_once(' ').unwrap().0.to_owned())
        .collect()
}

/// The label names of a sample line whose values hold no quote, comma or equals sign.
fn names(line: &str) -> Vec<String> {
    let (_, block) = line.split_once('{').unwrap();
    let block = block.rsplit_once('}').unwrap().0;
    block
        .split("\",")
        .map(|pair| pair.split_once('=').unwrap().0.to_owned())
        .collect()
}

fixture_test! {
    async fn int_002_labels_are_the_k7_names_and_hold_no_status_time_or_address(pool: PgPool) {
        let now = database_now(&pool).await;
        record(&pool, PVE_01_SSH, now - TimeDelta::seconds(5), Some(25)).await;
        sqlx::query("UPDATE checks.checks SET resolved_ip = '10.10.0.2' WHERE id = $1::uuid")
            .bind(PVE_01_SSH)
            .execute(&pool)
            .await
            .unwrap();
        let (router, _) = rig(&pool, now);

        let text = body(&router).await;

        for line in text.lines().filter(|line| !line.starts_with('#')) {
            let found = names(line);
            if line.starts_with("anima_device_status{") {
                assert_eq!(found, ["device_id", "device_name", "site"], "{line}");
            } else if line.starts_with("anima_check_uptime_ratio{") {
                assert_eq!(found[..6], CHECK_LABELS, "{line}");
                assert_eq!(found[6], "window", "{line}");
                assert_eq!(found.len(), 7, "{line}");
            } else {
                assert_eq!(found, CHECK_LABELS, "{line}");
            }
            let labels = line.rsplit_once(' ').unwrap().0;
            let day = now.format("%Y-%m-%d").to_string();
            let leaks = ["10.10.0.2", "pve-01.home.lan", "\"up\"", "\"Up\"", "\"unknown\"", &day];
            for leak in leaks {
                assert!(!labels.contains(leak), "{leak} in {line}");
            }
        }
        let device = format!("device_id=\"{PVE_01}\",device_name=\"pve-01\",site=\"{HOME_SITE}\"");
        assert!(text.contains(&device));
    }

    async fn int_002_labels_do_not_change_when_status_and_values_change(pool: PgPool) {
        let now = database_now(&pool).await;
        let (router, clock) = rig(&pool, now);
        let before = body(&router).await;

        set_statuses(&pool, "down", &[(PVE_01_PING, "degraded")]).await;
        clock.advance(TimeDelta::seconds(10));
        let after = body(&router).await;

        assert_ne!(before, after);
        assert_eq!(label_sets(&before), label_sets(&after));
    }

    async fn int_002_hostile_names_are_cleaned_and_escaped(pool: PgPool) {
        let hostile = "evil\"} anima_check_up{check_id=\"x\"} 1\n\\\u{202E}\u{200B}";
        for table in ["devices.devices", "checks.checks"] {
            sqlx::query(&format!("UPDATE {table} SET name = $2 WHERE id = $1::uuid"))
                .bind(if table == "checks.checks" { PVE_01_PING } else { PVE_01 })
                .bind(hostile)
                .execute(&pool)
                .await
                .unwrap();
        }
        let (router, _) = rig(&pool, database_now(&pool).await);

        let text = body(&router).await;

        let escaped = r#"evil\"} anima_check_up{check_id=\"x\"} 1\\"#;
        let line = text
            .lines()
            .find(|line| line.starts_with("anima_check_status{") && line.contains(PVE_01_PING))
            .unwrap();
        assert!(line.contains(&format!("check_name=\"{escaped}\",check_type=\"icmp\"")), "{line}");
        assert!(line.contains(&format!("device_name=\"{escaped}\",site=")), "{line}");
        assert!(!text.contains('\u{202E}') && !text.contains('\u{200B}'));
        assert!(text.lines().all(|line| line.starts_with("# ") || line.starts_with("anima_")));
        assert!(!text.contains("anima_check_up{check_id=\"x\"}"));
    }
}
