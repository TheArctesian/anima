//! `anima model source`: the approved organisations of the sources (LIBM-003).

use super::documents::{
    ModelApproval, ModelSetApproval, ModelSourceAdd, ModelSources, model_approval,
    model_set_approval, model_source_add, model_sources,
};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::enum_name;

#[derive(Debug, clap::Subcommand)]
pub enum SourceCommand {
    /// List the approved organisations.
    List,
    /// Add an organisation to the approved list.
    Add {
        /// The Hugging Face organisation, such as bartowski.
        organisation: String,
    },
    /// Show or set how the publisher of a base model joins the list: base-model (a person who
    /// adds a model approves its publisher) or by-hand.
    Mode { mode: Option<Mode> },
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum Mode {
    BaseModel,
    ByHand,
}

impl From<Mode> for model_set_approval::ModelPublisherApproval {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::BaseModel => Self::BASE_MODEL,
            Mode::ByHand => Self::BY_HAND,
        }
    }
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
        if let Self::Mode { mode } = self {
            let shown = match mode {
                None => enum_name(
                    &hub.run::<ModelApproval>(model_approval::Variables)
                        .await?
                        .model_publisher_approval,
                ),
                Some(mode) => enum_name(
                    &hub.run::<ModelSetApproval>(model_set_approval::Variables {
                        mode: mode.into(),
                    })
                    .await?
                    .set_model_publisher_approval,
                ),
            };
            return invocation.output.data(&shown, |shown| {
                let mut table = Table::record();
                table.pair("Publisher approval", shown.as_str());
                table
            });
        }
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
            Self::Mode { .. } => return Ok(()),
        };
        invocation.output.data(&sources, table)
    }
}
