//! `anima notify deliveries`: what the hub sent, newest first (NOT-004), in pages (CMD-026).

use super::channel::resolve;
use super::documents::{DeliveryList, delivery_list};
use crate::events::paging::{MAX_PAGE, Page, fetch, note_next, page_size};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::text::enum_name;
use crate::output::{Cell, Table, Tone};

#[derive(Debug, clap::Args)]
pub struct DeliveriesArgs {
    /// Only deliveries through this channel, by name or ID.
    #[arg(long)]
    channel: Option<String>,
    /// Deliveries per page, 1 to 200.
    #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(i64).range(1..=MAX_PAGE))]
    limit: i64,
    /// Start after this cursor (from the previous page).
    #[arg(long, conflicts_with = "all")]
    after: Option<String>,
    /// Every page, not only the first.
    #[arg(long)]
    all: bool,
}

impl DeliveriesArgs {
    /// Prints the deliveries as a table or, with `--json`, as the K1 `DeliveryConnection`.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown channel; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let channel_id = match &self.channel {
            Some(channel) => Some(resolve(&hub, channel).await?),
            None => None,
        };
        let first = Some(page_size(self.limit, self.all));
        let deliveries = fetch(self.after, self.all, async |after| {
            let variables = delivery_list::Variables {
                channel_id: channel_id.clone(),
                first,
                after,
            };
            Ok(hub.run::<DeliveryList>(variables).await?.deliveries)
        })
        .await?;
        invocation.output.data(&deliveries, table)?;
        note_next(
            invocation,
            &deliveries,
            "deliveries",
            "anima notify deliveries",
        );
        Ok(())
    }
}

impl Page for delivery_list::DeliveryListDeliveries {
    fn next_cursor(&self) -> Option<String> {
        self.page_info
            .end_cursor
            .clone()
            .filter(|_| self.page_info.has_next_page)
    }

    fn append(&mut self, later: Self) {
        self.edges.extend(later.edges);
        self.page_info = later.page_info;
    }

    fn end(&mut self) {
        self.page_info.has_next_page = false;
    }

    fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

fn table(deliveries: &delivery_list::DeliveryListDeliveries) -> Table {
    let mut table = Table::new(vec!["AT", "CHANNEL", "KIND", "RESULT"]);
    for edge in &deliveries.edges {
        let delivery = &edge.node;
        table.row(vec![
            delivery.at.as_str().into(),
            delivery.channel_name.as_str().into(),
            enum_name(&delivery.kind).into(),
            result_cell(delivery.ok, delivery.reason.as_ref(), delivery.code),
        ]);
    }
    table
}

/// `ok`, or `failed:` with the K1 reason and the HTTP or SMTP code. The hub's reasons are
/// fixed words, so no text from the target is shown (CR-14).
pub fn result_cell(ok: bool, reason: Option<&impl serde::Serialize>, code: Option<i64>) -> Cell {
    if ok {
        return Cell::toned("ok", Tone::Good);
    }
    let reason = reason.map_or_default(enum_name);
    let code = code.map_or_default(|code| format!(" {code}"));
    Cell::toned(&format!("failed: {reason}{code}"), Tone::Bad)
}
