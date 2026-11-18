//! The places GraphQL part: `Query.places`, `Query.place` and the six place mutations (K1;
//! PLC-001 to PLC-006). Each change writes its row, its audit entry and its event in one
//! transaction.

use async_graphql::{
    Context, Error as GraphQlError, ErrorExtensions, ID, Object, Result, SimpleObject, Value, value,
};
use chrono::{DateTime, Utc};

use super::{
    Error, Holders, Place, References, Removal, Split, create, current,
    inputs::{PlaceChangeInput, PlaceInput, PlaceSplitInput, entry, parse, parse_all, patch},
    list, merge, remove, set_site, sites_of, split, trail, update,
};
use crate::{
    api_errors::{
        ErrorCode, FieldError, begin, coded, commit, internal, invalid, pool, schema_dependency,
    },
    fleet::{
        access::Authenticated,
        devices,
        events::{AuditAction, Trail},
    },
    ids::{PlaceId, SiteId},
};

/// K1 `Place`.
#[derive(SimpleObject)]
#[graphql(name = "Place")]
struct PlaceView {
    id: ID,
    /// The place this one is inside; null for a top-level place.
    parent_id: Option<ID>,
    /// The name the operator gave.
    name: String,
    /// What to call the place: the operator's name, never the geocoder's (PLC-003).
    title: String,
    /// Top-level places only (PLC-002).
    latitude: Option<f64>,
    longitude: Option<f64>,
    radius_m: Option<i32>,
    category: String,
    icon: String,
    aliases: Vec<String>,
    geocoder_name: Option<String>,
    /// The sites placed here (PLC-006).
    site_ids: Vec<ID>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl PlaceView {
    fn new(place: Place, sites: &[SiteId]) -> Self {
        let centre = place.centre;
        Self {
            id: ID(place.id.to_string()),
            parent_id: place.parent.map(|p| ID(p.to_string())),
            title: place.name.clone(),
            name: place.name,
            latitude: centre.map(|c| c.latitude),
            longitude: centre.map(|c| c.longitude),
            radius_m: centre.and_then(|c| i32::try_from(c.radius_m).ok()),
            category: place.category.as_str().to_owned(),
            icon: place.icon,
            aliases: place.aliases,
            geocoder_name: place.geocoder_name,
            site_ids: sites.iter().map(|s| ID(s.to_string())).collect(),
            created_at: place.created_at,
            updated_at: place.updated_at,
        }
    }
}

/// `CONFLICT` with the count of each sort of reference (PLC-005).
fn conflict(found: &References) -> GraphQlError {
    let held: Vec<Value> = found
        .held
        .iter()
        .map(|(domain, count)| value!({ "domain": (*domain), "count": (*count) }))
        .collect();
    let other: u64 = found.held.iter().map(|(_, count)| count).sum();
    coded(
        format!(
            "This place has references: {} links, {} sites, {} child places and {other} \
             other records. Give a replacement place.",
            found.links, found.sites, found.children
        ),
        ErrorCode::Conflict,
    )
    .extend_with(|_, extensions| {
        extensions.set(
            "references",
            value!({
                "links": found.links,
                "sites": found.sites,
                "children": found.children,
                "held": (Value::List(held)),
            }),
        );
    })
}

/// `VALIDATION` and `CONFLICT` for what the operator can fix; anything else is `INTERNAL`.
fn refused(error: Error) -> GraphQlError {
    match error {
        Error::Invalid(faults) => {
            let fields: Vec<_> = faults
                .iter()
                .map(|fault| FieldError::new(fault.field, fault.message))
                .collect();
            invalid(&fields)
        }
        Error::NotFound => coded("No place has this ID.", ErrorCode::NotFound),
        Error::Merged => coded(
            "This place was merged into another place.",
            ErrorCode::Conflict,
        ),
        Error::InUse(found) => conflict(&found),
        Error::Holder(unavailable) => {
            tracing::warn!(%unavailable, "a holder of places could not answer");
            coded(
                "Some data that uses this place is not available. Try again.",
                ErrorCode::Unavailable,
            )
        }
        Error::Database(_) | Error::Links(_) => internal(error),
    }
}

async fn shown(ctx: &Context<'_>, places: Vec<Place>) -> Result<Vec<PlaceView>> {
    let ids: Vec<PlaceId> = places.iter().map(|place| place.id).collect();
    let mut sites = sites_of(pool(ctx)?, &ids).await.map_err(refused)?;
    Ok(places
        .into_iter()
        .map(|place| {
            let here = sites.remove(&place.id).unwrap_or_default();
            PlaceView::new(place, &here)
        })
        .collect())
}

async fn show(ctx: &Context<'_>, place: Place) -> Result<PlaceView> {
    let mut views = shown(ctx, vec![place]).await?;
    views
        .pop()
        .ok_or_else(|| refused(Error::Database(sqlx::Error::RowNotFound)))
}

#[derive(Default)]
pub struct Query;

#[Object(name = "PlacesQuery")]
impl Query {
    /// The places that are not merged, by name.
    #[graphql(guard = "Authenticated")]
    async fn places(&self, ctx: &Context<'_>) -> Result<Vec<PlaceView>> {
        let places = list(pool(ctx)?).await.map_err(refused)?;
        shown(ctx, places).await
    }

    /// One place. A place that was merged gives the place it was merged into (PLC-004).
    #[graphql(guard = "Authenticated")]
    async fn place(&self, ctx: &Context<'_>, id: ID) -> Result<Option<PlaceView>> {
        let Ok(id) = id.parse::<PlaceId>() else {
            return Ok(None);
        };
        match current(pool(ctx)?, id).await.map_err(refused)? {
            Some(place) => Ok(Some(show(ctx, place).await?)),
            None => Ok(None),
        }
    }
}

#[derive(Default)]
pub struct Mutation;

#[Object(name = "PlacesMutation")]
impl Mutation {
    /// Makes a place (PLC-001, PLC-002).
    #[graphql(guard = "Authenticated")]
    async fn create_place(&self, ctx: &Context<'_>, input: PlaceInput) -> Result<PlaceView> {
        let entry = entry(input)?;
        let change = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let place = create(&mut tx, entry, change.now).await.map_err(refused)?;
        change
            .write(&mut tx, trail::created(&place))
            .await
            .map_err(internal)?;
        commit(tx).await?;
        show(ctx, place).await
    }

    /// Changes a place. A name from a geocoder never replaces `name` (PLC-003).
    #[graphql(guard = "Authenticated")]
    async fn update_place(
        &self,
        ctx: &Context<'_>,
        id: ID,
        input: PlaceChangeInput,
    ) -> Result<PlaceView> {
        let id = parse(&id, "id")?;
        let patch = patch(input)?;
        let change = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let (place, changed) = update(&mut tx, id, patch, change.now)
            .await
            .map_err(refused)?;
        if !changed.is_empty() {
            change
                .write(&mut tx, trail::updated(&place, &changed))
                .await
                .map_err(internal)?;
        }
        commit(tx).await?;
        show(ctx, place).await
    }

    /// Moves every reference of `from` to `into` and returns the remaining place (PLC-004).
    #[graphql(guard = "Authenticated")]
    async fn merge_places(&self, ctx: &Context<'_>, from: ID, into: ID) -> Result<PlaceView> {
        let ends = (parse(&from, "from")?, parse(&into, "into")?);
        let change = Trail::from_context(ctx)?;
        let holders = schema_dependency::<Holders>(ctx)?;
        let mut tx = begin(ctx).await?;
        let done = merge(&mut tx, holders, ends, change.now)
            .await
            .map_err(refused)?;
        let entry = trail::merged(&done, AuditAction::Update);
        change.write(&mut tx, entry).await.map_err(internal)?;
        commit(tx).await?;
        show(ctx, done.into).await
    }

    /// Makes a new top-level place from `input.new` and moves the named children, sites and
    /// links of `id` to it. Returns the new place (PLC-004).
    #[graphql(guard = "Authenticated")]
    async fn split_place(
        &self,
        ctx: &Context<'_>,
        id: ID,
        input: PlaceSplitInput,
    ) -> Result<PlaceView> {
        let id = parse(&id, "id")?;
        let parts = Split {
            children: parse_all(input.children.as_deref(), "children")?,
            sites: parse_all(input.sites.as_deref(), "sites")?,
            links: parse_all(input.links.as_deref(), "links")?,
            new: entry(input.new)?,
        };
        let change = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let done = split(&mut tx, id, parts, change.now)
            .await
            .map_err(refused)?;
        change
            .write(&mut tx, trail::divided(&done))
            .await
            .map_err(internal)?;
        commit(tx).await?;
        show(ctx, done.new).await
    }

    /// Deletes a place that has no reference. A place with references is merged into
    /// `replacement`; with none given the answer is `CONFLICT` with the count of each sort of
    /// reference (PLC-005). Returns the ID of the place.
    #[graphql(guard = "Authenticated")]
    async fn delete_place(&self, ctx: &Context<'_>, id: ID, replacement: Option<ID>) -> Result<ID> {
        let ends = (
            parse(&id, "id")?,
            replacement
                .as_ref()
                .map(|r| parse(r, "replacement"))
                .transpose()?,
        );
        let change = Trail::from_context(ctx)?;
        let holders = schema_dependency::<Holders>(ctx)?;
        let mut tx = begin(ctx).await?;
        let done: Removal = remove(&mut tx, holders, ends, change.now)
            .await
            .map_err(refused)?;
        change
            .write(&mut tx, trail::removed(&done))
            .await
            .map_err(internal)?;
        commit(tx).await?;
        Ok(id)
    }

    /// Links a site to a place, or with a null place removes the link. Returns the place, or
    /// null when the link was removed (PLC-006).
    #[graphql(guard = "Authenticated")]
    async fn set_site_place(
        &self,
        ctx: &Context<'_>,
        site: ID,
        place: Option<ID>,
    ) -> Result<Option<PlaceView>> {
        let site: SiteId = parse(&site, "site")?;
        let place: Option<PlaceId> = place.as_ref().map(|p| parse(p, "place")).transpose()?;
        if place.is_some() {
            let known = devices::sites(pool(ctx)?).await.map_err(internal)?;
            if known.iter().all(|s| s.id != site) {
                return Err(coded("No site has this ID.", ErrorCode::NotFound));
            }
        }
        let change = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let siting = set_site(&mut tx, site, place).await.map_err(refused)?;
        if let Some(entry) = trail::sited(&siting) {
            change.write(&mut tx, entry).await.map_err(internal)?;
        }
        commit(tx).await?;
        match place {
            Some(id) => match current(pool(ctx)?, id).await.map_err(refused)? {
                Some(found) => Ok(Some(show(ctx, found).await?)),
                None => Ok(None),
            },
            None => Ok(None),
        }
    }
}
