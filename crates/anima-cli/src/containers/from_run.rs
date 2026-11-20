//! `anima containers from-run -- docker run ...`: the Compose file of one `docker run` command
//! (HOST-022). The hub reads the words of the command: it expands nothing, runs nothing and
//! reaches no device. The file goes to standard output and the notes to standard error.

use std::path::Path;

use super::documents::{ComposeFromDockerRunRead, compose_from_docker_run_read};
use crate::failure::{Failure, Hint};
use crate::hub::Hub;
use crate::input;
use crate::invocation::Invocation;
use crate::output::Table;

/// The most bytes of a command: the limit of the hub.
pub const LARGEST_COMMAND: usize = 64 * 1024;

pub type Converted = compose_from_docker_run_read::ComposeFromDockerRunReadComposeFromDockerRun;
type Note = compose_from_docker_run_read::ComposeRunNote;
type Refusal = compose_from_docker_run_read::ComposeRunRefusal;

#[derive(Debug, clap::Args)]
pub struct FromRunArgs {
    /// The `docker run` command, after `--`. Your shell fills its variables and removes its
    /// quotes first. With no command here, it is read from standard input as you wrote it.
    #[arg(last = true, value_name = "COMMAND")]
    command: Vec<String>,
}

/// The answer of the hub for the command line `command`.
///
/// # Errors
/// The hub's refusal.
pub async fn fetch(hub: &Hub, command: String) -> Result<Converted, Failure> {
    let variables = compose_from_docker_run_read::Variables { command };
    Ok(hub
        .run::<ComposeFromDockerRunRead>(variables)
        .await?
        .compose_from_docker_run)
}

/// The words that a shell already read, as one command line: each in single quotes, so that
/// the hub reads each as the same word and fills nothing in it.
fn quoted(words: &[String]) -> String {
    let quote = |word: &String| format!("'{}'", word.replace('\'', "'\\''"));
    words.iter().map(quote).collect::<Vec<_>>().join(" ")
}

/// The place in `words` of the character `at` (from 1) of [`quoted`]`(words)`: the number of
/// the word and of the character in it, each from 1. A quote that the line adds is at the
/// nearest character of its word.
fn place_in_words(words: &[String], at: u32) -> Option<(usize, usize)> {
    let mut places = Vec::new();
    for (index, word) in words.iter().enumerate() {
        let number = index + 1;
        if index > 0 {
            places.push((number, 1));
        }
        places.push((number, 1));
        let mut last = 1;
        for (character, text) in word.chars().enumerate() {
            last = character + 1;
            let width = if text == '\'' { 4 } else { 1 };
            places.extend(std::iter::repeat_n((number, last), width));
        }
        places.push((number, last));
    }
    let index = usize::try_from(at).ok()?.checked_sub(1)?;
    places.get(index).or_else(|| places.last()).copied()
}

/// Why the hub did not convert the command, as a sentence.
#[must_use]
pub fn refusal_sentence(refusal: &Refusal) -> &'static str {
    match refusal {
        Refusal::TOO_LARGE => "The command is larger than 64 KiB.",
        Refusal::NO_COMMAND => "There is no command.",
        Refusal::NOT_DOCKER_RUN => "The command is not `docker run`.",
        Refusal::COMMAND_SUBSTITUTION => {
            "The command has `$(` or a backtick, which a shell runs as a second command. \
             Write the value in its place."
        }
        Refusal::PIPE => "The command has a pipe. Give only the `docker run` command.",
        Refusal::MORE_THAN_ONE_COMMAND => {
            "The text holds more than one command. Give only one `docker run` command; end a \
             line that continues with a backslash."
        }
        Refusal::REDIRECTION => "The command has a redirection (`<` or `>`). Remove it.",
        Refusal::SHELL_SYNTAX => "The command has round brackets or a comment. Remove them.",
        Refusal::VARIABLE_NOT_KEPT => {
            "The command has a variable that cannot stay in a Compose file. Only `$NAME`, \
             `${NAME}` and `${NAME:-text}` with no quote can."
        }
        Refusal::ARITHMETIC_EXPANSION => {
            "The command has `$[`, which a shell changes into the result of a sum. Write the \
             result in its place."
        }
        Refusal::BRACE_EXPANSION => {
            "The command has a list or a range in braces, such as `{80,443}`, which a shell \
             makes into more than one word. Write each word, or put the braces in quotes."
        }
        Refusal::TILDE_EXPANSION => {
            "The command has a `~`, which a shell changes into your home folder. Write the full \
             path."
        }
        Refusal::PATTERN => {
            "The command has `*`, `?` or `[` out of quotes, which a shell matches against the \
             names of files. Put the word in quotes."
        }
        Refusal::UNCLOSED_QUOTE => "A quote in the command is not closed.",
        Refusal::CONTROL_CHARACTER => "The command holds a control character.",
        Refusal::OPTION_WITHOUT_VALUE => "The last option of the command needs a value.",
        Refusal::NO_IMAGE => "The command names no image.",
        Refusal::TOO_MANY_NETWORKS => {
            "The command has more than 32 networks or more than 32 network aliases."
        }
        Refusal::FILE_TOO_LARGE => {
            "The Compose file would be larger than 256 KiB, the most that the check reads."
        }
        Refusal::Other(_) => "The hub did not convert the command.",
    }
}

/// What a note tells the operator, as a sentence.
#[must_use]
pub fn note_sentence(note: &Note) -> &'static str {
    match note {
        Note::DETACH_NOT_NEEDED => "`-d` is left out: Compose starts a service in the background.",
        Note::REMOVE_NOT_KEPT => {
            "`--rm` is left out: a Compose file has no key for it, and the container stays \
             until the project is removed."
        }
        Note::VARIABLES_KEPT => {
            "A `$NAME` stays as text: Compose fills it when it starts the project."
        }
        Note::DOLLAR_WRITTEN => "A `$` that is a character is written `$$`, as Compose needs it.",
        Note::EXTERNAL_NETWORK => {
            "A network of the command is `external`: it must exist before the project starts."
        }
        Note::VOLUME_NAME_KEPT => {
            "A volume keeps its name of Docker, so the project uses the same volume."
        }
        Note::UNKNOWN_OPTION_READ_AS_FLAG => {
            "An option that Docker does not have was read as an option with no value: make \
             sure that the image in the file is the right word."
        }
        Note::Other(_) => "The hub gave a note that this version does not know.",
    }
}

impl FromRunArgs {
    /// Prints the Compose file on standard output, and the notes and the options that were
    /// not converted on standard error; with `--json`, the K1 `ComposeFromDockerRun`.
    ///
    /// # Errors
    /// [`Failure::Input`] when standard input cannot be read or is larger than 64 KiB; the
    /// hub's refusal; [`Failure::Usage`] for a command that the hub did not convert.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let words = (!self.command.is_empty()).then_some(self.command.as_slice());
        let command = if self.command.is_empty() {
            let what = "the docker run command";
            let from = Path::new("-");
            let bytes = input::read(invocation.prompter, from, what, LARGEST_COMMAND)?;
            String::from_utf8(bytes.to_vec()).map_err(|_| Failure::Input {
                what: what.to_owned(),
                reason: "it is not UTF-8 text".to_owned(),
            })?
        } else {
            quoted(&self.command)
        };
        let converted = fetch(&invocation.hub()?, command).await?;
        if invocation.output.json() {
            invocation.output.data(&converted, |_| Table::record())?;
        }
        let Some(file) = &converted.compose_file else {
            return Err(refused(&converted, words));
        };
        if !invocation.output.json() {
            invocation.output.file(file)?;
        }
        for note in &converted.notes {
            invocation
                .output
                .note(&format!("note: {}", note_sentence(note)));
        }
        if !converted.not_converted.is_empty() {
            let options = converted.not_converted.join(", ");
            invocation
                .output
                .note(&format!("Not converted, add by hand if needed: {options}"));
        }
        Ok(())
    }
}

/// The failure of a command that the hub did not convert. The place is in the words that the
/// operator gave after `--`, or in the line read from standard input.
fn refused(converted: &Converted, words: Option<&[String]>) -> Failure {
    let sentence = converted
        .refusal
        .as_ref()
        .map_or("The hub did not convert the command.", refusal_sentence);
    let at = converted.refused_at.and_then(|at| u32::try_from(at).ok());
    let place = match (at, words) {
        (Some(at), Some(words)) => place_in_words(words, at)
            .map_or_else(String::new, |(word, character)| {
                format!(" (word {word}, character {character})")
            }),
        (Some(at), None) => format!(" (character {at})"),
        (None, _) => String::new(),
    };
    Failure::Usage {
        message: format!("{sentence}{place}"),
        hint: Some(Hint::run("anima containers from-run --help")),
    }
}

#[cfg(test)]
mod tests {
    use super::{place_in_words, quoted};

    #[test]
    fn host_022_each_word_goes_to_the_hub_in_single_quotes() {
        let words = ["docker", "run", "-e", "A=it's $5", "nginx"].map(str::to_owned);
        assert_eq!(
            quoted(&words),
            r"'docker' 'run' '-e' 'A=it'\''s $5' 'nginx'"
        );
    }

    #[test]
    fn host_022_a_place_in_the_quoted_line_is_a_place_in_the_words() {
        let words = ["docker", "run", "-e", "A=it's", "ng|nx"].map(str::to_owned);
        let line = quoted(&words);
        let at = |text: &str| u32::try_from(line.find(text).unwrap() + 1).unwrap();
        assert_eq!(place_in_words(&words, 1), Some((1, 1)));
        assert_eq!(place_in_words(&words, at("run")), Some((2, 1)));
        assert_eq!(place_in_words(&words, at("s'")), Some((4, 6)));
        assert_eq!(place_in_words(&words, at("|")), Some((5, 3)));
        let end = u32::try_from(line.chars().count()).unwrap();
        assert_eq!(place_in_words(&words, end), Some((5, 5)));
        assert_eq!(place_in_words(&words, end + 10), Some((5, 5)));
    }
}
