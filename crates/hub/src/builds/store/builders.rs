//! SQL on `builds.builders`.

use std::net::IpAddr;

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor};

use crate::{
    builds::{
        Error,
        model::{Builder, Platform},
    },
    ids::{BuilderId, TokenId},
};

const SELECT: &str = "SELECT id, name, os, arch, client_addr, enrolled_at, last_seen_at \
                      FROM builds.builders ";

#[derive(sqlx::FromRow)]
struct Row {
    id: BuilderId,
    name: String,
    os: String,
    arch: String,
    client_addr: IpAddr,
    enrolled_at: DateTime<Utc>,
    last_seen_at: Option<DateTime<Utc>>,
}

impl TryFrom<Row> for Builder {
    type Error = Error;

    fn try_from(row: Row) -> Result<Self, Error> {
        Ok(Self {
            id: row.id,
            name: row.name,
            platform: Platform::from_stored(&row.os, &row.arch).ok_or(Error::Unreadable)?,
            client: row.client_addr,
            enrolled_at: row.enrolled_at,
            last_seen_at: row.last_seen_at,
        })
    }
}

pub(in crate::builds) async fn insert_builder(
    conn: &mut PgConnection,
    builder: &Builder,
    token: TokenId,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO builds.builders (id, name, os, arch, client_addr, token_id, enrolled_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(builder.id)
    .bind(&builder.name)
    .bind(builder.platform.os_str())
    .bind(builder.platform.arch_str())
    .bind(builder.client)
    .bind(token)
    .bind(builder.enrolled_at)
    .execute(conn)
    .await?;
    Ok(())
}

/// The builder that holds this token.
pub(in crate::builds) async fn builder_by_token(
    db: impl PgExecutor<'_>,
    token: TokenId,
) -> Result<Option<Builder>, Error> {
    let sql = format!("{SELECT}WHERE token_id = $1");
    let row: Option<Row> = sqlx::query_as(&sql).bind(token).fetch_optional(db).await?;
    row.map(Builder::try_from).transpose()
}

/// Records that the builder called at `now` (BLD-007).
pub(in crate::builds) async fn touch(
    db: impl PgExecutor<'_>,
    builder: BuilderId,
    now: DateTime<Utc>,
) -> Result<(), Error> {
    sqlx::query("UPDATE builds.builders SET last_seen_at = $2 WHERE id = $1")
        .bind(builder)
        .bind(now)
        .execute(db)
        .await?;
    Ok(())
}

/// Deletes the builder; returns it and its token, or `None` when there is no such builder.
pub(in crate::builds) async fn delete_builder(
    conn: &mut PgConnection,
    id: BuilderId,
) -> Result<Option<(Builder, TokenId)>, Error> {
    let row: Option<(
        BuilderId,
        String,
        String,
        String,
        IpAddr,
        DateTime<Utc>,
        TokenId,
    )> = sqlx::query_as(
        "DELETE FROM builds.builders WHERE id = $1 \
             RETURNING id, name, os, arch, client_addr, enrolled_at, token_id",
    )
    .bind(id)
    .fetch_optional(conn)
    .await?;
    row.map(|(id, name, os, arch, client_addr, enrolled_at, token)| {
        let row = Row {
            id,
            name,
            os,
            arch,
            client_addr,
            enrolled_at,
            last_seen_at: None,
        };
        Ok((Builder::try_from(row)?, token))
    })
    .transpose()
}

/// Every builder, in enrolment order.
pub(in crate::builds) async fn builders(db: impl PgExecutor<'_>) -> Result<Vec<Builder>, Error> {
    let sql = format!("{SELECT}ORDER BY enrolled_at, id");
    let rows: Vec<Row> = sqlx::query_as(&sql).fetch_all(db).await?;
    rows.into_iter().map(Builder::try_from).collect()
}
