//! The K1 inputs of the place mutations and their conversion to the places model.

use async_graphql::{Error as GraphQlError, ID, InputObject, MaybeUndefined, Result};

use super::{Entry, Patch};
use crate::api_errors::{FieldError, invalid};

/// K1 `PlaceInput`.
#[derive(InputObject)]
#[graphql(name = "PlaceInput")]
pub(super) struct PlaceInput {
    /// One line of 1 to 120 characters.
    pub(super) name: String,
    pub(super) parent_id: Option<ID>,
    /// Latitude, longitude and radius go together, and only on a top-level place.
    pub(super) latitude: Option<f64>,
    pub(super) longitude: Option<f64>,
    pub(super) radius_m: Option<i32>,
    /// home, work, site, shop, leisure, travel, storage or other.
    pub(super) category: String,
    pub(super) icon: String,
    pub(super) aliases: Option<Vec<String>>,
    pub(super) geocoder_name: Option<String>,
}

/// K1 `PlaceChangeInput`: a field that is left out keeps its value.
#[derive(InputObject)]
#[graphql(name = "PlaceChangeInput")]
pub(super) struct PlaceChangeInput {
    pub(super) name: Option<String>,
    /// Null makes the place a top-level place.
    pub(super) parent_id: MaybeUndefined<ID>,
    pub(super) latitude: Option<f64>,
    pub(super) longitude: Option<f64>,
    pub(super) radius_m: Option<i32>,
    pub(super) category: Option<String>,
    pub(super) icon: Option<String>,
    pub(super) aliases: Option<Vec<String>>,
    /// Null removes the geocoder name.
    pub(super) geocoder_name: MaybeUndefined<String>,
}

/// K1 `PlaceSplitInput`.
#[derive(InputObject)]
#[graphql(name = "PlaceSplitInput")]
pub(super) struct PlaceSplitInput {
    /// The new top-level place.
    pub(super) new: PlaceInput,
    /// Child places of the place that move into the new place.
    pub(super) children: Option<Vec<ID>>,
    /// Sites that move to the new place.
    pub(super) sites: Option<Vec<ID>>,
    /// Record links that move to the new place.
    pub(super) links: Option<Vec<ID>>,
}

pub(super) fn bad_id(field: &'static str) -> GraphQlError {
    invalid(&[FieldError::new(field, "This is not a valid ID.")])
}

pub(super) fn parse<T: std::str::FromStr>(id: &ID, field: &'static str) -> Result<T> {
    id.parse().map_err(|_| bad_id(field))
}

pub(super) fn parse_all<T: std::str::FromStr>(
    ids: Option<&[ID]>,
    field: &'static str,
) -> Result<Vec<T>> {
    ids.unwrap_or_default()
        .iter()
        .map(|id| parse(id, field))
        .collect()
}

pub(super) fn entry(input: PlaceInput) -> Result<Entry> {
    Ok(Entry {
        name: input.name,
        parent: input
            .parent_id
            .as_ref()
            .map(|id| parse(id, "parentId"))
            .transpose()?,
        latitude: input.latitude,
        longitude: input.longitude,
        radius_m: input.radius_m,
        category: input.category,
        icon: input.icon,
        aliases: input.aliases.unwrap_or_default(),
        geocoder_name: input.geocoder_name,
    })
}

pub(super) fn patch(input: PlaceChangeInput) -> Result<Patch> {
    let parent = match input.parent_id {
        MaybeUndefined::Undefined => None,
        MaybeUndefined::Null => Some(None),
        MaybeUndefined::Value(id) => Some(Some(parse(&id, "parentId")?)),
    };
    let geocoder_name = match input.geocoder_name {
        MaybeUndefined::Undefined => None,
        MaybeUndefined::Null => Some(None),
        MaybeUndefined::Value(name) => Some(Some(name)),
    };
    Ok(Patch {
        name: input.name,
        parent,
        latitude: input.latitude,
        longitude: input.longitude,
        radius_m: input.radius_m,
        category: input.category,
        icon: input.icon,
        aliases: input.aliases,
        geocoder_name,
    })
}
