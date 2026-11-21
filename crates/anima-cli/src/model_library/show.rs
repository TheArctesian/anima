//! `anima model show`: one model with its variants, files and SHA-256 values.

use super::documents::{ModelShow, model_show};
use super::gib;
use crate::failure::{Failure, Hint};
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct ShowArgs {
    /// The model's name, an alias, or name:tag.
    name: String,
}

impl ShowArgs {
    /// Prints the model as a record and a line for each file, or with `--json` the model.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown model; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let model = hub
            .run::<ModelShow>(model_show::Variables {
                name: self.name.clone(),
            })
            .await?
            .library_model
            .ok_or_else(|| Failure::NotFound {
                message: format!("No model has the name or alias {}.", self.name),
                hint: Some(Hint::run("anima model list")),
            })?;
        invocation.output.data(&model, record)
    }
}

fn number(value: Option<i64>) -> String {
    value.map_or_else(|| "unknown".to_owned(), |each| each.to_string())
}

#[expect(clippy::cast_possible_truncation, reason = "a count far below 2^53")]
fn record(model: &model_show::ModelFields) -> Table {
    let mut table = Table::record();
    let capabilities: Vec<String> = model.capabilities.iter().map(enum_name).collect();
    table.pair("Name", model.name.as_str());
    table.pair("Family", model.family.clone().unwrap_or_default());
    table.pair(
        "Parameters",
        number(model.parameter_count.map(|each| each as i64)),
    );
    table.pair("Context", number(model.context_length));
    table.pair("Layers", number(model.layer_count));
    table.pair("Capabilities", capabilities.join(", "));
    table.pair("Licence", model.licence.clone().unwrap_or_default());
    table.pair("Publisher", model.publisher.clone().unwrap_or_default());
    table.pair("Aliases", model.aliases.join(", "));
    for variant in &model.variants {
        let head = format!(
            "{} {} {} from {}@{}",
            enum_name(&variant.format),
            variant.quantisation,
            gib(variant.size_bytes),
            variant.source_repository,
            &variant.revision[..variant.revision.len().min(12)]
        );
        table.pair(&variant.address, head);
        for file in &variant.files {
            table.pair(
                "",
                format!("{} {} {}", file.sha256, gib(file.size_bytes), file.path),
            );
        }
    }
    table
}
