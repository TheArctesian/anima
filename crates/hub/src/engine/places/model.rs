//! A place and the words around it (K-E4 Section 8; PLC-001 to PLC-006).

use chrono::{DateTime, Utc};

use crate::ids::{PlaceId, RecordLinkId, SiteId};

/// What sort of place it is. The hub keeps this closed list; the client chooses the words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Home,
    Work,
    Site,
    Shop,
    Leisure,
    Travel,
    Storage,
    Other,
}

impl Category {
    pub const ALL: [Self; 8] = [
        Self::Home,
        Self::Work,
        Self::Site,
        Self::Shop,
        Self::Leisure,
        Self::Travel,
        Self::Storage,
        Self::Other,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Work => "work",
            Self::Site => "site",
            Self::Shop => "shop",
            Self::Leisure => "leisure",
            Self::Travel => "travel",
            Self::Storage => "storage",
            Self::Other => "other",
        }
    }

    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|category| category.as_str() == text)
    }
}

/// Where a top-level place is, and how far it reaches. A child place has none (PLC-002).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Centre {
    pub latitude: f64,
    pub longitude: f64,
    pub radius_m: u32,
}

/// A stored place.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub id: PlaceId,
    /// The place this one is inside; `None` for a top-level place.
    pub parent: Option<PlaceId>,
    /// The name the operator gave. It is the title of the place; a geocoder never changes it
    /// (PLC-003).
    pub name: String,
    pub centre: Option<Centre>,
    pub category: Category,
    pub icon: String,
    pub aliases: Vec<String>,
    /// A name from a geocoder, kept beside `name`.
    pub geocoder_name: Option<String>,
    /// Set when the place was merged into another (PLC-004).
    pub merged_into: Option<PlaceId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A place that passed every check and is ready to store.
#[derive(Clone, Debug, PartialEq)]
pub struct Draft {
    pub name: String,
    pub parent: Option<PlaceId>,
    pub centre: Option<Centre>,
    pub category: Category,
    pub icon: String,
    pub aliases: Vec<String>,
    pub geocoder_name: Option<String>,
}

/// The input of a new place, as the operator typed it: nothing is checked yet.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Entry {
    pub name: String,
    pub parent: Option<PlaceId>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub radius_m: Option<i32>,
    pub category: String,
    pub icon: String,
    pub aliases: Vec<String>,
    pub geocoder_name: Option<String>,
}

/// What the operator wants to change. A field left `None` keeps its value.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Patch {
    pub name: Option<String>,
    /// `Some(None)` makes the place a top-level place; `Some(Some(id))` puts it inside `id`.
    pub parent: Option<Option<PlaceId>>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub radius_m: Option<i32>,
    pub category: Option<String>,
    pub icon: Option<String>,
    pub aliases: Option<Vec<String>>,
    /// `Some(None)` removes the geocoder name.
    pub geocoder_name: Option<Option<String>>,
}

/// The new top-level place of a split and what moves into it (PLC-004).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Split {
    pub new: Entry,
    pub children: Vec<PlaceId>,
    pub sites: Vec<SiteId>,
    pub links: Vec<RecordLinkId>,
}

/// One input field that the operator must fix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fault {
    pub field: &'static str,
    pub message: &'static str,
}

/// How many references a place has, by sort (PLC-005).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct References {
    pub links: u64,
    pub sites: u64,
    pub children: u64,
    /// One count for each later domain that holds a `PlaceId`, by the domain's name.
    pub held: Vec<(&'static str, u64)>,
}

impl References {
    #[must_use]
    pub fn total(&self) -> u64 {
        self.links + self.sites + self.children + self.held.iter().map(|(_, n)| n).sum::<u64>()
    }
}

/// What a merge moved. `held` is the same shape as in [`References`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Moved {
    pub links: u64,
    pub sites: u64,
    pub children: u64,
    pub held: Vec<(&'static str, u64)>,
}

/// The result of a merge: the remaining place and what moved to it.
#[derive(Clone, Debug, PartialEq)]
pub struct Merged {
    pub from: Place,
    pub into: Place,
    pub moved: Moved,
}

/// The result of a split: the new place and what moved to it.
#[derive(Clone, Debug, PartialEq)]
pub struct Divided {
    /// The place that was split, as it was.
    pub from: Place,
    pub new: Place,
    pub moved: Moved,
}

/// What a delete did.
#[derive(Clone, Debug, PartialEq)]
pub enum Removal {
    /// The place had no reference and is gone.
    Deleted(Place),
    /// The place had references: the call merged it into the replacement.
    Replaced(Box<Merged>),
}

/// What `set_site` changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Siting {
    pub site: SiteId,
    pub before: Option<PlaceId>,
    pub after: Option<PlaceId>,
}
