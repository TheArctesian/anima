//! What the command and the MCP tools ask of the hub about places. Both call these, so the two
//! make the same requests (CMD-030). Each answer is the K1 `Place` as JSON.

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use super::documents::{
    PlaceAdd, PlaceClearGeocoder, PlaceList, PlaceMerge, PlaceRemove, PlaceSet, PlaceShow,
    PlaceSite, PlaceSplit, PlaceTopLevel, place_add, place_clear_geocoder, place_list, place_merge,
    place_remove, place_set, place_show, place_site, place_split, place_top_level,
};
use crate::failure::Failure;
use crate::hub::Hub;

/// The fields of a place, as K1 names them. A field that is `None` is left out.
#[derive(Clone, Debug, Default, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Fields {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius_m: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aliases: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geocoder_name: Option<String>,
}

/// What `set` changes: the fields, and the two changes that K1 writes as a null.
#[derive(Clone, Debug, Default)]
pub struct Change {
    pub fields: Fields,
    /// Take the place out of its parent. It then needs its own centre and radius.
    pub top_level: bool,
    /// Remove the name from a geocoder.
    pub clear_geocoder_name: bool,
}

fn usage(message: &str) -> Failure {
    Failure::Usage {
        message: message.to_owned(),
        hint: None,
    }
}

/// `from` as the generated K1 type `T`.
fn typed<T: DeserializeOwned>(from: &impl Serialize) -> Result<T, Failure> {
    serde_json::to_value(from)
        .and_then(serde_json::from_value)
        .map_err(|error| usage(&format!("The place is not valid: {error}.")))
}

fn json(value: &impl Serialize) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

/// # Errors
/// The hub's refusal.
pub async fn list(hub: &Hub) -> Result<Value, Failure> {
    let answer = hub.run::<PlaceList>(place_list::Variables).await?;
    Ok(json(&answer.places))
}

/// The place, or null when there is none. A merged place gives the place it was merged into.
///
/// # Errors
/// The hub's refusal.
pub async fn show(hub: &Hub, id: String) -> Result<Value, Failure> {
    let answer = hub.run::<PlaceShow>(place_show::Variables { id }).await?;
    Ok(json(&answer.place))
}

/// # Errors
/// `Usage` when name, category or icon is missing; the hub's refusal (`VALIDATION`).
pub async fn add(hub: &Hub, fields: &Fields) -> Result<Value, Failure> {
    if fields.name.is_none() || fields.category.is_none() || fields.icon.is_none() {
        return Err(usage("Give a name, a category and an icon."));
    }
    let input = typed(fields)?;
    let answer = hub.run::<PlaceAdd>(place_add::Variables { input }).await?;
    Ok(json(&answer.create_place))
}

/// # Errors
/// `Usage` when nothing is to change, or `top_level` has no centre and radius; the hub's refusal.
pub async fn set(hub: &Hub, id: String, change: Change) -> Result<Value, Failure> {
    let Change {
        mut fields,
        top_level,
        clear_geocoder_name,
    } = change;
    let mut answer = Value::Null;
    if clear_geocoder_name {
        let variables = place_clear_geocoder::Variables { id: id.clone() };
        answer = json(&hub.run::<PlaceClearGeocoder>(variables).await?.update_place);
    }
    if top_level {
        let (Some(latitude), Some(longitude), Some(radius_m)) = (
            fields.latitude.take(),
            fields.longitude.take(),
            fields.radius_m.take(),
        ) else {
            return Err(usage(
                "A top-level place needs a latitude, a longitude and a radius.",
            ));
        };
        let variables = place_top_level::Variables {
            id: id.clone(),
            latitude,
            longitude,
            radius_m,
        };
        answer = json(&hub.run::<PlaceTopLevel>(variables).await?.update_place);
        fields.parent_id = None;
    }
    let changes = json(&fields);
    if changes.as_object().is_some_and(|fields| !fields.is_empty()) {
        let input = typed(&fields)?;
        let variables = place_set::Variables { id, input };
        answer = json(&hub.run::<PlaceSet>(variables).await?.update_place);
    }
    if answer.is_null() {
        return Err(usage("Give at least one field to change."));
    }
    Ok(answer)
}

/// # Errors
/// The hub's refusal (`CONFLICT` for a merged place, `VALIDATION` for a merge into itself).
pub async fn merge(hub: &Hub, from: String, into: String) -> Result<Value, Failure> {
    let answer = hub
        .run::<PlaceMerge>(place_merge::Variables { from, into })
        .await?;
    Ok(json(&answer.merge_places))
}

/// The new place, made from `new`, with the named children, sites and links moved to it.
pub struct Division {
    pub new: Fields,
    pub children: Vec<String>,
    pub sites: Vec<String>,
    pub links: Vec<String>,
}

/// # Errors
/// The hub's refusal (`VALIDATION` for a name that is not a reference of the place).
pub async fn split(hub: &Hub, id: String, division: Division) -> Result<Value, Failure> {
    let list = |ids: Vec<String>| (!ids.is_empty()).then_some(ids);
    let input = typed(&serde_json::json!({
        "new": division.new,
        "children": list(division.children),
        "sites": list(division.sites),
        "links": list(division.links),
    }))?;
    let answer = hub
        .run::<PlaceSplit>(place_split::Variables { id, input })
        .await?;
    Ok(json(&answer.split_place))
}

/// The ID of the place, which is gone or merged into `replacement`.
///
/// # Errors
/// The hub's refusal: `CONFLICT` with the counts when the place has references and no replacement.
pub async fn remove(hub: &Hub, id: String, replacement: Option<String>) -> Result<String, Failure> {
    let variables = place_remove::Variables { id, replacement };
    Ok(hub.run::<PlaceRemove>(variables).await?.delete_place)
}

/// Links a site to a place, or with `None` removes the link. Null when it was removed.
///
/// # Errors
/// The hub's refusal (`NOT_FOUND` for an unknown site or place).
pub async fn site(hub: &Hub, site: String, place: Option<String>) -> Result<Value, Failure> {
    let answer = hub
        .run::<PlaceSite>(place_site::Variables { site, place })
        .await?;
    Ok(json(&answer.set_site_place))
}
