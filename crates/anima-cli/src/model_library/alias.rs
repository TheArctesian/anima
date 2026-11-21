//! `anima model alias`: points an alias at a model (LIBM-007, SEL-003).

use super::documents::{ModelAlias, model_alias};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct AliasArgs {
    /// The alias, such as coder or fast.
    alias: String,
    /// The model's name, an alias, or name:tag.
    model: String,
}

impl AliasArgs {
    /// Prints the alias and its model.
    ///
    /// # Errors
    /// The hub's refusal; [`Failure::NotFound`] for an unknown model.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let set = hub
            .run::<ModelAlias>(model_alias::Variables {
                alias: self.alias,
                model: self.model,
            })
            .await?
            .set_model_alias;
        invocation.output.data(&set, |set| {
            let mut table = Table::record();
            table.pair("Alias", set.alias.as_str());
            table.pair("Model", set.model.as_str());
            table
        })
    }
}
