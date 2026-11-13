//! `anima check`: the checks of the hub's devices (CMD-002). A check's kind and settings are
//! a JSON file (`--from-file`) holding the K1 `CheckConfigInput`; see `config_file`.

pub mod config_file;
pub mod defaults;
pub mod read;
pub mod resolve;
pub mod settings;
pub mod summary;
pub mod watch;
pub mod write;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum CheckCommand {
    /// List the checks of a device, with their kind and key settings.
    List(read::list::ListArgs),
    /// Show one check by ID, or by exact name with --device.
    Get(read::get::GetArgs),
    /// Create a check from a JSON file with its kind and settings.
    Create(write::create::CreateArgs),
    /// Change a check's settings.
    Update(write::update::UpdateArgs),
    /// Delete a check and its heartbeats.
    Delete(write::delete::DeleteArgs),
    /// Stop a check's runs.
    Pause(write::pause::PauseArgs),
    /// Restart a paused check.
    Resume(write::pause::ResumeArgs),
    /// Show a page of a check's heartbeats, newest first.
    Heartbeats(read::heartbeats::HeartbeatsArgs),
    /// Print each heartbeat until interrupted.
    Watch(watch::WatchArgs),
    /// The values a new check starts with.
    Defaults {
        #[command(subcommand)]
        command: DefaultsCommand,
    },
    /// Which channels a check tells.
    Notifications {
        #[command(subcommand)]
        command: NotificationsCommand,
    },
}

#[derive(Debug, clap::Subcommand)]
pub enum DefaultsCommand {
    /// Show the defaults.
    Get(defaults::get::GetArgs),
    /// Change the defaults.
    Set(defaults::set::SetArgs),
}

#[derive(Debug, clap::Subcommand)]
pub enum NotificationsCommand {
    /// Set the channels a check tells and how often it repeats.
    Set(write::notifications::NotificationsSetArgs),
}

impl CheckCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal, an unknown device or check, or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
            Self::Get(args) => args.run(invocation).await,
            Self::Create(args) => args.run(invocation).await,
            Self::Update(args) => args.run(invocation).await,
            Self::Delete(args) => args.run(invocation).await,
            Self::Pause(args) => args.run(invocation).await,
            Self::Resume(args) => args.run(invocation).await,
            Self::Heartbeats(args) => args.run(invocation).await,
            Self::Watch(args) => args.run(invocation).await,
            Self::Defaults { command } => match command {
                DefaultsCommand::Get(args) => args.run(invocation).await,
                DefaultsCommand::Set(args) => args.run(invocation).await,
            },
            Self::Notifications { command } => match command {
                NotificationsCommand::Set(args) => args.run(invocation).await,
            },
        }
    }
}
