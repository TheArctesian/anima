//! R-044: the STATUS order is worst first (DOWN, DEGRADED, UNKNOWN, MAINTENANCE, UP) and the
//! UPTIME order is lowest 24 h uptime first with no figure last; both break ties by name in
//! byte order, so an upper-case name comes before a lower-case one.

use sqlx::PgPool;

use super::{names, seed_standing};
use crate::devices::harness::{data, schema_for};

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices", "checks"))
)]
async fn r_044_status_and_uptime_orders_follow_k1_with_names_in_byte_order(pool: PgPool) {
    seed_standing(&pool).await;
    for (old, new) in [
        ("pihole", "Zeta"),
        ("win-desktop", "Win-desktop"),
        ("mac-office-2", "Mac-office-2"),
    ] {
        sqlx::query("UPDATE devices.devices SET name = $2 WHERE name = $1")
            .bind(old)
            .bind(new)
            .execute(&pool)
            .await
            .unwrap();
    }
    let schema = schema_for(&pool);
    let list = |sort: &str| {
        format!("{{ devices(sort: {sort}, first: 200) {{ edges {{ node {{ name }} }} }} }}")
    };

    // Down, Degraded, Unknown (upper case first), Maintenance (jellyfin), Up (Zeta first).
    let by_status = data(&schema, &list("STATUS")).await;
    assert_eq!(
        names(&by_status["devices"]),
        [
            "pve-01",
            "truenas",
            "docker-host",
            "Mac-office-2",
            "Win-desktop",
            "home-assistant",
            "linux-pc-home",
            "linux-pc-office-2",
            "mac-office-1",
            "jellyfin",
            "Zeta",
            "backup-02",
            "ollama",
        ]
    );

    // 0.25, 0.5, 0.75, 1, then no figure: upper-case names first.
    let by_uptime = data(&schema, &list("UPTIME")).await;
    assert_eq!(
        names(&by_uptime["devices"]),
        [
            "truenas",
            "pve-01",
            "docker-host",
            "ollama",
            "Mac-office-2",
            "Win-desktop",
            "Zeta",
            "backup-02",
            "home-assistant",
            "jellyfin",
            "linux-pc-home",
            "linux-pc-office-2",
            "mac-office-1",
        ]
    );
}
