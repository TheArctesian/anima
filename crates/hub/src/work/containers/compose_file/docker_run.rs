//! The change of a `docker run` command into a Compose file (HOST-022). It reads the words of
//! the command with its own reader, which expands nothing, and it runs nothing.

#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::Enum;

use super::{
    LARGEST_COMMAND,
    draft::Draft,
    options,
    words::{self, Word},
    writer::{self, Value},
};

/// Why a command was not converted, as a fixed code (K1 `ComposeRunRefusal`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "ComposeRunRefusal")]
pub enum RefusalCode {
    /// The command is larger than 64 KiB.
    TooLarge,
    /// The text holds no command.
    NoCommand,
    /// The command is not `docker run` or `docker container run`.
    NotDockerRun,
    /// The command has `$(` or a backtick: the shell would run a second command.
    CommandSubstitution,
    /// The command has a pipe.
    Pipe,
    /// The text holds more than one command: `;`, `&&`, `||`, `&`, or a line break with no
    /// backslash before it.
    MoreThanOneCommand,
    /// The command sends its input or its output to a different place.
    Redirection,
    /// The command has a form of the shell that the converter does not read: a group in
    /// round brackets, or a comment.
    ShellSyntax,
    /// The command has a variable that cannot stay as text in a Compose file.
    VariableNotKept,
    /// A quote is not closed.
    UnclosedQuote,
    /// The command holds a control character.
    ControlCharacter,
    /// The last option needs a value and has none.
    OptionWithoutValue,
    /// The command names no image.
    NoImage,
}

/// A command that the converter does not convert: the code, and the position in the command
/// (in characters, from 1) where it stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub code: RefusalCode,
    pub at: u32,
}

/// What the caller must know about the file (K1 `ComposeRunNote`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "ComposeRunNote")]
pub enum Note {
    /// `-d` changes nothing: Compose starts a service in the background.
    DetachNotNeeded,
    /// `--rm` has no key in a Compose file: the container stays until the project is removed.
    RemoveNotKept,
    /// A `$NAME` or `${NAME}` stays as text: Compose fills it when it starts the project.
    VariablesKept,
    /// A `$` that is a character is written `$$`, as Compose needs it.
    DollarWritten,
    /// A network of the command is `external`: it must exist before the project starts.
    ExternalNetwork,
    /// A volume keeps its name of Docker, so the project uses the same volume.
    VolumeNameKept,
    /// An option that Docker does not have was read as an option with no value: check that
    /// the image is the right word.
    UnknownOptionReadAsFlag,
}

/// The Compose file of one command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Converted {
    /// The text of the Compose file.
    pub compose_file: String,
    /// The name of the one service of the file.
    pub service: String,
    pub notes: Vec<Note>,
    /// The options that are not in the file, by name only.
    pub not_converted: Vec<String>,
}

/// The words of a command that are not read yet.
struct Arguments<'a> {
    words: &'a [Word],
    end: u32,
}

impl<'a> Arguments<'a> {
    fn take(&mut self) -> Option<&'a Word> {
        let (first, rest) = self.words.split_first()?;
        self.words = rest;
        Some(first)
    }

    fn take_if(&mut self, wanted: impl Fn(&str) -> bool) -> bool {
        let found = self.words.first().is_some_and(|word| wanted(&word.text));
        if found {
            self.words = &self.words[1..];
        }
        found
    }

    fn refusal(&self, code: RefusalCode) -> Refusal {
        let at = self.words.first().map_or(self.end, |word| word.at);
        Refusal { code, at }
    }

    /// The value of the option before it.
    fn value(&mut self) -> Result<&'a str, Refusal> {
        let missing = self.refusal(RefusalCode::OptionWithoutValue);
        self.take().map(|word| word.text.as_str()).ok_or(missing)
    }
}

/// Changes one `docker run` command into a Compose file with one service.
///
/// # Errors
/// [`Refusal`] for a text that is not one `docker run` command that the converter can read.
pub fn convert(command: &str) -> Result<Converted, Refusal> {
    if command.len() > LARGEST_COMMAND {
        let code = RefusalCode::TooLarge;
        return Err(Refusal { code, at: 1 });
    }
    let split = words::split(command)?;
    let end = u32::try_from(command.chars().count()).unwrap_or(u32::MAX);
    let mut arguments = Arguments {
        words: &split.words,
        end: end.max(1),
    };
    docker_run(&mut arguments)?;
    let mut draft = Draft::default();
    let image = read_options(&mut arguments, &mut draft)?;
    let image = image.ok_or_else(|| arguments.refusal(RefusalCode::NoImage))?;
    let command = arguments.words.iter().map(|word| Value::text(&word.text));
    let (service, file) = draft.file(image, command.collect());
    if split.variables {
        draft.note(Note::VariablesKept);
    }
    if split.dollars {
        draft.note(Note::DollarWritten);
    }
    Ok(Converted {
        compose_file: writer::write(&file),
        service,
        notes: draft.notes,
        not_converted: draft.not_converted,
    })
}

/// Takes `docker run` or `docker container run`, with or without `sudo` before it.
fn docker_run(arguments: &mut Arguments<'_>) -> Result<(), Refusal> {
    if arguments.words.is_empty() {
        let code = RefusalCode::NoCommand;
        return Err(Refusal { code, at: 1 });
    }
    arguments.take_if(|word| word == "sudo");
    if arguments.take_if(|word| word == "docker" || word.ends_with("/docker")) {
        arguments.take_if(|word| word == "container");
        if arguments.take_if(|word| word == "run") {
            return Ok(());
        }
    }
    Err(arguments.refusal(RefusalCode::NotDockerRun))
}

/// Reads the options up to the image, and gives the image.
fn read_options<'a>(
    arguments: &mut Arguments<'a>,
    draft: &mut Draft,
) -> Result<Option<&'a str>, Refusal> {
    while let Some(word) = arguments.take() {
        let text = word.text.as_str();
        if text == "--" {
            return Ok(arguments.take().map(|word| word.text.as_str()));
        }
        if let Some(long) = text.strip_prefix("--") {
            long_option(long, arguments, draft)?;
        } else if let Some(letters) = text.strip_prefix('-').filter(|rest| !rest.is_empty()) {
            short_options(letters, arguments, draft)?;
        } else {
            return Ok(Some(text));
        }
    }
    Ok(None)
}

/// `--option`, `--option value` or `--option=value`.
fn long_option(
    long: &str,
    arguments: &mut Arguments<'_>,
    draft: &mut Draft,
) -> Result<(), Refusal> {
    let (name, given) = match long.split_once('=') {
        Some((name, value)) => (name, Some(value)),
        None => (long, None),
    };
    let option = format!("--{name}");
    match options::long(name) {
        None => draft.unknown(&option, given.is_none()),
        Some(reads) if reads.takes_value() => {
            let value = match given {
                Some(value) => value,
                None => arguments.value()?,
            };
            draft.apply(reads, &option, value);
        }
        Some(reads) => draft.flag(reads, &option, given),
    }
    Ok(())
}

/// One or more options of one letter in one word: `-it`, `-dp 80:80`, `-p80:80`.
fn short_options(
    letters: &str,
    arguments: &mut Arguments<'_>,
    draft: &mut Draft,
) -> Result<(), Refusal> {
    let mut rest = letters.chars();
    while let Some(letter) = rest.next() {
        let option = format!("-{letter}");
        match options::short(letter) {
            None => draft.unknown(&option, true),
            Some(reads) if reads.takes_value() => {
                let joined = rest.as_str();
                let joined = joined.strip_prefix('=').unwrap_or(joined);
                let value = if joined.is_empty() {
                    arguments.value()?
                } else {
                    joined
                };
                draft.apply(reads, &option, value);
                return Ok(());
            }
            Some(reads) => draft.flag(reads, &option, None),
        }
    }
    Ok(())
}
