//! `DeviceSort.STATUS` and `UPTIME`, and their cursors (CHK-004, CHK-011, API-006).

use serde_json::Value;
use sqlx::PgPool;

use super::{BY_STATUS, BY_UPTIME, names, seed_standing};
use crate::devices::harness::{data, only_error, schema_for, validation_fields};

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../fixtures", scripts("devices", "checks"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

fn page(sort: &str, first: usize, after: Option<&str>, filter: &str) -> String {
    let after = after.map_or_else(String::new, |cursor| format!(r#", after: "{cursor}""#));
    format!(
        "{{ devices(sort: {sort}, first: {first}{after}, filter: {filter}) {{ totalCount \
         pageInfo {{ hasNextPage endCursor }} edges {{ cursor node {{ name }} }} }} }}"
    )
}

/// Every page of the sort, following `endCursor`, as (names, totals, page sizes).
async fn walk(pool: &PgPool, sort: &str, first: usize, filter: &str) -> (Vec<String>, Vec<usize>) {
    let schema = schema_for(pool);
    let (mut all, mut sizes, mut after) = (Vec::new(), Vec::new(), None::<String>);
    loop {
        let result = data(&schema, &page(sort, first, after.as_deref(), filter)).await;
        let connection = &result["devices"];
        let shown = names(connection);
        sizes.push(shown.len());
        all.extend(shown.into_iter().map(str::to_owned));
        for edge in connection["edges"].as_array().unwrap() {
            assert!(edge["cursor"].as_str().unwrap().len() > 3);
        }
        if connection["pageInfo"]["hasNextPage"] != true {
            return (all, sizes);
        }
        after = connection["pageInfo"]["endCursor"]
            .as_str()
            .map(str::to_owned);
    }
}

async fn end_cursor(pool: &PgPool, sort: &str) -> String {
    let result = data(&schema_for(pool), &page(sort, 2, None, "{}")).await;
    result["devices"]["pageInfo"]["endCursor"]
        .as_str()
        .unwrap()
        .to_owned()
}

async fn refused_cursor(pool: &PgPool, sort: &str, cursor: &str) -> Value {
    only_error(&schema_for(pool), &page(sort, 2, Some(cursor), "{}")).await
}

fixture_test! {
    async fn chk_004_status_sort_lists_worst_first_then_by_name(pool: PgPool) {
        seed_standing(&pool).await;
        let result = data(&schema_for(&pool), &page("STATUS", 200, None, "{}")).await;
        assert_eq!(names(&result["devices"]), BY_STATUS);
        assert_eq!(result["devices"]["totalCount"], 13);
    }

    async fn chk_011_uptime_sort_lists_lowest_first_with_no_figure_last_then_by_name(pool: PgPool) {
        seed_standing(&pool).await;
        let result = data(&schema_for(&pool), &page("UPTIME", 200, None, "{}")).await;
        assert_eq!(names(&result["devices"]), BY_UPTIME);
    }

    async fn api_006_name_sort_keeps_the_name_order_with_a_status_filter(pool: PgPool) {
        seed_standing(&pool).await;
        let result = data(&schema_for(&pool), &page("NAME", 200, None, "{ status: [UP, DOWN] }")).await;
        assert_eq!(names(&result["devices"]), ["backup-02", "ollama", "pihole", "pve-01", "truenas"]);
    }

    async fn api_006_paging_a_status_sort_visits_every_device_once_in_order(pool: PgPool) {
        seed_standing(&pool).await;
        // Pages of 4 split the Unknown block, and the last page is short.
        let (visited, sizes) = walk(&pool, "STATUS", 4, "{}").await;
        assert_eq!(visited, BY_STATUS);
        assert_eq!(sizes, [4, 4, 4, 1]);
        let (single, _) = walk(&pool, "STATUS", 1, "{}").await;
        assert_eq!(single, BY_STATUS);
    }

    async fn api_006_paging_an_uptime_sort_visits_every_device_once_in_order(pool: PgPool) {
        seed_standing(&pool).await;
        // Pages of 5 end on the last device with a figure and inside the block with none.
        let (visited, sizes) = walk(&pool, "UPTIME", 5, "{}").await;
        assert_eq!(visited, BY_UPTIME);
        assert_eq!(sizes, [5, 5, 3]);
        let (single, _) = walk(&pool, "UPTIME", 1, "{}").await;
        assert_eq!(single, BY_UPTIME);
    }

    async fn api_006_paging_a_status_sort_under_a_filter_counts_the_filtered_set(pool: PgPool) {
        seed_standing(&pool).await;
        let (visited, sizes) = walk(&pool, "STATUS", 2, "{ status: [UP, DOWN, UNKNOWN] }").await;
        assert_eq!(visited.len(), 11);
        assert_eq!(&visited[..3], ["pve-01", "truenas", "home-assistant"]);
        assert_eq!(sizes, [2, 2, 2, 2, 2, 1]);
        let first = data(&schema_for(&pool), &page("STATUS", 2, None, "{ status: [UP] }")).await;
        assert_eq!(first["devices"]["totalCount"], 3);
    }

    async fn api_006_a_status_cursor_is_refused_under_another_order(pool: PgPool) {
        seed_standing(&pool).await;
        let cursor = end_cursor(&pool, "STATUS").await;
        for sort in ["UPTIME", "NAME"] {
            let error = refused_cursor(&pool, sort, &cursor).await;
            assert_eq!(validation_fields(&error), ["after"], "{sort}");
        }
        assert!(only_cursor_works(&pool, "STATUS", &cursor).await);
    }

    async fn api_006_an_uptime_cursor_is_refused_under_another_order(pool: PgPool) {
        seed_standing(&pool).await;
        let cursor = end_cursor(&pool, "UPTIME").await;
        for sort in ["STATUS", "NAME"] {
            let error = refused_cursor(&pool, sort, &cursor).await;
            assert_eq!(validation_fields(&error), ["after"], "{sort}");
        }
        assert!(only_cursor_works(&pool, "UPTIME", &cursor).await);
    }

    async fn api_006_a_name_cursor_is_refused_under_a_status_or_uptime_order(pool: PgPool) {
        seed_standing(&pool).await;
        let cursor = end_cursor(&pool, "NAME").await;
        for sort in ["STATUS", "UPTIME"] {
            let error = refused_cursor(&pool, sort, &cursor).await;
            assert_eq!(validation_fields(&error), ["after"], "{sort}");
        }
    }

    async fn api_006_a_damaged_sorted_cursor_is_validation_on_after(pool: PgPool) {
        seed_standing(&pool).await;
        let cursor = end_cursor(&pool, "STATUS").await;
        let name = cursor.rsplit_once('.').unwrap().1;
        for damaged in [
            "s1.status.zz.n1.00".to_owned(),
            "s1.status.0000000000000001".to_owned(),
            "s1.status.1.n1.abcd".to_owned(),
            format!("s1.status.00000000000000zz.n1.{name}"),
            "s1.rank.0000000000000001.n1.6162".to_owned(),
            "s1.status.0000000000000001.n1.zz".to_owned(),
        ] {
            let error = refused_cursor(&pool, "STATUS", &damaged).await;
            assert_eq!(validation_fields(&error), ["after"], "{damaged}");
        }
    }

    async fn api_006_every_bad_argument_is_reported_with_a_bad_status_cursor(pool: PgPool) {
        let error = only_error(
            &schema_for(&pool),
            r#"{ devices(sort: STATUS, first: 0, after: "n1.00", filter: { siteId: "x" }) { totalCount } }"#,
        )
        .await;
        assert_eq!(validation_fields(&error), ["first", "after", "siteId"]);
    }
}

/// Whether the cursor continues the walk under its own sort, with the next page non-empty.
async fn only_cursor_works(pool: &PgPool, sort: &str, cursor: &str) -> bool {
    let result = data(&schema_for(pool), &page(sort, 2, Some(cursor), "{}")).await;
    !names(&result["devices"]).is_empty()
}
