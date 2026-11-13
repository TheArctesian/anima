//! `anima audit list`: who changed what (K1 `audit`), newest first, in pages (CMD-026), with
//! who did it: a person, an AI agent or the hub (LOG-012, LOG-020).

use super::documents::{AuditList, audit_list};
use crate::connection_page;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::enum_name;
use crate::paging::{CommandLine, MAX_PAGE, fetch, note_next, page_size};

/// What the list shows for an entry written before the hub recorded actor kinds (LOG-020).
pub const NOT_RECORDED: &str = "Not recorded";

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// Only the entries of this actor: a person (user), an AI agent (agent) or the hub itself.
    #[arg(long, value_enum)]
    actor: Option<ActorArg>,
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

/// K1 `ActorKind`.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum ActorArg {
    User,
    Agent,
    Hub,
}

impl ListArgs {
    /// Prints the entries as a table or, with `--json`, as the K1 `AuditConnection`, change
    /// records included (the hub has already replaced secret values in them).
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let command =
            CommandLine::new("anima audit list").choices("--actor", self.actor.as_slice());
        let hub = invocation.hub()?;
        let first = Some(page_size(self.limit, self.all, MAX_PAGE));
        let (hub, actor) = (&hub, self.actor);
        let entries = fetch(self.after, self.all, move |after| async move {
            let variables = audit_list::Variables {
                first,
                after,
                actor_kind: actor.map(ActorArg::k1),
            };
            Ok(hub.run::<AuditList>(variables).await?.audit)
        })
        .await?;
        invocation.output.data(&entries, table)?;
        note_next(invocation, &entries, "entries", &command);
        Ok(())
    }
}

connection_page!(audit_list::AuditListAudit);

fn table(entries: &audit_list::AuditListAudit) -> Table {
    let mut table = Table::new(vec![
        "AT", "ACTOR", "OPERATOR", "TOKEN", "KEY", "ACTION", "TARGET", "FROM",
    ]);
    for edge in &entries.edges {
        let entry = &edge.node;
        let actor = entry
            .actor_kind
            .as_ref()
            .map_or_else(|| NOT_RECORDED.to_owned(), enum_name);
        table.row(vec![
            entry.at.as_str().into(),
            actor.into(),
            entry.operator_name.as_str().into(),
            entry
                .token_kind
                .as_ref()
                .map_or_else(|| "-".to_owned(), enum_name)
                .into(),
            entry.api_key_name.as_deref().unwrap_or("-").into(),
            entry.action.as_str().into(),
            format!("{} {}", entry.target_type, entry.target_id).into(),
            entry.client_addr.as_str().into(),
        ]);
    }
    table
}

impl ActorArg {
    const fn k1(self) -> audit_list::ActorKind {
        match self {
            Self::User => audit_list::ActorKind::USER,
            Self::Agent => audit_list::ActorKind::AGENT,
            Self::Hub => audit_list::ActorKind::HUB,
        }
    }
}
