//! `anima model list`: the models of the library (SEL-005).

use super::documents::{ModelList, model_list};
use super::gib;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct ListArgs {}

impl ListArgs {
    /// Prints one row for each variant, or with `--json` the models.
    ///
    /// # Errors
    /// The hub's refusal; a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let models = hub
            .run::<ModelList>(model_list::Variables)
            .await?
            .library_models;
        invocation.output.data(&models, |models| table(models))?;
        if models.is_empty() {
            invocation.output.note(
                "The library is empty. Add a model with anima model add <organisation/name>.",
            );
        }
        Ok(())
    }
}

fn table(models: &[model_list::ModelListLibraryModels]) -> Table {
    let mut table = Table::new(vec![
        "ADDRESS",
        "FORMAT",
        "SIZE",
        "CONTEXT",
        "CAPABILITIES",
        "ALIASES",
    ]);
    for model in models {
        let capabilities: Vec<String> = model.capabilities.iter().map(enum_name).collect();
        let context = model
            .context_length
            .map_or_else(String::new, |each| each.to_string());
        for variant in &model.variants {
            table.row(vec![
                variant.address.as_str().into(),
                enum_name(&variant.format).into(),
                gib(variant.size_bytes).into(),
                context.as_str().into(),
                capabilities.join(", ").into(),
                model.aliases.join(", ").into(),
            ]);
        }
    }
    table
}
