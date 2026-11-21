//! All SQL on schema `compute` for the library: I/O only. Time comes in as a parameter.
//! Writes take the caller's connection so the caller adds its audit entry in the same
//! transaction.

mod writes;

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor};
use uuid::Uuid;

pub(super) use writes::{save_model, save_variant, set_alias};

use super::{
    Error,
    capability::Capability,
    model::{Facts, Format, LayerShape, Model, Variant, VariantFile},
};

/// The approved organisations in name order.
pub(super) async fn sources(db: impl PgExecutor<'_>) -> Result<Vec<String>, Error> {
    Ok(sqlx::query_scalar(
        "SELECT organisation FROM compute.approved_sources ORDER BY lower(organisation)",
    )
    .fetch_all(db)
    .await?)
}

/// Adds an organisation; returns false when it was there already (in any case).
pub(super) async fn add_source(
    conn: &mut PgConnection,
    organisation: &str,
    now: DateTime<Utc>,
) -> Result<bool, Error> {
    let added = sqlx::query(
        "INSERT INTO compute.approved_sources (organisation, added_at) VALUES ($1, $2) \
         ON CONFLICT DO NOTHING",
    )
    .bind(organisation)
    .bind(now)
    .execute(conn)
    .await?;
    Ok(added.rows_affected() == 1)
}

/// The id and the name of the model with this name or alias.
pub(super) async fn resolve(
    db: impl PgExecutor<'_>,
    name: &str,
) -> Result<Option<(Uuid, String)>, Error> {
    Ok(sqlx::query_as(
        "SELECT m.id, m.name FROM compute.models m WHERE m.name = $1 \
         UNION ALL \
         SELECT m.id, m.name FROM compute.aliases a JOIN compute.models m ON m.id = a.model_id \
         WHERE a.alias = $1 LIMIT 1",
    )
    .bind(name)
    .fetch_optional(db)
    .await?)
}

#[derive(sqlx::FromRow)]
struct ModelRow {
    id: Uuid,
    name: String,
    family: Option<String>,
    parameter_count: Option<i64>,
    context_length: Option<i32>,
    layers: Option<i32>,
    hidden_size: Option<i32>,
    attention_heads: Option<i32>,
    key_value_heads: Option<i32>,
    capabilities: Vec<String>,
    licence: Option<String>,
    publisher: Option<String>,
    added_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct VariantRow {
    model_id: Uuid,
    tag: String,
    format: String,
    quantisation: String,
    repository: String,
    revision: String,
}

#[derive(sqlx::FromRow)]
struct FileRow {
    model_id: Uuid,
    tag: String,
    path: String,
    size_bytes: i64,
    sha256: String,
}

fn unsigned(value: Option<i32>) -> Result<Option<u32>, Error> {
    value
        .map(|each| u32::try_from(each).map_err(|_| Error::Unreadable))
        .transpose()
}

fn model(row: ModelRow, variants: Vec<Variant>, aliases: Vec<String>) -> Result<Model, Error> {
    let capabilities = row
        .capabilities
        .iter()
        .map(|text| Capability::from_stored(text).ok_or(Error::Unreadable))
        .collect::<Result<_, _>>()?;
    let facts = Facts {
        family: row.family,
        parameter_count: row
            .parameter_count
            .map(|each| u64::try_from(each).map_err(|_| Error::Unreadable))
            .transpose()?,
        context_length: unsigned(row.context_length)?,
        shape: LayerShape {
            layers: unsigned(row.layers)?,
            hidden_size: unsigned(row.hidden_size)?,
            attention_heads: unsigned(row.attention_heads)?,
            key_value_heads: unsigned(row.key_value_heads)?,
        },
        capabilities,
        licence: row.licence,
        publisher: row.publisher,
    };
    Ok(Model {
        name: row.name,
        facts,
        variants,
        aliases,
        added_at: row.added_at,
    })
}

/// The models with their variants, files and aliases, in name order: every model when `only`
/// is `None`. Four queries, whatever the count of models.
pub(super) async fn load(conn: &mut PgConnection, only: Option<Uuid>) -> Result<Vec<Model>, Error> {
    let filter = "WHERE $1::uuid IS NULL OR model_id = $1";
    let models: Vec<ModelRow> = sqlx::query_as(
        "SELECT * FROM compute.models WHERE $1::uuid IS NULL OR id = $1 ORDER BY name",
    )
    .bind(only)
    .fetch_all(&mut *conn)
    .await?;
    let variants: Vec<VariantRow> = sqlx::query_as(&format!(
        "SELECT model_id, tag, format, quantisation, repository, revision \
         FROM compute.variants {filter} ORDER BY model_id, tag"
    ))
    .bind(only)
    .fetch_all(&mut *conn)
    .await?;
    let files: Vec<FileRow> = sqlx::query_as(&format!(
        "SELECT model_id, tag, path, size_bytes, sha256 FROM compute.variant_files {filter} \
         ORDER BY model_id, tag, path"
    ))
    .bind(only)
    .fetch_all(&mut *conn)
    .await?;
    let aliases: Vec<(Uuid, String)> = sqlx::query_as(&format!(
        "SELECT model_id, alias FROM compute.aliases {filter} ORDER BY alias"
    ))
    .bind(only)
    .fetch_all(&mut *conn)
    .await?;
    assemble(models, variants, files, aliases)
}

fn assemble(
    models: Vec<ModelRow>,
    variants: Vec<VariantRow>,
    files: Vec<FileRow>,
    aliases: Vec<(Uuid, String)>,
) -> Result<Vec<Model>, Error> {
    let mut files_of: HashMap<(Uuid, String), Vec<VariantFile>> = HashMap::new();
    for row in files {
        files_of
            .entry((row.model_id, row.tag))
            .or_default()
            .push(VariantFile {
                path: row.path,
                size_bytes: u64::try_from(row.size_bytes).map_err(|_| Error::Unreadable)?,
                sha256: row.sha256,
            });
    }
    let mut variants_of: HashMap<Uuid, Vec<Variant>> = HashMap::new();
    for row in variants {
        let files = files_of
            .remove(&(row.model_id, row.tag.clone()))
            .unwrap_or_default();
        variants_of.entry(row.model_id).or_default().push(Variant {
            format: Format::from_stored(&row.format).ok_or(Error::Unreadable)?,
            tag: row.tag,
            quantisation: row.quantisation,
            repository: row.repository,
            revision: row.revision,
            files,
        });
    }
    let mut aliases_of: HashMap<Uuid, Vec<String>> = HashMap::new();
    for (id, alias) in aliases {
        aliases_of.entry(id).or_default().push(alias);
    }
    models
        .into_iter()
        .map(|row| {
            let id = row.id;
            model(
                row,
                variants_of.remove(&id).unwrap_or_default(),
                aliases_of.remove(&id).unwrap_or_default(),
            )
        })
        .collect()
}
