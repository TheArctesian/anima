//! The place tools, in the `records` set: where things are, as every extension names them
//! (PLC-001 to PLC-006). A place is the record `place:<id>`.

use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::hub::Hub;
use crate::mcp::arguments::{object, parse};
use crate::mcp::catalogue::Access;
use crate::mcp::definition::{Definition, Effect};
use crate::mcp::outcome::ToolError;
use crate::places::calls::{self, Change, Division, Fields};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaceTool {
    List,
    Get,
    Add,
    Set,
    Merge,
    Split,
    Remove,
    Site,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListInput {}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetInput {
    /// The place id. A merged place gives the place it was merged into.
    id: String,
}

/// The fields of a place, as K1 names them. A top-level place needs `latitude`, `longitude`
/// and `radiusM`; a place with a `parentId` has none of the three. Never put a name from a
/// geocoder in `name`: use `geocoderName`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AddInput {
    /// One line of 1 to 120 characters.
    name: String,
    /// The id of the place this one is inside.
    parent_id: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    /// How far the place reaches, in metres: 1 to 100000.
    radius_m: Option<i64>,
    /// home, work, site, shop, leisure, travel, storage or other.
    category: String,
    /// The name of an icon, lower case with hyphens, such as `house`.
    icon: String,
    /// Other names of the place, at most 20.
    aliases: Option<Vec<String>>,
    geocoder_name: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetInput {
    /// The place id.
    id: String,
    /// The fields to change; a field that is left out keeps its value.
    #[serde(flatten)]
    fields: Fields,
    /// Take the place out of its parent. Give `latitude`, `longitude` and `radiusM` too.
    #[serde(default)]
    top_level: bool,
    /// Remove the name from a geocoder.
    #[serde(default)]
    clear_geocoder_name: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MergeInput {
    /// The place to merge away. It stays resolvable but is no longer listed or changeable.
    from: String,
    /// The place that stays and gets every reference of `from`.
    into: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SplitInput {
    /// The place to split.
    id: String,
    /// The new top-level place: a name, a category, an icon, and a latitude, a longitude and a
    /// radius.
    new: AddInput,
    /// Child places of `id` that move into the new place.
    #[serde(default)]
    children: Vec<String>,
    /// Sites that move to the new place.
    #[serde(default)]
    sites: Vec<String>,
    /// Record links of `id` that move to the new place.
    #[serde(default)]
    links: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RemoveInput {
    /// The place id.
    id: String,
    /// The place to merge into when `id` has references.
    replacement: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SiteInput {
    /// The site id.
    site: String,
    /// The place id. Leave it out to remove the link of the site.
    place: Option<String>,
}

impl SetInput {
    fn into_call(self) -> (String, Change) {
        let change = Change {
            fields: self.fields,
            top_level: self.top_level,
            clear_geocoder_name: self.clear_geocoder_name,
        };
        (self.id, change)
    }
}

impl SplitInput {
    fn into_call(self) -> (String, Division) {
        let division = Division {
            new: self.new.into(),
            children: self.children,
            sites: self.sites,
            links: self.links,
        };
        (self.id, division)
    }
}

impl From<AddInput> for Fields {
    fn from(input: AddInput) -> Self {
        Self {
            name: Some(input.name),
            parent_id: input.parent_id,
            latitude: input.latitude,
            longitude: input.longitude,
            radius_m: input.radius_m,
            category: Some(input.category),
            icon: Some(input.icon),
            aliases: input.aliases,
            geocoder_name: input.geocoder_name,
        }
    }
}

fn list() -> Definition {
    Definition::new::<ListInput>(
        "anima_places_list",
        Access::Read,
        Effect::Reads,
        "Lists the places that are not merged, by name: id, name, category, icon, \
         other names, the centre and radius of a top-level place, the parent of a \
         child place and the sites placed there. A place is the record `place:<id>`.",
    )
    .returns(&[("places", "array", "The places.")])
}

fn get() -> Definition {
    Definition::new::<GetInput>(
        "anima_place_get",
        Access::Read,
        Effect::Reads,
        "Gets one place by id. A place that was merged gives the place it was merged \
         into. Null when there is no such place.",
    )
    .returns(&[("place", "object", "The place, or null.")])
}

fn add() -> Definition {
    Definition::new::<AddInput>(
        "anima_place_add",
        Access::Write,
        Effect::Adds,
        "Adds a place. A top-level place needs a centre and a radius; a child place \
         (with parentId) has none. Returns the place. Next: anima_place_site to put a \
         site there.",
    )
    .returns(&[("place", "object", "The new place.")])
}

fn set() -> Definition {
    Definition::new::<SetInput>(
        "anima_place_set",
        Access::Write,
        Effect::Sets,
        "Changes a place; a field that is left out keeps its value. A name from a \
         geocoder goes in geocoderName and never replaces name. Returns the place.",
    )
    .returns(&[("place", "object", "The changed place.")])
}

fn merge() -> Definition {
    Definition::new::<MergeInput>(
        "anima_place_merge",
        Access::Write,
        Effect::Acts,
        "Merges one place into another: every link, site, child place and other \
         reference of `from` moves to `into`, and the name of `from` becomes one of \
         its other names. `place:<from>` still resolves. This cannot be undone. \
         Returns the remaining place.",
    )
    .returns(&[("place", "object", "The remaining place.")])
}

fn split() -> Definition {
    Definition::new::<SplitInput>(
        "anima_place_split",
        Access::Write,
        Effect::Acts,
        "Makes a new top-level place and moves the named child places, sites and \
         record links of one place to it. What is not named stays. Returns the new \
         place.",
    )
    .returns(&[("place", "object", "The new place.")])
}

fn remove() -> Definition {
    Definition::new::<RemoveInput>(
        "anima_place_remove",
        Access::Write,
        Effect::Removes,
        "Deletes a place. A place that has references (links, sites, child places) is \
         refused with CONFLICT and the count of each; give `replacement` to merge it \
         into another place instead.",
    )
    .returns(&[("removed", "string", "The id of the place.")])
}

fn site() -> Definition {
    Definition::new::<SiteInput>(
        "anima_place_site",
        Access::Write,
        Effect::Sets,
        "Links a site to a place; a site has at most one place. Leave `place` out to \
         remove the link. This says where the operator placed the site, not where its \
         devices are. Returns the place, or null when the link was removed.",
    )
    .returns(&[("place", "object", "The place, or null.")])
}

impl PlaceTool {
    pub const ALL: [Self; 8] = [
        Self::List,
        Self::Get,
        Self::Add,
        Self::Set,
        Self::Merge,
        Self::Split,
        Self::Remove,
        Self::Site,
    ];

    #[must_use]
    pub fn definition(self) -> Definition {
        match self {
            Self::List => list(),
            Self::Get => get(),
            Self::Add => add(),
            Self::Set => set(),
            Self::Merge => merge(),
            Self::Split => split(),
            Self::Remove => remove(),
            Self::Site => site(),
        }
    }

    /// Runs the tool.
    ///
    /// # Errors
    /// [`ToolError`] for wrong arguments or the hub's refusal.
    pub async fn call(self, hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
        let place = |value: Value| Ok(Value::Object(object([("place", value)])));
        match self {
            Self::List => {
                parse::<ListInput>(arguments)?;
                Ok(Value::Object(object([("places", calls::list(hub).await?)])))
            }
            Self::Get => {
                let GetInput { id } = parse(arguments)?;
                place(calls::show(hub, id).await?)
            }
            Self::Add => {
                let input: AddInput = parse(arguments)?;
                place(calls::add(hub, &input.into()).await?)
            }
            Self::Set => {
                let (id, change) = parse::<SetInput>(arguments)?.into_call();
                place(calls::set(hub, id, change).await?)
            }
            Self::Merge => {
                let MergeInput { from, into } = parse(arguments)?;
                place(calls::merge(hub, from, into).await?)
            }
            Self::Split => {
                let (id, division) = parse::<SplitInput>(arguments)?.into_call();
                place(calls::split(hub, id, division).await?)
            }
            Self::Remove => {
                let RemoveInput { id, replacement } = parse(arguments)?;
                let removed = calls::remove(hub, id, replacement).await?;
                Ok(Value::Object(object([("removed", json!(removed))])))
            }
            Self::Site => {
                let SiteInput {
                    site,
                    place: target,
                } = parse(arguments)?;
                place(calls::site(hub, site, target).await?)
            }
        }
    }
}
