//! `anima model add`: reads a repository of an approved organisation (LIBM-004).

use super::documents::{ModelAdd, model_add};
use super::gib;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct AddArgs {
    /// The Hugging Face repository, organisation/name, such as unsloth/Qwen3-8B-GGUF.
    repository: String,
    /// The model name; by default the hub makes one from the repository name.
    #[arg(long)]
    name: Option<String>,
}

impl AddArgs {
    /// Prints the variants that the hub stored.
    ///
    /// # Errors
    /// The hub's refusal, for example for an organisation that is not approved.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let model = hub
            .run::<ModelAdd>(model_add::Variables {
                repository: self.repository,
                name: self.name,
            })
            .await?
            .add_library_model;
        invocation.output.data(&model, |model| {
            let mut table = Table::new(vec!["ADDRESS", "FORMAT", "SIZE"]);
            for variant in &model.variants {
                table.row(vec![
                    variant.address.as_str().into(),
                    enum_name(&variant.format).into(),
                    gib(variant.size_bytes).into(),
                ]);
            }
            table
        })
    }
}
