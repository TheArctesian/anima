//! `anima timeline list`: the hub's timeline, newest first, with pages (CMD-026).

use chrono::Utc;

use super::documents::{TimelineList, timeline_list};
use super::window::window;
use crate::connection_page;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::enum_name;
use crate::paging::{CommandLine, MAX_PAGE, fetch, note_next, page_size};

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// Start of the span (RFC 3339, such as 2026-11-01T00:00:00Z); one day before --to when
    /// not given.
    #[arg(long)]
    from: Option<String>,
    /// End of the span, not included (RFC 3339); now when not given.
    #[arg(long)]
    to: Option<String>,
    /// Only entries of this category; repeat for more than one.
    #[arg(long, value_enum)]
    category: Vec<CategoryArg>,
    /// Entries per page, 1 to 200.
    #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(i64).range(1..=MAX_PAGE))]
    limit: i64,
    /// Start after this cursor (from the previous page).
    #[arg(long, conflicts_with = "all")]
    after: Option<String>,
    /// Every page, not only the first.
    #[arg(long)]
    all: bool,
}

/// K1 `EvidenceCategory`, the closed list of TML-004.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum CategoryArg {
    Sleep,
    Workout,
    Visit,
    Travel,
    Code,
    Message,
    Post,
    Media,
    Reading,
    Spending,
    Document,
    Device,
    Session,
    Note,
}

impl CategoryArg {
    const fn k1(self) -> timeline_list::EvidenceCategory {
        use timeline_list::EvidenceCategory as K1;
        match self {
            Self::Sleep => K1::SLEEP,
            Self::Workout => K1::WORKOUT,
            Self::Visit => K1::VISIT,
            Self::Travel => K1::TRAVEL,
            Self::Code => K1::CODE,
            Self::Message => K1::MESSAGE,
            Self::Post => K1::POST,
            Self::Media => K1::MEDIA,
            Self::Reading => K1::READING,
            Self::Spending => K1::SPENDING,
            Self::Document => K1::DOCUMENT,
            Self::Device => K1::DEVICE,
            Self::Session => K1::SESSION,
            Self::Note => K1::NOTE,
        }
    }
}

impl ListArgs {
    /// `anima timeline list` with the span and filters in use, for the next-page hint.
    fn next_page_command(&self, (from, to): (&str, &str)) -> CommandLine {
        CommandLine::new("anima timeline list", self.limit)
            .flag("--from", Some(from))
            .flag("--to", Some(to))
            .choices("--category", &self.category)
    }

    /// Prints the entries as a table or, with `--json`, as the K1 `EvidenceConnection`.
    ///
    /// # Errors
    /// [`Failure::Usage`] for an unusable `--from` or `--to`; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let (from, to) =
            window(self.from.as_deref(), self.to.as_deref(), Utc::now()).map_err(|error| {
                Failure::Usage {
                    message: error.to_string(),
                    hint: None,
                }
            })?;
        let command = self.next_page_command((&from, &to));
        let hub = invocation.hub()?;
        let categories = Some(self.category.iter().map(|c| c.k1()).collect())
            .filter(|categories: &Vec<_>| !categories.is_empty());
        let first = Some(page_size(self.limit, self.all, MAX_PAGE));
        let variables = timeline_list::Variables {
            from,
            to,
            categories,
            first,
            after: None,
        };
        let (hub, variables) = (&hub, &variables);
        let timeline = fetch(self.after, self.all, move |after| async move {
            let variables = timeline_list::Variables {
                after,
                ..variables.clone()
            };
            Ok(hub.run::<TimelineList>(variables).await?.timeline)
        })
        .await?;
        invocation.output.data(&timeline, table)?;
        note_next(invocation, &timeline, "entries", &command);
        Ok(())
    }
}

connection_page!(timeline_list::TimelineListTimeline);

fn table(timeline: &timeline_list::TimelineListTimeline) -> Table {
    let mut table = Table::new(vec![
        "START", "CATEGORY", "SOURCE", "BASIS", "TITLE", "RECORD", "NOTES",
    ]);
    for edge in &timeline.edges {
        let entry = &edge.node;
        let mut notes = entry.uncertainty.clone();
        if let Some(leader) = &entry.superseded_by {
            notes.push(format!("Superseded by {leader}."));
        }
        table.row(vec![
            entry.start.as_str().into(),
            enum_name(&entry.category).into(),
            entry.source.as_str().into(),
            enum_name(&entry.basis).into(),
            entry.title.as_str().into(),
            entry.reference.as_str().into(),
            notes.join(" ").as_str().into(),
        ]);
    }
    table
}
