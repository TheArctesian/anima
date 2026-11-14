//! `anima extension on <key>` and `anima extension off <key>` (EXT-011, EXT-012).

use std::collections::BTreeMap;

use super::documents::{ExtensionList, ExtensionSet, extension_list, extension_set};
use super::list::state;
use super::needs::also_turned_on;
use crate::failure::{Failure, Hint};
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct OnArgs {
    /// The extension's key, such as projects.
    key: String,
}

#[derive(Debug, clap::Args)]
pub struct OffArgs {
    /// The extension's key, such as projects.
    key: String,
}

impl OnArgs {
    /// Turns the extension on, after a confirmation (or `--yes`, CMD-024) when that also turns
    /// others on, and prints each extension whose switch changed, or that none did.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown key; [`Failure::Declined`] when the operator says
    /// no; the hub's refusal (`CONFLICT` for an extension that is not built).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let listed = hub.run::<ExtensionList>(extension_list::Variables).await?;
        let known: BTreeMap<&str, (bool, &[String])> = listed
            .extensions
            .iter()
            .map(|extension| {
                (
                    extension.key.as_str(),
                    (extension.on, extension.needs.as_slice()),
                )
            })
            .collect();
        if !known.contains_key(self.key.as_str()) {
            return Err(Failure::NotFound {
                message: format!("There is no extension with the key {}.", self.key),
                hint: Some(Hint::run("anima extension list")),
            });
        }
        let also = also_turned_on(&known, &self.key);
        if !also.is_empty() {
            invocation.prompter.confirm(&format!(
                "Turning on {} also turns on {}. Continue?",
                self.key,
                also.join(", ")
            ))?;
        }
        set(invocation, &self.key, true).await
    }
}

impl OffArgs {
    /// Turns the extension off and prints it, or that it was already off.
    ///
    /// # Errors
    /// The hub's refusal: `CONFLICT`, naming the extension that needs it (exit 6).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        set(invocation, &self.key, false).await
    }
}

async fn set(invocation: &Invocation, key: &str, on: bool) -> Result<(), Failure> {
    let hub = invocation.hub()?;
    let variables = extension_set::Variables {
        key: key.to_owned(),
        on,
    };
    let answer = hub.run::<ExtensionSet>(variables).await?;
    // The hub lists only the switches it changed: none when the extension was already so.
    if answer.set_extension.is_empty() && !invocation.output.json() {
        let state = if on { "on" } else { "off" };
        return invocation
            .output
            .line(&format!("{key} is already {state}; nothing changed."));
    }
    invocation.output.data(&answer.set_extension, table)
}

fn table(extensions: &Vec<extension_set::ExtensionSetSetExtension>) -> Table {
    let mut table = Table::new(vec!["KEY", "NAME", "STATE"]);
    for extension in extensions {
        table.row(vec![
            extension.key.as_str().into(),
            extension.name.as_str().into(),
            state(extension.fixed, extension.built, extension.on),
        ]);
    }
    table
}
