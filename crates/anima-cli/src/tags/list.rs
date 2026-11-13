//! `anima tag list`: every tag in use. K1 has no pages for tags.

use super::documents::{TagList, tag_list};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

/// Prints the tags as a table or, with `--json`, as an array of text.
///
/// # Errors
/// [`Failure`] for a hub refusal.
pub async fn run(invocation: &Invocation) -> Result<(), Failure> {
    let hub = invocation.hub()?;
    let tags = hub.run::<TagList>(tag_list::Variables).await?.tags;
    invocation.output.data(&tags, |tags| {
        let mut table = Table::new(vec!["TAG"]);
        for tag in tags {
            table.row(vec![tag.as_str().into()]);
        }
        table
    })
}
