//! `anima extension list`.

use super::documents::{ExtensionList, extension_list};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::text::enum_name;
use crate::output::{Cell, Table, Tone};

#[derive(Debug, clap::Args)]
pub struct ListArgs;

impl ListArgs {
    /// Prints the extensions as a table or, with `--json`, as the K1 `Extension` list.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let answer = hub.run::<ExtensionList>(extension_list::Variables).await?;
        invocation.output.data(&answer.extensions, table)
    }
}

fn table(extensions: &Vec<extension_list::ExtensionListExtensions>) -> Table {
    let mut table = Table::new(vec!["KEY", "NAME", "STATE", "DATA", "NEEDS", "NEEDED BY"]);
    for extension in extensions {
        table.row(vec![
            extension.key.as_str().into(),
            extension.name.as_str().into(),
            state(extension.fixed, extension.built, extension.on),
            enum_name(&extension.data_class).into(),
            words(&extension.needs).into(),
            words(&extension.needed_by).into(),
        ]);
    }
    table
}

/// `fixed` (part of the engine), `not built`, `on` or `off`, in colour.
#[must_use]
pub fn state(fixed: bool, built: bool, on: bool) -> Cell {
    match (fixed, built, on) {
        (true, _, _) => Cell::toned("fixed", Tone::Muted),
        (_, false, _) => Cell::toned("not built", Tone::Muted),
        (_, _, true) => Cell::toned("on", Tone::Good),
        _ => Cell::new("off"),
    }
}

fn words(keys: &[String]) -> String {
    if keys.is_empty() {
        "-".to_owned()
    } else {
        keys.join(", ")
    }
}
