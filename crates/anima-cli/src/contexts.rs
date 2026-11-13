//! `anima context`: names for hub origins, and which one is current (CMD-015).

pub mod config;
pub mod select;

use serde::Serialize;

use crate::failure::Failure;
use crate::hub::Origin;
use crate::invocation::Invocation;
use crate::output::{Cell, Table, Tone};
use config::{Config, Context};

#[derive(Debug, clap::Subcommand)]
pub enum ContextCommand {
    /// List the contexts; the current one is marked.
    List,
    /// Add a context for a hub.
    Add {
        name: String,
        #[arg(help = "The hub origin, such as https://hub.anima.ing")]
        hub: String,
        /// Make it the current context.
        #[arg(long = "use")]
        make_current: bool,
    },
    /// Make a context the current one.
    Use { name: String },
    /// Remove a context (its stored token stays; `anima auth logout` deletes it).
    Remove { name: String },
}

/// One context as `context list --json` prints it.
#[derive(Serialize)]
struct Listed<'a> {
    name: &'a str,
    hub: &'a str,
    current: bool,
}

impl ContextCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a bad name or hub, or a configuration file that cannot be changed.
    pub fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let dir = invocation.config_dir()?;
        let mut config = Config::load(dir)?;
        let note = match self {
            Self::List => return list(invocation, &config),
            Self::Add {
                name,
                hub,
                make_current,
            } => add(&mut config, &name, &hub, make_current)?,
            Self::Use { name } => {
                known(&config, &name)?;
                config.current = Some(name.clone());
                format!("The current context is {name}.")
            }
            Self::Remove { name } => {
                known(&config, &name)?;
                if !invocation
                    .prompter
                    .confirm(&format!("Remove the context {name}?"))?
                {
                    invocation.output.note("Nothing was removed.");
                    return Ok(());
                }
                config.contexts.remove(&name);
                if config.current.as_deref() == Some(name.as_str()) {
                    config.current = None;
                }
                format!("Removed the context {name}.")
            }
        };
        config.save(dir)?;
        invocation.output.note(&note);
        Ok(())
    }
}

fn list(invocation: &Invocation, config: &Config) -> Result<(), Failure> {
    let listed: Vec<Listed<'_>> = config
        .contexts
        .iter()
        .map(|(name, context)| Listed {
            name,
            hub: &context.hub,
            current: config.current.as_deref() == Some(name.as_str()),
        })
        .collect();
    invocation.output.data(&listed, |listed| {
        let mut table = Table::new(vec!["CURRENT", "NAME", "HUB"]);
        for context in listed {
            let mark = if context.current { "*" } else { "" };
            table.row(vec![
                Cell::toned(mark, Tone::Good),
                context.name.into(),
                context.hub.into(),
            ]);
        }
        table
    })?;
    if config.contexts.is_empty() {
        invocation
            .output
            .note("No contexts yet: add one with `anima context add <name> <hub>`.");
    }
    Ok(())
}

fn add(config: &mut Config, name: &str, hub: &str, make_current: bool) -> Result<String, Failure> {
    check_name(name)?;
    let origin = Origin::parse(hub).map_err(|error| Failure::Usage {
        message: format!("{hub}: {error}."),
        hint: None,
    })?;
    if config.contexts.contains_key(name) {
        return Err(Failure::Conflict {
            message: format!("A context named {name} exists already."),
        });
    }
    config.contexts.insert(
        name.to_owned(),
        Context {
            hub: origin.as_text(),
        },
    );
    if make_current || config.current.is_none() {
        config.current = Some(name.to_owned());
        return Ok(format!(
            "Added the context {name} for {origin}; it is current."
        ));
    }
    Ok(format!("Added the context {name} for {origin}."))
}

fn known(config: &Config, name: &str) -> Result<(), Failure> {
    if config.contexts.contains_key(name) {
        return Ok(());
    }
    Err(Failure::NotFound {
        message: format!("There is no context named {name}."),
        hint: Some("anima context list".to_owned()),
    })
}

/// 1 to 64 letters, digits, `.`, `_` or `-`, so a name is safe in a table and a shell.
fn check_name(name: &str) -> Result<(), Failure> {
    let fits = (1..=64).contains(&name.len())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte));
    if fits {
        return Ok(());
    }
    Err(Failure::Usage {
        message: "A context name is 1 to 64 letters, digits, '.', '_' or '-'.".to_owned(),
        hint: None,
    })
}
