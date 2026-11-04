use super::*;
use crate::devices::store;

fn names(devices: &[Device]) -> Vec<&str> {
    devices.iter().map(|device| device.name.as_str()).collect()
}

fn filter() -> DeviceFilter {
    DeviceFilter::default()
}

/// Walks every page of `size` and returns the names in visiting order.
async fn walk(pool: &sqlx::PgPool, filter: &DeviceFilter, size: usize) -> (Vec<String>, i64) {
    let mut seen = Vec::new();
    let mut after: Option<String> = None;
    loop {
        let (page, has_next, total) = store::list(pool, filter, size, after.as_deref())
            .await
            .unwrap();
        seen.extend(page.iter().map(|device| device.name.clone()));
        if !has_next {
            return (seen, total);
        }
        after = page.last().map(|device| device.name.clone());
    }
}

store_test! {
    async fn api_006_paging_by_two_visits_every_device_once_in_byte_order(pool: PgPool) {
        let (seen, total) = walk(&pool, &filter(), 2).await;
        let mut sorted = seen.clone();
        sorted.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
        assert_eq!(seen, sorted);
        assert_eq!(seen.len(), 13);
        assert_eq!(total, 13);
        let (last, has_next, total) = store::list(&pool, &filter(), 2, Some("pve-01")).await.unwrap();
        assert_eq!((names(&last), has_next, total), (vec!["truenas", "win-desktop"], false, 13));
        let (empty, has_next, total) = store::list(&pool, &filter(), 2, Some("zzz")).await.unwrap();
        assert_eq!((empty.len(), has_next, total), (0, false, 13));
    }

    async fn api_006_a_mixed_case_name_sorts_first_and_agrees_with_page_after(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();
        let device = NewDevice { name: "Zeta-box".into(), ..super::record("Zeta-box") };
        let id = device_id("0190a000-0000-7000-8002-0000000000f1");
        store::create_device(&mut conn, id, instant("2026-11-02T10:00:00Z"), &device).await.unwrap();
        let (all, ..) = store::list(&pool, &filter(), 200, None).await.unwrap();
        assert_eq!(all[0].name, "Zeta-box");
        let expected: Vec<_> = page_after(&all, |d| d.name.as_str(), Some("Zeta-box"), 3)
            .rows.into_iter().cloned().collect();
        let (page, ..) = store::list(&pool, &filter(), 3, Some("Zeta-box")).await.unwrap();
        assert_eq!(page, expected);
        assert_eq!(names(&page), ["backup-02", "docker-host", "home-assistant"]);
    }

    async fn api_006_total_counts_the_filtered_set_ignoring_the_cursor(pool: PgPool) {
        let by_site = DeviceFilter { site: Some(site_id(OFFICE_2)), ..filter() };
        let (page, has_next, total) = store::list(&pool, &by_site, 1, Some("linux-pc-office-2")).await.unwrap();
        assert_eq!((names(&page), has_next, total), (vec!["mac-office-2"], true, 3));
    }

    async fn dev_003_name_filter_is_a_case_insensitive_substring(pool: PgPool) {
        let by_name = DeviceFilter { name: Some("PC".into()), ..filter() };
        let (found, _, total) = store::list(&pool, &by_name, 50, None).await.unwrap();
        assert_eq!(names(&found), ["linux-pc-home", "linux-pc-office-2"]);
        assert_eq!(total, 2);
    }

    async fn dev_003_percent_underscore_and_backslash_in_the_name_filter_match_literally(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();
        let device = NewDevice { name: "100%_disk\\x".into(), ..super::record("p") };
        let id = device_id("0190a000-0000-7000-8002-0000000000f1");
        store::create_device(&mut conn, id, instant("2026-11-02T10:00:00Z"), &device).await.unwrap();
        for (input, expected) in [("%", 1), ("_", 1), ("\\", 1), ("0%_d", 1), ("0_%d", 0), ("xyz", 0)] {
            let by_name = DeviceFilter { name: Some(input.into()), ..filter() };
            let (_, _, total) = store::list(&pool, &by_name, 5, None).await.unwrap();
            assert_eq!(total, expected, "{input:?}");
        }
    }

    async fn dev_003_tags_filter_requires_all_given_tags(pool: PgPool) {
        let both = DeviceFilter { tags: vec!["vm".into(), "docker".into()], ..filter() };
        let (found, ..) = store::list(&pool, &both, 50, None).await.unwrap();
        assert_eq!(names(&found), ["docker-host"]);
        let one = DeviceFilter { tags: vec!["infra".into()], ..filter() };
        let (found, _, total) = store::list(&pool, &one, 50, None).await.unwrap();
        assert_eq!((names(&found), total), (vec!["pihole", "pve-01", "truenas"], 3));
        let none = DeviceFilter { tags: vec!["vm".into(), "nope".into()], ..filter() };
        assert_eq!(store::list(&pool, &none, 50, None).await.unwrap().2, 0);
    }

    async fn vpn_002_site_filter_lists_only_that_sites_devices(pool: PgPool) {
        let by_site = DeviceFilter { site: Some(site_id(OFFICE_1)), ..filter() };
        let (found, _, total) = store::list(&pool, &by_site, 50, None).await.unwrap();
        assert_eq!((names(&found), total), (vec!["mac-office-1"], 1));
    }

    async fn vpn_002_devices_by_sites_returns_only_those_sites_in_name_order(pool: PgPool) {
        let found = store::devices_by_sites(&pool, &[site_id(OFFICE_1), site_id(OFFICE_2)]).await.unwrap();
        assert_eq!(names(&found), ["linux-pc-office-2", "mac-office-1", "mac-office-2", "win-desktop"]);
        assert_eq!(store::devices_by_sites(&pool, &[]).await.unwrap().len(), 0);
    }

    async fn vpn_002_sites_are_listed_by_name_and_fetched_by_id(pool: PgPool) {
        let sites = store::list_sites(&pool).await.unwrap();
        let listed: Vec<_> = sites.iter().map(|site| site.name.as_str()).collect();
        assert_eq!(listed, ["home", "office-1", "office-2"]);
        let some = store::sites_by_ids(&pool, &[site_id(OFFICE_2), site_id(HOME)]).await.unwrap();
        assert_eq!(some.len(), 2);
        assert_eq!(some[0].subnet.to_string(), "10.10.0.0/24");
    }

    async fn k3_tags_are_distinct_and_in_byte_order(pool: PgPool) {
        let tags = store::tags(&pool).await.unwrap();
        assert_eq!(tags.len(), 12);
        assert!(tags.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(tags.first().map(String::as_str), Some("ai"));
    }

    async fn k3_in_maintenance_reads_the_flag_and_reports_a_missing_device(pool: PgPool) {
        assert!(store::in_maintenance(&pool, device_id(JELLYFIN)).await.unwrap());
        assert!(!store::in_maintenance(&pool, device_id(PVE_01)).await.unwrap());
        let missing = device_id("0190a000-0000-7000-8002-0000000000f1");
        assert!(matches!(store::in_maintenance(&pool, missing).await, Err(Error::NotFound(_))));
        assert!(matches!(store::get(&pool, missing).await, Err(Error::NotFound(Missing::Device(_)))));
    }
}
