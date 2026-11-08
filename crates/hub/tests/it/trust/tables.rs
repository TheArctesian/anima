//! Migration 0920 (D-31, DB-010): the accepted host keys and server certificates are in schema
//! `trust` and no longer in `sessions`, and the runtime role's rights moved with them: it may
//! read, insert and upsert, never delete, truncate or move a row to another device (R-016,
//! R-040). What those rights let the runtime role do is tested in `sessions::privileges` and
//! `sessions::tables::server_certificates`.

use sqlx::PgPool;

#[sqlx::test(migrations = "./migrations")]
async fn d_31_host_keys_and_server_certificates_are_in_schema_trust(pool: PgPool) {
    let tables: Vec<(String, String)> = sqlx::query_as(
        "select table_schema::text, table_name::text from information_schema.tables \
         where table_name in ('host_keys', 'server_certificates') \
         order by table_name, table_schema",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let expected = [("trust", "host_keys"), ("trust", "server_certificates")]
        .map(|(schema, table)| (schema.to_owned(), table.to_owned()));
    assert_eq!(tables, expected);
}

#[sqlx::test(migrations = "./migrations")]
async fn r_016_r_040_the_runtime_role_keeps_its_rights_on_the_moved_tables(pool: PgPool) {
    let mut checks = vec![
        (
            "has_schema_privilege('anima_runtime', 'trust', 'USAGE')".to_owned(),
            true,
        ),
        (
            "has_schema_privilege('anima_runtime', 'trust', 'CREATE')".to_owned(),
            false,
        ),
    ];
    for (table, column) in [
        ("trust.host_keys", "fingerprint_sha256"),
        ("trust.server_certificates", "fingerprint_sha256"),
    ] {
        let table_right =
            |right: &str| format!("has_table_privilege('anima_runtime', '{table}', '{right}')");
        let column_right = |column: &str| {
            format!("has_column_privilege('anima_runtime', '{table}', '{column}', 'UPDATE')")
        };
        checks.extend([
            (table_right("SELECT"), true),
            (table_right("INSERT"), true),
            (table_right("UPDATE"), false),
            (table_right("DELETE"), false),
            (table_right("TRUNCATE"), false),
            (column_right(column), true),
            (column_right("device_id"), false),
        ]);
    }
    let mut found = Vec::new();
    for (expression, expected) in &checks {
        let held: bool = sqlx::query_scalar(&format!("select {expression}"))
            .fetch_one(&pool)
            .await
            .unwrap();
        found.push((expression.clone(), held, *expected));
    }
    for (expression, held, expected) in found {
        assert_eq!(held, expected, "{expression}");
    }
}
