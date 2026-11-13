//! One run of `anima`: its environment, its global flags, and from them the hub, the token
//! and the output. Commands take an [`Invocation`] and ask it for what they need.

use std::path::Path;

use serde::Serialize;

use crate::auth::token_file::TokenFile;
use crate::auth::token_store::TokenStore;
use crate::contexts::config::{Config, ConfigError};
use crate::contexts::select::{Selection, select};
use crate::environment::Environment;
use crate::failure::Failure;
use crate::hub::{Hub, Origin, Token};
use crate::output::Output;
use crate::prompt::Prompter;

/// The flags every command takes (CMD-013, CMD-020, CMD-023, CMD-024).
#[derive(Debug, Default, Clone, clap::Args)]
pub struct Globals {
    /// Print JSON with the hub's field names; errors as JSON on standard error.
    #[arg(long, global = true)]
    pub json: bool,
    /// Never ask a question; fail and name the flag that is missing.
    #[arg(long, global = true)]
    pub no_input: bool,
    /// Answer yes to every confirmation, such as a delete.
    #[arg(long, global = true)]
    pub yes: bool,
    #[arg(
        long,
        global = true,
        value_name = "ORIGIN",
        help = "The hub origin, such as https://hub.anima.ing (before contexts and ANIMA_HUB)"
    )]
    pub hub: Option<String>,
    /// The context to use instead of the current one.
    #[arg(long, global = true, value_name = "NAME")]
    pub context: Option<String>,
}

/// Where the token came from (CMD-013).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TokenSource {
    /// `ANIMA_TOKEN`.
    Environment,
    /// The key store or the token file.
    Store,
}

pub struct Invocation {
    environment: Environment,
    globals: Globals,
    pub output: Output,
    pub prompter: Prompter,
}

impl Invocation {
    #[must_use]
    pub fn new(environment: Environment, globals: Globals) -> Self {
        let output = Output::new(
            globals.json,
            environment.terminals.stdout,
            environment.no_colour,
        );
        let prompter = Prompter::new(environment.terminals.stdin, globals.no_input, globals.yes);
        Self {
            environment,
            globals,
            output,
            prompter,
        }
    }

    /// The configuration folder.
    ///
    /// # Errors
    /// [`ConfigError::NoFolder`] when neither `XDG_CONFIG_HOME` nor `HOME` is set.
    pub fn config_dir(&self) -> Result<&Path, Failure> {
        Ok(self
            .environment
            .config_dir
            .as_deref()
            .ok_or(ConfigError::NoFolder)?)
    }

    /// The contexts file.
    ///
    /// # Errors
    /// [`Failure::Config`] when it cannot be read.
    pub fn config(&self) -> Result<Config, Failure> {
        Ok(Config::load(self.config_dir()?)?)
    }

    /// The hub this run talks to (CMD-013).
    ///
    /// # Errors
    /// As [`select`], and when the configuration cannot be read.
    pub fn origin(&self) -> Result<Origin, Failure> {
        let selection = Selection {
            hub: self.globals.hub.as_deref(),
            context: self.globals.context.as_deref(),
            hub_variable: self.environment.hub.as_deref(),
        };
        let config = if self.globals.hub.is_some() {
            Config::default()
        } else {
            self.config()?
        };
        select(selection, &config)
    }

    /// Where tokens are kept (CMD-012).
    #[must_use]
    pub fn token_store(&self) -> TokenStore {
        let file = self.environment.config_dir.as_deref().map(TokenFile::new);
        TokenStore::for_system(file, self.environment.token_file)
    }

    /// Whether standard input is a terminal.
    #[must_use]
    pub const fn stdin_terminal(&self) -> bool {
        self.environment.terminals.stdin
    }

    /// Whether `ANIMA_TOKEN` is set.
    #[must_use]
    pub const fn has_token_variable(&self) -> bool {
        self.environment.token.is_some()
    }

    /// A client for the selected hub with the operator's token: `ANIMA_TOKEN`, else the stored
    /// one (CMD-013).
    ///
    /// # Errors
    /// [`Failure::SignedOut`] when there is no token; a store or hub setup failure.
    pub fn signed_in(&self) -> Result<(Hub, TokenSource), Failure> {
        let origin = self.origin()?;
        let (token, source) = match self.variable_token()? {
            Some(token) => (token, TokenSource::Environment),
            None => match self.token_store().read(&origin)? {
                Some(token) => (token, TokenSource::Store),
                None => {
                    return Err(Failure::SignedOut {
                        origin: origin.as_text(),
                    });
                }
            },
        };
        Ok((Hub::new(origin, Some(token))?, source))
    }

    /// [`Invocation::signed_in`] without the source.
    ///
    /// # Errors
    /// As [`Invocation::signed_in`].
    pub fn hub(&self) -> Result<Hub, Failure> {
        self.signed_in().map(|(hub, _)| hub)
    }

    fn variable_token(&self) -> Result<Option<Token>, Failure> {
        let Some(text) = &self.environment.token else {
            return Ok(None);
        };
        Token::new(text.clone())
            .map(Some)
            .ok_or_else(|| Failure::Usage {
                message: "ANIMA_TOKEN is not a token: it is empty or holds spaces.".to_owned(),
                hint: None,
            })
    }
}
