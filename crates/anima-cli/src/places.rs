//! `anima place`: where things are, as every extension names them (PLC-001 to PLC-006). A place
//! is the record `place:<id>`; the operator's name is its title.

pub mod calls;
pub mod documents;
mod shown;

use calls::{Change, Division, Fields};

use crate::failure::Failure;
use crate::invocation::Invocation;

/// The fields of a place. For `add` give a name, a category and an icon; a top-level place also
/// needs a latitude, a longitude and a radius.
#[derive(Debug, clap::Args)]
pub struct FieldArgs {
    /// The name (one line of 1 to 120 characters).
    #[arg(long)]
    name: Option<String>,
    /// The place this one is inside. A place inside another has no centre and no radius.
    #[arg(long)]
    parent: Option<String>,
    #[arg(long, allow_hyphen_values = true)]
    latitude: Option<f64>,
    #[arg(long, allow_hyphen_values = true)]
    longitude: Option<f64>,
    /// How far the place reaches, in metres (1 to 100000).
    #[arg(long)]
    radius_m: Option<i64>,
    /// home, work, site, shop, leisure, travel, storage or other.
    #[arg(long)]
    category: Option<String>,
    /// The name of an icon, such as `house`.
    #[arg(long)]
    icon: Option<String>,
    /// Another name for the place. Repeat for more; on `set` the list replaces the old one.
    #[arg(long = "alias")]
    aliases: Vec<String>,
    /// A name from a geocoder. It never replaces the name.
    #[arg(long)]
    geocoder_name: Option<String>,
}

impl FieldArgs {
    fn fields(self) -> Fields {
        Fields {
            name: self.name,
            parent_id: self.parent,
            latitude: self.latitude,
            longitude: self.longitude,
            radius_m: self.radius_m,
            category: self.category,
            icon: self.icon,
            aliases: (!self.aliases.is_empty()).then_some(self.aliases),
            geocoder_name: self.geocoder_name,
        }
    }
}

impl FieldArgs {
    /// The fields; with `no_aliases` the empty list, which removes every alias.
    fn fields_without_aliases(self, no_aliases: bool) -> Fields {
        let mut fields = self.fields();
        if no_aliases {
            fields.aliases = Some(Vec::new());
        }
        fields
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum PlaceCommand {
    /// List the places that are not merged.
    List,
    /// Show one place. A merged place shows the place it was merged into.
    Show { id: String },
    /// Add a place.
    Add(FieldArgs),
    /// Change a place. A field that you leave out keeps its value.
    Set {
        id: String,
        #[command(flatten)]
        fields: FieldArgs,
        /// Take the place out of its parent; needs --latitude, --longitude and --radius-m.
        #[arg(long)]
        top_level: bool,
        /// Remove the name from a geocoder.
        #[arg(long)]
        no_geocoder_name: bool,
        /// Remove every other name of the place.
        #[arg(long, conflicts_with = "aliases")]
        no_aliases: bool,
    },
    /// Move every reference of one place to another and keep the other.
    Merge { from: String, into: String },
    /// Make a new top-level place (the fields) and move the named references of a place to it.
    Split {
        id: String,
        #[command(flatten)]
        new: FieldArgs,
        /// A child place to move. Repeat for more.
        #[arg(long = "child")]
        children: Vec<String>,
        /// A site to move. Repeat for more.
        #[arg(long = "site")]
        sites: Vec<String>,
        /// A record link to move. Repeat for more.
        #[arg(long = "link")]
        links: Vec<String>,
    },
    /// Delete a place. With references, give a replacement: the place is then merged into it.
    Remove {
        id: String,
        #[arg(long)]
        replacement: Option<String>,
    },
    /// Link a site to a place, or with no place remove the link.
    Site { site: String, place: Option<String> },
}

impl PlaceCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal (`VALIDATION`, `NOT_FOUND`, `CONFLICT` with the counts of a
    /// refused delete) or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let output = invocation.output;
        match self {
            Self::List => output.data(&calls::list(&hub).await?, shown::rows),
            Self::Show { id } => output.data(&calls::show(&hub, id).await?, shown::record),
            Self::Add(fields) => {
                output.data(&calls::add(&hub, &fields.fields()).await?, shown::record)
            }
            Self::Set {
                id,
                fields,
                top_level,
                no_geocoder_name,
                no_aliases,
            } => {
                let change = Change {
                    fields: fields.fields_without_aliases(no_aliases),
                    top_level,
                    clear_geocoder_name: no_geocoder_name,
                };
                output.data(&calls::set(&hub, id, change).await?, shown::record)
            }
            Self::Merge { from, into } => {
                output.data(&calls::merge(&hub, from, into).await?, shown::record)
            }
            Self::Split {
                id,
                new,
                children,
                sites,
                links,
            } => {
                let division = Division {
                    new: new.fields(),
                    children,
                    sites,
                    links,
                };
                output.data(&calls::split(&hub, id, division).await?, shown::record)
            }
            Self::Remove { id, replacement } => {
                let removed = calls::remove(&hub, id, replacement).await?;
                if output.json() {
                    return output.raw(&serde_json::json!({ "id": removed }));
                }
                output.line(&format!("Removed the place {removed}."))
            }
            Self::Site { site, place } => {
                let answer = calls::site(&hub, site, place).await?;
                if answer.is_null() {
                    return output.line("The site has no place.");
                }
                output.data(&answer, shown::record)
            }
        }
    }
}
