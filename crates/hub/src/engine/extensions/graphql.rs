//! The extensions GraphQL part: `Query.extensions` and `Mutation.setExtension` (K1; EXT-001,
//! EXT-011, EXT-012).
//!
//! `setExtension` writes every changed switch, one audit entry and one event for each, in one
//! transaction: if any step fails or the request is refused, nothing is stored. It is closed
//! to API keys by the access guard (D-23).

mod trail;

use std::collections::{HashMap, HashSet};

use async_graphql::{Context, Error as GraphQlError, Object, Result, SimpleObject};
use chrono::{DateTime, Utc};

use super::{
    DataClass, Entry, Error, Key, NavigationGroup, on_set, registry, store, switch,
    switching::needed_by,
};
use crate::{
    api_errors::{FieldError, begin, commit, internal, invalid, pool},
    fleet::{access::Authenticated, events::Trail},
};

/// Where the client shows an extension.
#[derive(SimpleObject)]
#[graphql(name = "NavigationEntry")]
struct NavigationView {
    group: NavigationGroup,
    label: String,
    /// A client route, for example /projects.
    route: String,
}

/// One extension and whether it is on.
#[derive(SimpleObject)]
#[graphql(name = "Extension")]
struct ExtensionView {
    /// Stable identifier, for example projects.
    key: String,
    name: String,
    data_class: DataClass,
    /// False: listed, not built. setExtension refuses it with CONFLICT.
    built: bool,
    /// True: part of the engine, always on, no switch (Integrations, ENG-002).
    fixed: bool,
    on: bool,
    /// Keys this one cannot work without. Setting it on sets them on too (EXT-011).
    needs: Vec<String>,
    works_with: Vec<String>,
    /// Keys that are on and need this one. Not empty: setting it off is refused (EXT-012).
    needed_by: Vec<String>,
    /// Show it only while `on`.
    navigation: Option<NavigationView>,
    /// Null: never changed.
    changed_at: Option<DateTime<Utc>>,
}

/// The GraphQL error of a failed step.
fn refused(error: impl Into<Error>) -> GraphQlError {
    error.into().into_graphql()
}

fn texts(keys: &[Key]) -> Vec<String> {
    keys.iter().map(|key| key.as_str().to_owned()).collect()
}

impl ExtensionView {
    fn of(entry: &Entry, on_now: &HashSet<Key>, changed: &HashMap<String, DateTime<Utc>>) -> Self {
        Self {
            key: entry.key.as_str().to_owned(),
            name: entry.name.to_owned(),
            data_class: entry.data_class,
            built: entry.built,
            fixed: entry.fixed,
            on: on_now.contains(&entry.key),
            needs: texts(entry.needs),
            works_with: texts(entry.works_with),
            needed_by: texts(&needed_by(registry(), on_now, entry.key)),
            navigation: entry.navigation.map(|nav| NavigationView {
                group: nav.group,
                label: nav.label.to_owned(),
                route: nav.route.to_owned(),
            }),
            changed_at: changed.get(entry.key.as_str()).copied(),
        }
    }
}

#[derive(Default)]
pub struct Query;

#[Object(name = "ExtensionsQuery")]
impl Query {
    /// All fifteen, in the order of the registry.
    #[graphql(guard = "Authenticated")]
    async fn extensions(&self, ctx: &Context<'_>) -> Result<Vec<ExtensionView>> {
        let db = pool(ctx)?;
        let stored = store::switched_on(db).await.map_err(refused)?;
        let changed = store::changes(db).await.map_err(refused)?;
        let on_now = on_set(registry(), stored.iter().map(String::as_str));
        let changed: HashMap<_, _> = changed.into_iter().collect();
        Ok(registry()
            .iter()
            .map(|entry| ExtensionView::of(entry, &on_now, &changed))
            .collect())
    }
}

#[derive(Default)]
pub struct Mutation;

#[Object(name = "ExtensionsMutation")]
impl Mutation {
    /// On also sets on each extension it needs (EXT-011). Off is refused with `CONFLICT`
    /// while an extension that is on needs it (EXT-012). Returns each switch that changed,
    /// the named one first.
    #[graphql(guard = "Authenticated")]
    async fn set_extension(
        &self,
        ctx: &Context<'_>,
        key: String,
        on: bool,
    ) -> Result<Vec<ExtensionView>> {
        let key = Key::parse(&key)
            .ok_or_else(|| invalid(&[FieldError::new("key", "No extension has this key.")]))?;
        let first = Trail::from_context(ctx)?;
        let (now, by) = (first.now, first.operator().to_string());
        drop(first);
        let mut tx = begin(ctx).await?;
        let changed = switch(&mut tx, registry(), (key, on), (now, &by))
            .await
            .map_err(Error::into_graphql)?;
        for changed_key in &changed {
            Trail::from_context(ctx)?
                .write(&mut tx, trail::switched(*changed_key, on))
                .await
                .map_err(internal)?;
        }
        commit(tx).await?;
        let stored = store::switched_on(pool(ctx)?).await.map_err(refused)?;
        let on_now = on_set(registry(), stored.iter().map(String::as_str));
        let stamps = changed
            .iter()
            .map(|k| (k.as_str().to_owned(), now))
            .collect();
        Ok(changed
            .iter()
            .map(|changed_key| ExtensionView::of(changed_key.entry(), &on_now, &stamps))
            .collect())
    }
}
