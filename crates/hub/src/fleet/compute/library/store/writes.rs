//! The writes of the library. Each runs on the caller's connection, inside its transaction.

use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

use super::super::{
    Error,
    model::{Facts, Variant},
};

fn signed(value: Option<u32>) -> Option<i32> {
    value.and_then(|each| i32::try_from(each).ok())
}

/// Stores the model with these facts and returns its id. A model of this name keeps its id,
/// its time of addition and its publisher when the new facts give none; its other facts are
/// replaced by those read now.
pub(in super::super) async fn save_model(
    conn: &mut PgConnection,
    name: &str,
    facts: &Facts,
    now: DateTime<Utc>,
) -> Result<Uuid, Error> {
    let capabilities: Vec<&str> = facts
        .capabilities
        .iter()
        .map(|each| each.as_str())
        .collect();
    let shape = facts.shape;
    Ok(sqlx::query_scalar(
        "INSERT INTO compute.models (id, name, family, parameter_count, context_length, layers, \
             hidden_size, attention_heads, key_value_heads, capabilities, licence, publisher, \
             added_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) \
         ON CONFLICT (name) DO UPDATE SET family = EXCLUDED.family, \
             parameter_count = EXCLUDED.parameter_count, \
             context_length = EXCLUDED.context_length, layers = EXCLUDED.layers, \
             hidden_size = EXCLUDED.hidden_size, attention_heads = EXCLUDED.attention_heads, \
             key_value_heads = EXCLUDED.key_value_heads, capabilities = EXCLUDED.capabilities, \
             licence = EXCLUDED.licence, \
             publisher = coalesce(EXCLUDED.publisher, compute.models.publisher) \
         RETURNING id",
    )
    .bind(Uuid::now_v7())
    .bind(name)
    .bind(facts.family.as_deref())
    .bind(
        facts
            .parameter_count
            .and_then(|each| i64::try_from(each).ok()),
    )
    .bind(signed(facts.context_length))
    .bind(signed(shape.layers))
    .bind(signed(shape.hidden_size))
    .bind(signed(shape.attention_heads))
    .bind(signed(shape.key_value_heads))
    .bind(capabilities)
    .bind(facts.licence.as_deref())
    .bind(facts.publisher.as_deref())
    .bind(now)
    .fetch_one(conn)
    .await?)
}

/// Stores the variant of the model, replacing a variant of the same tag and its files.
pub(in super::super) async fn save_variant(
    conn: &mut PgConnection,
    model: Uuid,
    variant: &Variant,
) -> Result<(), Error> {
    sqlx::query("DELETE FROM compute.variants WHERE model_id = $1 AND tag = $2")
        .bind(model)
        .bind(&variant.tag)
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        "INSERT INTO compute.variants (model_id, tag, format, quantisation, repository, revision) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(model)
    .bind(&variant.tag)
    .bind(variant.format.as_str())
    .bind(&variant.quantisation)
    .bind(&variant.repository)
    .bind(&variant.revision)
    .execute(&mut *conn)
    .await?;
    let paths: Vec<&str> = variant
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect();
    let sizes: Vec<i64> = variant
        .files
        .iter()
        .map(|file| i64::try_from(file.size_bytes).unwrap_or(i64::MAX))
        .collect();
    let hashes: Vec<&str> = variant
        .files
        .iter()
        .map(|file| file.sha256.as_str())
        .collect();
    sqlx::query(
        "INSERT INTO compute.variant_files (model_id, tag, path, size_bytes, sha256) \
         SELECT $1, $2, * FROM unnest($3::text[], $4::bigint[], $5::text[])",
    )
    .bind(model)
    .bind(&variant.tag)
    .bind(paths)
    .bind(sizes)
    .bind(hashes)
    .execute(conn)
    .await?;
    Ok(())
}

/// Points the alias at the model; returns the name of the model it pointed at before.
pub(in super::super) async fn set_alias(
    conn: &mut PgConnection,
    alias: &str,
    model: Uuid,
) -> Result<Option<String>, Error> {
    let before: Option<String> = sqlx::query_scalar(
        "SELECT m.name FROM compute.aliases a JOIN compute.models m ON m.id = a.model_id \
         WHERE a.alias = $1 FOR UPDATE OF a",
    )
    .bind(alias)
    .fetch_optional(&mut *conn)
    .await?;
    sqlx::query(
        "INSERT INTO compute.aliases (alias, model_id) VALUES ($1, $2) \
         ON CONFLICT (alias) DO UPDATE SET model_id = EXCLUDED.model_id",
    )
    .bind(alias)
    .bind(model)
    .execute(conn)
    .await?;
    Ok(before)
}
