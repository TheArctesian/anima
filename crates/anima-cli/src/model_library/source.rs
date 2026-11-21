//! `anima model source`: the approved organisations of the sources (LIBM-003).

use super::documents::{ModelSourceAdd, ModelSources, model_source_add, model_sources};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Subcommand)]
pub enum SourceCommand {
    /// List the approved organisations.
    List,
    /// Add an organisation to the approved list.
    Add {
        /// The Hugging Face organisation, such as bartowski.
        organisation: String,
    },
}

fn table(sources: &Vec<String>) -> Table {
    let mut table = Table::new(vec!["ORGANISATION"]);
    for source in sources {
        table.row(vec![source.as_str().into()]);
    }
    table
}

impl SourceCommand {
    /// Prints the approved organisations.
    ///
    /// # Errors
    /// The hub's refusal; a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let sources = match self {
            Self::List => {
                hub.run::<ModelSources>(model_sources::Variables)
                    .await?
                    .approved_model_sources
            }
            Self::Add { organisation } => {
                hub.run::<ModelSourceAdd>(model_source_add::Variables { organisation })
                    .await?
                    .add_approved_model_source
            }
        };
        invocation.output.data(&sources, table)
    }
}
