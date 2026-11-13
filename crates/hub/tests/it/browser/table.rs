//! The token kinds in `access.api_tokens` (K2 0010-0029) and how long each is kept (A-5).

use anima_hub::access;
use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;

use crate::hub::{set_operator, start};

/// One token row of the fixture operator.
struct Row {
    fill: u8,
    kind: &'static str,
    scope: &'static str,
    name: Option<&'static str>,
    prefix: Option<&'static str>,
    expires_at: DateTime<Utc>,
}

impl Row {
    fn of(fill: u8, kind: &'static str) -> Self {
        let key = kind == "api_key";
        Self {
            fill,
            kind,
            scope: "write",
            name: key.then_some("deploy"),
            prefix: key.then_some("anima_AAAAAA"),
            expires_at: start() + TimeDelta::days(1),
        }
    }
}

async fn insert(pool: &PgPool, row: &Row) -> Result<(), sqlx::Error> {
    sqlx::query(
        "insert into access.api_tokens (id, operator_id, token_hash, kind, name, display_prefix, \
         scope, expires_at, created_at, made_for) \
         select gen_random_uuid(), id, $1, $2, $3, $4, $5, $6, $7, \
         case when $2 = 'api_key' then 'user' end from access.operators",
    )
    .bind(vec![row.fill; 32])
    .bind(row.kind)
    .bind(row.name)
    .bind(row.prefix)
    .bind(row.scope)
    .bind(row.expires_at)
    .bind(start())
    .execute(pool)
    .await
    .map(|_| ())
}

#[sqlx::test(migrations = "./migrations")]
async fn d_3_d_23_each_kind_keeps_its_rules_in_the_table(pool: PgPool) {
    set_operator(&pool).await;
    let accepted = [
        Row::of(1, "login"),
        Row::of(2, "browser"),
        Row::of(3, "api_key"),
        Row {
            scope: "read",
            name: Some("reader"),
            ..Row::of(4, "api_key")
        },
        Row {
            name: Some("n"),
            ..Row::of(5, "api_key")
        },
    ];
    for row in &accepted {
        insert(&pool, row)
            .await
            .unwrap_or_else(|e| panic!("{} {e}", row.kind));
    }
    let long_name: &'static str = "n".repeat(65).leak();
    let max_name: &'static str = "m".repeat(64).leak();
    insert(
        &pool,
        &Row {
            name: Some(max_name),
            ..Row::of(6, "api_key")
        },
    )
    .await
    .unwrap();
    let refused = [
        (
            "read browser",
            Row {
                scope: "read",
                ..Row::of(10, "browser")
            },
        ),
        (
            "read login",
            Row {
                scope: "read",
                ..Row::of(11, "login")
            },
        ),
        ("unknown kind", Row::of(12, "share")),
        (
            "key without name",
            Row {
                name: None,
                ..Row::of(13, "api_key")
            },
        ),
        (
            "key with empty name",
            Row {
                name: Some(""),
                ..Row::of(14, "api_key")
            },
        ),
        (
            "key with a 65-character name",
            Row {
                name: Some(long_name),
                ..Row::of(15, "api_key")
            },
        ),
        (
            "key without prefix",
            Row {
                prefix: None,
                ..Row::of(16, "api_key")
            },
        ),
        (
            "key with short prefix",
            Row {
                prefix: Some("anima_AAAAA"),
                ..Row::of(17, "api_key")
            },
        ),
        (
            "key with long prefix",
            Row {
                prefix: Some("anima_AAAAAAA"),
                ..Row::of(18, "api_key")
            },
        ),
        ("duplicate key name", Row::of(19, "api_key")),
        (
            "browser with a name",
            Row {
                name: Some("x"),
                ..Row::of(20, "browser")
            },
        ),
        (
            "login with a prefix",
            Row {
                prefix: Some("anima_AAAAAA"),
                ..Row::of(21, "login")
            },
        ),
    ];
    for (case, row) in &refused {
        assert!(insert(&pool, row).await.is_err(), "{case}");
    }
    let default_kind: String = sqlx::query_scalar(
        "insert into access.api_tokens (id, operator_id, token_hash, scope, expires_at, created_at) \
         select gen_random_uuid(), id, $1, 'write', $2, $2 from access.operators returning kind",
    )
    .bind(vec![30_u8; 32])
    .bind(start())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(default_kind, "login");
}

#[sqlx::test(migrations = "./migrations")]
async fn a_5_prune_drops_sign_ins_at_expiry_and_keys_30_days_later(pool: PgPool) {
    set_operator(&pool).await;
    let now = start() + TimeDelta::days(100);
    let at = |days: i64| now - TimeDelta::days(days);
    let rows = [
        Row {
            expires_at: now,
            ..Row::of(1, "login")
        },
        Row {
            expires_at: now,
            ..Row::of(2, "browser")
        },
        Row {
            expires_at: now + TimeDelta::seconds(1),
            ..Row::of(3, "browser")
        },
        Row {
            expires_at: at(30),
            name: Some("old"),
            ..Row::of(4, "api_key")
        },
        Row {
            expires_at: at(30) + TimeDelta::seconds(1),
            name: Some("recent"),
            ..Row::of(5, "api_key")
        },
        Row {
            expires_at: now,
            name: Some("today"),
            ..Row::of(6, "api_key")
        },
    ];
    for row in &rows {
        insert(&pool, row).await.unwrap();
    }

    let pruned = access::prune(&pool, now).await.unwrap();

    assert_eq!(pruned.tokens, 3);
    let mut left: Vec<String> =
        sqlx::query_scalar("select kind || ':' || coalesce(name, '') from access.api_tokens")
            .fetch_all(&pool)
            .await
            .unwrap();
    left.sort();
    assert_eq!(left, ["api_key:recent", "api_key:today", "browser:"]);
}
