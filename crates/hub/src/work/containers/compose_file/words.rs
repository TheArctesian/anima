//! The words of a shell command line, for the `docker run` converter. It expands nothing and
//! runs nothing: it reads single quotes, double quotes, the backslash and the line
//! continuation, and it refuses each form that a shell would run or fill.
//!
//! Each word is text for a Compose file. A `$NAME` or `${NAME}` that a shell would fill stays
//! as it is, because Compose fills it when it starts the project. A `$` that the shell would
//! keep as a character becomes `$$`, which is how Compose writes that character.

use super::docker_run::{Refusal, RefusalCode};

/// One word, with the position of its first character in the command (from 1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Word {
    pub text: String,
    pub at: u32,
}

/// The words of a command line.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Split {
    pub words: Vec<Word>,
    /// A word holds a variable that Compose fills.
    pub variables: bool,
    /// A `$` that is a character was written as `$$`.
    pub dollars: bool,
}

struct Reader<'a> {
    characters: &'a [char],
    next: usize,
    word: Option<Word>,
    split: Split,
}

type Step = Result<(), Refusal>;

/// The words of `command`.
///
/// # Errors
/// [`Refusal`] with the fixed code of the first form that the converter does not read.
pub(super) fn split(command: &str) -> Result<Split, Refusal> {
    let characters: Vec<char> = command.trim_end().chars().collect();
    let mut reader = Reader {
        characters: &characters,
        next: 0,
        word: None,
        split: Split::default(),
    };
    while let Some(character) = reader.peek() {
        reader.outside(character)?;
    }
    reader.end_word();
    Ok(reader.split)
}

fn name_start(character: char) -> bool {
    character.is_ascii_alphabetic() || character == '_'
}

impl Reader<'_> {
    fn peek(&self) -> Option<char> {
        self.characters.get(self.next).copied()
    }

    fn position(&self) -> u32 {
        u32::try_from(self.next + 1).unwrap_or(u32::MAX)
    }

    fn refuse(&self, code: RefusalCode) -> Step {
        Err(Refusal {
            code,
            at: self.position(),
        })
    }

    /// The word that is open; a quote with nothing in it also opens a word.
    fn word(&mut self) -> &mut String {
        let at = self.position();
        let word = self.word.get_or_insert_with(|| Word {
            text: String::new(),
            at,
        });
        &mut word.text
    }

    fn end_word(&mut self) {
        self.split.words.extend(self.word.take());
    }

    /// Adds a character that is itself: a `$` is written as Compose writes it.
    fn keep(&mut self, character: char) {
        self.word().push(character);
        if character == '$' {
            self.word().push('$');
            self.split.dollars = true;
        }
        self.next += 1;
    }

    /// One step outside quotes.
    fn outside(&mut self, character: char) -> Step {
        let following = self.characters.get(self.next + 1).copied();
        match character {
            ' ' | '\t' | '\r' => {
                self.end_word();
                self.next += 1;
                Ok(())
            }
            '\'' => self.single(),
            '"' => self.double(),
            '\\' => self.escaped(false),
            '$' => self.dollar(false),
            '`' => self.refuse(RefusalCode::CommandSubstitution),
            '|' if following != Some('|') => self.refuse(RefusalCode::Pipe),
            '\n' | ';' | '&' | '|' => self.refuse(RefusalCode::MoreThanOneCommand),
            '<' | '>' => self.refuse(RefusalCode::Redirection),
            '(' | ')' => self.refuse(RefusalCode::ShellSyntax),
            '#' if self.word.is_none() => self.refuse(RefusalCode::ShellSyntax),
            other if other.is_control() => self.refuse(RefusalCode::ControlCharacter),
            other => {
                self.keep(other);
                Ok(())
            }
        }
    }

    /// A text in single quotes: each character is itself.
    fn single(&mut self) -> Step {
        let opened = self.next;
        self.word();
        self.next += 1;
        loop {
            match self.peek() {
                None => {
                    self.next = opened;
                    return self.refuse(RefusalCode::UnclosedQuote);
                }
                Some('\'') => {
                    self.next += 1;
                    return Ok(());
                }
                Some(other) if other.is_control() && !matches!(other, '\n' | '\t') => {
                    return self.refuse(RefusalCode::ControlCharacter);
                }
                Some(other) => self.keep(other),
            }
        }
    }

    /// A text in double quotes: the shell fills a variable and reads a backslash in it.
    fn double(&mut self) -> Step {
        let opened = self.next;
        self.word();
        self.next += 1;
        loop {
            match self.peek() {
                None => {
                    self.next = opened;
                    return self.refuse(RefusalCode::UnclosedQuote);
                }
                Some('"') => {
                    self.next += 1;
                    return Ok(());
                }
                Some('\\') => self.escaped(true)?,
                Some('$') => self.dollar(true)?,
                Some('`') => return self.refuse(RefusalCode::CommandSubstitution),
                Some(other) if other.is_control() && !matches!(other, '\n' | '\t') => {
                    return self.refuse(RefusalCode::ControlCharacter);
                }
                Some(other) => self.keep(other),
            }
        }
    }

    /// A backslash. Before a line break it joins two lines. In double quotes it is itself
    /// before each character that is not special there.
    fn escaped(&mut self, quoted: bool) -> Step {
        self.next += 1;
        if self.peek() == Some('\r') && self.characters.get(self.next + 1) == Some(&'\n') {
            self.next += 1;
        }
        match self.peek() {
            None => {}
            Some('\n') => self.next += 1,
            Some(special) if !quoted || matches!(special, '$' | '`' | '"' | '\\') => {
                if special.is_control() {
                    return self.refuse(RefusalCode::ControlCharacter);
                }
                self.keep(special);
            }
            Some(_) => self.word().push('\\'),
        }
        Ok(())
    }

    /// A `$`: a variable that stays for Compose, a form that the converter refuses, or the
    /// character itself.
    fn dollar(&mut self, quoted: bool) -> Step {
        match self.characters.get(self.next + 1).copied() {
            Some('(') => self.refuse(RefusalCode::CommandSubstitution),
            Some('{') => self.braced(),
            Some(start) if name_start(start) => {
                self.word().push('$');
                self.next += 1;
                while let Some(part) = self.peek().filter(|c| name_start(*c) || c.is_ascii_digit())
                {
                    self.word().push(part);
                    self.next += 1;
                }
                self.split.variables = true;
                Ok(())
            }
            Some('"') if quoted => {
                self.keep('$');
                Ok(())
            }
            Some(other) if other.is_ascii_digit() || "?$!@*#-'\"".contains(other) => {
                self.refuse(RefusalCode::VariableNotKept)
            }
            _ => {
                self.keep('$');
                Ok(())
            }
        }
    }

    /// `${NAME}`, or `${NAME` with a default or an error text that Compose also reads.
    fn braced(&mut self) -> Step {
        let rest = &self.characters[self.next + 2..];
        let Some(length) = rest.iter().position(|c| *c == '}') else {
            return self.refuse(RefusalCode::VariableNotKept);
        };
        let inside: String = rest[..length].iter().collect();
        let after = inside.trim_start_matches(|c: char| name_start(c) || c.is_ascii_digit());
        let named = inside.len() > after.len() && inside.starts_with(name_start);
        let operator = ["", ":-", "-", ":?", "?", ":+", "+"]
            .iter()
            .any(|operator| after.is_empty() == operator.is_empty() && after.starts_with(operator));
        let plain = |c: char| !c.is_control() && !matches!(c, '$' | '`' | '{' | '\\');
        if !named || !operator || !inside.chars().all(plain) {
            return self.refuse(RefusalCode::VariableNotKept);
        }
        let word = self.word();
        word.push_str("${");
        word.push_str(&inside);
        word.push('}');
        self.next += length + 3;
        self.split.variables = true;
        Ok(())
    }
}
