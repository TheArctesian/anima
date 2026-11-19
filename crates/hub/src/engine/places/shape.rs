//! The rules for the shape of a place (K-E4 Section 8; PLC-001, PLC-002, PLC-003). Pure: no
//! database and no clock.

use super::model::{Category, Centre, Draft, Entry, Fault, Patch, Place};

/// The longest name or alias, in characters.
const MAX_NAME: usize = 120;
/// The most aliases a place has.
pub const MAX_ALIASES: usize = 20;
/// The most levels a tree of places has: a top-level place is level 1.
pub const MAX_DEPTH: usize = 5;
const MAX_ICON: usize = 40;
const MAX_RADIUS_M: i32 = 100_000;

fn one_line(text: &str) -> Option<String> {
    let text = text.trim();
    let valid =
        !text.is_empty() && text.chars().count() <= MAX_NAME && !text.chars().any(char::is_control);
    valid.then(|| text.to_owned())
}

fn is_icon(icon: &str) -> bool {
    let mut chars = icon.chars();
    chars.next().is_some_and(|first| first.is_ascii_lowercase())
        && icon.len() <= MAX_ICON
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// The centre the three numbers make, `None` for none of them. `Err` when only some are given
/// or one is out of range.
fn centre(
    latitude: Option<f64>,
    longitude: Option<f64>,
    radius_m: Option<i32>,
) -> Result<Option<Centre>, Fault> {
    let (Some(latitude), Some(longitude), Some(radius)) = (latitude, longitude, radius_m) else {
        return if latitude.is_none() && longitude.is_none() && radius_m.is_none() {
            Ok(None)
        } else {
            Err(Fault {
                field: "latitude",
                message: "Give latitude, longitude and radiusM together.",
            })
        };
    };
    if !(-90.0..=90.0).contains(&latitude) {
        return Err(Fault {
            field: "latitude",
            message: "Give a latitude from -90 to 90.",
        });
    }
    if !(-180.0..=180.0).contains(&longitude) {
        return Err(Fault {
            field: "longitude",
            message: "Give a longitude from -180 to 180.",
        });
    }
    let radius_m = u32::try_from(radius)
        .ok()
        .filter(|_| (1..=MAX_RADIUS_M).contains(&radius))
        .ok_or(Fault {
            field: "radiusM",
            message: "Give a radius from 1 to 100000 metres.",
        })?;
    Ok(Some(Centre {
        latitude,
        longitude,
        radius_m,
    }))
}

fn aliases(names: &[String]) -> Result<Vec<String>, Fault> {
    let bad = Fault {
        field: "aliases",
        message: "Give at most 20 other names, each one line of 1 to 120 characters.",
    };
    let mut kept: Vec<String> = Vec::new();
    for name in names {
        let name = one_line(name).ok_or(bad)?;
        if !kept.contains(&name) {
            kept.push(name);
        }
    }
    if kept.len() > MAX_ALIASES {
        return Err(bad);
    }
    Ok(kept)
}

fn geocoder(name: Option<&str>) -> Result<Option<String>, Fault> {
    name.map(|name| {
        one_line(name).ok_or(Fault {
            field: "geocoderName",
            message: "Give one line of 1 to 120 characters.",
        })
    })
    .transpose()
}

/// The place the entry describes, or every field the operator must fix.
///
/// # Errors
/// One [`Fault`] for each field that is not valid. A top-level place needs a centre and a
/// radius; a child place must have none (PLC-002).
pub(super) fn check(entry: Entry) -> Result<Draft, Vec<Fault>> {
    let mut faults = Vec::new();
    let name = one_line(&entry.name);
    if name.is_none() {
        faults.push(Fault {
            field: "name",
            message: "Give one line of 1 to 120 characters.",
        });
    }
    let category = Category::parse(&entry.category);
    if category.is_none() {
        faults.push(Fault {
            field: "category",
            message: "Give home, work, site, shop, leisure, travel, storage or other.",
        });
    }
    if !is_icon(&entry.icon) {
        faults.push(Fault {
            field: "icon",
            message: "Give an icon name: lower-case letters, digits and hyphens, starting with a letter.",
        });
    }
    let aliases = aliases(&entry.aliases)
        .map_err(|fault| faults.push(fault))
        .ok();
    let geocoder_name = geocoder(entry.geocoder_name.as_deref())
        .map_err(|fault| faults.push(fault))
        .ok()
        .flatten();
    let centre = centre(entry.latitude, entry.longitude, entry.radius_m)
        .and_then(|centre| match (entry.parent, centre) {
            (None, None) => Err(Fault {
                field: "latitude",
                message: "A top-level place needs latitude, longitude and radiusM.",
            }),
            (Some(_), Some(_)) => Err(Fault {
                field: "latitude",
                message: "A place inside another place has no centre and no radius.",
            }),
            (_, centre) => Ok(centre),
        })
        .map_err(|fault| faults.push(fault))
        .ok();
    match (name, category, aliases, centre) {
        (Some(name), Some(category), Some(aliases), Some(centre)) if faults.is_empty() => {
            Ok(Draft {
                name,
                parent: entry.parent,
                centre,
                category,
                icon: entry.icon,
                aliases,
                geocoder_name,
            })
        }
        _ => Err(faults),
    }
}

/// The place `current` becomes with `patch`, checked as a whole. A place that moves inside
/// another one and gives no centre loses its centre and radius.
///
/// # Errors
/// As [`check`].
pub(super) fn apply(current: &Place, patch: Patch) -> Result<Draft, Vec<Fault>> {
    let parent = patch.parent.unwrap_or(current.parent);
    let touches_centre =
        patch.latitude.is_some() || patch.longitude.is_some() || patch.radius_m.is_some();
    let kept = current.centre.filter(|_| parent.is_none());
    let (latitude, longitude, radius_m) = if touches_centre {
        let base = current.centre;
        (
            patch.latitude.or(base.map(|c| c.latitude)),
            patch.longitude.or(base.map(|c| c.longitude)),
            patch
                .radius_m
                .or(base.and_then(|c| i32::try_from(c.radius_m).ok())),
        )
    } else {
        (
            kept.map(|c| c.latitude),
            kept.map(|c| c.longitude),
            kept.and_then(|c| i32::try_from(c.radius_m).ok()),
        )
    };
    check(Entry {
        name: patch.name.unwrap_or_else(|| current.name.clone()),
        parent,
        latitude,
        longitude,
        radius_m,
        category: patch
            .category
            .unwrap_or_else(|| current.category.as_str().to_owned()),
        icon: patch.icon.unwrap_or_else(|| current.icon.clone()),
        aliases: patch.aliases.unwrap_or_else(|| current.aliases.clone()),
        geocoder_name: patch
            .geocoder_name
            .unwrap_or_else(|| current.geocoder_name.clone()),
    })
}

/// The names of the fields that `draft` changes in `place`.
#[must_use]
pub(super) fn changes(place: &Place, draft: &Draft) -> Vec<&'static str> {
    let same = [
        ("name", place.name == draft.name),
        ("parentId", place.parent == draft.parent),
        ("centre", place.centre == draft.centre),
        ("category", place.category == draft.category),
        ("icon", place.icon == draft.icon),
        ("aliases", place.aliases == draft.aliases),
        ("geocoderName", place.geocoder_name == draft.geocoder_name),
    ];
    same.into_iter()
        .filter(|(_, unchanged)| !unchanged)
        .map(|(field, _)| field)
        .collect()
}

/// The aliases of the remaining place after a merge: its own, then the name and the aliases of
/// the merged place, without repeats and without the remaining place's own name. Extra names
/// beyond [`MAX_ALIASES`] are dropped, so a merge never fails for this (PLC-004).
#[must_use]
pub(super) fn merged_aliases(into: &Place, from: &Place) -> Vec<String> {
    let mut names = into.aliases.clone();
    for name in std::iter::once(&from.name).chain(&from.aliases) {
        if *name != into.name && !names.contains(name) {
            names.push(name.clone());
        }
    }
    names.truncate(MAX_ALIASES);
    names
}

/// Whether a subtree that is `height` levels below its root fits when the root sits at
/// `root_depth` (a top-level place is depth 1).
#[must_use]
pub(super) const fn fits(root_depth: usize, height: usize) -> bool {
    root_depth + height <= MAX_DEPTH
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry() -> Entry {
        Entry {
            name: " Home ".to_owned(),
            latitude: Some(52.0),
            longitude: Some(4.0),
            radius_m: Some(50),
            category: "home".to_owned(),
            icon: "house".to_owned(),
            ..Entry::default()
        }
    }

    #[test]
    fn plc_001_a_place_has_a_name_a_centre_a_radius_a_category_an_icon_and_aliases() {
        let draft = check(Entry {
            aliases: vec!["Flat".to_owned(), "Flat".to_owned()],
            ..entry()
        })
        .unwrap();
        assert_eq!(draft.name, "Home");
        assert_eq!(draft.aliases, ["Flat"]);
        assert_eq!(draft.centre.unwrap().radius_m, 50);
    }

    #[test]
    fn plc_002_a_child_place_has_no_radius_and_a_top_level_place_has_one() {
        let parent = Some(crate::ids::PlaceId::from(uuid::Uuid::nil()));
        let child = check(Entry { parent, ..entry() }).unwrap_err();
        assert_eq!(child[0].field, "latitude");
        let bare = Entry {
            latitude: None,
            longitude: None,
            radius_m: None,
            ..entry()
        };
        assert!(
            check(Entry {
                parent,
                ..bare.clone()
            })
            .is_ok()
        );
        assert!(check(bare).is_err());
    }

    #[test]
    fn plc_001_every_bad_field_is_named() {
        let faults = check(Entry {
            name: String::new(),
            latitude: Some(91.0),
            category: "castle".to_owned(),
            icon: "Big Icon".to_owned(),
            ..entry()
        })
        .unwrap_err();
        let fields: Vec<_> = faults.iter().map(|f| f.field).collect();
        assert_eq!(fields, ["name", "category", "icon", "latitude"]);
    }

    #[test]
    fn plc_002_the_tree_is_at_most_five_levels_deep() {
        assert!(fits(3, 2));
        assert!(!fits(3, 3));
    }
}
