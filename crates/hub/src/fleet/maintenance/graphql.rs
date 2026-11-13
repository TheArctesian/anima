//! The maintenance GraphQL part: root `Query` and `Mutation` (K1; API-003, MW-001 to MW-004).
//!
//! Each mutation validates, then writes its change, one audit entry and one event in one
//! transaction: if any step fails, nothing is stored.

mod error;
mod input;
mod trail;
mod window;

use async_graphql::{Context, ID, Object, Result};
use uuid::Uuid;

use self::{
    error::to_graphql,
    input::{MaintenanceWindowInput, window_id},
};
use super::{FieldError, Window, model::NewWindow, rules::validate, store};
use crate::api_errors::{begin, commit, internal, invalid, pool};
use crate::{
    fleet::{access::Authenticated, events::Trail},
    ids::WindowId,
};

/// The window an update names and the window to replace it with, with every field error
/// together.
fn checked_update(id: &ID, input: MaintenanceWindowInput) -> Result<(WindowId, NewWindow)> {
    match (window_id(id), validate(&input.into())) {
        (Ok(id), Ok(new)) => Ok((id, new)),
        (id, new) => {
            let errors: Vec<FieldError> = id
                .err()
                .into_iter()
                .chain(new.err().into_iter().flatten())
                .collect();
            Err(invalid(&errors))
        }
    }
}

#[derive(Default)]
pub struct Query;

#[Object(name = "MaintenanceQuery")]
impl Query {
    #[graphql(guard = "Authenticated")]
    async fn maintenance_windows(&self, ctx: &Context<'_>) -> Result<Vec<Window>> {
        store::all(pool(ctx)?).await.map_err(to_graphql)
    }
}

#[derive(Default)]
pub struct Mutation;

#[Object(name = "MaintenanceMutation")]
impl Mutation {
    #[graphql(guard = "Authenticated")]
    async fn create_maintenance_window(
        &self,
        ctx: &Context<'_>,
        input: MaintenanceWindowInput,
    ) -> Result<Window> {
        let new = validate(&input.into()).map_err(|errors| invalid(&errors))?;
        let trail = Trail::from_context(ctx)?;
        let id = WindowId::from(Uuid::now_v7());
        let mut tx = begin(ctx).await?;
        let window = store::insert(&mut tx, id, trail.now, &new)
            .await
            .map_err(to_graphql)?;
        trail
            .write(&mut tx, trail::window_added(&window))
            .await
            .map_err(internal)?;
        commit(tx).await?;
        Ok(window)
    }

    /// Replaces the whole window, including its targets.
    #[graphql(guard = "Authenticated")]
    async fn update_maintenance_window(
        &self,
        ctx: &Context<'_>,
        id: ID,
        input: MaintenanceWindowInput,
    ) -> Result<Window> {
        let (id, new) = checked_update(&id, input)?;
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let (before, after) = store::replace(&mut tx, id, trail.now, &new)
            .await
            .map_err(to_graphql)?;
        trail
            .write(&mut tx, trail::window_updated(&before, &after))
            .await
            .map_err(internal)?;
        commit(tx).await?;
        Ok(after)
    }

    #[graphql(guard = "Authenticated")]
    async fn delete_maintenance_window(&self, ctx: &Context<'_>, id: ID) -> Result<ID> {
        let id = window_id(&id).map_err(|e| invalid(&[e]))?;
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let window = store::delete(&mut tx, id).await.map_err(to_graphql)?;
        trail
            .write(&mut tx, trail::window_deleted(&window))
            .await
            .map_err(internal)?;
        commit(tx).await?;
        Ok(ID(window.id.to_string()))
    }

    /// Switches a MANUAL window on or off; any other kind is refused with VALIDATION.
    #[graphql(guard = "Authenticated")]
    async fn set_manual_window_active(
        &self,
        ctx: &Context<'_>,
        id: ID,
        active: bool,
    ) -> Result<Window> {
        let id = window_id(&id).map_err(|e| invalid(&[e]))?;
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let window = store::set_manual(&mut tx, id, trail.now, active)
            .await
            .map_err(to_graphql)?;
        trail
            .write(&mut tx, trail::manual_switched(&window))
            .await
            .map_err(internal)?;
        commit(tx).await?;
        Ok(window)
    }
}
