//! The library GraphQL part: root `Query` and `Mutation` (K1; LIBM-001 to LIBM-008, SEL-003,
//! SEL-005). Each mutation writes its change, one audit entry and one event in one
//! transaction. `addLibraryModel` reads the repository before the transaction starts.

mod trail;
mod types;

use async_graphql::{Context, Error as GraphQlError, Object, Result};

use self::types::AliasView;
use super::{Error, HuggingFace, Missing, Model};
use crate::{
    api_errors::{ErrorCode, begin, coded, commit, internal, invalid, pool},
    fleet::{access::Authenticated, events::Trail},
};

/// The GraphQL error with its `extensions.code`; the message holds no input value.
fn to_graphql(error: Error) -> GraphQlError {
    match error {
        Error::NotFound(Missing::Model) => coded("No such model.", ErrorCode::NotFound),
        Error::NotFound(Missing::Repository) => coded("No such repository.", ErrorCode::NotFound),
        Error::Validation(fields) => invalid(&fields),
        Error::Refused(refusal) => invalid(&[refusal.field()]),
        Error::Source(super::FetchError::NotFound) => {
            coded("No such public repository.", ErrorCode::NotFound)
        }
        Error::Source(_) => coded(
            "The source of the models did not answer. Try again later.",
            ErrorCode::Unavailable,
        ),
        Error::Database(_) | Error::Unreadable => internal(error),
    }
}

#[derive(Default)]
pub struct Query;

#[Object(name = "LibraryQuery")]
impl Query {
    /// Every model of the library in name order (SEL-005).
    #[graphql(guard = "Authenticated")]
    async fn library_models(&self, ctx: &Context<'_>) -> Result<Vec<Model>> {
        super::list(pool(ctx)?).await.map_err(to_graphql)
    }

    /// The model of this name, alias or `name:tag`; null when there is none.
    #[graphql(guard = "Authenticated")]
    async fn library_model(&self, ctx: &Context<'_>, name: String) -> Result<Option<Model>> {
        match super::get(pool(ctx)?, &name).await {
            Ok(model) => Ok(Some(model)),
            Err(Error::NotFound(_)) => Ok(None),
            Err(other) => Err(to_graphql(other)),
        }
    }

    /// The Hugging Face organisations from which a variant can come (LIBM-003).
    #[graphql(guard = "Authenticated")]
    async fn approved_model_sources(&self, ctx: &Context<'_>) -> Result<Vec<String>> {
        super::sources(pool(ctx)?).await.map_err(to_graphql)
    }
}

#[derive(Default)]
pub struct Mutation;

#[Object(name = "LibraryMutation")]
impl Mutation {
    /// Reads a repository of an approved organisation and stores its model and variants
    /// (LIBM-004). A variant of the same tag is replaced.
    #[graphql(guard = "Authenticated")]
    async fn add_library_model(
        &self,
        ctx: &Context<'_>,
        repository: String,
        name: Option<String>,
    ) -> Result<Model> {
        let trail = Trail::from_context(ctx)?;
        let fetch = HuggingFace::public().map_err(|error| to_graphql(error.into()))?;
        let read = super::read(pool(ctx)?, &fetch, &repository, name.as_deref())
            .await
            .map_err(to_graphql)?;
        let mut tx = begin(ctx).await?;
        let model = super::save(&mut tx, &read, trail.now)
            .await
            .map_err(to_graphql)?;
        trail
            .write(&mut tx, trail::model_added(&model, repository.trim()))
            .await
            .map_err(internal)?;
        commit(tx).await?;
        Ok(model)
    }

    /// Points an alias at a model; tools that use the alias use the new model (SEL-003).
    #[graphql(guard = "Authenticated")]
    async fn set_model_alias(
        &self,
        ctx: &Context<'_>,
        alias: String,
        model: String,
    ) -> Result<AliasView> {
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let (alias, before) = super::set_alias(&mut tx, &alias, &model)
            .await
            .map_err(to_graphql)?;
        let entry = trail::alias_set(&alias.alias, &alias.model, before.as_deref());
        trail.write(&mut tx, entry).await.map_err(internal)?;
        commit(tx).await?;
        Ok(alias.into())
    }

    /// Adds an organisation to the approved sources; returns the list.
    #[graphql(guard = "Authenticated")]
    async fn add_approved_model_source(
        &self,
        ctx: &Context<'_>,
        organisation: String,
    ) -> Result<Vec<String>> {
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let added = super::add_source(&mut tx, &organisation, trail.now)
            .await
            .map_err(to_graphql)?;
        if added {
            let entry = trail::source_added(organisation.trim());
            trail.write(&mut tx, entry).await.map_err(internal)?;
        }
        commit(tx).await?;
        super::sources(pool(ctx)?).await.map_err(to_graphql)
    }
}
